//! Start headless BenchMesh with the computer, for the user BenchMesh runs as (no administrator
//! rights):
//! - Linux: a systemd user service (`~/.config/systemd/user/benchmesh.service`). It starts at
//!   login, or at power-on once the user "lingers" (`sudo loginctl enable-linger "$USER"`, once).
//! - macOS: a LaunchAgent (`~/Library/LaunchAgents/online.benchmesh.headless.plist`), at login.
//! - Windows: the user's Run key (`HKCU\...\CurrentVersion\Run`, value `BenchMeshHeadless`), at sign-in.
//!
//! The entry runs the [`Launch`] command; it is restarted after a crash (systemd, launchd) but
//! not after Shut down from the UI (exit code 0).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// The command that starts this program headless.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl Launch {
    /// As a person would type it (shown in the UI).
    pub fn display(&self) -> String {
        std::iter::once(self.program.display().to_string()).chain(self.args.iter().cloned()).map(|a| shell_word(&a)).collect::<Vec<_>>().join(" ")
    }
}

fn shell_word(a: &str) -> String {
    if !a.is_empty() && a.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:=@\\".contains(c)) {
        a.to_string()
    } else {
        format!("\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub supported: bool,
    pub enabled: bool,
    /// How it is started, e.g. "systemd user service".
    pub method: Option<&'static str>,
    /// The entry's file or registry key.
    pub location: Option<String>,
    /// "boot" (power-on, before anyone logs in) or "login".
    pub starts_at: Option<&'static str>,
    /// Linux: the command that makes it start at power-on.
    pub boot_command: Option<String>,
    pub note: Option<String>,
    /// Why it is not supported here.
    pub reason: Option<String>,
    /// What the entry runs.
    pub command: Option<String>,
}

impl Status {
    pub fn unsupported(reason: &str) -> Status {
        Status { reason: Some(reason.into()), ..Default::default() }
    }

    pub fn json(&self) -> Value {
        json!({
            "supported": self.supported, "enabled": self.enabled, "method": self.method,
            "location": self.location, "starts_at": self.starts_at, "boot_command": self.boot_command,
            "note": self.note, "reason": self.reason, "command": self.command,
        })
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from)
}

/// Run a helper program; its error output on failure.
#[allow(dead_code)] // not every platform uses it
fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let mut c = std::process::Command::new(program);
    c.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flashes up
    }
    let out = c.output().map_err(|e| format!("{program}: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(format!("{program} {}: {}", args.join(" "), if err.is_empty() { out.status.to_string() } else { err }))
    }
}

// ---- Linux: systemd user service ---------------------------------------------------------

const UNIT: &str = "benchmesh.service";
pub const LINGER_COMMAND: &str = "sudo loginctl enable-linger \"$USER\"";

/// One `ExecStart=` word: quoted, with systemd's specifiers (`%`) and variables (`$`) escaped.
fn systemd_word(a: &str) -> String {
    format!("\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\"").replace('%', "%%").replace('$', "$$"))
}

pub fn systemd_unit(l: &Launch) -> String {
    let exec: Vec<String> = std::iter::once(l.program.display().to_string()).chain(l.args.iter().cloned()).map(|a| systemd_word(&a)).collect();
    format!(
        "# Written by BenchMesh (Instruments - System - Start with this computer); switched off there.\n\
         [Unit]\n\
         Description=BenchMesh lab instruments, headless (remote access)\n\
         \n\
         [Service]\n\
         ExecStart={}\n\
         Restart=on-failure\n\
         RestartSec=5\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exec.join(" ")
    )
}

#[allow(dead_code)]
fn unit_path() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".config")))?;
    Some(config.join("systemd/user").join(UNIT))
}

