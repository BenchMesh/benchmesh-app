//! Recordings: periodic sampling of instrument operations into SQLite (`bm-store`), the
//! `/recordings` API and the live `/recordings/ws/{id}` stream.
//!
//! Differences from the former Python service (deliberate fixes):
//! - values are read through each device's actor (serialised with polling and API calls;
//!   Python called drivers directly, bypassing the device lock);
//! - SI value dicts are stored as plain numbers, so charts and CSV get numbers;
//! - timestamps are ISO 8601 UTC with `Z` (Python omitted the zone → browsers showed local time);
//! - live points carry `elapsed_seconds` (Python omitted it → live charts plotted at x=0);
//! - recordings left open by a previous run are closed at startup;
//! - a second pause keeps the original pause time; stopping twice keeps the first end time;
//! - CSV columns are the union of all points' keys (Python crashed on new keys).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use bm_driver::engine::Output;
use bm_store::{iso, now, DataPoint, Series, Store, StoreError};
use serde_json::{json, Map, Value};
use tokio::sync::broadcast;

use crate::devices::Devices;
use crate::S;

struct Active {
    paused: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<()>,
    tx: broadcast::Sender<String>,
}

pub struct Recordings {
    pub store: Arc<Store>,
    devices: Arc<Devices>,
    active: Mutex<HashMap<i64, Active>>,
}

#[derive(Debug, Clone)]
struct Channel {
    device_id: String,
    class_name: String,
    channel: u8,
    method_name: String,
}

impl Channel {
    fn key(&self) -> String {
        format!("{}_{}_{}_{}", self.device_id, self.class_name, self.channel, self.method_name)
    }
}

/// Numeric value of an operation result: SI dicts → number, numbers stay, other values as-is.
fn sample_value(v: Value) -> Option<Value> {
    match &v {
        Value::Null => None,
        Value::Object(o) if o.contains_key("si") => {
            o.get("sci").and_then(|s| s.as_str()).and_then(|s| s.trim().parse::<f64>().ok()).map(|n| json!(n))
        }
        _ => Some(v),
    }
}

impl Recordings {
    pub fn new(store: Arc<Store>, devices: Arc<Devices>) -> Arc<Self> {
        if let Ok(n) = store.close_orphans() {
            if n > 0 {
                eprintln!("closed {n} recording(s) left open by a previous run");
            }
        }
        Arc::new(Recordings { store, devices, active: Mutex::new(HashMap::new()) })
    }

    /// Stop every active recording (server shutdown).
    pub fn shutdown(&self) {
        let ids: Vec<i64> = self.active.lock().unwrap().keys().copied().collect();
        for id in ids {
            let _ = self.stop(id);
        }
    }

    fn validate_channels(&self, channels: &Value) -> Result<Vec<Channel>, String> {
        let list = channels.as_array().ok_or("`channels` must be a list")?;
        if list.is_empty() {
            return Err("at least one channel is required".into());
        }
        let mut out = Vec::new();
        for c in list {
            let s = |k: &str| c.get(k).and_then(|v| v.as_str()).map(str::to_string).ok_or(format!("channel needs `{k}`"));
            let ch = Channel {
                device_id: s("device_id")?,
                class_name: s("class_name")?,
                channel: c.get("channel").and_then(|v| v.as_u64()).filter(|n| (1..=9).contains(n)).ok_or("channel number 1-9 required")? as u8,
                method_name: s("method_name")?,
            };
            let (def, _, _) = self.devices.lookup(&ch.device_id).ok_or(format!("unknown device {:?}", ch.device_id))?;
            let readable = def.classes.get(&ch.class_name).and_then(|b| b.operations.get(&ch.method_name)).is_some_and(|o| o.get.is_some());
            if !readable {
                return Err(format!("{} {} has no readable operation {:?}", ch.device_id, ch.class_name, ch.method_name));
            }
            out.push(ch);
        }
        Ok(out)
    }

