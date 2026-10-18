//! BenchMesh desktop app: starts the embedded API server (bm-server) and opens a window
//! on its UI. The UI is served by the server itself, so the desktop app, remote browsers
//! and the headless server all run the same code.
//!
//! - Device definitions and the built UI ship as app resources (`devices/`, `ui/`).
//! - User data: `$BENCHMESH_DATA_DIR` or `~/.benchmesh` (config.yaml lives there).
//! - Listens on 127.0.0.1:57666, or a free port if that is taken. The API has no
//!   authentication, so LAN access is opt-in: `BENCHMESH_LISTEN=0.0.0.0`.
//! - `BENCHMESH_SIMULATE=captures/<session>` replays recorded instrument traffic (no hardware).
//! - Closing the app stops every device (definitions' on_disconnect steps run, e.g. the
//!   OWON load is returned to local mode).
//! - Instrument panels can be detached into their own undecorated windows (`panel-*`, opened
//!   by the UI; capability `capabilities/panels.json`). Closing the main window closes them too.
//! - Linux/Wayland: windows cannot ask to stay on top under Wayland, so the app runs through
//!   XWayland (`GDK_BACKEND=x11`) unless `BENCHMESH_NATIVE_WAYLAND=1`.
//! - Updates: one check per start, offered by the UI (`update.rs`).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use bm_server::run::{start, RunningServer, ServerOptions};
use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};

mod update;

struct Server(Mutex<Option<RunningServer>>);

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

fn data_dir() -> PathBuf {
    env_path("BENCHMESH_DATA_DIR").unwrap_or_else(|| {
        let home = env_path("HOME").or_else(|| env_path("USERPROFILE")).unwrap_or_else(|| PathBuf::from("."));
        home.join(".benchmesh")
    })
}

/// SIGTERM (logout, shutdown, `kill`) and SIGINT exit like closing the window, so the
/// devices' on_disconnect steps still run (e.g. the OWON load returns to local mode).
fn exit_on_signals(handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(_) => return,
        };
        rt.block_on(async {
            #[cfg(unix)]
            {
                let mut term = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                    Ok(s) => s,
                    Err(_) => return,
                };
                tokio::select! {
                    _ = term.recv() => {}
                    _ = tokio::signal::ctrl_c() => {}
                }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
        });
        handle.exit(0);
    });
}

/// Wayland does not let applications keep a window above others (the compositor decides), so
/// "keep on top" for detached instrument windows needs X11. Under XWayland GNOME/KDE honour it.
#[cfg(target_os = "linux")]
fn prefer_x11_for_keep_on_top() {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let chosen = std::env::var_os("GDK_BACKEND").is_some();
    let native = std::env::var("BENCHMESH_NATIVE_WAYLAND").is_ok_and(|v| v == "1");
    if wayland && !chosen && !native {
        // SAFETY: called first thing in main, before any other thread exists.
        unsafe { std::env::set_var("GDK_BACKEND", "x11") };
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    prefer_x11_for_keep_on_top();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(update::PendingState::default())
        .invoke_handler(tauri::generate_handler![update::update_check, update::update_install, update::update_open_page])
        // the main window owns the app: closing it closes detached instrument windows too
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            let resources = app.path().resource_dir()?;
            let data = data_dir();
            std::fs::create_dir_all(&data)?;
            let host = std::env::var("BENCHMESH_LISTEN").unwrap_or_else(|_| "127.0.0.1".into());
            let server = start(ServerOptions {
                config: env_path("BENCHMESH_CONFIG").unwrap_or_else(|| data.join("config.yaml")),
                devices_dir: env_path("BENCHMESH_DEVICES").unwrap_or_else(|| resources.join("devices")),
                ui_dir: Some(env_path("BENCHMESH_UI").unwrap_or_else(|| resources.join("ui"))),
                listen: vec![format!("{host}:57666"), format!("{host}:0")],
                // a simulation never writes into the user's real recordings
                recordings_db: env_path("BENCHMESH_SIMULATE").is_none().then(|| data.join("recordings.db")),
                simulate: env_path("BENCHMESH_SIMULATE"),
                ws_interval: Duration::from_millis(200),
            })?;
            for w in &server.warnings {
                eprintln!("warning: {w}");
            }
            let url = format!("{}/ui/", server.url());
            eprintln!("BenchMesh API/UI at {url}");
            app.manage(Server(Mutex::new(Some(server))));
            exit_on_signals(app.handle().clone());
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                .title("BenchMesh")
                .inner_size(1600.0, 1000.0)
                .min_inner_size(900.0, 600.0)
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start BenchMesh");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            if let Some(server) = handle.state::<Server>().0.lock().unwrap().take() {
                eprintln!("stopping devices…");
                server.stop();
                eprintln!("stopped");
            }
        }
    });
}
