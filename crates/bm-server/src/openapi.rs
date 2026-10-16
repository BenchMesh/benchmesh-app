//! OpenAPI 3.1 description of the API, built from a route table (served at
//! `/openapi.json` and `/openapi.yaml`; the UI renders it with Swagger UI). A test
//! checks that every route registered in the router is documented here.

use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Map, Value};

/// (method, path, tag, summary)
pub const ROUTES: &[(&str, &str, &str, &str)] = &[
    ("get", "/", "system", "Redirect to the UI"),
    ("get", "/status", "system", "Device connection counts"),
    ("get", "/version", "system", "Application version"),
    ("get", "/drivers", "configuration", "Available device definitions"),
    ("get", "/drivers/{id}", "configuration", "Models supported by a device definition"),
    ("get", "/config", "configuration", "Configured devices"),
    ("post", "/config", "configuration", "Replace the device configuration (restarts devices)"),
    ("post", "/devices", "configuration", "Add a device (saved and started)"),
    ("put", "/devices/{id}", "configuration", "Update a device (saved and reconnected)"),
    ("delete", "/devices/{id}", "configuration", "Remove a device"),
    ("get", "/serial-ports", "configuration", "Serial ports (query `exclude=a,b`)"),
    ("get", "/usbtmc-devices", "configuration", "USBTMC devices (query `exclude=a,b`)"),
    ("get", "/instruments", "instruments", "Instruments with classes, channels and health (ETag / 304)"),
    ("get", "/instruments/{klass}", "instruments", "Instruments of one class"),
    ("get", "/instruments/{klass}/{id}", "instruments", "UI features of an instrument class (modes, ranges, limits)"),
    ("get", "/instruments/{klass}/{id}/methods", "instruments", "Operations of an instrument class"),
    ("get", "/instruments/{klass}/{id}/{ch}/{op}", "instrument-control", "Read an operation → {path, value}"),
    ("post", "/instruments/{klass}/{id}/{ch}/{op}/{value}", "instrument-control", "Set an operation (204); 409 = precondition not met"),
    ("get", "/metrics", "monitoring", "Per-device performance metrics"),
    ("get", "/metrics/{id}", "monitoring", "Performance metrics of one device"),
    ("get", "/recordings", "recordings", "List recordings"),
    ("post", "/recordings/start", "recordings", "Start a recording {name, interval_seconds, channels, description?}"),
    ("get", "/recordings/{id}", "recordings", "Recording details"),
    ("delete", "/recordings/{id}", "recordings", "Delete a recording and its data"),
    ("post", "/recordings/{id}/pause", "recordings", "Pause a recording"),
    ("post", "/recordings/{id}/resume", "recordings", "Resume a recording"),
    ("post", "/recordings/{id}/stop", "recordings", "Stop a recording"),
    ("get", "/recordings/{id}/data", "recordings", "Recorded points (query `offset`, `limit` ≤ 100000)"),
    ("get", "/recordings/{id}/export", "recordings", "Recorded points as CSV"),
    ("get", "/ai/context", "ai-assistant", "Installation description for AI assistants (`format=markdown|json`)"),
    ("get", "/ai/system-prompt", "ai-assistant", "Alias of /ai/context"),
    ("get", "/openapi.json", "system", "This specification (JSON)"),
    ("get", "/openapi.yaml", "system", "This specification (YAML)"),
    ("get", "/docs", "system", "API reference (redirects to the UI)"),
    ("get", "/redoc", "system", "API reference (redirects to the UI)"),
    ("get", "/ws/registry", "websockets", "WebSocket: live registry {device: {IDN, CLASS: {status_chN}}} every 0.2 s"),
    ("get", "/ws/metrics", "websockets", "WebSocket: metrics summary every 30 s"),
    ("get", "/recordings/ws/{id}", "websockets", "WebSocket: live data points of a recording"),
];

