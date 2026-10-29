//! Running BenchMesh headless - no window; used from other computers through remote access
//! (contract `ai_context/HEADLESS.md`). Shared by `benchmesh-server` and the desktop app's
//! `benchmesh --headless`, on every OS:
//!
//! ```text
//! benchmesh --headless --remote-password '<at least 10 characters>'   # once; the hash is kept
//! benchmesh --headless                                               # later starts
//! ```
//! The password may come from `BENCHMESH_REMOTE_PASSWORD` instead (not shown in process lists).
//! The process then runs until Ctrl-C / SIGTERM or until the UI asks it (Instruments - System) to
//! shut down or restart; a restart starts the same command line again (Unix: in place, keeping
//! the process id, so systemd / launchd keep tracking it). An extension of the distribution may
//! take over once everything has stopped (`system::System::hand_over`), e.g. to install a new
//! version first.

use crate::run::{start, RunningServer, ServerOptions};
use crate::system::{Action, AfterStop};

/// Refuses a command line with anything this program does not know: an unknown option, an
/// option without its value, a stray argument. `known`: each option and whether it takes a value.
/// Ignoring them would run a different BenchMesh than asked for, silently (real instruments
/// instead of a simulation, remote access off after a typo in `--remote-password`).
pub fn check_args(args: &[String], known: &[(&str, bool)]) -> Result<(), String> {
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        match known.iter().find(|(f, _)| f == a) {
            Some((_, false)) => i += 1,
            Some((f, true)) => match args.get(i + 1) {
                Some(v) if !known.iter().any(|(k, _)| k == v) => i += 2,
                _ => return Err(format!("{f} needs a value")),
            },
            None if a.starts_with('-') => return Err(format!("unknown option {a}")),
            None => return Err(format!("unexpected argument {a}")),
        }
    }
    Ok(())
}

/// `--help` or `-h` anywhere on the command line.
pub fn wants_help(args: &[String]) -> bool {
    args.iter().skip(1).any(|a| a == "--help" || a == "-h")
}

/// `--help`: the usage, exit 0; a command line [`check_args`] refuses: why and the usage on
/// stderr, exit 2. None: go on.
pub fn refuse_bad_args(args: &[String], known: &[(&str, bool)], usage: &str) -> Option<i32> {
    if wants_help(args) {
        println!("{usage}");
        return Some(0);
    }
    match check_args(args, known) {
        Ok(()) => None,
        Err(why) => {
            eprintln!("{why}\n\n{usage}");
            Some(2)
        }
    }
}

/// The command line value of `flag` (`--flag value`).
pub fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
}

/// The remote access password given at start: `--remote-password`, else `BENCHMESH_REMOTE_PASSWORD`.
pub fn password_from(args: &[String]) -> Option<String> {
    arg(args, "--remote-password").or_else(|| std::env::var("BENCHMESH_REMOTE_PASSWORD").ok()).filter(|p| !p.is_empty())
}

/// Start, serve until told to stop, stop the devices cleanly; restart when the UI asked.
/// Returns the exit code.
pub fn run(opts: ServerOptions, remote_password: Option<String>) -> i32 {
    let server = match start(opts) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    for w in &server.warnings {
        eprintln!("warning: {w}");
    }
    if let Some(p) = remote_password {
        if let Err(e) = server.set_remote_password(&p) {
            eprintln!("remote access password: {e}");
            server.stop();
            return 1;
        }
    }
    eprintln!("BenchMesh on {} (this computer)", server.url());
    for line in remote_summary(&server.remote_status()) {
        eprintln!("{line}");
    }
    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{e}");
            server.stop();
            return 1;
        }
    };
    let action = rt.block_on(wait(&server));
    eprintln!("stopping devices…");
    server.stop();
    match action {
        Some(Action::Restart) => restart(None),
        Some(Action::HandOver(then)) => match then() {
            AfterStop::Restart(program) => restart(program),
            AfterStop::Exit(code) => code,
        },
        Some(Action::Shutdown) | None => 0,
    }
}

/// Where other computers find it, or why they cannot.
pub fn remote_summary(status: &serde_json::Value) -> Vec<String> {
    if !status["enabled"].as_bool().unwrap_or(false) || !status["password_set"].as_bool().unwrap_or(false) {
        return vec!["remote access is off: other computers cannot reach this BenchMesh - start it with --remote-password '<password>' (or BENCHMESH_REMOTE_PASSWORD)".into()];
    }
    if let Some(e) = status["error"].as_str() {
        return vec![format!("remote access: {e}")];
    }
    let mut lines = vec!["remote access - open in a browser on the local network:".to_string()];
    lines.extend(status["urls"].as_array().into_iter().flatten().filter_map(|u| u.as_str()).map(|u| format!("  {u}")));
    if let Some(f) = status["fingerprint"].as_str() {
        lines.push(format!("certificate fingerprint (compare with what the browser shows): {f}"));
    }
    lines
}

