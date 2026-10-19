//! Automation (contract `ai_context/AUTOMATION.md`): flows built by hand, by the assistant
//! (the user's own LLM key) or by an MCP client, all checked by one validator and run by one
//! executor. Routes under `/automation`.

pub mod assist;
pub mod exec;
pub mod flow;
pub mod store;
pub mod vocab;

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Map, Value};

use crate::devices::Devices;
use crate::recordings::Recordings;
use crate::{detail, S};
use exec::{Runner, StartError};
use store::{key_hint, Flows, Settings, SettingsFile, PROVIDERS};
use vocab::Vocab;

pub struct Automation {
    pub devices: Arc<Devices>,
    pub runner: Arc<Runner>,
    pub flows: Flows,
    pub settings: SettingsFile,
}

impl Automation {
    /// `data_dir`: where flows and settings persist (`None`: in memory).
    pub fn new(devices: Arc<Devices>, recordings: Arc<Recordings>, data_dir: Option<PathBuf>) -> Arc<Automation> {
        Arc::new(Automation {
            runner: Runner::new(devices.clone(), recordings),
            flows: Flows::new(data_dir.as_ref().map(|d| d.join("flows"))),
            settings: SettingsFile::open(data_dir.map(|d| d.join("automation.json"))),
            devices,
        })
    }

    pub fn vocab(&self) -> Vocab {
        Vocab::from_devices(&self.devices)
    }

    /// `{ok, issues, energizes}` for a document.
    pub fn validate(&self, doc: &Value) -> Value {
        let (_, issues, energize) = flow::check(doc, &self.vocab());
        json!({"ok": !flow::has_errors(&issues), "issues": issues, "energizes": energize})
    }

    /// Save a flow (new when `id` is None). Errors: the issues (nothing saved).
    pub fn save(&self, id: Option<&str>, doc: &Value) -> Result<(String, Vec<flow::Issue>), (StatusCode, Value)> {
        let (_, issues, _) = flow::check(doc, &self.vocab());
        // offline instruments are warnings: a flow may be written before the bench is wired
        if flow::has_errors(&issues) {
            return Err((StatusCode::UNPROCESSABLE_ENTITY, json!({"detail": "the flow has errors", "issues": issues})));
        }
        let id = match id {
            Some(i) => i.to_string(),
            None => self.flows.new_id(doc["name"].as_str().unwrap_or("flow")),
        };
        self.flows.put(&id, doc).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, json!({"detail": e})))?;
        Ok((id, issues))
    }

    /// Start a run from `{flow_id | flow, params?, arm?}`; the run status, or an error body.
    pub fn start(&self, b: &Map<String, Value>) -> Result<Value, (StatusCode, Value)> {
        let (doc, flow_id) = match (b.get("flow_id").and_then(|v| v.as_str()), b.get("flow")) {
            (Some(id), None) => (self.flows.get(id).ok_or((StatusCode::NOT_FOUND, json!({"detail": format!("no flow {id:?}")})))?, Some(id.to_string())),
            (None, Some(f)) => (f.clone(), None),
            _ => return Err((StatusCode::UNPROCESSABLE_ENTITY, json!({"detail": "give `flow_id` or `flow`"}))),
        };
        let params = match b.get("params") {
            None | Some(Value::Null) => None,
            Some(Value::Object(m)) => Some(m),
            Some(_) => return Err((StatusCode::UNPROCESSABLE_ENTITY, json!({"detail": "`params` must be an object"}))),
        };
        let arm = b.get("arm").and_then(|v| v.as_bool()).unwrap_or(false);
        match self.runner.start(&doc, flow_id, params, arm) {
            Ok(run) => Ok(run.json(0)),
            Err(StartError::Busy) => Err((StatusCode::CONFLICT, json!({"detail": "a flow is already running — abort it first"}))),
            Err(StartError::Invalid(issues)) => Err((StatusCode::UNPROCESSABLE_ENTITY, json!({"detail": "the flow has errors", "issues": issues}))),
            Err(StartError::Params(m)) => Err((StatusCode::UNPROCESSABLE_ENTITY, json!({"detail": m}))),
            Err(StartError::NeedsArm(list)) => Err((
                StatusCode::CONFLICT,
                json!({"detail": "this flow can switch outputs on: start it with \"arm\": true after the user agreed", "energizes": list}),
            )),
        }
    }

    pub fn run_status(&self, after: u64) -> Value {
        match self.runner.current() {
            Some(r) => r.json(after),
            None => json!({"state": "idle", "events": []}),
        }
    }

    /// Assistant settings for the UI: keys only as hints.
    pub fn assistant_json(&self) -> Value {
        let s = self.settings.get();
        let keys: Map<String, Value> = PROVIDERS.iter().map(|p| (p.to_string(), s.keys.get(*p).map(|k| json!(key_hint(k))).unwrap_or(Value::Null))).collect();
        json!({"provider": s.provider, "model": assist::model(&s), "models": s.models, "base_url": s.base_urls.get(&s.provider).cloned().unwrap_or_default(), "keys": keys, "providers": PROVIDERS})
    }

    pub fn mcp_access(&self) -> String {
        self.settings.get().mcp
    }
}

