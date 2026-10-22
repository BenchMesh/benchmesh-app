//! Virtual SCPI instruments (contract `ai_context/SCPI.md`): every configured instrument gets a
//! raw SCPI socket on this computer (`TCPIP0::127.0.0.1::<port>::SOCKET`), so VISA programs -
//! MATLAB, LabVIEW, PyVISA, TestStand - use it like a LAN instrument, whatever its own link
//! (serial, USBTMC) and dialect. Off by default; access `read` (queries) or `control` (also
//! setters). Commands run through the same device thread as the REST API.

pub mod dialect;
pub mod session;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock, Weak};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::task::{JoinHandle, JoinSet};

use crate::config::str_field;
use crate::remote::Principal;
use crate::{detail, AppState, S};

pub const DEFAULT_BASE_PORT: u16 = 5025;
/// Clients at a time per instrument.
const MAX_CONNECTIONS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Off,
    Read,
    Control,
}

impl Access {
    fn parse(s: &str) -> Option<Access> {
        match s {
            "off" => Some(Access::Off),
            "read" => Some(Access::Read),
            "control" => Some(Access::Control),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
struct Settings {
    /// `off` (default), `read` or `control`.
    access: String,
    /// The first instrument's port; the others follow.
    base_port: u16,
    /// Port per device id: kept while the device is configured, so addresses stay put.
    ports: BTreeMap<String, u16>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { access: "off".into(), base_port: DEFAULT_BASE_PORT, ports: BTreeMap::new() }
    }
}

impl Settings {
    /// Keep the ports of configured devices, drop the others, give new devices the lowest free
    /// port from `base_port`. True when something changed.
    fn assign(&mut self, ids: &[String]) -> bool {
        let before = self.ports.clone();
        self.ports.retain(|id, _| ids.contains(id));
        for id in ids {
            if self.ports.contains_key(id) {
                continue;
            }
            let used: Vec<u16> = self.ports.values().copied().collect();
            if let Some(p) = (self.base_port..=u16::MAX).find(|p| !used.contains(p)) {
                self.ports.insert(id.clone(), p);
            }
        }
        self.ports != before
    }
}

struct Listener {
    port: u16,
    task: Option<JoinHandle<()>>,
    error: Option<String>,
}

pub struct Scpi {
    /// `scpi.json` in the data dir; `None` = in memory (simulations, tests).
    path: Option<PathBuf>,
    settings: Mutex<Settings>,
    listeners: Mutex<BTreeMap<String, Listener>>,
    serve: OnceLock<(Weak<AppState>, tokio::runtime::Handle)>,
}

impl Default for Scpi {
    fn default() -> Self {
        Scpi::new(None)
    }
}

pub fn address(port: u16) -> String {
    format!("TCPIP0::127.0.0.1::{port}::SOCKET")
}

impl Scpi {
    pub fn new(data_dir: Option<PathBuf>) -> Scpi {
        let path = data_dir.map(|d| d.join("scpi.json"));
        let settings = path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| match serde_json::from_slice(&b) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("scpi settings: {e} (using defaults)");
                    None
                }
            })
            .unwrap_or_default();
        Scpi { path, settings: Mutex::new(settings), listeners: Default::default(), serve: OnceLock::new() }
    }

    pub fn access(&self) -> Access {
        Access::parse(&self.settings.lock().unwrap().access).unwrap_or(Access::Off)
    }

    /// Give the server state and runtime; opens the listeners when access is on.
    pub fn attach(&self, state: Weak<AppState>, rt: tokio::runtime::Handle) {
        let _ = self.serve.set((state, rt));
        self.sync();
    }

    fn save(&self, s: &Settings) -> Result<(), String> {
        match &self.path {
            Some(p) => crate::automation::store::write_private(p, &serde_json::to_vec_pretty(s).unwrap()),
            None => Ok(()),
        }
    }

    /// Match ports and listeners to the configured devices and the access setting (after every
    /// configuration change).
    pub fn sync(&self) {
        let Some((state, rt)) = self.serve.get() else { return };
        let Some(state) = state.upgrade() else { return };
        let ids: Vec<String> = state.devices.entries().iter().filter_map(|e| str_field(e, "id").map(str::to_string)).collect();
        let settings = {
            let mut s = self.settings.lock().unwrap();
            if s.assign(&ids) {
                if let Err(e) = self.save(&s) {
                    eprintln!("scpi settings: {e}");
                }
            }
            s.clone()
        };
        let on = Access::parse(&settings.access).unwrap_or(Access::Off) != Access::Off;
        let mut ls = self.listeners.lock().unwrap();
        // close what is gone, moved, off or died
        ls.retain(|id, l| {
            let keep = on && settings.ports.get(id) == Some(&l.port) && l.task.as_ref().is_some_and(|t| !t.is_finished());
            if !keep {
                if let Some(t) = l.task.take() {
                    t.abort();
                }
            }
            keep
        });
        if !on {
            return;
        }
        for (id, &port) in &settings.ports {
            if ls.contains_key(id) {
                continue;
            }
            let l = match listen(std::sync::Arc::downgrade(&state), rt, id.clone(), port) {
                Ok(task) => {
                    eprintln!("scpi: {id} on {}", address(port));
                    Listener { port, task: Some(task), error: None }
                }
                Err(e) => {
                    eprintln!("scpi: {id}: {e}");
                    Listener { port, task: None, error: Some(e) }
                }
            };
            ls.insert(id.clone(), l);
        }
    }

    /// Close every listener and connection (server shutdown).
    pub fn stop(&self) {
        for (_, mut l) in std::mem::take(&mut *self.listeners.lock().unwrap()) {
            if let Some(t) = l.task.take() {
                t.abort();
            }
        }
    }

    pub fn status(&self, s: &AppState) -> Value {
        let settings = self.settings.lock().unwrap().clone();
        let ls = self.listeners.lock().unwrap();
        let instruments: Vec<Value> = s
            .devices
            .entries()
            .iter()
            .filter_map(|e| {
                let id = str_field(e, "id")?;
                let port = settings.ports.get(id).copied();
                let l = ls.get(id);
                Some(json!({
                    "id": id,
                    "name": str_field(e, "name").unwrap_or(id),
                    "port": port,
                    "address": port.map(address),
                    "listening": l.is_some_and(|l| l.task.is_some()),
                    "error": l.and_then(|l| l.error.clone()),
                }))
            })
            .collect();
        json!({"access": settings.access, "base_port": settings.base_port, "host": "127.0.0.1", "instruments": instruments})
    }

    fn update(&self, access: Option<&str>, base_port: Option<u16>) -> Result<(), String> {
        let next = {
            let mut s = self.settings.lock().unwrap().clone();
            if let Some(a) = access {
                s.access = a.to_string();
            }
            if let Some(p) = base_port.filter(|p| *p != s.base_port) {
                s.base_port = p;
                s.ports.clear(); // renumbered from the new base by `sync`
            }
            s
        };
        self.save(&next)?;
        *self.settings.lock().unwrap() = next;
        self.sync();
        Ok(())
    }
}

