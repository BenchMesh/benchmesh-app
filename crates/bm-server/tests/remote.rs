//! Remote access end to end over real HTTPS: the network listener, sign-in, the session cookie,
//! local-only routes, the cross-site checks, and sessions (with their WebSockets) ending.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use bm_server::remote::password::hash_with;
use bm_server::run::{start, RunningServer, ServerOptions};
use futures_util::StreamExt;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const PASSWORD: &str = "bench password 1";
const VIEW_PASSWORD: &str = "look but do not touch";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// Accepts exactly the certificate whose fingerprint the app shows (what a user checks by eye).
#[derive(Debug)]
struct Pinned(String);

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(&self, cert: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
        let fp = ring::digest::digest(&ring::digest::SHA256, cert).as_ref().iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":");
        if fp == self.0 { Ok(ServerCertVerified::assertion()) } else { Err(rustls::Error::General("fingerprint mismatch".into())) }
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(m, c, d, &rustls::crypto::ring::default_provider().signature_verification_algorithms)
    }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, d, &rustls::crypto::ring::default_provider().signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider().signature_verification_algorithms.supported_schemes()
    }
}

struct Bench {
    server: RunningServer,
    remote: SocketAddr,
    tls: tokio_rustls::TlsConnector,
    dir: PathBuf,
}