    fn start(self: &Arc<Self>, name: &str, description: Option<&str>, interval: f64, channels: &Value) -> Result<Series, ApiError> {
        let chans = self.validate_channels(channels).map_err(ApiError::Invalid)?;
        if !(0.1..=86_400.0).contains(&interval) {
            return Err(ApiError::Invalid("interval_seconds must be between 0.1 and 86400".into()));
        }
        let name = name.trim();
        if name.is_empty() || name.len() > 200 {
            return Err(ApiError::Invalid("name must be 1-200 characters".into()));
        }
        let series = self.store.create_series(name, description, interval, channels)?;
        let paused = Arc::new(AtomicBool::new(false));
        let (tx, _) = broadcast::channel(256);
        let task = tokio::spawn(sample_loop(self.clone(), series.clone(), chans, paused.clone(), tx.clone()));
        self.active.lock().unwrap().insert(series.id, Active { paused, task, tx });
        Ok(series)
    }

    fn pause(&self, id: i64) -> Result<Series, ApiError> {
        let active = self.active.lock().unwrap();
        let a = active.get(&id).ok_or(ApiError::NotFound(format!("Recording {id} is not active")))?;
        let s = self.store.series(id)?;
        a.paused.store(true, Ordering::SeqCst);
        if s.paused_at.is_some() {
            return Ok(s); // already paused: keep the original pause time
        }
        Ok(self.store.update_state(id, Some(now()), None, 0.0)?)
    }

    fn resume(&self, id: i64) -> Result<Series, ApiError> {
        let active = self.active.lock().unwrap();
        let a = active.get(&id).ok_or(ApiError::NotFound(format!("Recording {id} is not active")))?;
        let s = self.store.series(id)?;
        let paused_for = s.paused_at.map(|p| (now() - p).num_microseconds().unwrap_or(0) as f64 / 1e6).unwrap_or(0.0);
        let s = self.store.update_state(id, None, None, paused_for)?;
        a.paused.store(false, Ordering::SeqCst);
        Ok(s)
    }

    fn stop(&self, id: i64) -> Result<Series, ApiError> {
        if let Some(a) = self.active.lock().unwrap().remove(&id) {
            a.task.abort();
        }
        let s = self.store.series(id)?;
        if s.end_time.is_some() {
            return Ok(s); // already stopped
        }
        let t = now();
        let open_pause = s.paused_at.map(|p| (t - p).num_microseconds().unwrap_or(0) as f64 / 1e6).unwrap_or(0.0);
        Ok(self.store.update_state(id, None, Some(t), open_pause)?)
    }

    fn subscribe(&self, id: i64) -> Option<broadcast::Receiver<String>> {
        self.active.lock().unwrap().get(&id).map(|a| a.tx.subscribe())
    }
}

async fn sample_loop(rec: Arc<Recordings>, series: Series, chans: Vec<Channel>, paused: Arc<AtomicBool>, tx: broadcast::Sender<String>) {
    let mut tick = tokio::time::interval(Duration::from_secs_f64(series.interval_seconds));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        if paused.load(Ordering::SeqCst) {
            continue;
        }
        let mut m = Map::new();
        for ch in &chans {
            let Some((_, client, _)) = rec.devices.lookup(&ch.device_id) else { continue };
            let c = ch.clone();
            let r = tokio::task::spawn_blocking(move || client.get(&c.class_name, &c.method_name, c.channel)).await;
            if let Ok(Ok(Output::Value(v))) = r {
                if let Some(v) = sample_value(v) {
                    m.insert(ch.key(), v);
                }
            }
        }
        if m.is_empty() {
            continue; // nothing readable (device offline): no point, like the Python service
        }
        let ts = now();
        let store = rec.store.clone();
        let measurements = Value::Object(m);
        let saved = tokio::task::spawn_blocking({
            let measurements = measurements.clone();
            move || store.add_point(series.id, ts, &measurements)
        })
        .await;
        if let Ok(Ok(p)) = saved {
            let _ = tx.send(json!({"type": "data_point", "series_id": series.id, "timestamp": iso(&p.timestamp),
                "elapsed_seconds": elapsed(&series, &p), "measurements": measurements}).to_string());
        }
    }
}

