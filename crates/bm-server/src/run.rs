//! Start/stop the server as a library (used by `benchmesh-server` and the desktop app).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bm_driver::actor::ActorConfig;

use crate::config::{validate_entry, ConfigFile};
use crate::devices::{hardware_connectors, simulated_connectors, Devices};
use crate::{router, AppState};

#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// config.yaml (created on first save if missing).
    pub config: PathBuf,
    /// Device definitions directory (`devices/`).
    pub devices_dir: PathBuf,
    /// Built UI (`index.html` + assets) served under /ui, if any.
    pub ui_dir: Option<PathBuf>,
    /// Listen addresses tried in order (e.g. `127.0.0.1:57666`, then `127.0.0.1:0`).
    pub listen: Vec<String>,
    /// Recordings database (`None` = in memory, nothing persisted).
    pub recordings_db: Option<PathBuf>,
    /// Replay this capture session instead of opening real ports (no hardware needed).
    pub simulate: Option<PathBuf>,
    pub ws_interval: Duration,
}

pub struct RunningServer {
    pub addr: SocketAddr,
    devices: Arc<Devices>,
    recordings: Arc<crate::recordings::Recordings>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Problems with individual config entries (devices skipped at start).
    pub warnings: Vec<String>,
}

impl RunningServer {
    /// Base URL for local clients.
    pub fn url(&self) -> String {
        let host = if self.addr.ip().is_unspecified() { "127.0.0.1".to_string() } else { self.addr.ip().to_string() };
        format!("http://{host}:{}", self.addr.port())
    }

    /// Stop serving, then stop every device (runs the definitions' on_disconnect steps).
    pub fn stop(mut self) {
        self.shutdown_now();
    }

    fn shutdown_now(&mut self) {
        self.recordings.shutdown();
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        self.devices.stop_all();
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.shutdown_now();
    }
}

/// Load definitions + config, start one device thread per configured device and serve the
/// API/UI on a dedicated runtime thread. Returns once the listener is bound.
pub fn start(opts: ServerOptions) -> Result<RunningServer, String> {
    let catalog = bm_devicedef::load_catalog(&opts.devices_dir)
        .map_err(|e| format!("device definitions in {}:\n{}", opts.devices_dir.display(), e.join("\n")))?;
    let cfg = ConfigFile { path: opts.config.clone() };
    let entries = cfg.load()?;
    let connectors = match &opts.simulate {
        Some(session) => simulated_connectors(session.clone()),
        None => hardware_connectors(),
    };
    let devices = Arc::new(Devices::new(catalog, connectors, ActorConfig::default()));
    let mut warnings = Vec::new();
    for e in entries {
        let known = devices.driver_ids();
        match validate_entry(&e, &known).and_then(|_| devices.start(e.clone())) {
            Ok(()) => {}
            Err(err) => warnings.push(format!("device {:?} not started: {err}", e.get("id"))),
        }
    }
    let store = match &opts.recordings_db {
        Some(p) => bm_store::Store::open(p),
        None => bm_store::Store::in_memory(),
    }
    .map_err(|e| format!("recordings database: {e}"))?;
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_name("bm-http").build().map_err(|e| e.to_string())?;
    let recordings = {
        let _guard = rt.enter(); // recording tasks are spawned on this runtime
        crate::recordings::Recordings::new(Arc::new(store), devices.clone())
    };
    let state = Arc::new(AppState {
        devices: devices.clone(),
        recordings: recordings.clone(),
        config: cfg,
        ui_dir: opts.ui_dir.clone().filter(|p| p.join("index.html").exists()),
        ws_interval: opts.ws_interval,
        metrics_interval: Duration::from_secs(30),
        config_lock: Default::default(),
    });

    let listener = rt.block_on(async {
        let mut last = String::from("no listen address given");
        for a in &opts.listen {
            match tokio::net::TcpListener::bind(a).await {
                Ok(l) => return Ok(l),
                Err(e) => last = format!("{a}: {e}"),
            }
        }
        Err(last)
    });
    let listener = match listener {
        Ok(l) => l,
        Err(e) => {
            devices.stop_all();
            return Err(format!("cannot listen: {e}"));
        }
    };
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let app = router(state);
    let thread = std::thread::Builder::new()
        .name("bm-server".into())
        .spawn(move || {
            rt.block_on(async move {
                let _ = axum::serve(listener, app)
                    .with_graceful_shutdown(async {
                        let _ = rx.await;
                    })
                    .await;
            });
        })
        .map_err(|e| e.to_string())?;
    Ok(RunningServer { addr, devices, recordings, shutdown: Some(tx), thread: Some(thread), warnings })
}
