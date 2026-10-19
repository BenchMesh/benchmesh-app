//! Cross-site request guard. The API has no authentication, so a web page the user happens to
//! visit must not be able to drive the instruments through the browser: a form POST or a plain
//! `fetch` needs no CORS permission to *send* (only to read the answer), and WebSockets ignore
//! CORS entirely. Every request is checked instead:
//!
//! - `Host` must name this machine the way people reach it - a loopback name, an IP literal, a
//!   single-label or `.local` name, or one listed in `BENCHMESH_ALLOWED_HOSTS` (comma separated).
//!   A public domain name is refused, which stops DNS rebinding (an attacker's domain pointed at
//!   127.0.0.1 is same-origin with itself).
//! - A request carrying `Origin` (browsers send it on every cross-origin request, on POST/PUT/
//!   DELETE and on WebSocket upgrades) must come from the page's own origin - the UI is served by
//!   this server - or from a loopback origin (the Vite dev server proxies with its own origin).
//!
//! Requests without these headers (Node-RED, scripts, curl) are not browsers and pass.

use std::net::IpAddr;

use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

/// The host part of a `Host` value or an origin's authority (`[::1]:57666` → `::1`).
fn host_of(authority: &str) -> &str {
    if let Some(rest) = authority.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest);
    }
    authority.rsplit_once(':').map_or(authority, |(h, port)| if port.chars().all(|c| c.is_ascii_digit()) { h } else { authority })
}

fn is_loopback(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    h == "localhost" || h.ends_with(".localhost") || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// A name that reaches this machine without public DNS (so it cannot be rebound by a website).
fn host_allowed(host: &str, extra: &[String]) -> bool {
    let h = host.trim_end_matches('.').to_ascii_lowercase();
    is_loopback(&h)
        || h.parse::<IpAddr>().is_ok()
        || !h.contains('.')
        || h.ends_with(".local")
        || extra.iter().any(|e| e.eq_ignore_ascii_case(&h))
}

/// `Ok` when the request may proceed, else why it is refused.
pub fn check(host: Option<&str>, origin: Option<&str>, extra: &[String]) -> Result<(), String> {
    if let Some(h) = host {
        if !host_allowed(host_of(h), extra) {
            return Err(format!(
                "host '{h}' is not this BenchMesh server's address (add it to BENCHMESH_ALLOWED_HOSTS to reach the server by that name)"
            ));
        }
    }
    let Some(origin) = origin else { return Ok(()) };
    let authority = origin.split_once("://").map(|(_, a)| a).unwrap_or("");
    let same_origin = host.is_some_and(|h| authority.eq_ignore_ascii_case(h));
    if same_origin || (!authority.is_empty() && is_loopback(host_of(authority))) {
        Ok(())
    } else {
        Err(format!("cross-site request from '{origin}' refused: only pages served by this BenchMesh server may use its API"))
    }
}

fn allowed_hosts() -> Vec<String> {
    std::env::var("BENCHMESH_ALLOWED_HOSTS")
        .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default()
}

pub async fn guard(req: Request, next: Next) -> Response {
    let verdict = {
        let get = |name| req.headers().get(name).and_then(|v| v.to_str().ok());
        check(get(header::HOST), get(header::ORIGIN), &allowed_hosts())
    };
    match verdict {
        Ok(()) => next.run(req).await,
        Err(why) => {
            eprintln!("refused {} {}: {why}", req.method(), req.uri().path());
            crate::detail(StatusCode::FORBIDDEN, why)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: &str = "127.0.0.1:57666";

    #[test]
    fn pages_of_this_server_and_local_tools_pass() {
        assert!(check(Some(H), Some("http://127.0.0.1:57666"), &[]).is_ok(), "the UI itself");
        assert!(check(Some(H), None, &[]).is_ok(), "scripts, Node-RED");
        assert!(check(None, None, &[]).is_ok());
        assert!(check(Some(H), Some("http://localhost:52893"), &[]).is_ok(), "Vite dev server proxy");
        assert!(check(Some("[::1]:57666"), Some("http://[::1]:57666"), &[]).is_ok());
        assert!(check(Some("192.168.1.20:57666"), Some("http://192.168.1.20:57666"), &[]).is_ok(), "LAN by IP");
        assert!(check(Some("labpc:57666"), Some("http://labpc:57666"), &[]).is_ok(), "intranet name");
        assert!(check(Some("labpc.local:57666"), None, &[]).is_ok());
        assert!(check(Some("bench.example.org"), Some("https://bench.example.org"), &["bench.example.org".into()]).is_ok(), "reverse proxy name, allowed");
    }

    #[test]
    fn other_websites_and_rebinding_are_refused() {
        assert!(check(Some(H), Some("https://evil.example"), &[]).unwrap_err().contains("evil.example"));
        assert!(check(Some(H), Some("null"), &[]).is_err(), "sandboxed frames");
        assert!(check(Some("evil.example:57666"), Some("http://evil.example:57666"), &[]).is_err(), "DNS rebinding");
        assert!(check(Some("bench.example.org"), None, &[]).is_err());
        assert!(check(Some("192.168.1.20:57666"), Some("http://192.168.1.21:57666"), &[]).is_err(), "another LAN page");
    }

    #[test]
    fn hosts_are_split_from_ports() {
        assert_eq!(host_of("[::1]:57666"), "::1");
        assert_eq!(host_of("localhost:1"), "localhost");
        assert_eq!(host_of("labpc"), "labpc");
    }
}