const TAGS: &[(&str, &str)] = &[
    ("system", "Service status, version, documentation"),
    ("configuration", "Device configuration, drivers, port discovery"),
    ("instruments", "Instrument listing, classes, features"),
    ("instrument-control", "Read and set instrument operations"),
    ("monitoring", "Performance metrics"),
    ("recordings", "Data recording"),
    ("ai-assistant", "AI assistant context"),
    ("websockets", "Real-time WebSocket streams"),
];

pub fn spec() -> Value {
    let mut paths: Map<String, Value> = Map::new();
    for (method, path, tag, summary) in ROUTES {
        let params: Vec<Value> = path
            .split('/')
            .filter_map(|seg| seg.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
            .map(|name| json!({"name": name, "in": "path", "required": true, "schema": {"type": "string"}}))
            .collect();
        let mut op = json!({
            "tags": [tag], "summary": summary, "parameters": params,
            "responses": {
                "200": {"description": "Success"},
                "default": {"description": "Error", "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Error"}}}}
            }
        });
        if matches!(*method, "post" | "put") && !path.contains("/{value}") && !path.ends_with("/pause")
            && !path.ends_with("/resume") && !path.ends_with("/stop")
        {
            op["requestBody"] = json!({"content": {"application/json": {"schema": {"type": "object"}}}});
        }
        paths.entry(path.to_string()).or_insert_with(|| json!({}))[*method] = op;
    }
    json!({
        "openapi": "3.1.0",
        "info": {"title": "BenchMesh API", "version": env!("CARGO_PKG_VERSION"),
                 "description": "Lab instrument control. Instrument operations are defined per device by `devices/*.yaml`; \
                                 use `GET /instruments/{klass}/{id}/methods` for what a device supports."},
        "tags": TAGS.iter().map(|(n, d)| json!({"name": n, "description": d})).collect::<Vec<_>>(),
        "paths": paths,
        "components": {"schemas": {"Error": {"type": "object", "properties": {"detail": {"type": "string"}}, "required": ["detail"]}}}
    })
}

pub async fn json() -> Json<Value> {
    Json(spec())
}

pub async fn yaml() -> Response {
    let text = serde_yaml_ng::to_string(&spec()).unwrap_or_default();
    ([(header::CONTENT_TYPE, "application/x-yaml")], text).into_response()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Every `.route("path", method(...))` in the router source must be documented.
    #[test]
    fn every_router_route_is_documented() {
        let sources = [include_str!("lib.rs"), include_str!("recordings.rs")];
        let mut in_router = BTreeSet::new();
        for src in sources {
            for line in src.lines().filter(|l| l.trim_start().starts_with(".route(\"")) {
                let path = line.split('"').nth(1).unwrap();
                for m in ["get", "post", "put", "delete"] {
                    // `.route("/x", get(a).post(b))`: the first method follows ", ", later ones "."
                    if [", ", "."].iter().any(|p| line.contains(&format!("{p}{m}("))) {
                        in_router.insert((m.to_string(), path.to_string()));
                    }
                }
            }
        }
        assert!(in_router.len() > 30, "parsed {} routes", in_router.len());
        let documented: BTreeSet<(String, String)> = ROUTES.iter().map(|(m, p, _, _)| (m.to_string(), p.to_string())).collect();
        let missing: Vec<_> = in_router.difference(&documented).collect();
        let stale: Vec<_> = documented.difference(&in_router).collect();
        assert!(missing.is_empty(), "routes missing from the OpenAPI table: {missing:?}");
        assert!(stale.is_empty(), "documented routes that do not exist: {stale:?}");
    }

    #[test]
    fn spec_is_valid_shape() {
        let s = spec();
        assert_eq!(s["openapi"], "3.1.0");
        let op = &s["paths"]["/instruments/{klass}/{id}/{ch}/{op}"]["get"];
        assert_eq!(op["parameters"].as_array().unwrap().len(), 4);
        assert!(s["paths"]["/recordings/start"]["post"]["requestBody"].is_object());
    }
}
