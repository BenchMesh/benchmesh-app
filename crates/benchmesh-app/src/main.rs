//! BenchMesh desktop app: starts the embedded API server (bm-server) and opens a window
//! on its UI. The UI is served by the server itself, so the desktop app, remote browsers
//! and the headless server all run the same code.
//!
//! - Device definitions and the built UI ship as app resources (`devices/`, `ui/`).
//! - User data: `$BENCHMESH_DATA_DIR` or `~/.benchmesh` (config.yaml lives there).
//! - Listens on 127.0.0.1:57666, or a free port if that is taken; other web sites are refused
//!   by the server's cross-site guard (`bm_server::guard`). Other computers on the network
//!   use remote access (HTTPS + password, switched on in the UI; `bm_server::remote`).
//! - `BENCHMESH_SIMULATE=captures/<session>` replays recorded instrument traffic (no hardware).
//! - Closing the app stops every device (definitions' on_disconnect steps run, e.g. the
//!   OWON load is returned to local mode).
//! - Instrument panels can be detached into their own undecorated windows (`panel-*`, opened
//!   by the UI; capability `capabilities/panels.json`). Closing the main window closes them too.
//! - Linux AppImage: writes its desktop entry and icon for the user at start (`desktop_entry.rs`),
//!   so docks show the BenchMesh icon and the applications menu lists it.
//! - Linux/Wayland: windows cannot ask to stay on top under Wayland, so the app runs through
//!   XWayland (`GDK_BACKEND=x11`) unless `BENCHMESH_NATIVE_WAYLAND=1`. Under X11 the windows
//!   do not wait for the compositor between frames (`frame_sync.rs`: GNOME 42 sometimes never
//!   answers, and the window's picture stopped for good).
//! - What the distribution adds (the official build: app updates; nothing when built from this
//!   repository) comes from `benchmesh-extensions` (`ai_context/OPEN_CORE.md`). Feedback / releases
//!   pages open in the system browser by name (`links.rs`, capability `capabilities/links.json`).
//! - Files the UI saves (recording CSV, flow JSON, console log) go through the system's Save
//!   dialog (`save.rs`, same capability): the webview would drop them into Downloads silently.
//! - Windows: the UI's "Install driver" (USBTMC instruments) starts this executable again
//!   elevated with `--install-winusb` (`bm_transport::winusb`); `main` hands that run over first.
//! - `benchmesh --mcp`: no window - a stdio bridge to the running app's MCP endpoint for AI
//!   clients that start a process (Claude Desktop; `bm_server::mcp::stdio_bridge`).
//! - `benchmesh --headless [--remote-password P] [--port N] [--data-dir D] [--config F]
//!   [--simulate CAPTURE]`: no window - the server alone, used from other computers through
//!   remote access, on every OS (`headless.rs`, `bm_server::headless`, `ai_context/HEADLESS.md`);
//!   any other option is refused (exit 2).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use bm_server::extension::{Extensions, Host};
use bm_server::run::{start, RunningServer, ServerOptions};
use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};

#[cfg(target_os = "linux")]
mod desktop_entry;
#[cfg(target_os = "linux")]
mod frame_sync;
mod headless;
mod links;
mod save;

struct Server(Mutex<Option<RunningServer>>);

pub(crate) fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

/// Where `--mcp` finds the running app (`BENCHMESH_URL`, default the app's first port).
fn mcp_base_url() -> String {
    std::env::var("BENCHMESH_URL").unwrap_or_else(|_| "http://127.0.0.1:57666".into())
}

pub(crate) fn data_dir() -> PathBuf {
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
    // started elevated by the embedded server to install a Windows USB driver (no window)
    if let Some(code) = bm_server::winusb_helper_main() {
        std::process::exit(code);
    }
    if std::env::args().any(|a| a == "--mcp") {
        std::process::exit(bm_server::mcp::stdio_bridge(&mcp_base_url()));
    }
    let context = tauri::generate_context!();
    if std::env::args().any(|a| a == "--headless") {
        std::process::exit(headless::run(context.package_info()));
    }
    #[cfg(target_os = "linux")]
    prefer_x11_for_keep_on_top();
    // an AppImage installs no desktop entry: without one the dock shows a generic icon
    #[cfg(target_os = "linux")]
    desktop_entry::install_for_appimage();
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    #[cfg(target_os = "linux")]
    let builder = builder.plugin(frame_sync::plugin());
    let builder = benchmesh_extensions::desktop(builder, std::sync::Arc::new(stop_server));
    let app = builder
        .invoke_handler(tauri::generate_handler![links::open_page, save::save_file])
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
            // a simulation keeps everything in memory
            let user_data = env_path("BENCHMESH_SIMULATE").is_none().then(|| data.clone());
            let extensions = Extensions {
                distribution: benchmesh_extensions::DISTRIBUTION,
                list: benchmesh_extensions::server(&Host { data_dir: user_data.clone(), headless: false }),
            };
            let server = start(ServerOptions {
                config: env_path("BENCHMESH_CONFIG").unwrap_or_else(|| data.join("config.yaml")),
                devices_dir: env_path("BENCHMESH_DEVICES").unwrap_or_else(|| resources.join("devices")),
                ui_dir: Some(env_path("BENCHMESH_UI").unwrap_or_else(|| resources.join("ui"))),
                listen: vec!["127.0.0.1:57666".into(), "127.0.0.1:0".into()],
                // a simulation never writes into the user's real recordings
                recordings_db: env_path("BENCHMESH_SIMULATE").is_none().then(|| data.join("recordings.db")),
                simulate: env_path("BENCHMESH_SIMULATE"),
                ws_interval: Duration::from_millis(200),
                data_dir: user_data,
                // the window owns the process: closing it stops BenchMesh
                headless: None,
                extensions,
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
                // Windows / Linux: the UI draws the title bar and the window controls
                // (frontend/src/ui/bench/TitleBar.tsx); macOS keeps its native title bar
                .decorations(cfg!(target_os = "macos"))
                .build()?;
            Ok(())
        })
        .build(context)
        .expect("failed to start BenchMesh");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            stop_server(handle);
        }
    });
}

/// Stop the embedded server and every device (their on_disconnect steps run); once.
fn stop_server(handle: &tauri::AppHandle) {
    let server = handle.try_state::<Server>().and_then(|s| s.0.lock().unwrap().take());
    if let Some(server) = server {
        eprintln!("stopping devices…");
        server.stop();
        eprintln!("stopped");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_mac_bundle_asks_for_the_local_network_with_a_reason() {
        // merged into the app's Info.plist by the bundler; macOS 15+ shows it in the permission prompt
        let plist = include_str!("../Info.plist");
        assert!(plist.contains("<key>NSLocalNetworkUsageDescription</key>"));
        assert!(plist.contains("lab instruments on your local network"));
    }
}
