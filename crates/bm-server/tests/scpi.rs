//! Virtual SCPI instruments end to end: a server with simulated instruments (the 2026-10-16
//! bench capture), raw TCP clients as VISA would connect (`TCPIP0::127.0.0.1::<port>::SOCKET`).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use bm_server::run::{start, RunningServer, ServerOptions};
use serde_json::{json, Value};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// A base port with a few free ports above it.
fn free_base() -> u16 {
    loop {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let p = l.local_addr().unwrap().port();
        drop(l);
        if p < 60000 && (p..p + 8).all(|q| std::net::TcpListener::bind(("127.0.0.1", q)).is_ok()) {
            return p;
        }
    }
}

struct Bench {
    server: Option<RunningServer>,
    dir: PathBuf,
    base: u16,
}

impl Drop for Bench {
    fn drop(&mut self) {
        if let Some(s) = self.server.take() {
            s.stop();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn bench(name: &str, access: &str) -> Bench {
    let dir = std::env::temp_dir().join(format!("bm-scpi-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let base = free_base();
    std::fs::write(dir.join("scpi.json"), json!({"access": access, "base_port": base}).to_string()).unwrap();
    let server = start(ServerOptions {
        config: repo().join("captures/bench-config.yaml"),
        devices_dir: repo().join("devices"),
        ui_dir: None,
        listen: vec!["127.0.0.1:0".into()],
        recordings_db: None,
        simulate: Some(repo().join("captures/20261016-203514")),
        ws_interval: Duration::from_millis(200),
        data_dir: Some(dir.clone()),
        headless: None,
        extensions: Default::default(),
    })
    .unwrap();
    let b = Bench { server: Some(server), dir, base };
    // wait until the simulated instruments are polled
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        let v = http(&b, "GET", "/instruments/PSU/tenma-72-2540/status", None).1;
        if v["channels"]["1"]["status"]["VOUT"].is_object() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    b
}

fn http(b: &Bench, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
    let addr = b.server.as_ref().unwrap().addr;
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let body = body.map(|v| v.to_string()).unwrap_or_default();
    write!(s, "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    let code = out[9..12].parse().unwrap();
    let json = out.split("\r\n\r\n").nth(1).and_then(|b| serde_json::from_str(b).ok()).unwrap_or(Value::Null);
    (code, json)
}

/// The port of an instrument from `GET /scpi`.
fn port_of(b: &Bench, id: &str) -> u16 {
    let v = http(b, "GET", "/scpi", None).1;
    let i = v["instruments"].as_array().unwrap().iter().find(|i| i["id"] == id).unwrap_or_else(|| panic!("{id} in {v}"));
    i["port"].as_u64().unwrap() as u16
}

struct Client {
    w: TcpStream,
    r: BufReader<TcpStream>,
}

fn connect(port: u16) -> Client {
    let w = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
    w.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let r = BufReader::new(w.try_clone().unwrap());
    Client { w, r }
}

impl Client {
    fn send(&mut self, line: &str) {
        self.w.write_all(format!("{line}\n").as_bytes()).unwrap();
    }

    fn ask(&mut self, line: &str) -> String {
        self.send(line);
        let mut out = String::new();
        self.r.read_line(&mut out).unwrap();
        out.trim_end_matches('\n').to_string()
    }
}

#[test]
fn a_serial_power_supply_answers_scpi_like_a_lan_instrument() {
    let b = bench("psu", "control");
    let port = port_of(&b, "tenma-72-2540");
    assert!(port >= b.base && port < b.base + 8, "{port} from base {}", b.base);
    let v = http(&b, "GET", "/scpi", None).1;
    assert_eq!(v["access"], "control");
    let t = v["instruments"].as_array().unwrap().iter().find(|i| i["id"] == "tenma-72-2540").unwrap();
    assert_eq!(t["address"], format!("TCPIP0::127.0.0.1::{port}::SOCKET"));
    assert_eq!(t["listening"], true);

    let mut c = connect(port);
    let idn = c.ask("*IDN?");
    assert_eq!(idn, format!("BenchMesh,72-2540,tenma-72-2540,{}", env!("CARGO_PKG_VERSION")));
    let v: f64 = c.ask("VOLTAGE?").parse().expect("a number");
    assert!((0.0..=60.0).contains(&v), "{v}");
    let vout: f64 = c.ask("fetch:vout?").parse().expect("a number");
    assert!(vout >= 0.0);
    assert_eq!(c.ask("SYST:ERR?"), "0,\"No error\"");

    // several commands in one message; queries answered in one line
    let both = c.ask("VOLTAGE 5.0;*OPC?;PSU:CH1:VOLTAGE?");
    let parts: Vec<&str> = both.split(';').collect();
    assert_eq!(parts.len(), 2, "{both}");
    assert_eq!(parts[0], "1");
    assert_eq!(c.ask("SYST:ERR?"), "0,\"No error\"");

    // out of range: refused before anything is sent
    c.send("VOLTAGE 99");
    assert!(c.ask("SYST:ERR?").starts_with("-222,"), "out of range");
    // unknown header: an empty answer instead of a timeout, then the error
    assert_eq!(c.ask("NOPE?"), "");
    assert!(c.ask("SYSTem:ERRor?").starts_with("-113,"));
    assert_eq!(c.ask("*CLS;SYST:ERR?"), "0,\"No error\"");
    let headers = c.ask("SYST:HELP:HEAD?");
    assert!(headers.contains("PSU:VOLTAGE?") && headers.contains("PSU:OUTPUT <value>") && headers.contains("PSU:FETCh:VOUT?"), "{headers}");
}

#[test]
fn a_two_class_instrument_finds_the_class_or_asks_for_it() {
    let b = bench("spm", "control");
    let mut c = connect(port_of(&b, "owon-spm3103"));
    let meas: f64 = c.ask("MEASUREMENT?").parse().expect("DMM measurement");
    assert!(meas.is_finite());
    // the DMM is polled a moment after the power supply
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let m = loop {
        let m = c.ask("DMM:FETCH:MEAS?");
        if !m.is_empty() || std::time::Instant::now() > deadline {
            break m;
        }
        c.ask("*CLS;*OPC?");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(m.parse::<f64>().is_ok(), "{m:?} {}", c.ask("SYST:ERR?"));
    let v: f64 = c.ask("PSU:VOLTAGE?").parse().expect("PSU voltage");
    assert!(v >= 0.0);
}

#[test]
fn read_access_refuses_setters_and_off_closes_the_ports() {
    let b = bench("access", "read");
    let port = port_of(&b, "tenma-72-2540");
    let mut c = connect(port);
    assert!(!c.ask("VOLTAGE?").is_empty());
    c.send("VOLTAGE 5");
    assert!(c.ask("SYST:ERR?").starts_with("-203,"), "read only");

    let (code, v) = http(&b, "PUT", "/scpi", Some(json!({"access": "everything"})));
    assert_eq!(code, 422, "{v}");
    let (code, _) = http(&b, "PUT", "/scpi", Some(json!({"base_port": 80})));
    assert_eq!(code, 422);
    let (code, v) = http(&b, "PUT", "/scpi", Some(json!({"access": "off"})));
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["access"], "off");
    assert!(v["instruments"].as_array().unwrap().iter().all(|i| i["listening"] == false));
    std::thread::sleep(Duration::from_millis(100));
    assert!(TcpStream::connect(("127.0.0.1", port)).is_err(), "port closed");
    let mut buf = [0u8; 1];
    assert_eq!(c.r.read(&mut buf).unwrap_or(0), 0, "open connection ended");
    let settings: Value = serde_json::from_slice(&std::fs::read(b.dir.join("scpi.json")).unwrap()).unwrap();
    assert_eq!(settings["access"], "off");
    assert_eq!(settings["ports"]["tenma-72-2540"], port, "ports saved with the instruments");
}

#[test]
fn a_web_page_cannot_smuggle_commands_in_an_http_request() {
    let b = bench("http", "control");
    let mut c = connect(port_of(&b, "tenma-72-2540"));
    c.w.write_all(b"POST / HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: text/plain\r\n\r\nOUTPUT ON\n*IDN?\n").unwrap();
    let mut rest = Vec::new();
    let n = c.r.read_to_end(&mut rest).unwrap_or(0);
    assert_eq!(n, 0, "closed without an answer: {}", String::from_utf8_lossy(&rest));
}
