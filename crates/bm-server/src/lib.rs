//! BenchMesh API server. Paths and JSON shapes match the former Python service so the
//! React UI, Node-RED nodes and scripts keep working (see `ai_context/migration/`):
//!
//! - `GET  /instruments` (ETag / 304), `/instruments/{CLS}`, `/instruments/{CLS}/{id}` (features),
//!   `/instruments/{CLS}/{id}/methods`
//! - `GET  /instruments/{CLS}/{id}/{ch}/{op}` → `{"path","value"}`;
//!   `POST /instruments/{CLS}/{id}/{ch}/{op}/{value}` → 204
//! - `GET  /drivers`, `/drivers/{id}`, `/config`, `POST /config`, `POST /devices`,
//!   `PUT|DELETE /devices/{id}`, `POST /devices/detect`, `/serial-ports`, `/usbtmc-devices`, `/status`, `/version`
//! - `WS   /ws/registry` → `{dev: {IDN, CLS: {status_chN: {...}, updated_chN: ms}}}` every interval
//! - `/ui` static UI (`/docs` opens its API reference)
//!
//! Errors are `{"detail": "..."}`. Driver errors map to HTTP: bad argument 400,
//! unknown operation 404, refused precondition 409, device unreachable 503,
//! device timeout 504, malformed device reply 502.

pub mod automation;
pub mod community;
pub mod config;
pub mod detect;
pub mod ai;
pub mod devices;
pub mod guard;
pub mod mcp;
pub mod openapi;
pub mod opspec;
pub mod recordings;
pub mod remote;
pub mod run;

