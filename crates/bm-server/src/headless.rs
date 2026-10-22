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
//! the process id, so systemd / launchd keep tracking it).

use crate::run::{start, RunningServer, ServerOptions};
use crate::system::Action;

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
        Some(Action::Restart) => restart(),
        _ => 0,
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

/// The same program with the same arguments and environment, after everything was released.
fn restart() -> i32 {
    let exe = match std::env::current_exe() {
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
    fn the_summary_says_where_to_connect_or_why_nobody_can() {
        let off = remote_summary(&json!({"enabled": false, "password_set": false}));
        assert!(off[0].starts_with("remote access is off") && off[0].contains("--remote-password"), "{off:?}");
        let on = remote_summary(&json!({"enabled": true, "password_set": true, "error": null, "urls": ["https://192.168.1.20:57667/"], "fingerprint": "AB:CD"}));
        assert_eq!(on, ["remote access - open in a browser on the local network:", "  https://192.168.1.20:57667/", "certificate fingerprint (compare with what the browser shows): AB:CD"]);
        let busy = remote_summary(&json!({"enabled": true, "password_set": true, "error": "cannot listen on port 57667: in use"}));
        assert_eq!(busy, ["remote access: cannot listen on port 57667: in use"]);
    }
}
