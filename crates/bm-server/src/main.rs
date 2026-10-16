//! benchmesh-server — BenchMesh API + UI server (headless / lab server).
//!
//! ```text
//! benchmesh-server [--config FILE] [--devices DIR] [--ui DIR] [--host ADDR] [--port N]
//! ```
//! Defaults: `$BENCHMESH_CONFIG` or `~/.benchmesh/config.yaml`; `$BENCHMESH_DEVICES` or
//! `./devices`; UI `benchmesh-serial-service/frontend/dist` if present; `0.0.0.0:$API_PORT`
//! (57666). Ctrl-C stops all devices cleanly (definitions' on_disconnect steps run).

use std::path::PathBuf;
use std::time::Duration;

use bm_server::run::{start, ServerOptions};

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
}

fn main() {
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
    let ui_dir = arg(&args, "--ui").map(PathBuf::from).or_else(|| Some(PathBuf::from("benchmesh-serial-service/frontend/dist")));
    let host = arg(&args, "--host").unwrap_or_else(|| "0.0.0.0".into());
    let port = arg(&args, "--port").or_else(|| std::env::var("API_PORT").ok()).unwrap_or_else(|| "57666".into());
    let ws_interval = std::env::var("BM_WS_INTERVAL")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|s| Duration::from_millis((s * 1000.0) as u64))
        .unwrap_or(Duration::from_millis(200));

    let server = match start(ServerOptions {
        config: config.clone(),
        devices_dir,
        ui_dir,
        listen: vec![format!("{host}:{port}")],
        ws_interval,
    }) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    for w in &server.warnings {
        eprintln!("warning: {w}");
    }
    eprintln!("BenchMesh server on {} (listening {}) — config {}", server.url(), server.addr, config.display());
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("signal runtime");
    let _ = rt.block_on(tokio::signal::ctrl_c());
    eprintln!("stopping devices…");
    server.stop();
}