fn elapsed(s: &Series, p: &DataPoint) -> f64 {
    (p.timestamp - s.start_time).num_microseconds().unwrap_or(0) as f64 / 1e6
}

// ---- API ----------------------------------------------------------------------------

pub enum ApiError {
    Invalid(String),
    NotFound(String),
    Duplicate(String),
    Internal(String),
}

impl From<StoreError> for ApiError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::DuplicateName(_) => ApiError::Duplicate(e.to_string()),
            StoreError::NotFound(_) => ApiError::NotFound(e.to_string()),
            StoreError::Db(m) => ApiError::Internal(m),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (code, msg) = match self {
            ApiError::Invalid(m) => (StatusCode::UNPROCESSABLE_ENTITY, m),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            ApiError::Duplicate(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (code, Json(json!({"detail": msg}))).into_response()
    }
}

fn opt_iso(t: &Option<chrono::NaiveDateTime>) -> Value {
    t.as_ref().map(|t| json!(iso(t))).unwrap_or(Value::Null)
}

fn series_json(s: &Series) -> Value {
    json!({
        "id": s.id, "name": s.name, "description": s.description, "interval_seconds": s.interval_seconds,
        "channels": s.channels, "start_time": iso(&s.start_time), "end_time": opt_iso(&s.end_time),
        "paused_at": opt_iso(&s.paused_at), "pause_duration_seconds": s.pause_duration_seconds,
        "total_duration_seconds": s.total_duration_seconds(now()), "data_points_count": s.data_points_count,
        "status": s.status(),
    })
}

pub fn routes() -> Router<S> {
    Router::new()
        .route("/recordings", get(list))
        .route("/recordings/start", post(start))
        .route("/recordings/{id}", get(detail).delete(remove))
        .route("/recordings/{id}/pause", post(pause))
        .route("/recordings/{id}/resume", post(resume))
        .route("/recordings/{id}/stop", post(stop))
        .route("/recordings/{id}/data", get(data))
        .route("/recordings/{id}/export", get(export))
        .route("/recordings/ws/{id}", get(ws))
}

async fn start(State(s): State<S>, Json(b): Json<Value>) -> Result<Json<Value>, ApiError> {
    let name = b.get("name").and_then(|v| v.as_str()).ok_or(ApiError::Invalid("`name` is required".into()))?;
    let interval = b.get("interval_seconds").and_then(|v| v.as_f64()).ok_or(ApiError::Invalid("`interval_seconds` is required".into()))?;
    let description = b.get("description").and_then(|v| v.as_str());
    let channels = b.get("channels").cloned().unwrap_or(Value::Null);
    let series = s.recordings.start(name, description, interval, &channels)?;
    Ok(Json(json!({"series_id": series.id, "name": series.name, "status": "recording"})))
}

async fn pause(State(s): State<S>, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    let x = s.recordings.pause(id)?;
    Ok(Json(json!({"series_id": id, "name": x.name, "status": "paused", "paused_at": opt_iso(&x.paused_at)})))
}

async fn resume(State(s): State<S>, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    let x = s.recordings.resume(id)?;
    Ok(Json(json!({"series_id": id, "name": x.name, "status": "recording", "paused_at": null})))
}

async fn stop(State(s): State<S>, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    let x = s.recordings.stop(id)?;
    Ok(Json(json!({"series_id": id, "name": x.name, "status": "stopped", "data_points_count": x.data_points_count,
        "duration_seconds": x.total_duration_seconds(now()), "pause_duration_seconds": x.pause_duration_seconds})))
}

async fn list(State(s): State<S>) -> Result<Json<Value>, ApiError> {
    let all = s.recordings.store.list()?;
    Ok(Json(json!({"recordings": all.iter().map(series_json).collect::<Vec<_>>()})))
}

