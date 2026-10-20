//! The network listener: HTTPS (rustls) with a self-signed certificate made once per data dir,
//! accepting connections only from local-network addresses, plus the checks every network
//! request passes before its session is looked at.

use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::serve::{IncomingStream, Listener};
use ring::digest;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Semaphore};
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_HANDSHAKES: usize = 32;

/// Loopback, private (RFC 1918), shared/CGNAT (100.64/10, e.g. Tailscale), link-local, IPv6 ULA
/// and link-local addresses: the ones a home or lab network uses. Everything else (the port
/// forwarded from the internet, say) is dropped before TLS.
pub fn peer_allowed(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback() || v4.is_private() || v4.is_link_local() || (o[0] == 100 && (o[1] & 0xc0) == 64)
        }
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => peer_allowed(IpAddr::V4(v4)),
            None => {
                let first = v6.segments()[0];
                v6.is_loopback() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
            }
        },
    }
}

/// `Host` must name this computer without public DNS (an IP, a single label, `.local`), so a
/// website cannot rebind its own name to it; `Origin`, when sent, must be this very server.
pub fn check_request(host: Option<&str>, origin: Option<&str>) -> Result<(), String> {
    let Some(host) = host else { return Err("request without Host refused".into()) };
    let name = crate::guard::host_of(host).trim_end_matches('.').to_ascii_lowercase();
    let local_name = name.parse::<IpAddr>().is_ok() || (!name.is_empty() && !name.contains('.')) || name.ends_with(".local");
    if !local_name {
        return Err(format!("host '{host}' is not a local-network name of this computer"));
    }
    match origin {
        Some(o) if !o.eq_ignore_ascii_case(&format!("https://{host}")) => {
            Err(format!("cross-site request from '{o}' refused: only pages served by this BenchMesh server may use its API"))
        }
        _ => Ok(()),
    }
}

/// The server certificate and its SHA-256 fingerprint (shown in the app to check the browser's
/// warning against).
pub struct Tls {
    pub acceptor: TlsAcceptor,
    pub fingerprint: String,
}

/// Load `dir/cert.der` + `dir/key.der`, or make and save a new pair (`None`: in memory).
pub fn load_or_create_tls(dir: Option<&Path>) -> Result<Tls, String> {
    let saved = dir.and_then(|d| Some((std::fs::read(d.join("cert.der")).ok()?, std::fs::read(d.join("key.der")).ok()?)));
    let (cert, key) = match saved {
        Some(pair) => pair,
        None => {
            let mut names = vec!["localhost".to_string()];
            names.extend(local_addresses().iter().map(|ip| ip.to_string()));
            let made = rcgen::generate_simple_self_signed(names).map_err(|e| format!("certificate: {e}"))?;
            let pair = (made.cert.der().to_vec(), made.signing_key.serialize_der());
            if let Some(d) = dir {
                crate::automation::store::write_private(&d.join("cert.der"), &pair.0)?;
                crate::automation::store::write_private(&d.join("key.der"), &pair.1)?;
            }
            pair
        }
    };
    let fingerprint = digest::digest(&digest::SHA256, &cert).as_ref().iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":");
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .and_then(|b| b.with_no_client_auth().with_single_cert(vec![CertificateDer::from(cert)], PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key))))
        .map_err(|e| format!("TLS: {e}"))?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Tls { acceptor: TlsAcceptor::from(Arc::new(config)), fingerprint })
}

/// This computer's local-network addresses (for the URLs shown in the app).
pub fn local_addresses() -> Vec<IpAddr> {
    let mut v: Vec<IpAddr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .map(|i| i.ip())
        .filter(|ip| ip.is_ipv4() && !ip.is_loopback() && peer_allowed(*ip))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// The peer of a network connection (`ConnectInfo<Peer>`).
#[derive(Debug, Clone, Copy)]
pub struct Peer(pub SocketAddr);

impl axum::extract::connect_info::Connected<IncomingStream<'_, TlsListener>> for Peer {
    fn connect_info(stream: IncomingStream<'_, TlsListener>) -> Self {
        Peer(*stream.remote_addr())
    }
}

/// Connections that completed their TLS handshake. TCP accepts and handshakes run in a task of
/// their own so a slow client never holds up the others.
pub struct TlsListener {
    rx: mpsc::Receiver<(TlsStream<TcpStream>, SocketAddr)>,
    addr: SocketAddr,
}

impl Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.rx.recv().await {
            Some(c) => c,
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(self.addr)
    }
}

/// Start accepting on `tcp`; returns the listener for `axum::serve` and the accept task.
pub fn tls_listener(tcp: TcpListener, acceptor: TlsAcceptor) -> std::io::Result<(TlsListener, tokio::task::JoinHandle<()>)> {
    let addr = tcp.local_addr()?;
    let (tx, rx) = mpsc::channel(MAX_HANDSHAKES);
    let slots = Arc::new(Semaphore::new(MAX_HANDSHAKES));
    let task = tokio::spawn(async move {
        loop {
            let (stream, peer) = match tcp.accept().await {
                Ok(c) => c,
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
            };
            if !peer_allowed(peer.ip()) {
                eprintln!("remote access: connection from {peer} dropped (not a local-network address)");
                continue;
            }
            let Ok(slot) = slots.clone().try_acquire_owned() else { continue };
            let (acceptor, tx) = (acceptor.clone(), tx.clone());
            tokio::spawn(async move {
                let _ = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
                    if !starts_with_tls(&stream).await {
                        redirect_to_https(stream).await;
                    } else if let Ok(tls) = acceptor.accept(stream).await {
                        let _ = tx.send((tls, peer)).await;
                    }
                })
                .await;
                drop(slot);
            });
        }
    });
    Ok((TlsListener { rx, addr }, task))
}