fn reply(r: Result<Value, (StatusCode, Value)>, ok: StatusCode) -> Response {
    match r {
        Ok(v) => (ok, Json(v)).into_response(),
        Err((code, body)) => (code, Json(body)).into_response(),
    }
}

/// The body as a JSON object, or the 422 to answer.
fn object(body: &Bytes) -> Result<Map<String, Value>, Box<Response>> {
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(m)) => Ok(m),
        Ok(_) => Err(Box::new(detail(StatusCode::UNPROCESSABLE_ENTITY, "body must be a JSON object"))),
        Err(e) => Err(Box::new(detail(StatusCode::UNPROCESSABLE_ENTITY, format!("invalid JSON: {e}")))),
    }
}

pub fn routes() -> Router<S> {
    Router::new()
        .route("/automation/vocabulary", get(vocabulary))
        .route("/automation/flows", get(list_flows).post(create_flow))
        .route("/automation/flows/{id}", get(get_flow).put(put_flow).delete(delete_flow))
        .route("/automation/validate", post(validate))
        .route("/automation/run", get(run_status).post(run))
        .route("/automation/run/abort", post(abort))
        .route("/automation/run/continue", post(resume))
        .route("/automation/assistant", get(assistant).put(put_assistant))
        .route("/automation/assistant/models", get(models))
        .route("/automation/assist", post(assist_flow))
        .route("/automation/mcp", get(mcp).put(put_mcp))
}

async fn vocabulary(State(s): State<S>) -> Json<Value> {
    Json(json!(s.automation.vocab()))
}

async fn list_flows(State(s): State<S>) -> Json<Value> {
    Json(json!(s.automation.flows.list()))
}

async fn create_flow(State(s): State<S>, body: Bytes) -> Response {
    let b = match object(&body) {
        Ok(b) => Value::Object(b),
        Err(r) => return *r,
    };
    reply(s.automation.save(None, &b).map(|(id, issues)| json!({"id": id, "flow": b, "issues": issues})), StatusCode::CREATED)
}

async fn get_flow(State(s): State<S>, Path(id): Path<String>) -> Response {
    match s.automation.flows.get(&id) {
        Some(f) => Json(f).into_response(),
        None => detail(StatusCode::NOT_FOUND, format!("no flow {id:?}")),
    }
}

async fn put_flow(State(s): State<S>, Path(id): Path<String>, body: Bytes) -> Response {
    if !store::valid_id(&id) {
        return detail(StatusCode::NOT_FOUND, format!("no flow {id:?}"));
    }
    let b = match object(&body) {
        Ok(b) => Value::Object(b),
        Err(r) => return *r,
    };
    reply(s.automation.save(Some(&id), &b).map(|(id, issues)| json!({"id": id, "flow": b, "issues": issues})), StatusCode::OK)
}

async fn delete_flow(State(s): State<S>, Path(id): Path<String>) -> Response {
    if s.automation.flows.delete(&id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        detail(StatusCode::NOT_FOUND, format!("no flow {id:?}"))
    }
}

async fn validate(State(s): State<S>, body: Bytes) -> Response {
    match serde_json::from_slice::<Value>(&body) {
        Ok(doc) => Json(s.automation.validate(&doc)).into_response(),
        Err(e) => detail(StatusCode::UNPROCESSABLE_ENTITY, format!("invalid JSON: {e}")),
    }
}

async fn run(State(s): State<S>, body: Bytes) -> Response {
    match object(&body) {
        Ok(b) => reply(s.automation.start(&b), StatusCode::ACCEPTED),
        Err(r) => *r,
    }
}

async fn run_status(State(s): State<S>, Query(q): Query<std::collections::HashMap<String, String>>) -> Response {
    let after = match q.get("after").map(|v| v.parse::<u64>()) {
        None => 0,
        Some(Ok(n)) => n,
        Some(Err(_)) => return detail(StatusCode::UNPROCESSABLE_ENTITY, "`after` must be a sequence number"),
    };
    Json(s.automation.run_status(after)).into_response()
}

async fn abort(State(s): State<S>) -> Response {
    if s.automation.runner.abort() { StatusCode::NO_CONTENT.into_response() } else { detail(StatusCode::CONFLICT, "no flow is running") }
}

async fn resume(State(s): State<S>) -> Response {
    if s.automation.runner.resume() { StatusCode::NO_CONTENT.into_response() } else { detail(StatusCode::CONFLICT, "no flow is waiting") }
}

async fn assistant(State(s): State<S>) -> Json<Value> {
    Json(s.automation.assistant_json())
}

