//! The computer BenchMesh runs on (contract `ai_context/HEADLESS.md`): its resources for the UI's
//! System tab, and - when BenchMesh runs headless, without an app window - restart, shut down and
//! start with the computer, so a bench computer reached only through remote access can be looked
//! after from the browser. In the desktop app the window owns the process: these answer 409.
//! An extension of the distribution (`extension.rs`) may hand the process over the same way, e.g.
//! to install a new version once it has stopped (`System::hand_over`).

pub mod autostart;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use bm_store::now_ms;
use serde_json::{json, Value};
use sysinfo::{Components, Disks, ProcessRefreshKind, ProcessesToUpdate, RefreshKind};
use tokio::sync::watch;

use crate::{detail, S};

/// What the headless process does once it has stopped cleanly, after a hand-over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AfterStop {
    /// Start the same command line again, with this program instead of the running one if given.
    Restart(Option<PathBuf>),
    /// Exit with this code.
    Exit(i32),
}

/// Runs after the devices were stopped and everything was released.
pub type HandOver = Arc<dyn Fn() -> AfterStop + Send + Sync>;

/// What the UI asked the headless process to do.
#[derive(Clone)]
pub enum Action {
    /// Stop cleanly and start again with the same command line.
    Restart,
    /// Stop cleanly and exit.
    Shutdown,
    /// Stop cleanly, then let an extension decide (`System::hand_over`).
    HandOver(HandOver),
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Action::Restart => "Restart",
            Action::Shutdown => "Shutdown",
            Action::HandOver(_) => "HandOver",
        })
    }
}

/// Running headless: the process may restart / stop itself and register itself to start with
/// the computer.
#[derive(Debug, Clone)]
pub struct Headless {
    /// How to start this program headless again (the autostart entry); `None` for a simulation,
    /// which never starts with the computer.
    pub launch: Option<autostart::Launch>,
}

impl Default for System {
    fn default() -> Self {
        System::new(None, None)
    }
}

const DESKTOP: &str = "this BenchMesh runs in the desktop app: close the app window to stop it";
const SIMULATION: &str = "a simulation does not start with the computer";

pub struct System {
    probe: Mutex<sysinfo::System>,
    data_dir: Option<PathBuf>,
    headless: Option<Headless>,
    started_ms: i64,
    action: Arc<watch::Sender<Option<Action>>>,
}

impl System {
    pub fn new(data_dir: Option<PathBuf>, headless: Option<Headless>) -> System {
        let mut probe = sysinfo::System::new_with_specifics(RefreshKind::nothing());
        // the first CPU figure is a difference to this one
        probe.refresh_cpu_usage();
        refresh_own_process(&mut probe);
        System { probe: Mutex::new(probe), data_dir, headless, started_ms: now_ms(), action: Arc::new(watch::channel(None).0) }
    }

    pub fn is_headless(&self) -> bool {
        self.headless.is_some()
    }

    /// Completes with the action the UI asked for (headless only; never otherwise).
    pub async fn requested(&self) -> Action {
        let mut rx = self.action.subscribe();
        loop {
            if let Some(a) = rx.borrow_and_update().clone() {
                return a;
            }
            if rx.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
        }
    }

    fn request(&self, a: Action) -> Result<(), &'static str> {
        if self.headless.is_none() {
            return Err(DESKTOP);
        }
        self.action.send_replace(Some(a));
        Ok(())
    }

    /// Stop the headless process cleanly and run `then` (the desktop app: refused, its window owns
    /// the process).
    pub fn hand_over(&self, then: HandOver) -> Result<(), &'static str> {
        self.request(Action::HandOver(then))
    }

    /// Resources of this computer and of the BenchMesh process.
    pub fn snapshot(&self) -> Value {
        let mut p = self.probe.lock().unwrap();
        p.refresh_cpu_usage();
        p.refresh_memory();
        refresh_own_process(&mut p);
        let cpus = p.cpus().len().max(1);
        let own = sysinfo::get_current_pid().ok().and_then(|pid| p.process(pid));
        let load = sysinfo::System::load_average();
        json!({
            "host": sysinfo::System::host_name(),
            "os": sysinfo::System::long_os_version(),
            "arch": sysinfo::System::cpu_arch(),
            "cpus": cpus,
            "uptime_s": sysinfo::System::uptime(),
            // Windows has no load average (all zero there)
            "load": (!cfg!(windows)).then_some([load.one, load.five, load.fifteen]),
            "cpu_percent": round1(p.global_cpu_usage()),
            "memory": {"total": p.total_memory(), "used": p.used_memory()},
            "process": {
                "pid": std::process::id(),
                "memory": own.map(|o| o.memory()),
                // per core in sysinfo: as a share of the whole computer
                "cpu_percent": own.map(|o| round1(o.cpu_usage() / cpus as f32)),
                "started_ms": self.started_ms,
            },
            "disk": self.data_dir.as_deref().and_then(disk_of),
            "temperature_c": temperature(),
            "headless": self.is_headless(),
            "version": env!("CARGO_PKG_VERSION"),
        })
    }
}