/// Accept loop of one instrument's port (this computer only); dropping it ends its connections.
fn listen(state: Weak<AppState>, rt: &tokio::runtime::Handle, id: String, port: u16) -> Result<JoinHandle<()>, String> {
    let std = std::net::TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("cannot listen on port {port}: {e}"))?;
    std.set_nonblocking(true).map_err(|e| e.to_string())?;
    let _enter = rt.enter();
    let tcp = tokio::net::TcpListener::from_std(std).map_err(|e| e.to_string())?;
    Ok(rt.spawn(async move {
        let mut conns = JoinSet::new();
        loop {
            tokio::select! {
                a = tcp.accept() => match a {
                    Ok((stream, peer)) => {
                        if !peer.ip().is_loopback() || conns.len() >= MAX_CONNECTIONS {
                            continue;
                        }
                        let _ = stream.set_nodelay(true);
                        let Some(s) = state.upgrade() else { return };
                        conns.spawn(session::serve(s, id.clone(), stream));
                    }
                    Err(e) => {
                        eprintln!("scpi {id}: accept: {e}");
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                },
                Some(_) = conns.join_next(), if !conns.is_empty() => {}
            }
        }
    }))
}

// ---- routes -----------------------------------------------------------------------------

pub fn routes() -> Router<S> {
    Router::new()
        .route("/scpi", get(get_status).put(put_settings))
}

/// This computer only, whatever the remote gate did.
fn local_only(p: &Principal) -> Option<Response> {
    (!matches!(p, Principal::Local)).then(|| detail(StatusCode::FORBIDDEN, "virtual SCPI ports can be set up only in the BenchMesh app on this computer"))
}

async fn get_status(State(s): State<S>, p: Principal) -> Response {
    local_only(&p).unwrap_or_else(|| Json(s.scpi.status(&s)).into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsBody {
    access: Option<String>,
    base_port: Option<u32>,
}

async fn put_settings(State(s): State<S>, p: Principal, body: axum::body::Bytes) -> Response {
    if let Some(r) = local_only(&p) {
        return r;
    }
    let b: SettingsBody = match serde_json::from_slice(&body) {
        Ok(b) => b,
        Err(e) => return detail(StatusCode::UNPROCESSABLE_ENTITY, format!("body must be {{access?: off|read|control, base_port?: 1024-65000}}: {e}")),
    };
    if let Some(a) = b.access.as_deref().filter(|a| Access::parse(a).is_none()) {
        return detail(StatusCode::UNPROCESSABLE_ENTITY, format!("access {a:?}: use off, read or control"));
    }
    let port = match b.base_port {
        Some(p @ 1024..=65000) => Some(p as u16),
        Some(p) => return detail(StatusCode::UNPROCESSABLE_ENTITY, format!("base_port {p}: use 1024-65000")),
        None => None,
    };
    let scpi = &s.scpi;
    let access = b.access.clone();
    // binding sockets is quick but blocking
    let s2 = s.clone();
    let r = tokio::task::spawn_blocking(move || s2.scpi.update(access.as_deref(), port)).await;
    match r {
        Ok(Ok(())) => Json(scpi.status(&s)).into_response(),
        Ok(Err(e)) => detail(StatusCode::INTERNAL_SERVER_ERROR, format!("saving SCPI settings: {e}")),
        Err(e) => detail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports_stay_with_their_instruments_and_new_ones_take_the_lowest_free() {
        let mut s = Settings::default();
        let ids = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert!(s.assign(&ids(&["a", "b", "c"])));
        assert_eq!(s.ports, BTreeMap::from([("a".into(), 5025), ("b".into(), 5026), ("c".into(), 5027)]));
        assert!(!s.assign(&ids(&["a", "b", "c"])), "nothing changes");
        assert!(s.assign(&ids(&["a", "c", "d"])));
        assert_eq!(s.ports["c"], 5027, "c keeps its port");
        assert_eq!(s.ports["d"], 5026, "d takes the freed one");
        assert!(!s.ports.contains_key("b"));
    }

    #[test]
    fn settings_default_to_off_and_unknown_access_counts_as_off() {
        let s = Scpi::new(None);
        assert_eq!(s.access(), Access::Off);
        s.settings.lock().unwrap().access = "everything".into();
        assert_eq!(s.access(), Access::Off);
    }
}
