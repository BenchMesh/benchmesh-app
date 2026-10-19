//! MCP server (Model Context Protocol, Streamable HTTP with JSON responses) at `POST /mcp`:
//! AI clients read the bench, build and run flows, record - within the access the user set
//! (`off` / `read` / `control`, `/automation/mcp`). Contract: `ai_context/AUTOMATION.md`.
//! [`stdio_bridge`] serves clients that only start processes (Claude Desktop).

use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use bm_driver::engine::Output;
use serde_json::{json, Map, Value};

use crate::automation::assist::FLOW_REFERENCE;
use crate::automation::exec::numeric;
use crate::recordings::MAX_AGE;
use crate::S;

const PROTOCOLS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "BenchMesh controls the user's lab instruments (power supplies, multimeters, electronic loads, \
generators, oscilloscopes, LCR meters, spectrum analysers). Start with bench_overview. To automate, write a flow \
(flow_format explains the format), check it with validate_flow, save it with save_flow and run it with run_flow; \
follow it with run_status. Anything that can switch an output on needs the user's explicit agreement: ask before \
passing arm / confirm_energize. abort_run stops a run at once (cleanup still runs).";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Access {
    Off,
    Read,
    Control,
}

fn access(s: &S) -> Access {
    match s.automation.mcp_access().as_str() {
        "control" => Access::Control,
        "off" => Access::Off,
        _ => Access::Read,
    }
}

fn addr_props() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("device".into(), json!({"type": "string", "description": "Device id from bench_overview"}));
    m.insert("class".into(), json!({"type": "string", "description": "Instrument class: PSU, DMM, ELL, AWG, OSC, LCR, SAL"}));
    m.insert("channel".into(), json!({"type": "integer", "minimum": 1, "default": 1}));
    m
}

fn schema(props: Map<String, Value>, required: &[&str]) -> Value {
    json!({"type": "object", "properties": props, "required": required})
}

fn with(mut m: Map<String, Value>, extra: Value) -> Map<String, Value> {
    if let Value::Object(e) = extra {
        m.extend(e);
    }
    m
}

/// (name, minimum access, description, input schema)
fn tools() -> Vec<(&'static str, Access, &'static str, Value)> {
    let none = || schema(Map::new(), &[]);
    vec![
        ("bench_overview", Access::Read, "Instruments on the bench: id, name, model, online, live status per class/channel, status fields and operations (setters with type, min, max, unit, energizes).", none()),
        ("read_instrument", Access::Read, "Read one live status field (e.g. PSU VOUT, DMM MEAS, ELL IIN) or run one query operation.",
            schema(with(addr_props(), json!({"field": {"type": "string", "description": "Status field"}, "op": {"type": "string", "description": "Query operation (instead of field)"}})), &["device", "class"])),
        ("flow_format", Access::Read, "Reference of the BenchMesh flow format (benchmesh.flow/v1) with an example. Read it before writing a flow.", none()),
        ("list_flows", Access::Read, "Saved flows: id, name, description.", none()),
        ("get_flow", Access::Read, "One saved flow document.", schema(with(Map::new(), json!({"id": {"type": "string"}})), &["id"])),
        ("validate_flow", Access::Read, "Check a flow document against this bench: errors and warnings with paths, and which steps can switch outputs on.",
            schema(with(Map::new(), json!({"flow": {"type": "object"}})), &["flow"])),
        ("run_status", Access::Read, "State of the current or last run with its log (events after `after`).",
            schema(with(Map::new(), json!({"after": {"type": "integer", "description": "Only events with a larger seq"}})), &[])),
        ("list_recordings", Access::Read, "Recordings: id, name, state, start/stop time, series.", none()),
        ("get_recording", Access::Read, "Samples of a recording (t in ms since 1970, one value list per series), at most max_points rows.",
            schema(with(Map::new(), json!({"id": {"type": "integer"}, "max_points": {"type": "integer", "minimum": 10, "maximum": 5000, "default": 500}})), &["id"])),
        ("abort_run", Access::Read, "Abort the running flow now (its cleanup still runs; outputs it switched on are switched off).", none()),
        ("set_instrument", Access::Control, "Call one setter and wait until the instrument reports the value. Setters that can switch an output on need confirm_energize: true - only after the user agreed.",
            schema(with(addr_props(), json!({"op": {"type": "string"}, "value": {"type": ["number", "string", "boolean"]}, "confirm_energize": {"type": "boolean", "default": false}})), &["device", "class", "op", "value"])),
        ("save_flow", Access::Control, "Validate and save a flow (new, or replacing `id`). Returns its id. Saving never runs it.",
            schema(with(Map::new(), json!({"flow": {"type": "object"}, "id": {"type": "string"}})), &["flow"])),
        ("run_flow", Access::Control, "Start a saved flow (id) or a flow document. Flows that can switch outputs on need arm: true - only after the user agreed. Follow with run_status.",
            schema(with(Map::new(), json!({"id": {"type": "string"}, "flow": {"type": "object"}, "params": {"type": "object", "description": "Parameter overrides"}, "arm": {"type": "boolean", "default": false}})), &[])),
        ("continue_run", Access::Control, "Continue a run waiting at a prompt step (only after the user did what it asks).", none()),
        ("start_recording", Access::Control, "Start a recording of numeric status fields on the common time grid.",
            schema(with(Map::new(), json!({"name": {"type": "string"}, "interval_s": {"type": "integer", "minimum": 1, "maximum": 3600, "default": 1},
                "series": {"type": "array", "items": schema(with(addr_props(), json!({"field": {"type": "string"}, "label": {"type": "string"}})), &["device", "class", "field"])}})), &["name", "series"])),
        ("stop_recording", Access::Control, "Stop a recording.", schema(with(Map::new(), json!({"id": {"type": "integer"}})), &["id"])),
    ]
}