/// For the desktop app's `main`: the elevated Windows driver-install run (see `bm_transport::winusb`).
pub use bm_transport::winusb::helper_main as winusb_helper_main;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{OriginalUri, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use bm_devicedef::Repr;
use bm_driver::engine::Output;
use serde_json::{json, Map, Value};
use tower::ServiceBuilder;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::services::ServeDir;

use crate::config::{str_field, validate_entry, ConfigFile, DeviceEntry};
use crate::devices::{merge, Devices};

pub struct AppState {
    pub devices: Arc<Devices>,
    pub recordings: Arc<recordings::Recordings>,
    pub config: ConfigFile,
    pub ui_dir: Option<PathBuf>,
    pub ws_interval: Duration,
    /// `/ws/metrics` push interval (the UI says "Updates every 30s").
    pub metrics_interval: Duration,
    /// Serialises config changes (file + running devices).
    pub config_lock: tokio::sync::Mutex<()>,
    /// Community status of the models, from the website (`community.rs`).
    pub community: Arc<community::Community>,
    /// Flows, runs, the assistant and MCP settings (`automation/`).
    pub automation: Arc<automation::Automation>,
    /// Remote access from the local network (`remote/`).
    pub remote: Arc<remote::Remote>,
}

pub(crate) type S = Arc<AppState>;

/// The API + UI for the loopback listener: requests are `Principal::Local`; other web sites
/// are refused (guard.rs).
pub fn router(state: S) -> Router {
    routes(&state).layer(axum::middleware::from_fn(guard::guard)).with_state(state)
}

/// The same API + UI for the network listener of remote access: every request passes the
/// remote gate (session, scopes; `remote/mod.rs`).
pub fn network_router(state: S) -> Router {
    let gate = axum::middleware::from_fn_with_state(state.remote.clone(), remote::gate);
    routes(&state).layer(gate).with_state(state)
}

fn routes(state: &S) -> Router<S> {
    let mut r = Router::new()
        .route("/", get(root))
        .route("/status", get(status))
        .route("/version", get(version))
        .route("/instruments", get(instruments))
        .route("/instruments/{klass}", get(instruments_of_class))
        .route("/instruments/{klass}/{id}", get(features))
        .route("/instruments/{klass}/{id}/methods", get(methods))
        .route("/instruments/{klass}/{id}/{ch}/{op}", get(op_get))
        .route("/instruments/{klass}/{id}/{ch}/{op}/{value}", post(op_set))
        .route("/drivers", get(drivers))
        .route("/drivers/{id}", get(driver_models))
        .route("/catalog", get(catalog))
        .route("/config", get(get_config).post(post_config))
        .route("/devices", post(add_device))
        .route("/devices/{id}", put(update_device).delete(delete_device))
        .route("/devices/detect", post(detect_devices).get(detect_status))
        .route("/devices/{id}/wire", get(wire_log).delete(clear_wire_log))
        .route("/serial-ports", get(serial_ports))
        .route("/usbtmc-devices", get(usbtmc_devices))
        .route("/usbtmc-devices/driver", post(install_usbtmc_driver))
        .route("/ws/registry", get(ws_registry))
        .route("/metrics", get(metrics_summary))
        .route("/metrics/{id}", get(metrics_device))
        .route("/ws/metrics", get(ws_metrics))
        .route("/ai/context", get(ai::context))
        .route("/ai/system-prompt", get(ai::context))
        .route("/openapi.json", get(openapi::json))
        .route("/openapi.yaml", get(openapi::yaml))
        .route("/docs", get(api_docs))
        .route("/redoc", get(api_docs))
        .route("/mcp", get(mcp::get).post(mcp::post))
        .merge(recordings::routes())
        .merge(automation::routes())
        .merge(remote::routes());
    if let Some(ui) = &state.ui_dir {
        // Always revalidate UI files (304 when unchanged); otherwise browsers keep serving a
        // stale index.html after an upgrade.
        let no_cache = || SetResponseHeaderLayer::overriding(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        r = r.nest_service("/ui", ServiceBuilder::new().layer(no_cache()).service(ServeDir::new(ui).append_index_html_on_directories(true)));
    }
    // no CORS: only pages served by this server use the API
    r
}

// ---- helpers ------------------------------------------------------------------------

pub(crate) fn detail(code: StatusCode, msg: impl Into<String>) -> Response {
    (code, Json(json!({"detail": msg.into()}))).into_response()
}

fn driver_error(e: bm_driver::Error) -> Response {
    use bm_driver::Error as E;
    let code = match &e {
        E::BadArgument(_) => StatusCode::BAD_REQUEST,
        E::NotFound(_) => StatusCode::NOT_FOUND,
        E::Refused(_) => StatusCode::CONFLICT,
        E::Io(_) | E::UnknownModel(_) | E::WrongInstrument(_) => StatusCode::SERVICE_UNAVAILABLE,
        E::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
        E::Framing(_) | E::Parse(_) => StatusCode::BAD_GATEWAY,
        E::Expr(_) | E::Definition(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let msg = match e {
        E::Refused(m) | E::BadArgument(m) | E::NotFound(m) => m,
        other => other.to_string(),
    };
    detail(code, msg)
}

fn etag_of(v: &Value) -> String {
    let mut h = DefaultHasher::new();
    v.to_string().hash(&mut h);
    format!("\"{:016x}\"", h.finish())
}

fn with_etag(v: Value, headers: &HeaderMap) -> Response {
    let tag = etag_of(&v);
    if headers.get(header::IF_NONE_MATCH).and_then(|h| h.to_str().ok()) == Some(tag.as_str()) {
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, tag)]).into_response();
    }
    ([(header::ETAG, tag)], Json(v)).into_response()
}

fn valid_class(k: &str) -> bool {
    k.len() == 3 && k.chars().all(|c| c.is_ascii_uppercase())
}

// ---- system -------------------------------------------------------------------------

async fn root(State(s): State<S>) -> Response {
    if s.ui_dir.is_some() {
        Redirect::temporary("/ui/").into_response()
    } else {
        Json(json!({"name": "BenchMesh", "ui": null})).into_response()
    }
}

async fn status(State(s): State<S>) -> Json<Value> {
    let r = s.devices.registry.lock().unwrap();
    let total = s.devices.managed.lock().unwrap().len();
    let connected = r.devices.values().filter(|d| d.connected).count();
    Json(json!({"devices_total": total, "connected": connected, "disconnected": total - connected.min(total)}))
}

async fn version() -> Json<Value> {
    let v = env!("CARGO_PKG_VERSION");
    let release = option_env!("BENCHMESH_RELEASE").unwrap_or(v);
    Json(json!({
        "releaseVersion": release, "applicationName": "BenchMesh", "displayVersion": format!("BenchMesh {release}"),
        "version": v, "description": "Lab Instrument Control System", "name": "BenchMesh", "error": null
    }))
}

// ---- instruments --------------------------------------------------------------------

fn instrument_list(s: &AppState, class_filter: Option<&str>) -> Vec<Value> {
    let entries = s.devices.entries();
    let reg = s.devices.registry.lock().unwrap().devices.clone();
    let mut out = Vec::new();
    for e in entries {
        let id = str_field(&e, "id").unwrap_or_default().to_string();
        let Some((def, _, entry)) = s.devices.lookup(&id) else { continue };
        let model = s.devices.model_of(&id, &def, &entry);
        let mut item = Map::new();
        item.insert("id".into(), json!(id));
        if let Some(n) = str_field(&e, "name") {
            item.insert("name".into(), json!(n));
        }
        let st = reg.get(&id).cloned().unwrap_or_default();
        if let (true, Some(idn)) = (st.connected, &st.idn) {
            item.insert("IDN".into(), json!(idn));
        }
        if let (true, Some(port)) = (st.connected, &st.port) {
            item.insert("port".into(), json!(port));
            item.insert("transport".into(), json!(def.link.kinds().first().copied().unwrap_or_default()));
        }
        let health = match (st.connected, st.consecutive_failures) {
            (true, _) => "healthy",
            (false, 0) => "unknown",
            (false, _) => "dead",
        };
        let mut h = json!({"status": health, "consecutive_failures": st.consecutive_failures, "is_alive": st.connected});
        if let Some(err) = &st.last_error {
            h["last_error"] = json!(err);
        }
        item.insert("health".into(), h);
        if let Some(m) = &model {
            item.insert("model".into(), json!(m.name));
            item.insert("support".into(), json!(m.support.as_str()));
        }
        let classes: Vec<Value> = def
            .classes
            .iter()
            .filter(|(c, _)| class_filter.is_none_or(|f| f == c.as_str()))
            .map(|(c, b)| {
                let n = model.as_ref().and_then(|m| m.channels.get(c).copied()).unwrap_or(b.channels);
                json!({"class": c, "channels": (1..=n).map(|ch| format!("/instruments/{c}/{id}/{ch}")).collect::<Vec<_>>(),
                       "ui_component": b.ui_component})
            })
            .collect();
        if class_filter.is_some() && classes.is_empty() {
            continue;
        }
        item.insert("classes".into(), json!(classes));
        out.push(Value::Object(item));
    }
    out
}

async fn instruments(State(s): State<S>, headers: HeaderMap) -> Response {
    with_etag(json!(instrument_list(&s, None)), &headers)
}

async fn instruments_of_class(State(s): State<S>, Path(klass): Path<String>, headers: HeaderMap) -> Response {
    if !valid_class(&klass) {
        return detail(StatusCode::NOT_FOUND, format!("Invalid instrument class: {klass}"));
    }
    let list = instrument_list(&s, Some(&klass));
    if list.is_empty() {
        return detail(StatusCode::NOT_FOUND, format!("No instruments found with class: {klass}"));
    }
    with_etag(json!(list), &headers)
}

async fn features(State(s): State<S>, Path((klass, id)): Path<(String, String)>) -> Response {
    let Some((def, _, entry)) = s.devices.lookup(&id) else { return detail(StatusCode::NOT_FOUND, "Unknown device id") };
    let Some(b) = def.classes.get(&klass) else { return detail(StatusCode::NOT_FOUND, "Invalid instrument class") };
    let mut f = if b.features.is_null() { json!({}) } else { b.features.clone() };
    let model = s.devices.model_of(&id, &def, &entry);
    if let Some(over) = model.as_ref().and_then(|m| m.features.get(&klass)) {
        merge(&mut f, over);
    }
    let channels = model.as_ref().and_then(|m| m.channels.get(&klass).copied()).unwrap_or(b.channels);
    f["channels"] = json!(channels);
    Json(f).into_response()
}

async fn methods(State(s): State<S>, Path((klass, id)): Path<(String, String)>) -> Response {
    let Some((def, _, _)) = s.devices.lookup(&id) else { return detail(StatusCode::NOT_FOUND, "Unknown device id") };
    let Some(b) = def.classes.get(&klass) else { return detail(StatusCode::NOT_FOUND, "Invalid instrument class") };
    let channel = json!({"name": "channel", "type": "int", "required": true, "default": null, "description": "Channel number"});
    let mut list = Vec::new();
    for (op, o) in &b.operations {
        let desc = o.description.clone().unwrap_or_else(|| op.replace('_', " "));
        if o.get.is_some() {
            let returns = match o.repr {
                Repr::Si => "si",
                Repr::Bytes => "bytes",
                Repr::Raw => "any",
            };
            list.push(json!({"name": op, "full_name": format!("query_{op}"), "http_method": "GET",
                "description": format!("Query {desc}"), "parameters": [channel.clone()], "returns": returns,
                "example": format!("GET /instruments/{klass}/{id}/1/{op}")}));
        }
        if let Some((spec, energizes)) = opspec::setter(&def, o) {
            // Parameter type (Python-style names) from the value spec of the setter's caller param.
            let kind = match spec.as_ref().and_then(|v| v["type"].as_str()) {
                Some("number") => "float",
                Some("integer") => "int",
                Some("bool") => "bool",
                _ => "str",
            };
            let params = vec![channel.clone(), json!({"name": "value", "type": kind, "required": true, "default": null,
                               "description": "Value to set"})];
            let mut m = json!({"name": op, "full_name": format!("set_{op}"), "http_method": "POST",
                "description": format!("Set {desc}"), "parameters": params, "returns": "none",
                "example": format!("POST /instruments/{klass}/{id}/1/{op}/<value>")});
            if let Some(spec) = spec {
                m["value"] = spec;
            }
            if energizes {
                m["energizes"] = json!(true);
            }
            list.push(m);
        }
    }
    list.sort_by_key(|m| (m["http_method"] != "GET", m["name"].as_str().unwrap_or_default().to_string()));
    Json(json!({"device_id": id, "class": klass, "methods": list})).into_response()
}

/// Error responses are boxed: `Response` is large.
type Rejection = Box<Response>;

fn parse_target(s: &AppState, klass: &str, id: &str, ch: &str) -> Result<(bm_driver::actor::DeviceClient, u8), Rejection> {
    if !valid_class(klass) {
        return Err(Box::new(detail(StatusCode::NOT_FOUND, "Invalid instrument class")));
    }
    let ch: u8 = match ch.parse() {
        Ok(c @ 1..=9) => c,
        _ => return Err(Box::new(detail(StatusCode::NOT_FOUND, "Invalid channel"))),
    };
    let (def, client, _) = s.devices.lookup(id).ok_or_else(|| Box::new(detail(StatusCode::NOT_FOUND, "Unknown device id")))?;
    if !def.classes.contains_key(klass) {
        return Err(Box::new(detail(StatusCode::NOT_FOUND, format!("Device {id} has no class {klass}"))));
    }
    Ok((client, ch))
}

async fn op_get(State(s): State<S>, Path((klass, id, ch, op)): Path<(String, String, String, String)>, uri: OriginalUri) -> Response {
    let (client, ch) = match parse_target(&s, &klass, &id, &ch) {
        Ok(x) => x,
        Err(r) => return *r,
    };
    match tokio::task::spawn_blocking(move || client.get(&klass, &op, ch)).await {
        Ok(Ok(Output::Value(v))) => Json(json!({"path": uri.0.path(), "value": v})).into_response(),
        Ok(Ok(Output::Bytes(b))) => ([(header::CONTENT_TYPE, "application/octet-stream")], b).into_response(),
        Ok(Err(e)) => driver_error(e),
        Err(e) => detail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn op_set(State(s): State<S>, Path((klass, id, ch, op, value)): Path<(String, String, String, String, String)>) -> Response {
    let (client, ch) = match parse_target(&s, &klass, &id, &ch) {
        Ok(x) => x,
        Err(r) => return *r,
    };
    match tokio::task::spawn_blocking(move || client.set(&klass, &op, ch, &value)).await {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e)) => driver_error(e),
        Err(e) => detail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// ---- drivers & config ---------------------------------------------------------------

async fn drivers(State(s): State<S>) -> Json<Value> {
    let m: Map<String, Value> = s
        .devices
        .catalog
        .devices
        .values()
        .filter(|(_, d)| d.enabled)
        .map(|(_, d)| {
            (d.id.clone(), json!({"vendor": d.vendor, "family": d.family, "supported_transports": d.link.kinds()}))
        })
        .collect();
    Json(Value::Object(m))
}

/// Every instrument the definitions describe, by family: classes, links and each model's
/// verification status (tested / confirmed / awaiting). The list the online service will
/// mirror, and what the "Supported instruments" view shows.
async fn catalog(State(s): State<S>) -> Json<Value> {
    let families: Vec<Value> = s
        .devices
        .catalog
        .devices
        .values()
        .filter(|(_, d)| d.enabled)
        .map(|(_, d)| {
            let models: Vec<Value> = d
                .models
                .iter()
                .map(|m| {
                    let (community, support) = s.community.annotate(&d.id, &m.name, m.support.as_str());
                    json!({"name": m.name, "support": support, "note": m.note, "channels": m.channels, "community": community})
                })
                .collect();
            let docs: Vec<&String> = d.sources.iter().filter_map(|src| src.doc.as_ref()).collect();
            json!({"driver": d.id, "vendor": d.vendor, "family": d.family, "classes": d.classes.keys().collect::<Vec<_>>(),
                   "transports": d.link.kinds(), "models": models, "manuals": docs})
        })
        .collect();
    let mut counts = Map::new();
    for f in &families {
        for m in f["models"].as_array().into_iter().flatten() {
            let k = m["support"].as_str().unwrap_or("awaiting").to_string();
            let n = counts.get(&k).and_then(|v| v.as_u64()).unwrap_or(0) + 1;
            counts.insert(k, json!(n));
        }
    }
    Json(json!({"families": families, "models": counts, "community": s.community.summary()}))
}

async fn driver_models(State(s): State<S>, Path(id): Path<String>) -> Response {
    match s.devices.catalog.devices.get(&id) {
        Some((_, d)) => Json(json!(d.models.iter().map(|m| m.name.clone()).collect::<Vec<_>>())).into_response(),
        None => detail(StatusCode::NOT_FOUND, format!("Driver '{id}' not found")),
    }
}

async fn get_config(State(s): State<S>) -> Json<Value> {
    Json(json!({"devices": s.devices.entries()}))
}

fn entry_from(v: Value) -> Result<DeviceEntry, Rejection> {
    v.as_object().cloned().ok_or_else(|| Box::new(detail(StatusCode::UNPROCESSABLE_ENTITY, "device must be a JSON object")))
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.expect("blocking task panicked")
}

async fn post_config(State(s): State<S>, Json(body): Json<Value>) -> Response {
    let _g = s.config_lock.lock().await;
    let Some(list) = body.get("devices").and_then(|d| d.as_array()) else {
        return detail(StatusCode::UNPROCESSABLE_ENTITY, "body must be {\"devices\": [...]}");
    };
    let mut entries = Vec::new();
    for v in list {
        let e = match entry_from(v.clone()) {
            Ok(e) => e,
            Err(r) => return *r,
        };
        if let Err(m) = validate_entry(&e, &s.devices.driver_ids()) {
            return detail(StatusCode::UNPROCESSABLE_ENTITY, m);
        }
        if entries.iter().any(|x: &DeviceEntry| str_field(x, "id") == str_field(&e, "id")) {
            return detail(StatusCode::UNPROCESSABLE_ENTITY, format!("duplicate device id {:?}", str_field(&e, "id")));
        }
        entries.push(e);
    }
    if let Err(m) = s.config.save(&entries) {
        return detail(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save configuration: {m}"));
    }
    let devs = s.devices.clone();
    let started = entries.clone();
    let res = blocking(move || {
        devs.stop_all();
        started.into_iter().map(|e| devs.start(e)).collect::<Result<Vec<_>, _>>()
    })
    .await;
    if let Err(m) = res {
        return detail(StatusCode::INTERNAL_SERVER_ERROR, m);
    }
    Json(json!({"status": "success",
        "message": format!("Configuration updated with {} device(s) and saved to {}", entries.len(), s.config.path.display()),
        "devices": entries}))
    .into_response()
}

async fn add_device(State(s): State<S>, Json(body): Json<Value>) -> Response {
    let _g = s.config_lock.lock().await;
    let e = match entry_from(body) {
        Ok(e) => e,
        Err(r) => return *r,
    };
    if let Err(m) = validate_entry(&e, &s.devices.driver_ids()) {
        return detail(StatusCode::UNPROCESSABLE_ENTITY, m);
    }
    let id = str_field(&e, "id").unwrap_or_default().to_string();
    let mut all = s.devices.entries();
    if all.iter().any(|x| str_field(x, "id") == Some(id.as_str())) {
        return detail(StatusCode::CONFLICT, format!("Device {id} already exists in config"));
    }
    all.push(e.clone());
    if let Err(m) = s.config.save(&all) {
        return detail(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save config: {m}"));
    }
    if let Err(m) = s.devices.start(e) {
        return detail(StatusCode::INTERNAL_SERVER_ERROR, m);
    }
    Json(json!({"status": "success", "message": format!("Device {id} added and started")})).into_response()
}

async fn update_device(State(s): State<S>, Path(id): Path<String>, Json(body): Json<Value>) -> Response {
    let _g = s.config_lock.lock().await;
    let e = match entry_from(body) {
        Ok(e) => e,
        Err(r) => return *r,
    };
    if str_field(&e, "id") != Some(id.as_str()) {
        return detail(StatusCode::BAD_REQUEST, format!("Config id {:?} doesn't match URL device_id '{id}'", str_field(&e, "id")));
    }
    if let Err(m) = validate_entry(&e, &s.devices.driver_ids()) {
        return detail(StatusCode::UNPROCESSABLE_ENTITY, m);
    }
    let mut all = s.devices.entries();
    let Some(pos) = all.iter().position(|x| str_field(x, "id") == Some(id.as_str())) else {
        return detail(StatusCode::NOT_FOUND, format!("Device {id} not found"));
    };
    all[pos] = e.clone();
    if let Err(m) = s.config.save(&all) {
        return detail(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save config: {m}"));
    }
    let devs = s.devices.clone();
    let rid = id.clone();
    if let Err(m) = blocking(move || {
        devs.stop(&rid);
        devs.start(e)
    })
    .await
    {
        return detail(StatusCode::INTERNAL_SERVER_ERROR, m);
    }
    Json(json!({"status": "success", "message": format!("Device {id} updated and reconnected")})).into_response()
}

async fn delete_device(State(s): State<S>, Path(id): Path<String>) -> Response {
    let _g = s.config_lock.lock().await;
    let mut all = s.devices.entries();
    let before = all.len();
    all.retain(|x| str_field(x, "id") != Some(id.as_str()));
    if all.len() == before {
        return detail(StatusCode::NOT_FOUND, format!("Device {id} not found"));
    }
    if let Err(m) = s.config.save(&all) {
        return detail(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save config: {m}"));
    }
    let devs = s.devices.clone();
    let rid = id.clone();
    blocking(move || devs.stop(&rid)).await;
    Json(json!({"status": "success", "message": format!("Device {id} removed")})).into_response()
}

// ---- wire log (the instrument console) -------------------------------------------------

/// What was sent to and received from a device: entries after `after` (sequence number).
/// Bytes come as a Latin-1 string (one char per byte, binary preserved); `info` entries are
/// text (port opened / closed / errors). `dropped`: entries between `after` and the first
/// one returned were already discarded.
async fn wire_log(State(s): State<S>, Path(id): Path<String>, Query(q): Query<std::collections::HashMap<String, String>>) -> Response {
    if s.devices.lookup(&id).is_none() {
        return detail(StatusCode::NOT_FOUND, format!("Unknown device id {id:?}"));
    }
    let after = match q.get("after").map(|a| a.parse::<u64>()) {
        None => 0,
        Some(Ok(a)) => a,
        Some(Err(_)) => return detail(StatusCode::UNPROCESSABLE_ENTITY, "`after` must be a sequence number"),
    };
    let (entries, dropped) = s.devices.wire_log(&id).since(after);
    let next = entries.last().map(|e| e.seq).unwrap_or(after);
    let list: Vec<Value> = entries
        .iter()
        .map(|e| {
            let data: String = match e.dir {
                bm_driver::wirelog::Dir::Info => String::from_utf8_lossy(&e.data).to_string(),
                _ => e.data.iter().map(|&b| b as char).collect(),
            };
            json!({"seq": e.seq, "t": e.at_ms, "dir": e.dir.as_str(), "data": data, "len": e.len})
        })
        .collect();
    Json(json!({"device_id": id, "entries": list, "next": next, "dropped": dropped})).into_response()
}

async fn clear_wire_log(State(s): State<S>, Path(id): Path<String>) -> Response {
    if s.devices.lookup(&id).is_none() {
        return detail(StatusCode::NOT_FOUND, format!("Unknown device id {id:?}"));
    }
    s.devices.wire_log(&id).clear();
    StatusCode::NO_CONTENT.into_response()
}

// ---- discovery ----------------------------------------------------------------------

/// Start identifying the instruments on the attached USB ports (identify queries only) in the
/// background; `GET` shows its progress. A run in progress is not restarted.
async fn detect_devices(State(s): State<S>) -> Response {
    if !s.devices.hardware {
        return Json(json!({"running": false, "elapsed_ms": 0, "ports": []})).into_response();
    }
    {
        let mut job = s.devices.detect.lock().unwrap();
        if job.running {
            return Json(job.snapshot()).into_response();
        }
        *job = detect::Job { running: true, started: Some(std::time::Instant::now()), ..Default::default() };
    }
    let devs = s.devices.clone();
    std::thread::spawn(move || {
        let ports = detect::run(&devs);
        let mut job = devs.detect.lock().unwrap();
        job.elapsed_ms = job.started.map(|t| t.elapsed().as_millis() as u64).unwrap_or_default();
        job.ports = ports;
        job.running = false;
    });
    // the port list appears once the ports are enumerated (a few ms)
    for _ in 0..20 {
        if !s.devices.detect.lock().unwrap().ports.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    (StatusCode::ACCEPTED, Json(s.devices.detect.lock().unwrap().snapshot())).into_response()
}

/// The current or last Detect run: each port's progress (queued / probing with the probe on the
/// wire and how many of its probes were tried / done with the result).
async fn detect_status(State(s): State<S>) -> Json<Value> {
    Json(s.devices.detect.lock().unwrap().snapshot())
}

fn excluded(q: &std::collections::HashMap<String, String>) -> Vec<String> {
    q.get("exclude").map(|e| e.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default()
}

async fn serial_ports(Query(q): Query<std::collections::HashMap<String, String>>) -> Json<Value> {
    let ex = excluded(&q);
    let ports = blocking(bm_transport::discover::serial_ports).await;
    Json(json!(ports
        .into_iter()
        .filter(|p| !ex.contains(&p.device))
        .map(|p| {
            let hwid = p.usb.vendor_id.as_ref().map(|v| {
                format!("USB VID:PID={v}:{} SER={}", p.usb.product_id.clone().unwrap_or_default(), p.usb.serial.clone().unwrap_or_default())
            });
            json!({"device": p.device, "description": p.description, "manufacturer": p.usb.manufacturer,
                   "serial_number": p.usb.serial, "hwid": hwid})
        })
        .collect::<Vec<_>>()))
}

async fn usbtmc_devices(Query(q): Query<std::collections::HashMap<String, String>>) -> Json<Value> {
    let ex = excluded(&q);
    let devs = blocking(bm_transport::discover::usbtmc_devices).await;
    Json(json!(devs
        .into_iter()
        .filter(|d| !ex.contains(&d.device))
        .map(|d| json!({"device": d.device, "name": d.name, "vendor_id": d.usb.vendor_id, "product_id": d.usb.product_id,
                        "manufacturer": d.usb.manufacturer, "product": d.usb.product, "serial": d.usb.serial, "note": d.note,
                        "driver_installable": d.driver_installable}))
        .collect::<Vec<_>>()))
}

/// Why `device` cannot be given the WinUSB driver (`None`: it can).
fn driver_refusal(devs: &[bm_transport::discover::UsbTmcInfo], device: &str) -> Option<Response> {
    match devs.iter().find(|d| d.device == device) {
        None => Some(detail(StatusCode::NOT_FOUND, format!("{device}: no such USBTMC instrument attached"))),
        Some(d) if !d.driver_installable => Some(detail(StatusCode::CONFLICT, format!("{device} does not need a driver from BenchMesh"))),
        Some(_) => None,
    }
}

/// Give a Windows USBTMC instrument the WinUSB driver (`{device}` from `/usbtmc-devices` with
/// `driver_installable`). Windows asks the user for administrator approval; the request waits.
async fn install_usbtmc_driver(Json(body): Json<Value>) -> Response {
    let Some(device) = body.get("device").and_then(|d| d.as_str()).map(str::to_string) else {
        return detail(StatusCode::UNPROCESSABLE_ENTITY, "body needs {\"device\": \"usb:…\"}");
    };
    let devs = blocking(bm_transport::discover::usbtmc_devices).await;
    if let Some(refused) = driver_refusal(&devs, &device) {
        return refused;
    }
    let target = device.clone();
    match blocking(move || bm_transport::winusb::install(&target)).await {
        Ok(()) => Json(json!({"device": device, "installed": true})).into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => detail(StatusCode::FORBIDDEN, e.to_string()),
        Err(e) => {
            eprintln!("WinUSB driver installation for {device} failed: {e}");
            detail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

// ---- websocket ----------------------------------------------------------------------

async fn ws_registry(State(s): State<S>, p: remote::Principal, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| registry_stream(s, p, socket))
}

/// Push the registry every interval (the UI marks data stale after ~1 s of silence); ends
/// with the remote session.
async fn registry_stream(s: S, p: remote::Principal, mut socket: WebSocket) {
    let mut tick = tokio::time::interval(s.ws_interval);
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = p.revoked() => {
                let _ = socket.send(Message::Close(None)).await;
                return;
            }
        }
        let snapshot = s.devices.registry.lock().unwrap().to_json().to_string();
        if socket.send(Message::Text(snapshot.into())).await.is_err() {
            return;
        }
    }
}

// ---- metrics --------------------------------------------------------------------------

fn metrics_map(s: &AppState) -> Map<String, Value> {
    s.devices.registry.lock().unwrap().devices.iter().filter_map(|(id, d)| d.metrics.clone().map(|m| (id.clone(), m))).collect()
}

fn unix_now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

async fn metrics_summary(State(s): State<S>) -> Json<Value> {
    Json(json!({"summary": metrics_map(&s), "timestamp": unix_now()}))
}

async fn metrics_device(State(s): State<S>, Path(id): Path<String>) -> Response {
    match metrics_map(&s).remove(&id) {
        Some(mut m) => {
            m["timestamp"] = json!(unix_now());
            Json(m).into_response()
        }
        None => detail(StatusCode::NOT_FOUND, format!("No metrics available for device {id}")),
    }
}

async fn ws_metrics(State(s): State<S>, p: remote::Principal, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |mut socket| async move {
        let mut tick = tokio::time::interval(s.metrics_interval);
        loop {
            tokio::select! {
                _ = tick.tick() => {}
                _ = p.revoked() => {
                    let _ = socket.send(Message::Close(None)).await;
                    return;
                }
            }
            let snapshot = Value::Object(metrics_map(&s)).to_string();
            if socket.send(Message::Text(snapshot.into())).await.is_err() {
                return;
            }
        }
    })
}

/// API reference lives in the UI (Swagger UI bundled with the frontend: works offline).
async fn api_docs(State(s): State<S>) -> Response {
    if s.ui_dir.is_some() {
        Redirect::temporary("/ui/?openDocs=true").into_response()
    } else {
        Redirect::temporary("/openapi.json").into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm_transport::discover::{UsbInfo, UsbTmcInfo};

    fn tmc(device: &str, installable: bool) -> UsbTmcInfo {
        UsbTmcInfo { device: device.into(), name: device.into(), usb: UsbInfo::default(), note: installable.then(|| "no driver".into()), driver_installable: installable }
    }

    #[test]
    fn a_driver_is_only_installed_on_an_attached_instrument_that_needs_one() {
        let devs = [tmc("usb:1ab1:044d:DHO8A272907001", true), tmc("usb:5345:1235:OWON1", false)];
        assert!(driver_refusal(&devs, "usb:1ab1:044d:DHO8A272907001").is_none());
        assert_eq!(driver_refusal(&devs, "usb:5345:1235:OWON1").unwrap().status(), StatusCode::CONFLICT);
        assert_eq!(driver_refusal(&devs, "usb:0000:0000:NONE").unwrap().status(), StatusCode::NOT_FOUND);
    }
}
