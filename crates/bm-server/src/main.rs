//! benchmesh-server - BenchMesh API + UI server, headless (no window; `ai_context/HEADLESS.md`).
//!
//! ```text
//! benchmesh-server [--config FILE] [--devices DIR] [--ui DIR] [--port N] [--data-dir DIR]
//!                  [--remote-password PASSWORD]    # switches remote access on (hash kept)
//! benchmesh-server remote-password                # password from stdin; switches remote access on
//! ```
//! Defaults: `$BENCHMESH_CONFIG` or `<data dir>/config.yaml`; `$BENCHMESH_DEVICES` or
//! `./devices`; UI `frontend/dist` if present; `127.0.0.1:$API_PORT` (57666); data dir
//! `$BENCHMESH_DATA_DIR` or `~/.benchmesh`. Other computers reach it only through remote access
//! (HTTPS + password on port 57667, `ai_context/REMOTE_ACCESS.md`); the password may also come
//! from `$BENCHMESH_REMOTE_PASSWORD`. Remote browsers see the computer's resources and may
//! restart or shut it down, and make it start with the computer (Instruments - System).
//! Ctrl-C / SIGTERM stops all devices cleanly (definitions' on_disconnect steps run).
//! `--simulate captures/<session>` (or `$BENCHMESH_SIMULATE`) replays recorded instrument
//! traffic instead of opening ports - UI work and demos without hardware.
//! `benchmesh-server mcp` bridges stdio to a running server's MCP endpoint (`$BENCHMESH_URL`,
//! default `http://127.0.0.1:57666`) for AI clients that start a process.

use std::path::{Path, PathBuf};
use std::time::Duration;

use bm_server::headless::{arg, password_from};
use bm_server::run::ServerOptions;
use bm_server::system::{autostart::Launch, Headless};

fn absolute(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

fn main() {
    // started elevated by the server itself to install a Windows USB driver
    if let Some(code) = bm_transport::winusb::helper_main() {
        std::process::exit(code);
    }
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("mcp") {
        let url = std::env::var("BENCHMESH_URL").unwrap_or_else(|_| "http://127.0.0.1:57666".into());
        std::process::exit(bm_server::mcp::stdio_bridge(&url));
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
    let given_data_dir = arg(&args, "--data-dir").or_else(|| std::env::var("BENCHMESH_DATA_DIR").ok()).map(PathBuf::from);
    let home_data = given_data_dir.clone().unwrap_or_else(|| home.join(".benchmesh"));
    if args.get(1).map(String::as_str) == Some("remote-password") {
        std::process::exit(remote_password(home_data));
    }
    let config = arg(&args, "--config")
        .or_else(|| std::env::var("BENCHMESH_CONFIG").ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_data.join("config.yaml"));
    let devices_dir = arg(&args, "--devices")
        .or_else(|| std::env::var("BENCHMESH_DEVICES").ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("devices"));
    let ui_dir = arg(&args, "--ui").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("frontend/dist"));
    let port = arg(&args, "--port").or_else(|| std::env::var("API_PORT").ok()).unwrap_or_else(|| "57666".into());
    let ws_interval = std::env::var("BM_WS_INTERVAL")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|s| Duration::from_millis((s * 1000.0) as u64))
        .unwrap_or(Duration::from_millis(200));

    let simulate = arg(&args, "--simulate").or_else(|| std::env::var("BENCHMESH_SIMULATE").ok()).map(PathBuf::from);
    // A simulation must not write into the user's real data unless a data dir is given.
    let data_dir = match (&simulate, given_data_dir) {
        (Some(_), None) => None,
        (_, Some(d)) => Some(d),
        (None, None) => Some(home_data),
    };
    // what "start with the computer" runs: this server with the same files, wherever it starts from
    let mut launch_args = vec!["--config".to_string(), absolute(&config).display().to_string(), "--devices".into(), absolute(&devices_dir).display().to_string()];
    launch_args.extend(["--ui".into(), absolute(&ui_dir).display().to_string(), "--port".into(), port.clone()]);
    if let Some(d) = &data_dir {
        launch_args.extend(["--data-dir".into(), absolute(d).display().to_string()]);
    }
    let program = std::env::current_exe().unwrap_or_else(|_| PathBuf::from(&args[0]));
    // a simulation never starts with the computer
    let launch = simulate.is_none().then_some(Launch { program, args: launch_args });
    if let Some(s) = &simulate {
        eprintln!("SIMULATION: instruments replayed from {} (no hardware is used; recordings in memory)", s.display());
    }
    eprintln!("config {}", config.display());
    std::process::exit(bm_server::headless::run(
        ServerOptions {
            config,
            devices_dir,
            ui_dir: Some(ui_dir),
            listen: vec![format!("127.0.0.1:{port}")],
            recordings_db: data_dir.as_ref().map(|d| d.join("recordings.db")),
            ws_interval,
            simulate,
            data_dir,
            headless: Some(Headless { launch }),
        },
        password_from(&args),
    ));
}

/// `benchmesh-server remote-password`: read the password from stdin (one line, e.g.
/// `read -rs P && printf '%s' "$P" | benchmesh-server remote-password`), save its hash and switch
/// remote access on; a running server picks it up at its next start.
fn remote_password(data_dir: PathBuf) -> i32 {
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        eprintln!("cannot read the password from stdin");
        return 1;
    }
    let password = line.trim_end_matches(['\r', '\n']);
    if let Some(why) = bm_server::remote::password::refusal(password) {
        eprintln!("{why}");
        return 1;
    }
    let remote = bm_server::remote::Remote::new(Some(data_dir.clone()));
    match remote.set_password(bm_server::remote::password::hash(password), true) {
        Ok(()) => {
            eprintln!("remote access is on (settings in {}); restart the server to open https://<this computer>:{}/", data_dir.join("remote.json").display(), bm_server::remote::DEFAULT_PORT);
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}
