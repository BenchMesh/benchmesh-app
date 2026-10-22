//! `benchmesh-server` as a process, headless: the password at start switches remote access on,
//! the UI's Restart starts it again with the same command line, Shut down ends it cleanly.
//! Simulated instruments, its own data dir and ports: nothing of the user's is touched.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// (status code, body) or None while nothing answers.
fn request(port: u16, method: &str, path: &str, body: &str) -> Option<(u16, String)> {
    let mut s = TcpStream::connect(("127.0.0.1", port)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let ct = if body.is_empty() { String::new() } else { "Content-Type: application/json\r\n".into() };
    write!(s, "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{ct}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).ok()?;
    let mut out = String::new();
    s.read_to_string(&mut out).ok()?;
    let code = out.split(' ').nth(1)?.parse().ok()?;
    Some((code, out.split("\r\n\r\n").nth(1).unwrap_or("").to_string()))
}

fn system(port: u16) -> Option<serde_json::Value> {
    match request(port, "GET", "/system", "")? {
        (200, body) => serde_json::from_str(body.trim()).ok(),
        _ => None,
    }
}

fn until<T>(what: &str, mut f: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(v) = f() {
            return v;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The server process, killed if the test fails before it shut down (no orphan holding ports).
struct Child(std::process::Child);

impl Drop for Child {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

#[test]
fn a_headless_server_takes_its_password_restarts_and_shuts_down_from_the_ui() {
    let dir = std::env::temp_dir().join(format!("bm-headless-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // remote access on any free port (the default 57667 may be in use on this computer)
    std::fs::write(dir.join("remote.json"), r#"{"enabled": false, "port": 0}"#).unwrap();
    let port = free_port();
    let root = repo();
    let mut child = Child(Command::new(env!("CARGO_BIN_EXE_benchmesh-server"))
        .args(["--config", root.join("captures/bench-config.yaml").to_str().unwrap()])
        .args(["--devices", root.join("devices").to_str().unwrap()])
        .args(["--simulate", root.join("captures/20261016-203514").to_str().unwrap()])
        .args(["--data-dir", dir.to_str().unwrap(), "--ui", dir.join("no-ui").to_str().unwrap(), "--port", &port.to_string()])
        .env("BENCHMESH_REMOTE_PASSWORD", "a long enough password")
        .env("BENCHMESH_ONLINE", "off")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap());

    let first = until("the server", || system(port));
    assert_eq!(first["headless"], true);
    assert_eq!(first["process"]["pid"], child.0.id());
    // the password is applied right after the local listener opens
    until("remote access", || request(port, "GET", "/remote", "").filter(|(c, r)| *c == 200 && r.contains(r#""password_set":true"#) && r.contains(r#""listening":true"#)));
    assert!(std::fs::read_to_string(dir.join("remote.json")).unwrap().contains("\"password\""), "the hash is kept for later starts");

    // a simulation never registers itself to start with the computer
    assert_eq!(request(port, "PUT", "/system/autostart", "{}").unwrap().0, 400);
    let (code, body) = request(port, "PUT", "/system/autostart", r#"{"enabled": true}"#).unwrap();
    assert_eq!(code, 409, "{body}");
    assert!(request(port, "GET", "/system/autostart", "").unwrap().1.contains("a simulation does not start with the computer"));

    assert_eq!(request(port, "POST", "/system/restart", "").unwrap().0, 202);
    let again = until("the restarted server", || system(port).filter(|s| s["process"]["started_ms"] != first["process"]["started_ms"]));
    assert_eq!(again["headless"], true);
    #[cfg(unix)]
    assert_eq!(again["process"]["pid"], child.0.id(), "restarted in place: a service manager keeps tracking it");
    // the same password again: remote access stays as it is and listens again (no clash with the
    // listener the saved settings already opened)
    let hash_before = std::fs::read_to_string(dir.join("remote.json")).unwrap();
    until("remote access after the restart", || {
        request(port, "GET", "/remote", "").filter(|(c, r)| *c == 200 && r.contains(r#""listening":true"#) && r.contains(r#""error":null"#))
    });
    assert_eq!(std::fs::read_to_string(dir.join("remote.json")).unwrap(), hash_before, "an unchanged password is not hashed again");

    assert_eq!(request(port, "POST", "/system/shutdown", "").unwrap().0, 202);
    let status = until("the process to end", || child.0.try_wait().unwrap());
    assert!(status.success(), "shut down from the UI exits 0 (a service manager does not restart it): {status}");
    // Windows starts a new process for a restart: wait for that one to end too
    until("nothing listening", || request(port, "GET", "/system", "").is_none().then_some(()));
    let _ = std::fs::remove_dir_all(&dir);
}