fn rpc_result(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn rpc_error(id: &Value, code: i64, message: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message.into()}})
}

fn text_result(v: &Value, is_error: bool) -> Value {
    let text = match v {
        Value::String(s) => s.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    };
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

pub async fn get() -> Response {
    (StatusCode::METHOD_NOT_ALLOWED, [(header::ALLOW, "POST")], "BenchMesh MCP: POST JSON-RPC messages here (no server stream)").into_response()
}

pub async fn post(State(s): State<S>, body: Bytes) -> Response {
    let msg: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(rpc_error(&Value::Null, -32700, format!("parse error: {e}")))).into_response(),
    };
    match msg {
        Value::Array(batch) => {
            let mut out = Vec::new();
            for m in batch {
                if let Some(r) = handle(&s, m).await {
                    out.push(r);
                }
            }
            if out.is_empty() { StatusCode::ACCEPTED.into_response() } else { Json(Value::Array(out)).into_response() }
        }
        m => match handle(&s, m).await {
            Some(r) => Json(r).into_response(),
            None => StatusCode::ACCEPTED.into_response(),
        },
    }
}

/// One JSON-RPC message; `None` for notifications and responses.
async fn handle(s: &S, m: Value) -> Option<Value> {
    let id = m.get("id").cloned();
    let method = m.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let id = id.filter(|i| !i.is_null())?;
    if method.is_empty() {
        return None; // a response to something we never sent
    }
    let params = m.get("params").cloned().unwrap_or(json!({}));
    Some(match method {
        "initialize" => {
            let asked = params["protocolVersion"].as_str().unwrap_or("");
            let version = if PROTOCOLS.contains(&asked) { asked } else { PROTOCOLS[0] };
            rpc_result(&id, json!({
                "protocolVersion": version,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "benchmesh", "title": "BenchMesh", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCTIONS,
            }))
        }
        "ping" => rpc_result(&id, json!({})),
        "tools/list" => {
            let level = access(s);
            let list: Vec<Value> = tools()
                .into_iter()
                .filter(|(_, need, _, _)| level != Access::Off && *need <= level)
                .map(|(name, _, description, input)| json!({"name": name, "description": description, "inputSchema": input}))
                .collect();
            rpc_result(&id, json!({"tools": list}))
        }
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("");
            let args = params.get("arguments").and_then(|a| a.as_object()).cloned().unwrap_or_default();
            let Some((_, need, _, _)) = tools().into_iter().find(|t| t.0 == name) else {
                return Some(rpc_error(&id, -32602, format!("unknown tool {name:?}")));
            };
            let level = access(s);
            if level == Access::Off {
                return Some(rpc_result(&id, text_result(&json!("MCP access is off - enable it in BenchMesh: Automation → Settings → MCP access"), true)));
            }
            if need > level {
                return Some(rpc_result(&id, text_result(&json!(format!("{name} needs MCP access \"control\" - the user can allow it in BenchMesh: Automation → Settings → MCP access")), true)));
            }
            match call(s, name, &args).await {
                Ok(v) => rpc_result(&id, text_result(&v, false)),
                Err(v) => rpc_result(&id, text_result(&v, true)),
            }
        }
        _ => rpc_error(&id, -32601, format!("method {method:?} not found")),
    })
}