/// Ctrl-C / SIGTERM (None) or the UI's request.
async fn wait(server: &RunningServer) -> Option<Action> {
    tokio::select! {
        _ = shutdown_signal() => None,
        a = server.requested() => Some(a),
    }
}

/// Ctrl-C or SIGTERM: both stop the devices cleanly. A device thread finishes its current
/// exchange first; killing it mid-read leaves USBTMC instruments one reply behind.
pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// The same program (or `program`, the updated one) with the same arguments and environment,
/// after everything was released.
fn restart(program: Option<std::path::PathBuf>) -> i32 {
    let exe = match program.map(Ok).unwrap_or_else(std::env::current_exe) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("restart: {e}");
            return 1;
        }
    };
    eprintln!("restarting…");
    let mut cmd = std::process::Command::new(exe);
    cmd.args(std::env::args_os().skip(1));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // only returns when it failed
        let e = cmd.exec();
        eprintln!("restart: {e}");
        1
    }
    #[cfg(not(unix))]
    {
        match cmd.spawn() {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("restart: {e}");
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_password_comes_from_the_command_line() {
        assert_eq!(password_from(&args(&["benchmesh", "--headless", "--remote-password", "correct horse"])).as_deref(), Some("correct horse"));
        assert_eq!(arg(&args(&["x", "--port"]), "--port"), None, "a flag without its value");
    }

    #[test]
    fn unknown_options_missing_values_and_stray_arguments_are_refused() {
        let known = [("--headless", false), ("--port", true), ("--remote-password", true), ("--simulate", true)];
        assert_eq!(check_args(&args(&["benchmesh", "--headless", "--port", "58000", "--simulate", "captures/x"]), &known), Ok(()));
        assert_eq!(check_args(&args(&["benchmesh"]), &known), Ok(()));
        assert_eq!(check_args(&args(&["benchmesh", "--headless", "--remote-pasword", "secret"]), &known).unwrap_err(), "unknown option --remote-pasword");
        assert_eq!(check_args(&args(&["benchmesh", "--port=58000"]), &known).unwrap_err(), "unknown option --port=58000");
        assert_eq!(check_args(&args(&["benchmesh", "--port"]), &known).unwrap_err(), "--port needs a value");
        assert_eq!(check_args(&args(&["benchmesh", "--simulate", "--headless"]), &known).unwrap_err(), "--simulate needs a value", "a flag is not a value");
        assert_eq!(check_args(&args(&["benchmesh", "--headless", "58000"]), &known).unwrap_err(), "unexpected argument 58000");
        assert_eq!(check_args(&args(&["benchmesh", "--remote-password", "-starts-with-a-dash"]), &known), Ok(()), "a value may start with -");
        assert!(wants_help(&args(&["benchmesh", "--headless", "-h"])));
        assert!(!wants_help(&args(&["benchmesh", "--headless"])));
        assert_eq!(refuse_bad_args(&args(&["b", "--nope"]), &known, "usage"), Some(2));
        assert_eq!(refuse_bad_args(&args(&["b", "--help"]), &known, "usage"), Some(0));
        assert_eq!(refuse_bad_args(&args(&["b", "--headless"]), &known, "usage"), None);
    }

    #[test]
    fn the_summary_says_where_to_connect_or_why_nobody_can() {
        let off = remote_summary(&json!({"enabled": false, "password_set": false}));
        assert!(off[0].starts_with("remote access is off") && off[0].contains("--remote-password"), "{off:?}");
        let on = remote_summary(&json!({"enabled": true, "password_set": true, "error": null, "urls": ["https://192.168.1.20:57667/"], "fingerprint": "AB:CD"}));
        assert_eq!(on, ["remote access - open in a browser on the local network:", "  https://192.168.1.20:57667/", "certificate fingerprint (compare with what the browser shows): AB:CD"]);
        let busy = remote_summary(&json!({"enabled": true, "password_set": true, "error": "cannot listen on port 57667: in use"}));
        assert_eq!(busy, ["remote access: cannot listen on port 57667: in use"]);
    }
}