fn refresh_own_process(p: &mut sysinfo::System) {
    if let Ok(pid) = sysinfo::get_current_pid() {
        p.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing().with_cpu().with_memory());
    }
}

fn round1(v: f32) -> f64 {
    (v as f64 * 10.0).round() / 10.0
}

/// The disk holding `dir` (the data directory: recordings grow there).
fn disk_of(dir: &Path) -> Option<Value> {
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let disks = Disks::new_with_refreshed_list();
    let d = disks.list().iter().filter(|d| dir.starts_with(d.mount_point())).max_by_key(|d| d.mount_point().as_os_str().len())?;
    Some(json!({"path": dir.display().to_string(), "total": d.total_space(), "available": d.available_space()}))
}

/// The processor's temperature where the OS reports one (Linux incl. Raspberry Pi; often none
/// on Windows and macOS).
fn temperature() -> Option<f64> {
    let components = Components::new_with_refreshed_list();
    let readings: Vec<(String, f32)> = components.list().iter().filter_map(|c| Some((c.label().to_string(), c.temperature()?))).collect();
    cpu_temperature(&readings).map(round1)
}

/// The hottest processor sensor, else the hottest sensor.
fn cpu_temperature(readings: &[(String, f32)]) -> Option<f32> {
    let sane = |t: &f32| t.is_finite() && *t > -40.0 && *t < 150.0;
    let cpu = |l: &str| ["cpu", "package", "soc", "core", "tctl", "k10temp"].iter().any(|k| l.to_lowercase().contains(k));
    let max = |it: &mut dyn Iterator<Item = f32>| it.fold(None, |m: Option<f32>, t| Some(m.map_or(t, |m| m.max(t))));
    max(&mut readings.iter().filter(|(l, t)| cpu(l) && sane(t)).map(|(_, t)| *t)).or_else(|| max(&mut readings.iter().filter(|(_, t)| sane(t)).map(|(_, t)| *t)))
}

// ---- routes ---------------------------------------------------------------------------

pub fn routes() -> Router<S> {
    Router::new()
        .route("/system", get(get_system))
        .route("/system/restart", post(restart))
        .route("/system/shutdown", post(shutdown))
        .route("/system/autostart", get(get_autostart).put(put_autostart))
}

async fn get_system(State(s): State<S>) -> Json<Value> {
    let sys = s.system.clone();
    Json(tokio::task::spawn_blocking(move || sys.snapshot()).await.unwrap_or_else(|_| json!({})))
}

/// A flow switches outputs and cleans up after itself: never stopped from under it (409).
pub fn refused_while_running(s: &S) -> Option<Response> {
    let state = s.automation.run_status(u64::MAX)["state"].as_str().unwrap_or("idle").to_string();
    matches!(state.as_str(), "running" | "waiting" | "cleanup")
        .then(|| detail(StatusCode::CONFLICT, "a flow is running: stop it first (its cleanup switches off what it switched on)"))
}

async fn act(s: S, a: Action) -> Response {
    if let Some(r) = refused_while_running(&s) {
        return r;
    }
    match s.system.request(a.clone()) {
        Ok(()) => {
            eprintln!("system: {a:?} requested from the UI");
            (StatusCode::ACCEPTED, Json(json!({"action": format!("{a:?}").to_lowercase()}))).into_response()
        }
        Err(why) => detail(StatusCode::CONFLICT, why),
    }
}

async fn restart(State(s): State<S>) -> Response {
    act(s, Action::Restart).await
}

async fn shutdown(State(s): State<S>) -> Response {
    act(s, Action::Shutdown).await
}

fn autostart_json(s: &S) -> Value {
    match &s.system.headless {
        Some(Headless { launch: Some(l), .. }) => autostart::status(l).json(),
        Some(Headless { launch: None, .. }) => autostart::Status::unsupported(SIMULATION).json(),
        None => autostart::Status::unsupported(DESKTOP).json(),
    }
}

