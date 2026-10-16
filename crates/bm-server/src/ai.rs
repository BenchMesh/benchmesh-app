//! `GET /ai/context` (alias `/ai/system-prompt`): a description of this BenchMesh
//! installation for AI assistants — configured instruments, their live state and the
//! operations they support (from the device definitions: value types, ranges, units,
//! which setters can energise outputs).
//!
//! Query: `format=markdown|json` (default markdown), `include=` comma list of
//! `system,instruments,api,safety,examples` (default all).

use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use bm_devicedef::{DeviceDef, Safety, ValueKind};
use serde_json::{json, Value};

use crate::config::str_field;
use crate::S;

const SECTIONS: [&str; 5] = ["system", "instruments", "api", "safety", "examples"];

/// Operation summary from the definition: readable/settable, value type and limits.
fn operations(def: &DeviceDef, class: &str) -> Vec<Value> {
    let Some(b) = def.classes.get(class) else { return vec![] };
    b.operations
        .iter()
        .map(|(name, o)| {
            let set_cmd = o.set.as_ref().and_then(|a| a.steps().first().map(|s| s.command().to_string())).and_then(|c| def.commands.get(&c));
            let param = set_cmd.and_then(|c| c.params.values().find(|p| p.from.is_none()));
            let mut v = json!({"name": name, "get": o.get.is_some(), "set": o.set.is_some()});
            if let Some(d) = &o.description {
                v["description"] = json!(d);
            }
            if let Some(p) = param {
                v["value"] = match p.kind {
                    ValueKind::Enum => json!({"type": "enum", "values": p.values.keys().collect::<Vec<_>>()}),
                    _ => json!({"type": format!("{:?}", p.kind).to_lowercase(), "min": p.min, "max": p.max, "unit": p.unit}),
                };
            }
            if set_cmd.is_some_and(|c| c.safety == Safety::Energize) {
                v["energizes"] = json!(true);
            }
            v
        })
        .collect()
}

fn build(s: &crate::AppState, include: &[&str]) -> Value {
    let mut out = serde_json::Map::new();
    if include.contains(&"system") {
        out.insert("system".into(), json!({
            "name": "BenchMesh",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Lab instrument control: serial/USBTMC instruments exposed through one HTTP/WebSocket API. \
                            Instruments are described by declarative device definitions; status is polled continuously.",
        }));
    }
    if include.contains(&"instruments") {
        let reg = s.devices.registry.lock().unwrap().devices.clone();
        let list: Vec<Value> = s
            .devices
            .entries()
            .iter()
            .filter_map(|e| {
                let id = str_field(e, "id")?.to_string();
                let (def, _, _) = s.devices.lookup(&id)?;
                let st = reg.get(&id).cloned().unwrap_or_default();
                let classes: Vec<Value> = def
                    .classes
                    .iter()
                    .map(|(c, b)| json!({"class": c, "channels": b.channels, "status": st.status.get(c), "operations": operations(&def, c)}))
                    .collect();
                Some(json!({"id": id, "name": str_field(e, "name"), "driver": def.id, "vendor": def.vendor, "family": def.family,
                    "model": st.model, "status": if st.connected { "online" } else { "offline" }, "idn": st.idn, "classes": classes}))
            })
            .collect();
        out.insert("instruments".into(), json!(list));
    }
    if include.contains(&"api") {
        out.insert("api_patterns".into(), json!({
            "query": "GET /instruments/{CLASS}/{device_id}/{channel}/{operation} → {\"path\", \"value\"}",
            "set": "POST /instruments/{CLASS}/{device_id}/{channel}/{operation}/{value} → 204",
            "errors": "400 invalid value, 404 unknown device/operation, 409 precondition (message explains), 503 device offline, 504 timeout",
            "live_status": "WebSocket /ws/registry → {device_id: {IDN, CLASS: {status_chN: {...}}}} every 0.2 s",
            "recordings": "POST /recordings/start {name, interval_seconds, channels:[{device_id, class_name, channel, method_name}]}",
            "spec": "/openapi.json",
        }));
    }
    if include.contains(&"safety") {
        out.insert("safety".into(), json!([
            "Never switch outputs/inputs on (operations marked energizes) without the user's explicit confirmation.",
            "Check limits before setting voltage/current; the API rejects values outside the definition's range.",
            "A 409 response means a precondition is not met (e.g. load not in remote mode or wrong mode); follow its message.",
            "Read the instrument state back after setting it; some instruments silently ignore invalid commands.",
        ]));
    }
    if include.contains(&"examples") {
        let ex: Vec<Value> = s
            .devices
            .entries()
            .iter()
            .filter_map(|e| {
                let id = str_field(e, "id")?;
                let (def, _, _) = s.devices.lookup(id)?;
                let (c, b) = def.classes.iter().next()?;
                let get = b.operations.iter().find(|(_, o)| o.get.is_some()).map(|(n, _)| format!("GET /instruments/{c}/{id}/1/{n}"));
                Some(json!({"device_id": id, "examples": get.into_iter().collect::<Vec<_>>()}))
            })
            .collect();
        out.insert("examples".into(), json!(ex));
    }
    Value::Object(out)
}