/// Whether systemd keeps this user's services running without a login (starts them at boot).
#[allow(dead_code)]
fn lingers(user: &str) -> bool {
    Path::new("/var/lib/systemd/linger").join(user).exists()
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    pub fn status(l: &Launch) -> Status {
        if run("systemctl", &["--user", "show-environment"]).is_err() {
            return Status::unsupported("no systemd user manager here: start the command from your system's startup instead");
        }
        let Some(path) = unit_path() else { return Status::unsupported("no home directory") };
        let enabled = path.exists() && run("systemctl", &["--user", "is-enabled", UNIT]).is_ok_and(|o| o == "enabled");
        let user = std::env::var("USER").unwrap_or_default();
        let boot = !user.is_empty() && lingers(&user);
        Status {
            supported: true,
            enabled,
            method: Some("systemd user service"),
            location: Some(path.display().to_string()),
            starts_at: Some(if boot { "boot" } else { "login" }),
            boot_command: (!boot).then(|| LINGER_COMMAND.to_string()),
            note: (!boot).then(|| "Starts when you log in. To start at power-on without logging in, run the command once.".to_string()),
            reason: None,
            command: Some(l.display()),
        }
    }

    pub fn enable(l: &Launch) -> Result<(), String> {
        let path = unit_path().ok_or("no home directory")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, systemd_unit(l)).map_err(|e| format!("{}: {e}", path.display()))?;
        run("systemctl", &["--user", "daemon-reload"])?;
        // enabled for the next start only: this process already runs
        run("systemctl", &["--user", "enable", UNIT]).map(|_| ())
    }

    pub fn disable() -> Result<(), String> {
        let _ = run("systemctl", &["--user", "disable", UNIT]);
        if let Some(path) = unit_path().filter(|p| p.exists()) {
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        run("systemctl", &["--user", "daemon-reload"]).map(|_| ())
    }
}

// ---- macOS: LaunchAgent ------------------------------------------------------------------

const AGENT: &str = "online.benchmesh.headless";

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub fn launch_agent(l: &Launch) -> String {
    let args: String = std::iter::once(l.program.display().to_string())
        .chain(l.args.iter().cloned())
        .map(|a| format!("\n    <string>{}</string>", xml(&a)))
        .collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <!-- Written by BenchMesh (Instruments - System - Start with this computer); switched off there. -->\n\
         <plist version=\"1.0\">\n\
         <dict>\n  \
           <key>Label</key>\n  <string>{AGENT}</string>\n  \
           <key>ProgramArguments</key>\n  <array>{args}\n  </array>\n  \
           <key>RunAtLoad</key>\n  <true/>\n  \
           <key>KeepAlive</key>\n  <dict>\n    <key>SuccessfulExit</key>\n    <false/>\n  </dict>\n\
         </dict>\n\
         </plist>\n"
    )
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    fn path() -> Option<PathBuf> {
        home().map(|h| h.join("Library/LaunchAgents").join(format!("{AGENT}.plist")))
    }

    pub fn status(l: &Launch) -> Status {
        let Some(path) = path() else { return Status::unsupported("no home directory") };
        Status {
            supported: true,
            enabled: path.exists(),
            method: Some("LaunchAgent"),
            location: Some(path.display().to_string()),
            starts_at: Some("login"),
            boot_command: None,
            note: Some("Starts when you log in; with automatic login (System Settings - Users & Groups) that is at power-on.".into()),
            reason: None,
            command: Some(l.display()),
        }
    }

    pub fn enable(l: &Launch) -> Result<(), String> {
        let path = path().ok_or("no home directory")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        // loaded by launchd at the next login (loading it now would start a second BenchMesh)
        std::fs::write(&path, launch_agent(l)).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn disable() -> Result<(), String> {
        match path().filter(|p| p.exists()) {
            Some(p) => std::fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display())),
            None => Ok(()),
        }
    }
}

// ---- Windows: the user's Run key ---------------------------------------------------------

#[cfg_attr(not(windows), allow(dead_code))]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg_attr(not(windows), allow(dead_code))]
const RUN_VALUE: &str = "BenchMeshHeadless";

/// One word of a Windows command line (the rules programs' C runtime parses with).
pub fn windows_word(a: &str) -> String {
    if !a.is_empty() && !a.contains([' ', '\t', '"']) {
        return a.to_string();
    }
    let mut out = String::from("\"");
    let mut slashes = 0;
    for c in a.chars() {
        match c {
            '\\' => slashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(slashes * 2 + 1));
                out.push('"');
                slashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(slashes));
                out.push(c);
                slashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(slashes * 2));
    out.push('"');
    out
}

pub fn run_key_value(l: &Launch) -> String {
    std::iter::once(l.program.display().to_string()).chain(l.args.iter().cloned()).map(|a| windows_word(&a)).collect::<Vec<_>>().join(" ")
}

#[cfg(windows)]
mod platform {
    use super::*;