async fn get_autostart(State(s): State<S>) -> Json<Value> {
    let s2 = s.clone();
    Json(tokio::task::spawn_blocking(move || autostart_json(&s2)).await.unwrap_or_else(|_| json!({})))
}

async fn put_autostart(State(s): State<S>, body: Option<Json<Value>>) -> Response {
    let Some(enabled) = body.as_ref().and_then(|b| b.get("enabled")).and_then(Value::as_bool) else {
        return detail(StatusCode::BAD_REQUEST, "expected {\"enabled\": true|false}");
    };
    let launch = match &s.system.headless {
        Some(Headless { launch: Some(l), .. }) => l.clone(),
        Some(Headless { launch: None, .. }) => return detail(StatusCode::CONFLICT, SIMULATION),
        None => return detail(StatusCode::CONFLICT, DESKTOP),
    };
    let r = tokio::task::spawn_blocking(move || if enabled { autostart::enable(&launch) } else { autostart::disable() }).await;
    match r {
        Ok(Ok(())) => {
            eprintln!("system: start with the computer {}", if enabled { "on" } else { "off" });
            Json(autostart_json(&s)).into_response()
        }
        Ok(Err(e)) => {
            eprintln!("system: start with the computer: {e}");
            detail(StatusCode::INTERNAL_SERVER_ERROR, e)
        }
        Err(e) => detail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_processor_temperature_is_the_hottest_cpu_sensor() {
        let r = |v: &[(&str, f32)]| v.iter().map(|(l, t)| (l.to_string(), *t)).collect::<Vec<_>>();
        // Raspberry Pi
        assert_eq!(cpu_temperature(&r(&[("cpu_thermal temp1", 51.5)])), Some(51.5));
        assert_eq!(cpu_temperature(&r(&[("nvme Composite", 60.0), ("coretemp Core 0", 45.0), ("coretemp Package id 0", 48.0)])), Some(48.0));
        assert_eq!(cpu_temperature(&r(&[("acpitz temp1", 40.0)])), Some(40.0), "no processor sensor: the hottest one");
        assert_eq!(cpu_temperature(&r(&[("cpu", f32::NAN), ("bogus", 1000.0)])), None);
        assert_eq!(cpu_temperature(&[]), None);
    }

    #[test]
    fn the_desktop_app_cannot_be_restarted_from_the_ui() {
        let s = System::new(None, None);
        assert_eq!(s.request(Action::Restart).unwrap_err(), DESKTOP);
        assert_eq!(s.hand_over(std::sync::Arc::new(|| AfterStop::Exit(0))).unwrap_err(), DESKTOP);
        assert!(s.action.borrow().is_none());
    }

    #[tokio::test]
    async fn a_headless_process_hears_the_request() {
        let launch = autostart::Launch { program: "/opt/benchmesh".into(), args: vec!["--headless".into()] };
        let s = std::sync::Arc::new(System::new(None, Some(Headless { launch: Some(launch) })));
        let waiting = tokio::spawn({
            let s = s.clone();
            async move { s.requested().await }
        });
        tokio::task::yield_now().await;
        s.request(Action::Shutdown).unwrap();
        assert!(matches!(waiting.await.unwrap(), Action::Shutdown));
    }

    #[tokio::test]
    async fn an_extension_hands_the_headless_process_over() {
        let s = System::new(None, Some(Headless { launch: None }));
        s.hand_over(std::sync::Arc::new(|| AfterStop::Restart(Some("/opt/new/benchmesh".into())))).unwrap();
        let Action::HandOver(then) = s.requested().await else { panic!("not a hand-over") };
        assert_eq!(then(), AfterStop::Restart(Some("/opt/new/benchmesh".into())));
    }

    #[test]
    fn the_snapshot_describes_this_computer_and_process() {
        let dir = std::env::temp_dir();
        let v = System::new(Some(dir), None).snapshot();
        assert!(v["cpus"].as_u64().unwrap() >= 1);
        assert!(v["memory"]["total"].as_u64().unwrap() > 0);
        assert_eq!(v["process"]["pid"], std::process::id());
        assert!(v["process"]["memory"].as_u64().unwrap() > 0);
        assert_eq!(v["headless"], false);
        assert!(v["disk"]["total"].as_u64().is_some_and(|t| t > 0), "{v}");
    }
}