fn markdown(v: &Value) -> String {
    let mut md = String::from("# BenchMesh assistant context\n\n");
    if let Some(sys) = v.get("system") {
        md += &format!("## System Overview\n\n{} (version {})\n\n", sys["description"].as_str().unwrap_or(""), sys["version"].as_str().unwrap_or(""));
    }
    if let Some(list) = v.get("instruments").and_then(|l| l.as_array()) {
        md += "## Instruments\n\n";
        for i in list {
            let dot = if i["status"] == "online" { "🟢" } else { "🔴" };
            md += &format!("### {dot} {} (`{}`) — {} {}\n\n", i["name"].as_str().unwrap_or(""), i["id"].as_str().unwrap_or(""),
                i["vendor"].as_str().unwrap_or(""), i["family"].as_str().unwrap_or(""));
            for c in i["classes"].as_array().into_iter().flatten() {
                md += &format!("- **{}** ({} channel(s))\n", c["class"].as_str().unwrap_or(""), c["channels"]);
                for o in c["operations"].as_array().into_iter().flatten() {
                    let rw = match (o["get"].as_bool(), o["set"].as_bool()) {
                        (Some(true), Some(true)) => "get/set",
                        (Some(true), _) => "get",
                        _ => "set",
                    };
                    let warn = if o.get("energizes").is_some() { " ⚠ energizes" } else { "" };
                    let val = o.get("value").map(|v| format!(" — value {v}")).unwrap_or_default();
                    md += &format!("  - `{}` ({rw}){warn}{val}\n", o["name"].as_str().unwrap_or(""));
                }
            }
            md += "\n";
        }
    }
    if let Some(api) = v.get("api_patterns").and_then(|a| a.as_object()) {
        md += "## API\n\n";
        for (k, p) in api {
            md += &format!("- **{k}**: `{}`\n", p.as_str().unwrap_or(""));
        }
        md += "\n";
    }
    if let Some(sf) = v.get("safety").and_then(|a| a.as_array()) {
        md += "## Safety\n\n";
        for r in sf {
            md += &format!("- {}\n", r.as_str().unwrap_or(""));
        }
        md += "\n";
    }
    md
}

pub async fn context(State(s): State<S>, Query(q): Query<HashMap<String, String>>) -> Response {
    let include: Vec<&str> = match q.get("include") {
        Some(list) => list.split(',').map(str::trim).filter(|x| SECTIONS.contains(x)).collect(),
        None => SECTIONS.to_vec(),
    };
    let v = build(&s, &include);
    if q.get("format").map(String::as_str) == Some("json") {
        Json(v).into_response()
    } else {
        ([(header::CONTENT_TYPE, "text/markdown; charset=utf-8")], markdown(&v)).into_response()
    }
}