async fn detail(State(s): State<S>, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    let x = s.recordings.store.series(id)?;
    Ok(Json(json!({"series": series_json(&x), "total_points": x.data_points_count})))
}

async fn remove(State(s): State<S>, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    s.recordings.store.series(id)?;
    let _ = s.recordings.stop(id);
    s.recordings.store.delete(id)?;
    Ok(Json(json!({"status": "deleted", "series_id": id})))
}

async fn data(State(s): State<S>, Path(id): Path<i64>, Query(q): Query<HashMap<String, String>>) -> Result<Json<Value>, ApiError> {
    let num = |k: &str, d: i64| q.get(k).map(|v| v.parse::<i64>().map_err(|_| ApiError::Invalid(format!("`{k}` must be an integer")))).unwrap_or(Ok(d));
    let offset = num("offset", 0)?;
    let limit = num("limit", 1000)?;
    if offset < 0 || !(1..=100_000).contains(&limit) {
        return Err(ApiError::Invalid("offset must be >= 0 and limit 1-100000".into()));
    }
    let x = s.recordings.store.series(id)?;
    let pts = s.recordings.store.points(id, offset, limit)?;
    let points: Vec<Value> = pts
        .iter()
        .map(|p| json!({"id": p.id, "series_id": p.series_id, "timestamp": iso(&p.timestamp),
            "elapsed_seconds": elapsed(&x, p), "measurements": p.measurements}))
        .collect();
    Ok(Json(json!({"series_id": id, "total_points": x.data_points_count, "offset": offset, "limit": limit, "data_points": points})))
}

fn csv_field(v: &Value) -> String {
    let s = match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

async fn export(State(s): State<S>, Path(id): Path<i64>) -> Result<Response, ApiError> {
    let x = s.recordings.store.series(id)?;
    let pts = s.recordings.store.points(id, 0, i64::MAX)?;
    let mut keys: Vec<String> = pts
        .iter()
        .flat_map(|p| p.measurements.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default())
        .collect();
    keys.sort();
    keys.dedup();
    if keys.is_empty() {
        keys = x.channels.as_array().into_iter().flatten().map(|c| {
            format!("{}_{}_{}_{}", c["device_id"].as_str().unwrap_or(""), c["class_name"].as_str().unwrap_or(""),
                c["channel"], c["method_name"].as_str().unwrap_or(""))
        }).collect();
    }
    let mut out = String::from("timestamp");
    for k in &keys {
        out.push(',');
        out.push_str(&csv_field(&json!(k)));
    }
    out.push_str("\r\n");
    for p in &pts {
        out.push_str(&iso(&p.timestamp));
        for k in &keys {
            out.push(',');
            out.push_str(&csv_field(p.measurements.get(k).unwrap_or(&Value::Null)));
        }
        out.push_str("\r\n");
    }
    let filename = format!("{}.csv", x.name.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '.', "_"));
    Ok((
        [(header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
         (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\""))],
        out,
    )
        .into_response())
}

async fn ws(State(s): State<S>, Path(id): Path<i64>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| live(s, id, socket))
}

async fn live(s: S, id: i64, mut socket: WebSocket) {
    if s.recordings.store.series(id).is_err() {
        let _ = socket
            .send(Message::Close(Some(axum::extract::ws::CloseFrame { code: 1008, reason: "Recording not found".into() })))
            .await;
        return;
    }
    let mut rx = s.recordings.subscribe(id);
    loop {
        tokio::select! {
            msg = socket.recv() => match msg {
                Some(Ok(Message::Text(_))) => {
                    if socket.send(Message::Text(json!({"type": "pong"}).to_string().into())).await.is_err() { return; }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                _ => {}
            },
            point = async { match rx.as_mut() { Some(r) => r.recv().await.ok(), None => std::future::pending().await } } => {
                match point {
                    Some(p) => if socket.send(Message::Text(p.into())).await.is_err() { return; },
                    None => rx = None, // recording stopped (sender gone)
                }
            }
        }
    }
}