    pub fn status(l: &Launch) -> Status {
        Status {
            supported: true,
            enabled: run("reg", &["query", RUN_KEY, "/v", RUN_VALUE]).is_ok(),
            method: Some("Windows sign-in (Run key)"),
            location: Some(format!(r"{RUN_KEY}\{RUN_VALUE}")),
            starts_at: Some("login"),
            boot_command: None,
            note: Some("Starts when you sign in; with automatic sign-in that is at power-on.".into()),
            reason: None,
            command: Some(l.display()),
        }
    }

    pub fn enable(l: &Launch) -> Result<(), String> {
        run("reg", &["add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &run_key_value(l), "/f"]).map(|_| ())
    }

    pub fn disable() -> Result<(), String> {
        if run("reg", &["query", RUN_KEY, "/v", RUN_VALUE]).is_err() {
            return Ok(());
        }
        run("reg", &["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]).map(|_| ())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod platform {
    use super::*;

    pub fn status(_: &Launch) -> Status {
        Status::unsupported("not available on this operating system")
    }

    pub fn enable(_: &Launch) -> Result<(), String> {
        Err("not available on this operating system".into())
    }

    pub fn disable() -> Result<(), String> {
        Ok(())
    }
}

pub fn status(l: &Launch) -> Status {
    platform::status(l)
}

pub fn enable(l: &Launch) -> Result<(), String> {
    platform::enable(l)
}

pub fn disable() -> Result<(), String> {
    platform::disable()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch() -> Launch {
        Launch { program: "/home/u/Apps/BenchMesh 0.1.15.AppImage".into(), args: vec!["--headless".into(), "--data-dir".into(), "/home/u/100% $data".into()] }
    }

    #[test]
    fn the_systemd_unit_runs_the_command_and_restarts_only_after_a_crash() {
        let u = systemd_unit(&launch());
        assert!(u.contains("\nExecStart=\"/home/u/Apps/BenchMesh 0.1.15.AppImage\" \"--headless\" \"--data-dir\" \"/home/u/100%% $$data\"\n"), "{u}");
        assert!(u.contains("\nRestart=on-failure\n"), "Shut down (exit 0) stays down");
        assert!(u.contains("\nWantedBy=default.target\n"));
        // a user manager has no network-online.target; remote access listens on every interface anyway
        assert!(!u.contains("network-online"));
    }

    #[test]
    fn the_launch_agent_runs_the_command_and_restarts_only_after_a_crash() {
        let p = launch_agent(&Launch { program: "/Applications/BenchMesh.app/Contents/MacOS/benchmesh".into(), args: vec!["--headless".into(), "<&>".into()] });
        assert!(p.contains("<string>online.benchmesh.headless</string>"));
        assert!(p.contains("<array>\n    <string>/Applications/BenchMesh.app/Contents/MacOS/benchmesh</string>\n    <string>--headless</string>\n    <string>&lt;&amp;&gt;</string>\n  </array>"), "{p}");
        assert!(p.contains("<key>SuccessfulExit</key>\n    <false/>"));
        assert!(p.contains("<key>RunAtLoad</key>\n  <true/>"));
    }

    #[test]
    fn the_run_key_value_quotes_like_windows_programs_parse() {
        let l = Launch { program: r"C:\Program Files\BenchMesh\benchmesh.exe".into(), args: vec!["--headless".into(), "--data-dir".into(), r"D:\bench data\".into(), "say \"hi\"".into()] };
        assert_eq!(run_key_value(&l), r#""C:\Program Files\BenchMesh\benchmesh.exe" --headless --data-dir "D:\bench data\\" "say \"hi\"""#);
    }

    #[test]
    fn the_command_is_shown_as_typed() {
        assert_eq!(launch().display(), "\"/home/u/Apps/BenchMesh 0.1.15.AppImage\" --headless --data-dir \"/home/u/100% $data\"");
    }

    #[test]
    fn the_status_names_how_it_starts() {
        let s = Status { supported: true, enabled: true, method: Some("LaunchAgent"), starts_at: Some("login"), ..Default::default() }.json();
        assert_eq!(s["method"], "LaunchAgent");
        assert_eq!(s["starts_at"], "login");
        assert_eq!(Status::unsupported("why").json()["reason"], "why");
    }
}
