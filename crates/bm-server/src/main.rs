//! benchmesh-server — BenchMesh API + UI server.
//!
//! ```text
//! benchmesh-server [--config FILE] [--devices DIR] [--ui DIR] [--host ADDR] [--port N]
//! ```
//! Defaults: `$BENCHMESH_CONFIG` or `~/.benchmesh/config.yaml`; `$BENCHMESH_DEVICES` or
//! `./devices`; UI `benchmesh-serial-service/frontend/dist` if present; `0.0.0.0:$API_PORT`
//! (57666). Ctrl-C stops all devices cleanly (definitions' on_disconnect steps run).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bm_driver::actor::ActorConfig;
use bm_server::config::ConfigFile;
use bm_server::devices::{hardware_connectors, Devices};
use bm_server::{router, AppState};

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let home = std::env::var("HOME").unwrap_or_default();
    let config = arg(&args, "--config")
        .or_else(|| std::env::var("BENCHMESH_CONFIG").ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&home).join(".benchmesh/config.yaml"));
    let devices_dir = arg(&args, "--devices")
        .or_else(|| std::env::var("BENCHMESH_DEVICES").ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("devices"));
    let ui_dir = arg(&args, "--ui")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from("benchmesh-serial-service/frontend/dist")))
        .filter(|p| p.join("index.html").exists());
    let host = arg(&args, "--host").unwrap_or_else(|| "0.0.0.0".into());
    let port = arg(&args, "--port").or_else(|| std::env::var("API_PORT").ok()).unwrap_or_else(|| "57666".into());

    let catalog = match bm_devicedef::load_catalog(&devices_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cannot load device definitions from {}:\n{}", devices_dir.display(), e.join("\n"));
            std::process::exit(1);
        }
    };
    let cfg = ConfigFile { path: config.clone() };
    let entries = cfg.load().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    let devices = Arc::new(Devices::new(catalog, hardware_connectors(), ActorConfig::default()));
    for e in entries {
        let known = devices.driver_ids();
        match bm_server::config::validate_entry(&e, &known) {
            Ok(()) => {
                if let Err(err) = devices.start(e) {
                    eprintln!("device not started: {err}");
                }
            }
            Err(err) => eprintln!("skipping invalid config entry {:?}: {err}", e.get("id")),
        }
    }
    let state = Arc::new(AppState {
        devices: devices.clone(),
        config: cfg,
        ui_dir: ui_dir.clone(),
        ws_interval: Duration::from_millis(
            std::env::var("BM_WS_INTERVAL").ok().and_then(|v| v.parse::<f64>().ok()).map(|s| (s * 1000.0) as u64).unwrap_or(200),
        ),
        config_lock: Default::default(),
    });
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        eprintln!("cannot listen on {addr}: {e}");
        std::process::exit(1);
    });
    eprintln!(
        "BenchMesh server on http://{addr} — config {} — {} definitions — UI {}",
        config.display(),
        devices.catalog.devices.len(),
        ui_dir.map(|u| u.display().to_string()).unwrap_or_else(|| "not found".into())
    );
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("server error");
    eprintln!("stopping devices…");
    tokio::task::spawn_blocking(move || devices.stop_all()).await.ok();
}
