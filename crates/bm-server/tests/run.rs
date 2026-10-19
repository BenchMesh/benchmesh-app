//! Embedding API used by the desktop app: start, listen fallback, stop.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bm_server::run::{start, ServerOptions};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn get(addr: std::net::SocketAddr, path: &str) -> String {
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    write!(s, "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn starts_falls_back_to_a_free_port_and_stops() {
    let dir = std::env::temp_dir().join(format!("bm-run-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // Occupy a port so the first listen address fails.
    let blocker = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let taken = blocker.local_addr().unwrap().to_string();
    let server = start(ServerOptions {
        config: dir.join("config.yaml"), // missing: starts with no devices
        devices_dir: repo().join("devices"),
        ui_dir: None,
        listen: vec![taken.clone(), "127.0.0.1:0".into()],
        ws_interval: Duration::from_millis(200),
        recordings_db: None,
        simulate: None,
        data_dir: None,
    })
    .unwrap();
    assert_ne!(server.addr.to_string(), taken, "fell back to the next address");
    assert!(server.url().starts_with("http://127.0.0.1:"));
    let resp = get(server.addr, "/status");
    assert!(resp.starts_with("HTTP/1.1 200"), "{resp}");
    assert!(resp.contains(r#""devices_total":0"#), "{resp}");
    let addr = server.addr;
    server.stop();
    assert!(TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_err(), "listener closed after stop");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_definitions_dir_is_reported() {
    let err = start(ServerOptions {
        config: std::env::temp_dir().join("none.yaml"),
        devices_dir: PathBuf::from("/nonexistent/devices"),
        ui_dir: None,
        listen: vec!["127.0.0.1:0".into()],
        ws_interval: Duration::from_millis(200),
        recordings_db: None,
        simulate: None,
        data_dir: None,
    })
    .err()
    .unwrap();
    assert!(err.contains("device definitions"), "{err}");
}
