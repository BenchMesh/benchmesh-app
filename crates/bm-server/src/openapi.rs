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
    ("get", "/catalog", "instruments", "Supported instruments: families with classes, transports, manuals and each model's support status (tested / confirmed / awaiting) and community report (from the website) → {families, models: {status: count}, community: {source, fetched_at, error, models}}"),
    ("get", "/config", "configuration", "Configured devices"),
    ("post", "/config", "configuration", "Replace the device configuration (restarts devices)"),
    ("post", "/devices", "configuration", "Add a device (saved and started); `match` = identity {usb: {vid, pid, serial?, revision?, location?}, idn_serial?} instead of a fixed `port`/`device`"),
    ("post", "/devices/detect", "configuration", "Start identifying the instruments on the attached USB ports in the background (identify queries only, never to ports in use; one exchange per distinct probe) → 202 with the job; a running job is not restarted"),
    ("get", "/devices/detect", "configuration", "Detect progress → {running, elapsed_ms, ports: [{port, kind, usb, configured_as, locked, instrument, match, certainty, error, progress: {state: queued|probing|done, tried, total, current}}]}"),
    ("put", "/devices/{id}", "configuration", "Update a device (saved and reconnected)"),
    ("get", "/devices/{id}/wire", "monitoring", "Instrument console: bytes sent / received and port events after `after` (seq) → {entries: [{seq, t (ms), dir: tx|rx|info, data (Latin-1 bytes / text), len}], next, dropped}"),
    ("delete", "/devices/{id}/wire", "monitoring", "Clear an instrument's console (204)"),
    ("delete", "/devices/{id}", "configuration", "Remove a device"),
    ("get", "/serial-ports", "configuration", "Serial ports (query `exclude=a,b`)"),
    ("get", "/usbtmc-devices", "configuration", "USBTMC devices (query `exclude=a,b`) → [{device, name, vendor_id, product_id, manufacturer, product, serial, note, driver_installable}]"),
    ("post", "/usbtmc-devices/driver", "configuration", "Give a Windows USBTMC instrument Microsoft's WinUSB driver {device} (Windows asks for administrator approval; waits for it) → {device, installed}; 403 approval declined, 404 not attached, 409 needs no driver"),
    ("get", "/instruments", "instruments", "Instruments with classes, channels and health (ETag / 304)"),
    ("get", "/instruments/{klass}", "instruments", "Instruments of one class"),
    ("get", "/instruments/{klass}/{id}", "instruments", "UI features of an instrument class (modes, ranges, limits)"),
    ("get", "/instruments/{klass}/{id}/methods", "instruments", "Operations of an instrument class"),
    ("get", "/instruments/{klass}/{id}/{ch}/{op}", "instrument-control", "Read an operation → {path, value}"),
    ("post", "/instruments/{klass}/{id}/{ch}/{op}/{value}", "instrument-control", "Set an operation (204); 409 = precondition not met"),
    ("get", "/metrics", "monitoring", "Per-device performance metrics"),
    ("get", "/metrics/{id}", "monitoring", "Performance metrics of one device"),
    ("get", "/recordings", "recordings", "Recordings with series, sample count and last sample time (newest first)"),
    ("post", "/recordings", "recordings", "Create a recording {name, description?, interval_s 1-3600} → 201 (state recording, no series); 409 name taken"),
    ("get", "/recordings/{id}", "recordings", "One recording"),
    ("patch", "/recordings/{id}", "recordings", "Rename / describe a recording {name?, description?}; 409 name taken"),
    ("delete", "/recordings/{id}", "recordings", "Delete a recording and its data (204)"),
    ("post", "/recordings/{id}/series", "recordings", "Add a reading {device_id, class, channel, field, label?, unit?} → 201; 409 stopped or already recorded; 422 not a numeric status key"),
    ("delete", "/recordings/{id}/series/{sid}", "recordings", "Stop recording a reading (204; its data is kept)"),
    ("post", "/recordings/{id}/pause", "recordings", "Pause a recording (idempotent)"),
    ("post", "/recordings/{id}/resume", "recordings", "Resume a paused recording (idempotent)"),
    ("post", "/recordings/{id}/stop", "recordings", "Stop a recording (final, idempotent)"),
    ("get", "/recordings/{id}/data", "recordings", "Samples per series (query `from`, `to` ms; `max_points` 10-20000, default 2000; min/max decimation)"),
    ("get", "/recordings/{id}/export", "recordings", "CSV: time_utc, time_ms, one column per series"),
    ("get", "/ai/context", "ai-assistant", "Installation description for AI assistants (`format=markdown|json`)"),
    ("get", "/ai/system-prompt", "ai-assistant", "Alias of /ai/context"),
    ("get", "/openapi.json", "system", "This specification (JSON)"),
    ("get", "/openapi.yaml", "system", "This specification (YAML)"),
    ("get", "/docs", "system", "API reference (redirects to the UI)"),
    ("get", "/redoc", "system", "API reference (redirects to the UI)"),
    ("get", "/ws/registry", "websockets", "WebSocket: live registry {device: {IDN, CLASS: {status_chN}}} every 0.2 s"),
    ("get", "/ws/metrics", "websockets", "WebSocket: metrics summary every 30 s"),
    ("get", "/ws/recordings", "websockets", "WebSocket: snapshot, then samples per tick, recording changes and deletions"),
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
        if matches!(*method, "post" | "put" | "patch") && !path.contains("/{value}") && !path.ends_with("/pause")
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
                for m in ["get", "post", "put", "patch", "delete"] {
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
        assert!(s["paths"]["/recordings"]["post"]["requestBody"].is_object());
        assert!(s["paths"]["/recordings/{id}"]["patch"]["requestBody"].is_object());
        assert!(s["paths"]["/recordings/{id}/stop"]["post"].get("requestBody").is_none());
    }
}