/// A TLS connection opens with a handshake record (0x16); anything else is someone typing
/// `http://`.
async fn starts_with_tls(stream: &TcpStream) -> bool {
    let mut first = [0u8; 1];
    matches!(stream.peek(&mut first).await, Ok(1) if first[0] == 0x16)
}

/// Answer a plain-HTTP request with a redirect to the same address over HTTPS (nothing else is
/// ever served without TLS).
async fn redirect_to_https(mut stream: TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buf = vec![0u8; 4096];
    let mut n = 0;
    while n < buf.len() && !buf[..n].windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut buf[n..]).await {
            Ok(0) | Err(_) => break,
            Ok(k) => n += k,
        }
    }
    let reply = match https_location(&String::from_utf8_lossy(&buf[..n])) {
        Some(to) => format!("HTTP/1.1 308 Permanent Redirect\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
        None => "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
    };
    let _ = stream.write_all(reply.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// `https://<Host><path>` of a plain-HTTP request head, when its Host and path are well formed.
fn https_location(head: &str) -> Option<String> {
    let mut lines = head.lines();
    let path = lines.next()?.split(' ').nth(1).filter(|p| p.starts_with('/'))?;
    let host = lines.find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case("host")).map(|(_, v)| v.trim()))?;
    let safe = |s: &str, extra: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-._~:[]".contains(c) || extra.contains(c));
    (safe(host, "") && safe(path, "/?&=%+,;!$*'()@")).then(|| format!("https://{host}{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_local_network_peers_are_accepted() {
        for ok in ["127.0.0.1", "10.1.2.3", "172.16.0.9", "172.31.255.1", "192.168.1.20", "169.254.3.4", "100.64.0.1", "100.127.255.254", "::1", "fd12:3456::1", "fe80::1", "::ffff:192.168.1.5"] {
            assert!(peer_allowed(ok.parse().unwrap()), "{ok}");
        }
        for bad in ["8.8.8.8", "172.32.0.1", "100.128.0.1", "1.1.1.1", "2001:db8::1", "2a00:1450::1", "::ffff:8.8.8.8"] {
            assert!(!peer_allowed(bad.parse().unwrap()), "{bad}");
        }
    }

    #[test]
    fn network_requests_need_a_local_host_and_their_own_origin() {
        assert!(check_request(Some("192.168.1.20:57667"), Some("https://192.168.1.20:57667")).is_ok());
        assert!(check_request(Some("labpc:57667"), None).is_ok());
        assert!(check_request(Some("labpc.local:57667"), Some("https://labpc.local:57667")).is_ok());
        assert!(check_request(None, None).is_err());
        assert!(check_request(Some("evil.example:57667"), None).is_err(), "DNS rebinding");
        assert!(check_request(Some("192.168.1.20:57667"), Some("http://192.168.1.20:57667")).is_err(), "not over HTTPS");
        assert!(check_request(Some("192.168.1.20:57667"), Some("https://192.168.1.21:57667")).is_err());
        assert!(check_request(Some("192.168.1.20:57667"), Some("http://localhost:5173")).is_err(), "no loopback exception on the network");
        assert!(check_request(Some("192.168.1.20:57667"), Some("null")).is_err());
    }

    #[test]
    fn plain_http_is_sent_to_https() {
        assert_eq!(https_location("GET /ui/?a=1 HTTP/1.1\r\nHost: 192.168.1.20:57667\r\n\r\n").as_deref(), Some("https://192.168.1.20:57667/ui/?a=1"));
        assert_eq!(https_location("GET / HTTP/1.1\r\nhost: labpc.local:57667\r\n").as_deref(), Some("https://labpc.local:57667/"));
        assert_eq!(https_location("GET / HTTP/1.1\r\n\r\n"), None, "no Host");
        assert_eq!(https_location("GET / HTTP/1.1\r\nHost: a\r\nb.example\r\n").as_deref(), Some("https://a/"));
        assert_eq!(https_location("GET /x\"><script> HTTP/1.1\r\nHost: a\r\n"), None, "nothing unsafe is echoed");
        assert_eq!(https_location("GET / HTTP/1.1\r\nHost: a b\r\n"), None);
    }

    #[test]
    fn a_certificate_is_made_once_and_kept() {
        let dir = std::env::temp_dir().join(format!("bm-remote-tls-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = load_or_create_tls(Some(&dir)).unwrap();
        let b = load_or_create_tls(Some(&dir)).unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
        assert_eq!(a.fingerprint.len(), 32 * 3 - 1);
        assert_ne!(load_or_create_tls(None).unwrap().fingerprint, a.fingerprint);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(dir.join("key.der")).unwrap().permissions().mode() & 0o777, 0o600);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