impl Bench {
    fn start(name: &str) -> Bench {
        let dir = std::env::temp_dir().join(format!("bm-remote-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // switched on with a password and a view-only password, on any free port
        let settings = json!({"enabled": true, "port": 0, "password": hash_with(PASSWORD, 1000), "view_password": hash_with(VIEW_PASSWORD, 1000)});
        std::fs::write(dir.join("remote.json"), settings.to_string()).unwrap();
        let server = start(ServerOptions {
            config: dir.join("config.yaml"),
            devices_dir: repo().join("devices"),
            ui_dir: None,
            listen: vec!["127.0.0.1:0".into()],
            ws_interval: Duration::from_millis(50),
            recordings_db: None,
            simulate: None,
            data_dir: Some(dir.clone()),
            headless: None,
        })
        .unwrap();
        let st = local(server.addr, "GET", "/remote", None);
        assert_eq!(st["listening"], true, "{st}");
        let remote = SocketAddr::from(([127, 0, 0, 1], st["port"].as_u64().unwrap() as u16));
        let pinned = Pinned(st["fingerprint"].as_str().unwrap().to_string());
        let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(pinned))
            .with_no_client_auth();
        Bench { server, remote, tls: tokio_rustls::TlsConnector::from(Arc::new(cfg)), dir }
    }

    fn local(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        local(self.server.addr, method, path, body)
    }

    async fn tls_stream(&self) -> tokio_rustls::client::TlsStream<tokio::net::TcpStream> {
        let tcp = tokio::net::TcpStream::connect(self.remote).await.unwrap();
        self.tls.connect(ServerName::try_from("localhost").unwrap(), tcp).await.unwrap()
    }

    /// A request to the network listener → (status line + headers, body).
    async fn https(&self, method: &str, path: &str, headers: &[&str], body: &str) -> (String, String) {
        let host = format!("127.0.0.1:{}", self.remote.port());
        let extra: String = headers.iter().map(|h| format!("{h}\r\n")).collect();
        let req = format!("{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n{extra}Content-Length: {}\r\n\r\n{body}", body.len());
        let mut s = self.tls_stream().await;
        s.write_all(req.as_bytes()).await.unwrap();
        let mut out = Vec::new();
        let _ = s.read_to_end(&mut out).await;
        let out = String::from_utf8_lossy(&out).to_string();
        let (head, body) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
        (head.to_string(), body.to_string())
    }

    async fn sign_in(&self) -> String {
        self.sign_in_with(PASSWORD).await
    }

    async fn sign_in_with(&self, password: &str) -> String {
        let (head, _) = self.https("POST", "/remote/login", &["Content-Type: application/x-www-form-urlencoded"], &format!("password={}", password.replace(' ', "+"))).await;
        assert!(head.starts_with("HTTP/1.1 303"), "{head}");
        assert!(head.contains("location: /ui/"), "{head}");
        let cookie = head.lines().find_map(|l| l.strip_prefix("set-cookie: ")).expect("a session cookie").to_string();
        for attr in ["Secure", "HttpOnly", "SameSite=Strict", "Path=/"] {
            assert!(cookie.contains(attr), "{cookie}");
        }
        format!("Cookie: {}", cookie.split(';').next().unwrap())
    }
}

/// A request to the loopback listener (the app window).
fn local(addr: SocketAddr, method: &str, path: &str, body: Option<Value>) -> Value {
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let req = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len());
    let mut s = std::net::TcpStream::connect(addr).unwrap();
    std::io::Write::write_all(&mut s, req.as_bytes()).unwrap();
    let mut out = String::new();
    std::io::Read::read_to_string(&mut s, &mut out).unwrap();
    let text = out.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
    serde_json::from_str(text).unwrap_or(json!({"raw": out}))
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn signing_in_gives_the_bench_and_nothing_more() {
    let b = tokio::task::spawn_blocking(|| Bench::start("signin")).await.unwrap();

    let (head, _) = b.https("GET", "/ui/", &["Accept: text/html"], "").await;
    assert!(head.starts_with("HTTP/1.1 303") && head.contains("location: /remote/login"), "pages go to sign-in: {head}");
    assert!(head.contains("x-frame-options: DENY"), "{head}");
    assert!(head.contains("referrer-policy: same-origin"), "browsers keep sending Origin on form posts: {head}");
    let (head, _) = b.https("GET", "/status", &[], "").await;
    assert!(head.starts_with("HTTP/1.1 401"), "the API needs a session: {head}");
    let (head, page) = b.https("GET", "/remote/login", &[], "").await;
    assert!(head.starts_with("HTTP/1.1 200") && page.contains("type=\"password\""), "{head}");

    let (head, _) = b.https("POST", "/remote/login", &["Content-Type: application/x-www-form-urlencoded"], "password=wrong+password").await;
    assert!(head.contains("location: /remote/login?e=wrong"), "{head}");

    let cookie = b.sign_in().await;
    let (head, body) = b.https("GET", "/status", &[&cookie], "").await;
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(body.contains("devices_total"));
    let (_, who) = b.https("GET", "/remote/session", &[&cookie], "").await;
    assert_eq!(serde_json::from_str::<Value>(&who).unwrap()["principal"], "remote");

    // what a remote user may never do: widen their own access or reach AI clients' control
    for (m, p, body) in [("PUT", "/remote", r#"{"enabled":true,"port":1234}"#), ("GET", "/remote", ""), ("PUT", "/remote/password", r#"{"password":"0123456789abc"}"#), ("POST", "/mcp", "{}"), ("PUT", "/automation/mcp", "{}"), ("PUT", "/automation/assistant", "{}"), ("POST", "/usbtmc-devices/driver", "{}")] {
        let (head, body) = b.https(m, p, &[&cookie, "Content-Type: application/json"], body).await;
        assert!(head.starts_with("HTTP/1.1 403"), "{m} {p}: {head}");
        assert!(body.contains("only from the BenchMesh app on this computer"), "{body}");
    }
    let (head, _) = b.https("GET", "/automation/flows", &[&cookie], "").await;
    assert!(head.starts_with("HTTP/1.1 200"), "automation is allowed: {head}");

    // a page of another site cannot use the session (CSRF), nor can a rebound name
    let (head, _) = b.https("POST", "/devices", &[&cookie, "Origin: https://evil.example", "Content-Type: application/json"], "{}").await;
    assert!(head.starts_with("HTTP/1.1 403"), "{head}");
    let mut s = b.tls_stream().await;
    s.write_all(format!("GET /status HTTP/1.1\r\nHost: evil.example\r\n{cookie}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    let mut out = String::new();
    let _ = s.read_to_string(&mut out).await;
    assert!(out.starts_with("HTTP/1.1 403"), "{out}");

    // the app sees who is connected; the loopback listener keeps working without a password
    let st = b.local("GET", "/remote", None);
    assert_eq!(st["sessions"].as_array().unwrap().len(), 1, "{st}");
    assert_eq!(st["sessions"][0]["address"], "127.0.0.1");

    let (head, _) = b.https("POST", "/remote/logout", &[&cookie], "").await;
    assert!(head.contains("location: /remote/login") && head.contains("Max-Age=0"), "{head}");
    let (head, _) = b.https("GET", "/status", &[&cookie], "").await;
    assert!(head.starts_with("HTTP/1.1 401"), "signed out: {head}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_app_ends_sessions_and_their_websockets_and_switches_off() {
    let b = tokio::task::spawn_blocking(|| Bench::start("kill")).await.unwrap();
    let cookie = b.sign_in().await;

    let mut req = tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(format!("wss://127.0.0.1:{}/ws/registry", b.remote.port())).unwrap();
    req.headers_mut().insert("Cookie", cookie.trim_start_matches("Cookie: ").parse().unwrap());
    let (mut ws, _) = tokio_tungstenite::client_async(req, b.tls_stream().await).await.unwrap();
    assert!(ws.next().await.unwrap().unwrap().is_text(), "the registry streams");

    // "Disconnect all": the open WebSocket closes, the cookie is worthless
    b.local("DELETE", "/remote/sessions", None);
    let closed = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(m)) = ws.next().await {
            if m.is_close() {
                return true;
            }
        }
        true
    })
    .await;
    assert_eq!(closed, Ok(true), "the WebSocket ends with the session");
    let (head, _) = b.https("GET", "/status", &[&cookie], "").await;
    assert!(head.starts_with("HTTP/1.1 401"), "{head}");

    // a new password ends the sessions made with the old one
    let cookie = b.sign_in().await;
    let r = b.local("PUT", "/remote/password", Some(json!({"password": "short"})));
    assert!(r["detail"].as_str().unwrap().contains("at least 10"), "{r}");
    b.local("PUT", "/remote/password", Some(json!({"password": "a new bench password"})));
    let (head, _) = b.https("GET", "/status", &[&cookie], "").await;
    assert!(head.starts_with("HTTP/1.1 401"), "{head}");

    // switched off: nothing listens on the network port any more
    let st = b.local("PUT", "/remote", Some(json!({"enabled": false})));
    assert_eq!(st["listening"], false, "{st}");
    assert!(tokio::net::TcpStream::connect(b.remote).await.is_err(), "the network port is closed");
    // switching on needs no new password, and a password is needed in the first place
    let st = b.local("PUT", "/remote", Some(json!({"enabled": true, "port": 80})));
    assert!(st["detail"].as_str().unwrap().contains("1024"), "{st}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_view_only_password_shows_the_bench_and_changes_nothing() {
    let b = tokio::task::spawn_blocking(|| Bench::start("view")).await.unwrap();
    let cookie = b.sign_in_with(VIEW_PASSWORD).await;

    let (_, who) = b.https("GET", "/remote/session", &[&cookie], "").await;
    assert_eq!(serde_json::from_str::<Value>(&who).unwrap()["role"], "view");
    for p in ["/status", "/instruments", "/recordings", "/automation/flows"] {
        let (head, _) = b.https("GET", p, &[&cookie], "").await;
        assert!(head.starts_with("HTTP/1.1 200"), "{p}: {head}");
    }
    let mut req = tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(format!("wss://127.0.0.1:{}/ws/registry", b.remote.port())).unwrap();
    req.headers_mut().insert("Cookie", cookie.trim_start_matches("Cookie: ").parse().unwrap());
    let (mut ws, _) = tokio_tungstenite::client_async(req, b.tls_stream().await).await.unwrap();
    assert!(ws.next().await.unwrap().unwrap().is_text(), "live values stream");

    for (m, p, body) in [
        ("POST", "/instruments/PSU/psu-1/1/output/ON", ""),
        ("POST", "/devices", "{}"),
        ("DELETE", "/devices/psu-1", ""),
        ("POST", "/devices/psu-1/park", ""),
        ("POST", "/recordings", "{}"),
        ("POST", "/automation/run", "{}"),
    ] {
        let (head, body) = b.https(m, p, &[&cookie, "Content-Type: application/json"], body).await;
        assert!(head.starts_with("HTTP/1.1 403"), "{m} {p}: {head}");
        assert!(body.contains("view only"), "{m} {p}: {body}");
    }

    // the app tells the roles apart; the control password still controls
    let st = b.local("GET", "/remote", None);
    assert_eq!(st["view_password_set"], true);
    assert_eq!(st["sessions"][0]["role"], "view", "{st}");
    let control = b.sign_in().await;
    let (head, _) = b.https("POST", "/devices/psu-1/park", &[&control], "").await;
    assert!(!head.starts_with("HTTP/1.1 403"), "{head}");

    // the two passwords must differ; changing or removing the view-only one ends only its sessions
    let r = b.local("PUT", "/remote/view-password", Some(json!({"password": PASSWORD})));
    assert!(r["detail"].as_str().unwrap().contains("must differ"), "{r}");
    let r = b.local("PUT", "/remote/password", Some(json!({"password": VIEW_PASSWORD})));
    assert!(r["detail"].as_str().unwrap().contains("must differ"), "{r}");
    b.local("DELETE", "/remote/view-password", None);
    let (head, _) = b.https("GET", "/status", &[&cookie], "").await;
    assert!(head.starts_with("HTTP/1.1 401"), "view-only session ended: {head}");
    let (head, _) = b.https("GET", "/status", &[&control], "").await;
    assert!(head.starts_with("HTTP/1.1 200"), "control session kept: {head}");
    let (head, _) = b.https("POST", "/remote/login", &["Content-Type: application/x-www-form-urlencoded"], &format!("password={}", VIEW_PASSWORD.replace(' ', "+"))).await;
    assert!(head.contains("location: /remote/login?e=wrong"), "removed: {head}");
    assert_eq!(b.local("GET", "/remote", None)["view_password_set"], false);
}