fn arg_str<'a>(a: &'a Map<String, Value>, k: &str) -> Result<&'a str, Value> {
    a.get(k).and_then(|v| v.as_str()).ok_or_else(|| json!(format!("`{k}` (string) is required")))
}

fn channel(a: &Map<String, Value>) -> Result<u8, Value> {
    match a.get("channel") {
        None | Some(Value::Null) => Ok(1),
        Some(v) => v.as_u64().filter(|c| (1..=9).contains(c)).map(|c| c as u8).ok_or_else(|| json!("`channel` must be 1-9")),
    }
}

fn status_of(e: (StatusCode, Value)) -> Value {
    e.1
}

/// Wait for a short run (one MCP `set_instrument`) to end.
async fn wait_run(s: &S, id: u64) -> Value {
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        let st = s.automation.run_status(0);
        let done = st["id"].as_u64() != Some(id) || !matches!(st["state"].as_str(), Some("running" | "cleanup" | "waiting"));
        if done || Instant::now() >= until {
            return st;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn call(s: &S, name: &str, a: &Map<String, Value>) -> Result<Value, Value> {
    let auto = &s.automation;
    match name {
        "bench_overview" => {
            let vocab = auto.vocab();
            let reg = s.devices.registry.lock().unwrap();
            let devices: Vec<Value> = vocab
                .devices
                .iter()
                .map(|d| {
                    let mut v = json!(d);
                    if let Some(st) = reg.devices.get(&d.id).filter(|x| x.connected) {
                        let live: Map<String, Value> = st
                            .status
                            .iter()
                            .map(|(c, chans)| (c.clone(), json!(chans.iter().map(|(ch, m)| (format!("ch{ch}"), json!(m))).collect::<Map<_, _>>())))
                            .collect();
                        v["status"] = Value::Object(live);
                    }
                    v
                })
                .collect();
            Ok(json!({"devices": devices, "note": "SI status values are {si, sci, val}: use val (a number in base units)."}))
        }
        "read_instrument" => {
            let (device, class, ch) = (arg_str(a, "device")?, arg_str(a, "class")?, channel(a)?);
            let vocab = auto.vocab();
            let c = vocab.class(device, class).map_err(|m| json!(m))?;
            match (a.get("field").and_then(|v| v.as_str()), a.get("op").and_then(|v| v.as_str())) {
                (Some(f), None) => {
                    if !c.fields.contains_key(f) {
                        return Err(json!(format!("{device} {class} reports no field {f:?} (fields: {})", c.fields.keys().cloned().collect::<Vec<_>>().join(", "))));
                    }
                    let reg = s.devices.registry.lock().unwrap();
                    let v = reg.fresh_value(device, class, ch, f, MAX_AGE).cloned().ok_or_else(|| json!(format!("{device} has no fresh {f} (offline?)")))?;
                    Ok(json!({"device": device, "class": class, "channel": ch, "field": f, "value": numeric(&v).map(|x| json!(x)).unwrap_or(v)}))
                }
                (None, Some(op)) => {
                    if !c.operations.get(op).is_some_and(|o| o.get) {
                        return Err(json!(format!("{device} {class} has no query {op:?}")));
                    }
                    let (_, client, _) = s.devices.lookup(device).ok_or_else(|| json!("unknown device"))?;
                    let (cl, o) = (class.to_string(), op.to_string());
                    match tokio::task::spawn_blocking(move || client.get(&cl, &o, ch)).await {
                        Ok(Ok(Output::Value(v))) => Ok(json!({"device": device, "class": class, "channel": ch, "op": op, "value": numeric(&v).map(|x| json!(x)).unwrap_or(v)})),
                        Ok(Ok(Output::Bytes(b))) => Ok(json!(format!("{op} returned {} bytes of binary data", b.len()))),
                        Ok(Err(e)) => Err(json!(e.to_string())),
                        Err(e) => Err(json!(e.to_string())),
                    }
                }
                _ => Err(json!("give `field` (status) or `op` (query)")),
            }
        }
        "flow_format" => Ok(json!(FLOW_REFERENCE)),
        "list_flows" => Ok(json!(auto.flows.list())),
        "get_flow" => auto.flows.get(arg_str(a, "id")?).ok_or_else(|| json!("no such flow")),
        "validate_flow" => Ok(auto.validate(a.get("flow").ok_or_else(|| json!("`flow` is required"))?)),
        "run_status" => Ok(auto.run_status(a.get("after").and_then(|v| v.as_u64()).unwrap_or(0))),
        "list_recordings" => {
            let store = s.recordings.store.clone();
            let list = tokio::task::spawn_blocking(move || -> Result<Vec<Value>, bm_store::StoreError> {
                let mut out = Vec::new();
                for r in store.list_recordings()? {
                    let series: Vec<Value> = store.series_of(r.id)?.iter().map(|x| json!({"id": x.id, "label": x.label, "device": x.signal.device_id, "field": x.signal.field})).collect();
                    out.push(json!({"id": r.id, "name": r.name, "state": r.state.as_str(), "interval_s": r.interval_s, "started_at": r.started_at, "stopped_at": r.stopped_at, "series": series}));
                }
                Ok(out)
            })
            .await
            .map_err(|e| json!(e.to_string()))?
            .map_err(|e| json!(e.to_string()))?;
            Ok(json!(list))
        }
        "get_recording" => {
            let id = a.get("id").and_then(|v| v.as_i64()).ok_or_else(|| json!("`id` (integer) is required"))?;
            let max = a.get("max_points").and_then(|v| v.as_u64()).unwrap_or(500).clamp(10, 5000) as usize;
            let store = s.recordings.store.clone();
            tokio::task::spawn_blocking(move || -> Result<Value, bm_store::StoreError> {
                let r = store.recording(id)?;
                let to = r.stopped_at.unwrap_or_else(bm_store::now_ms);
                let mut series = Vec::new();
                for x in store.series_of(id)? {
                    let p = store.range(x.id, r.started_at, to, max)?;
                    series.push(json!({"label": x.label, "unit": x.unit, "t": p.t, "v": p.v}));
                }
                Ok(json!({"id": id, "name": r.name, "state": r.state.as_str(), "series": series}))
            })
            .await
            .map_err(|e| json!(e.to_string()))?
            .map_err(|e| json!(e.to_string()))
        }
        "abort_run" => Ok(json!(if auto.runner.abort() { "abort requested" } else { "no flow is running" })),
        "continue_run" => Ok(json!(if auto.runner.resume() { "continued" } else { "no run is waiting at a prompt" })),
        "set_instrument" => {
            let (device, class, ch, op) = (arg_str(a, "device")?, arg_str(a, "class")?, channel(a)?, arg_str(a, "op")?);
            let value = a.get("value").filter(|v| v.is_number() || v.is_string() || v.is_boolean()).ok_or_else(|| json!("`value` (number, string or boolean) is required"))?;
            let flow = json!({"schema": crate::automation::flow::SCHEMA, "name": format!("MCP: {device} {op} = {value}"),
                "steps": [{"do": "set", "device": device, "class": class, "channel": ch, "op": op, "value": value}]});
            let mut b = Map::new();
            b.insert("flow".into(), flow);
            b.insert("arm".into(), json!(a.get("confirm_energize").and_then(|v| v.as_bool()).unwrap_or(false)));
            let started = auto.start(&b).map_err(status_of)?;
            let st = wait_run(s, started["id"].as_u64().unwrap_or(0)).await;
            match st["state"].as_str() {
                Some("finished") => Ok(json!(format!("{device} {class} {op} = {value} (confirmed by the instrument)"))),
                _ => Err(json!({"error": st["error"], "log": st["events"]})),
            }
        }
        "save_flow" => {
            let flow = a.get("flow").ok_or_else(|| json!("`flow` is required"))?;
            let id = a.get("id").and_then(|v| v.as_str());
            if id.is_some_and(|i| !crate::automation::store::valid_id(i)) {
                return Err(json!("`id` must be lowercase letters, digits and -"));
            }
            let (id, issues) = auto.save(id, flow).map_err(status_of)?;
            Ok(json!({"id": id, "issues": issues, "note": "Saved; it appears in BenchMesh → Automation."}))
        }
        "run_flow" => {
            let mut b = Map::new();
            match (a.get("id").and_then(|v| v.as_str()), a.get("flow")) {
                (Some(i), None) => b.insert("flow_id".into(), json!(i)),
                (None, Some(f)) => b.insert("flow".into(), f.clone()),
                _ => return Err(json!("give `id` (saved flow) or `flow` (document)")),
            };
            for k in ["params", "arm"] {
                if let Some(v) = a.get(k) {
                    b.insert(k.into(), v.clone());
                }
            }
            auto.start(&b).map_err(status_of)
        }
        "start_recording" => {
            let name = arg_str(a, "name")?;
            let interval = a.get("interval_s").and_then(|v| v.as_i64()).unwrap_or(1);
            if !(1..=3600).contains(&interval) {
                return Err(json!("`interval_s` must be 1-3600"));
            }
            let series: Vec<Map<String, Value>> = a
                .get("series")
                .and_then(|v| v.as_array())
                .ok_or_else(|| json!("`series` (list) is required"))?
                .iter()
                .filter_map(|x| x.as_object())
                .map(|x| {
                    let mut m = Map::new();
                    m.insert("device_id".into(), x.get("device").cloned().unwrap_or(Value::Null));
                    m.insert("class".into(), x.get("class").cloned().unwrap_or(Value::Null));
                    m.insert("channel".into(), x.get("channel").cloned().unwrap_or(json!(1)));
                    m.insert("field".into(), x.get("field").cloned().unwrap_or(Value::Null));
                    if let Some(l) = x.get("label") {
                        m.insert("label".into(), l.clone());
                    }
                    m
                })
                .collect();
            if series.is_empty() {
                return Err(json!("`series` must not be empty"));
            }
            let id = s.recordings.start_with(name, interval, &series).await.map_err(|e| json!(e))?;
            Ok(json!({"id": id, "note": "Recording; see it in BenchMesh → Recording."}))
        }
        "stop_recording" => {
            let id = a.get("id").and_then(|v| v.as_i64()).ok_or_else(|| json!("`id` (integer) is required"))?;
            s.recordings.stop_id(id).await.map_err(|e| json!(e))?;
            Ok(json!("stopped"))
        }
        _ => Err(json!(format!("unknown tool {name:?}"))),
    }
}

// ---- stdio bridge --------------------------------------------------------------------------

/// Forward newline-delimited JSON-RPC from stdin to `{base}/mcp` and the replies to stdout
/// (`benchmesh --mcp`, `benchmesh-server mcp`). Returns the process exit code.
pub fn stdio_bridge(base: &str) -> i32 {
    use std::io::{BufRead, Write};
    let url = format!("{}/mcp", base.trim_end_matches('/'));
    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("benchmesh mcp: {e}");
            return 1;
        }
    };
    let _ = rustls::crypto::ring::default_provider().install_default();
    let http = reqwest::Client::builder().timeout(Duration::from_secs(120)).build().expect("http client");
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let id = serde_json::from_str::<Value>(&line).ok().and_then(|v| v.get("id").cloned()).unwrap_or(Value::Null);
        let reply = rt.block_on(async {
            let r = http.post(&url).header(header::CONTENT_TYPE, "application/json").header(header::ACCEPT, "application/json, text/event-stream").body(line.clone()).send().await;
            match r {
                Ok(r) if r.status() == StatusCode::ACCEPTED => None,
                Ok(r) => r.text().await.ok().filter(|t| !t.trim().is_empty()),
                Err(_) => (!id.is_null()).then(|| rpc_error(&id, -32000, format!("BenchMesh is not running (nothing answers at {url}) - start the BenchMesh app")).to_string()),
            }
        });
        if let Some(text) = reply {
            if writeln!(out, "{}", text.replace('\n', " ")).and_then(|_| out.flush()).is_err() {
                break;
            }
        }
    }
    0
}