/// `{provider?, model?, base_url?, api_key?}` — the key is stored for the (new) provider;
/// `""` removes it.
async fn put_assistant(State(s): State<S>, body: Bytes) -> Response {
    let b = match object(&body) {
        Ok(b) => b,
        Err(r) => return *r,
    };
    let text = |k: &str| -> Result<Option<String>, String> {
        match b.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(v)) => Ok(Some(v.trim().to_string())),
            Some(_) => Err(format!("`{k}` must be a string")),
        }
    };
    let fields = (|| Ok::<_, String>((text("provider")?, text("model")?, text("base_url")?, text("api_key")?)))();
    let (provider, model, base_url, key) = match fields {
        Ok(f) => f,
        Err(m) => return detail(StatusCode::UNPROCESSABLE_ENTITY, m),
    };
    if let Some(p) = &provider {
        if !PROVIDERS.contains(&p.as_str()) {
            return detail(StatusCode::UNPROCESSABLE_ENTITY, format!("`provider` must be one of {}", PROVIDERS.join(", ")));
        }
    }
    if let Some(u) = &base_url {
        if !u.is_empty() && !(u.starts_with("http://") || u.starts_with("https://")) {
            return detail(StatusCode::UNPROCESSABLE_ENTITY, "`base_url` must start with http:// or https://");
        }
    }
    let r = s.automation.settings.update(|x: &mut Settings| {
        if let Some(p) = provider {
            x.provider = p;
        }
        let p = x.provider.clone();
        if let Some(m) = model {
            x.models.insert(p.clone(), m);
        }
        if let Some(u) = base_url {
            x.base_urls.insert(p.clone(), u);
        }
        match key {
            Some(k) if k.is_empty() => {
                x.keys.remove(&p);
            }
            Some(k) => {
                x.keys.insert(p, k);
            }
            None => {}
        }
    });
    match r {
        Ok(_) => Json(s.automation.assistant_json()).into_response(),
        Err(e) => detail(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

async fn models(State(s): State<S>) -> Response {
    match assist::models(&s.automation.settings.get()).await {
        Ok(m) => Json(json!(m)).into_response(),
        Err(e) => detail(StatusCode::BAD_GATEWAY, e),
    }
}

async fn assist_flow(State(s): State<S>, body: Bytes) -> Response {
    let b = match object(&body) {
        Ok(b) => b,
        Err(r) => return *r,
    };
    let Some(instruction) = b.get("instruction").and_then(|v| v.as_str()).map(str::trim).filter(|i| !i.is_empty()) else {
        return detail(StatusCode::UNPROCESSABLE_ENTITY, "`instruction` (what the automation should do) is required");
    };
    if instruction.chars().count() > 8000 {
        return detail(StatusCode::UNPROCESSABLE_ENTITY, "`instruction` is limited to 8000 characters");
    }
    let current = b.get("flow").filter(|f| f.is_object());
    let settings = s.automation.settings.get();
    match assist::assist(&settings, &s.automation.vocab(), instruction, current).await {
        Ok(r) => Json(json!({"flow": r.flow, "explanation": r.explanation, "issues": r.issues, "attempts": r.attempts})).into_response(),
        Err(e) => {
            eprintln!("assistant: {e}");
            detail(StatusCode::BAD_GATEWAY, e)
        }
    }
}

/// MCP access plus what an AI client needs to connect: the HTTP endpoint (as this request
/// reached the server) and the stdio bridge command (this executable).
async fn mcp(State(s): State<S>, headers: axum::http::HeaderMap) -> Json<Value> {
    let host = headers.get(axum::http::header::HOST).and_then(|h| h.to_str().ok()).unwrap_or("127.0.0.1:57666");
    // an AppImage runs from a temporary mount: the AppImage file is the stable command
    let exe = std::env::var_os("APPIMAGE").map(PathBuf::from).or_else(|| std::env::current_exe().ok());
    let args = match exe.as_ref().and_then(|p| p.file_stem()).and_then(|n| n.to_str()) {
        Some(n) if n.starts_with("benchmesh-server") => json!(["mcp"]),
        _ => json!(["--mcp"]),
    };
    Json(json!({
        "access": s.automation.mcp_access(),
        "endpoint": format!("http://{host}/mcp"),
        "command": exe.map(|p| p.display().to_string()),
        "args": args,
        // the bridge assumes the default port; another port must be passed on
        "env": if host.ends_with(":57666") { Value::Null } else { json!({"BENCHMESH_URL": format!("http://{host}")}) },
    }))
}

async fn put_mcp(State(s): State<S>, body: Bytes) -> Response {
    let b = match object(&body) {
        Ok(b) => b,
        Err(r) => return *r,
    };
    let Some(access) = b.get("access").and_then(|v| v.as_str()).filter(|a| ["off", "read", "control"].contains(a)).map(str::to_string) else {
        return detail(StatusCode::UNPROCESSABLE_ENTITY, "`access` must be off, read or control");
    };
    match s.automation.settings.update(|x| x.mcp = access) {
        Ok(x) => Json(json!({"access": x.mcp})).into_response(),
        Err(e) => detail(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}
