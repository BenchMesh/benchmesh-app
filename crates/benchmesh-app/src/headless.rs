//! `benchmesh --headless`: the installed app without its window - the server alone, used from
//! other computers through remote access (`bm_server::headless`, `ai_context/HEADLESS.md`). The
//! same definitions and UI as the app (its resources), the same data dir; listens on
//! 127.0.0.1:57666 only, so it cannot run beside the app window (which would open the same
//! instruments).

use std::path::{Path, PathBuf};
use std::time::Duration;

use bm_server::headless::{arg, password_from, refuse_bad_args};
use bm_server::run::ServerOptions;
use bm_server::extension::{Extensions, Host};
use bm_server::system::{autostart::Launch, Headless};

use crate::{data_dir, env_path};

const USAGE: &str = "usage: benchmesh --headless [--remote-password PASSWORD] [--port N] [--data-dir DIR] [--config FILE]
                          [--simulate CAPTURE]";

/// The options of `benchmesh --headless`, and whether each takes a value.
const OPTIONS: &[(&str, bool)] = &[
    ("--headless", false),
    ("--remote-password", true),
    ("--port", true),
    ("--data-dir", true),
    ("--config", true),
    ("--simulate", true),
];

/// A capture session to replay instead of the instruments: `--simulate`, else `BENCHMESH_SIMULATE`.
fn simulate_from(args: &[String]) -> Option<PathBuf> {
    arg(args, "--simulate").map(PathBuf::from).or_else(|| env_path("BENCHMESH_SIMULATE"))
}

pub fn run(package: &tauri::PackageInfo) -> i32 {
    #[cfg(windows)]
    attach_parent_console();
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = refuse_bad_args(&args, OPTIONS, USAGE) {
        return code;
    }
    let resources = match tauri::utils::platform::resource_dir(package, &tauri::Env::default()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("the app's files: {e}");
            return 1;
        }
    };
    let data = arg(&args, "--data-dir").map(PathBuf::from).unwrap_or_else(data_dir);
    if let Err(e) = std::fs::create_dir_all(&data) {
        eprintln!("{}: {e}", data.display());
        return 1;
    }
    let config = arg(&args, "--config").map(PathBuf::from).or_else(|| env_path("BENCHMESH_CONFIG")).unwrap_or_else(|| data.join("config.yaml"));
    let port = arg(&args, "--port").unwrap_or_else(|| "57666".into());
    let simulate = simulate_from(&args);
    if let Some(s) = &simulate {
        eprintln!("SIMULATION: instruments replayed from {} (no hardware is used; recordings in memory)", s.display());
    }
    // a simulation never starts with the computer
    let launch = simulate.is_none().then(|| launch(&args));
    let data_dir = simulate.is_none().then(|| data.clone());
    let extensions = Extensions {
        distribution: benchmesh_extensions::DISTRIBUTION,
        list: benchmesh_extensions::server(&Host { data_dir: data_dir.clone(), headless: true }),
    };
    bm_server::headless::run(
        ServerOptions {
            config,
            devices_dir: env_path("BENCHMESH_DEVICES").unwrap_or_else(|| resources.join("devices")),
            ui_dir: Some(env_path("BENCHMESH_UI").unwrap_or_else(|| resources.join("ui"))),
            listen: vec![format!("127.0.0.1:{port}")],
            recordings_db: simulate.is_none().then(|| data.join("recordings.db")),
            ws_interval: Duration::from_millis(200),
            data_dir,
            simulate,
            headless: Some(Headless { launch }),
            extensions,
        },
        password_from(&args),
    )
}

/// What "start with the computer" runs: this app headless with the options given now (never
/// the password: its hash is kept in the data dir).
fn launch(args: &[String]) -> Launch {
    // an AppImage runs from a temporary mount: start the AppImage file itself
    let program = std::env::var_os("APPIMAGE").map(PathBuf::from).or_else(|| std::env::current_exe().ok()).unwrap_or_else(|| PathBuf::from("benchmesh"));
    Launch { program, args: launch_args(args) }
}

fn launch_args(args: &[String]) -> Vec<String> {
    let absolute = |p: String| std::path::absolute(Path::new(&p)).map(|a| a.display().to_string()).unwrap_or(p);
    let mut out = vec!["--headless".to_string()];
    for (flag, path) in [("--data-dir", true), ("--config", true), ("--port", false)] {
        if let Some(v) = arg(args, flag) {
            out.extend([flag.to_string(), if path { absolute(v) } else { v }]);
        }
    }
    out
}

/// The app is a GUI program on Windows (no console): print into the terminal it was started from.
#[cfg(windows)]
fn attach_parent_console() {
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    // SAFETY: plain Win32 call; failing (no parent console, e.g. started at sign-in) is fine
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn starting_with_the_computer_keeps_the_options_but_not_the_password() {
        let a = launch_args(&args(&["benchmesh", "--headless", "--remote-password", "secret secret", "--port", "58000", "--data-dir", "/srv/bench"]));
        assert_eq!(a, ["--headless", "--data-dir", "/srv/bench", "--port", "58000"]);
        assert_eq!(launch_args(&args(&["benchmesh", "--headless"])), ["--headless"]);
    }

    #[test]
    fn simulate_comes_from_the_command_line_and_is_never_kept_for_starting_with_the_computer() {
        let a = args(&["benchmesh", "--headless", "--simulate", "captures/20261016-203514", "--port", "58910"]);
        assert_eq!(simulate_from(&a), Some(PathBuf::from("captures/20261016-203514")));
        assert_eq!(launch_args(&a), ["--headless", "--port", "58910"]);
        assert_eq!(bm_server::headless::check_args(&a, OPTIONS), Ok(()));
    }

    #[test]
    fn the_headless_command_line_refuses_what_it_does_not_know() {
        let refused = |a: &[&str]| bm_server::headless::check_args(&args(a), OPTIONS).unwrap_err();
        assert_eq!(refused(&["benchmesh", "--headless", "--simulat", "x"]), "unknown option --simulat");
        assert_eq!(refused(&["benchmesh", "--headless", "--remote-password"]), "--remote-password needs a value");
        for flag in ["--remote-password", "--port", "--data-dir", "--config", "--simulate"] {
            assert!(USAGE.contains(flag), "{flag} in the usage");
        }
    }

    #[test]
    fn relative_paths_become_absolute() {
        let a = launch_args(&args(&["benchmesh", "--headless", "--config", "bench.yaml"]));
        assert!(Path::new(&a[2]).is_absolute() && a[2].ends_with("bench.yaml"), "{a:?}");
    }
}
