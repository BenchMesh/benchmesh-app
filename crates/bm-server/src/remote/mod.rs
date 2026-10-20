//! Remote access: the bench from a browser on the local network (contract
//! `ai_context/REMOTE_ACCESS.md`). Off by default; switched on in the app with a password, it
//! opens a second listener (HTTPS, every interface, port 57667) where every request passes
//! [`gate`]: local-network `Host`/`Origin`, a session cookie, and the route's [`scope`]. The
//! loopback listener stays as it is and its requests are [`Principal::Local`].

pub mod limiter;
pub mod net;
pub mod password;
pub mod scope;
pub mod sessions;

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use axum::extract::{ConnectInfo, FromRequestParts, Path, Query, Request, State};
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Form, Json, Router};
use bm_store::now_ms;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Semaphore;

use self::net::{Peer, Tls};
use self::password::PasswordHash;
use self::scope::Scope;
use self::sessions::{SessionView, Sessions};
use crate::{detail, S};

pub const DEFAULT_PORT: u16 = 57667;
const COOKIE: &str = "__Host-bm_session";
const LOCAL_ONLY: &str = "this setting can be changed only from the BenchMesh app on this computer";

/// Who sent a request. Decided by the listener it arrived on and its session, never by the client.
#[derive(Debug, Clone)]
pub enum Principal {
    /// The loopback listener: the app window, scripts and MCP clients on this computer.
    Local,
    /// The network listener without a session: may only sign in.
    Visitor(IpAddr),
    /// The network listener with a valid session.
    Remote(SessionView),
}

impl Principal {
    /// Completes when this principal's session ends (never for `Local`): long-lived streams
    /// such as WebSockets close then.
    pub async fn revoked(&self) {
        match self {
            Principal::Remote(v) => {
                let mut rx = v.revoked.clone();
                let _ = rx.wait_for(|ended| *ended).await;
            }
            _ => std::future::pending().await,
        }
    }
}

impl<St: Send + Sync> FromRequestParts<St> for Principal {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &St) -> Result<Self, Self::Rejection> {
        // only the network gate inserts one; everything else came in on the loopback listener
        Ok(parts.extensions.get::<Principal>().cloned().unwrap_or(Principal::Local))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    password: Option<PasswordHash>,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

impl Default for Settings {
    fn default() -> Self {
        Settings { enabled: false, port: DEFAULT_PORT, password: None }
    }
}

#[derive(Default)]
struct Listening {
    tasks: Vec<tokio::task::JoinHandle<()>>,
    addr: Option<SocketAddr>,
    error: Option<String>,
}

pub struct Remote {
    /// `remote.json` and `remote-tls/` in the data dir; `None` = in memory (simulations, tests).
    data_dir: Option<PathBuf>,
    settings: Mutex<Settings>,
    pub sessions: Sessions,
    limiter: limiter::Limiter,
    /// At most two password checks at a time (each costs ~0.3 s of CPU).
    verifying: Semaphore,
    tls: OnceLock<Result<Arc<Tls>, String>>,
    listening: Mutex<Listening>,
    /// The network router and the runtime it is served on (`attach`).
    serve: OnceLock<(Router, tokio::runtime::Handle)>,
}

impl Default for Remote {
    fn default() -> Self {
        Remote::new(None)
    }
}

impl Remote {
    pub fn new(data_dir: Option<PathBuf>) -> Remote {
        let settings = data_dir
            .as_ref()
            .and_then(|d| std::fs::read(d.join("remote.json")).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Remote {
            data_dir,
            settings: Mutex::new(settings),
            sessions: Sessions::default(),
            limiter: Default::default(),
            verifying: Semaphore::new(2),
            tls: OnceLock::new(),
            listening: Default::default(),
            serve: OnceLock::new(),
        }
    }

    /// Give the network router and runtime; opens the network listener when remote access is on.
    pub fn attach(&self, app: Router, rt: tokio::runtime::Handle) {
        let _ = self.serve.set((app, rt));
        self.apply();
    }

    fn save(&self, s: &Settings) -> Result<(), String> {
        match &self.data_dir {
            Some(d) => crate::automation::store::write_private(&d.join("remote.json"), &serde_json::to_vec_pretty(s).unwrap()),
            None => Ok(()),
        }
    }

    fn tls(&self) -> Result<Arc<Tls>, String> {
        self.tls.get_or_init(|| net::load_or_create_tls(self.data_dir.as_ref().map(|d| d.join("remote-tls")).as_deref()).map(Arc::new)).clone()
    }

    /// (Re)open or close the network listener to match the settings.
    fn apply(&self) {
        self.close();
        let s = self.settings.lock().unwrap().clone();
        let Some((app, rt)) = self.serve.get() else { return };
        if !s.enabled || s.password.is_none() {
            return;
        }
        let result = (|| -> Result<Listening, String> {
            let tls = self.tls()?;
            let std = std::net::TcpListener::bind(("0.0.0.0", s.port)).map_err(|e| format!("cannot listen on port {}: {e}", s.port))?;
            std.set_nonblocking(true).map_err(|e| e.to_string())?;
            let _enter = rt.enter();
            let tcp = tokio::net::TcpListener::from_std(std).map_err(|e| e.to_string())?;
            let addr = tcp.local_addr().map_err(|e| e.to_string())?;
            let (listener, accept) = net::tls_listener(tcp, tls.acceptor.clone()).map_err(|e| e.to_string())?;
            let svc = app.clone().into_make_service_with_connect_info::<Peer>();
            let serve = rt.spawn(async move {
                let _ = axum::serve(listener, svc).await;
            });
            Ok(Listening { tasks: vec![accept, serve], addr: Some(addr), error: None })
        })();
        let mut l = self.listening.lock().unwrap();
        match result {
            Ok(ok) => {
                eprintln!("remote access: listening on https://0.0.0.0:{}", s.port);
                *l = ok;
            }
            Err(e) => {
                eprintln!("remote access: {e}");
                l.error = Some(e);
            }
        }
    }

    fn close(&self) {
        let mut l = self.listening.lock().unwrap();
        for t in l.tasks.drain(..) {
            t.abort();
        }
        *l = Listening::default();
    }

    /// Close the network listener and end every session (server shutdown).
    pub fn stop(&self) {
        self.close();
        self.sessions.end_all();
    }

    fn password(&self) -> Option<PasswordHash> {
        self.settings.lock().unwrap().password.clone()
    }

    /// Set the password (ends every session) - also used by `benchmesh-server remote-password`.
    pub fn set_password(&self, hash: PasswordHash, enable: bool) -> Result<(), String> {
        let s = {
            let mut s = self.settings.lock().unwrap();
            s.password = Some(hash);
            s.enabled |= enable;
            s.clone()
        };
        self.save(&s)?;
        self.sessions.end_all();
        if enable {
            self.apply();
        }
        Ok(())
    }

    pub fn status(&self) -> Value {
        let s = self.settings.lock().unwrap().clone();
        let l = self.listening.lock().unwrap();
        let port = l.addr.map_or(s.port, |a| a.port());
        let urls: Vec<String> = net::local_addresses().iter().map(|ip| format!("https://{ip}:{port}/")).collect();
        let fingerprint = if s.enabled { self.tls.get().and_then(|t| t.as_ref().ok()).map(|t| t.fingerprint.clone()) } else { None };
        json!({
            "enabled": s.enabled,
            "port": port,
            "password_set": s.password.is_some(),
            "listening": l.addr.is_some(),
            "error": l.error,
            "urls": if l.addr.is_some() { urls } else { vec![] },
            "fingerprint": fingerprint,
            "sessions": self.sessions.list(now_ms()),
        })
    }
}

// ---- the network gate -----------------------------------------------------------------

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|c| c.trim().strip_prefix(COOKIE).and_then(|r| r.strip_prefix('=')).map(str::to_string))
        .filter(|t| !t.is_empty())
}

/// Framing and sniffing protections for everything the network listener serves (no HSTS: with
/// a self-signed certificate it would make the browser warning impossible to pass).
fn secured(mut r: Response) -> Response {
    let h = r.headers_mut();
    for (name, value) in [
        (header::X_FRAME_OPTIONS, "DENY"),
        (header::CONTENT_SECURITY_POLICY, "frame-ancestors 'none'"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        // not `no-referrer`: browsers then send `Origin: null` on form posts (refused as cross-site)
        (header::REFERRER_POLICY, "same-origin"),
    ] {
        h.entry(name).or_insert(HeaderValue::from_static(value));
    }
    r
}

fn see_other(to: &str) -> Response {
    (StatusCode::SEE_OTHER, [(header::LOCATION, to.to_string())]).into_response()
}

/// Every request on the network listener: local-network `Host`, same-origin `Origin`, a valid
/// session (or the sign-in page), and a route the session's scopes allow.
pub async fn gate(State(r): State<Arc<Remote>>, ConnectInfo(Peer(peer)): ConnectInfo<Peer>, mut req: Request, next: Next) -> Response {
    let refused = {
        let get = |name| req.headers().get(name).and_then(|v| v.to_str().ok());
        net::check_request(get(header::HOST), get(header::ORIGIN)).err()
    };
    if let Some(why) = refused {
        eprintln!("remote access: refused {} {} from {peer}: {why}", req.method(), req.uri().path());
        return secured(detail(StatusCode::FORBIDDEN, why));
    }
    let path = req.uri().path().to_string();
    let session = cookie_token(req.headers()).and_then(|t| r.sessions.touch(&t, now_ms()));
    let principal = match session {
        Some(v) if scope::of(req.method(), &path) == Scope::Admin => {
            let _ = v;
            return secured(detail(StatusCode::FORBIDDEN, LOCAL_ONLY));
        }
        Some(v) => Principal::Remote(v),
        None if path == "/remote/login" => Principal::Visitor(peer.ip()),
        None => {
            let page = req.method() == Method::GET && req.headers().get(header::ACCEPT).and_then(|v| v.to_str().ok()).is_some_and(|a| a.contains("text/html"));
            return secured(if page { see_other("/remote/login") } else { detail(StatusCode::UNAUTHORIZED, "sign in first: open this address in a browser") });
        }
    };
    req.extensions_mut().insert(principal);
    secured(next.run(req).await)
}

// ---- routes ---------------------------------------------------------------------------

pub fn routes() -> Router<S> {
    Router::new()
        .route("/remote", get(get_status).put(put_settings))
        .route("/remote/password", put(put_password))
        .route("/remote/sessions", delete(end_all))
        .route("/remote/sessions/{id}", delete(end_one))
        .route("/remote/session", get(session))
        .route("/remote/login", get(login_page).post(login))
        .route("/remote/logout", post(logout))
}

/// Admin routes check again here, whatever the gate did.
fn local_only(p: &Principal) -> Option<Response> {
    (!matches!(p, Principal::Local)).then(|| detail(StatusCode::FORBIDDEN, LOCAL_ONLY))
}

async fn get_status(State(s): State<S>, p: Principal) -> Response {
    local_only(&p).unwrap_or_else(|| Json(s.remote.status()).into_response())
}

#[derive(Deserialize)]
struct SettingsBody {
    enabled: bool,
    port: Option<u16>,
}

async fn put_settings(State(s): State<S>, p: Principal, Json(b): Json<SettingsBody>) -> Response {
    if let Some(r) = local_only(&p) {
        return r;
    }
    if let Some(port) = b.port.filter(|p| *p < 1024) {
        return detail(StatusCode::BAD_REQUEST, format!("port {port}: use a port from 1024 to 65535"));
    }
    let r = s.remote.clone();
    let saved = {
        let mut st = r.settings.lock().unwrap();
        if b.enabled && st.password.is_none() {
            return detail(StatusCode::CONFLICT, "set a password before switching remote access on");
        }
        st.enabled = b.enabled;
        st.port = b.port.unwrap_or(st.port);
        st.clone()
    };
    if let Err(e) = r.save(&saved) {
        return detail(StatusCode::INTERNAL_SERVER_ERROR, format!("saving remote access settings: {e}"));
    }
    if !saved.enabled {
        r.sessions.end_all();
        eprintln!("remote access: switched off, every session ended");
    }
    let r2 = r.clone();
    // binding and (the first time) making the certificate
    let _ = tokio::task::spawn_blocking(move || r2.apply()).await;
    Json(r.status()).into_response()
}

#[derive(Deserialize)]
struct PasswordBody {
    password: String,
}

async fn put_password(State(s): State<S>, p: Principal, Json(b): Json<PasswordBody>) -> Response {
    if let Some(r) = local_only(&p) {
        return r;
    }
    if let Some(why) = password::refusal(&b.password) {
        return detail(StatusCode::BAD_REQUEST, why);
    }
    let hash = match tokio::task::spawn_blocking(move || password::hash(&b.password)).await {
        Ok(h) => h,
        Err(e) => return detail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    match s.remote.set_password(hash, false) {
        Ok(()) => {
            eprintln!("remote access: password changed, every session ended");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => detail(StatusCode::INTERNAL_SERVER_ERROR, format!("saving remote access settings: {e}")),
    }
}

async fn end_all(State(s): State<S>, p: Principal) -> Response {
    if let Some(r) = local_only(&p) {
        return r;
    }
    s.remote.sessions.end_all();
    StatusCode::NO_CONTENT.into_response()
}

async fn end_one(State(s): State<S>, p: Principal, Path(id): Path<String>) -> Response {
    if let Some(r) = local_only(&p) {
        return r;
    }
    if s.remote.sessions.end_id(&id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        detail(StatusCode::NOT_FOUND, format!("no session {id}"))
    }
}

async fn session(p: Principal) -> Response {
    match p {
        Principal::Remote(v) => Json(json!({"principal": "remote", "address": v.address.to_string(), "since": v.since})).into_response(),
        Principal::Visitor(_) => detail(StatusCode::UNAUTHORIZED, "sign in first"),
        Principal::Local => Json(json!({"principal": "local"})).into_response(),
    }
}

#[derive(Deserialize)]
struct LoginQuery {
    e: Option<String>,
    s: Option<u64>,
}

async fn login_page(p: Principal, Query(q): Query<LoginQuery>) -> Response {
    match p {
        Principal::Local => detail(StatusCode::NOT_FOUND, "sign-in is for the network address of remote access"),
        Principal::Remote(_) => see_other("/ui/"),
        Principal::Visitor(_) => {
            let message = match (q.e.as_deref(), q.s) {
                (Some("wrong"), _) => "Wrong password.".to_string(),
                (Some("wait"), Some(n)) => format!("Too many tries. Try again in {n} s."),
                (Some("busy"), _) => "Busy, try again.".to_string(),
                (Some("off"), _) => "Remote access is not set up.".to_string(),
                _ => String::new(),
            };
            let page = LOGIN_HTML.replace("{{logo}}", LOGO).replace("{{message}}", &message);
            let csp = "default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'";
            ([(header::CONTENT_SECURITY_POLICY, csp), (header::CACHE_CONTROL, "no-store")], Html(page)).into_response()
        }
    }
}

const LOGIN_HTML: &str = include_str!("login.html");
const LOGO: &str = include_str!("../../../../docs/static/logo/benchmesh-mark-on-dark.svg");

#[derive(Deserialize)]
struct LoginForm {
    password: String,
}

async fn login(State(s): State<S>, p: Principal, headers: HeaderMap, Form(f): Form<LoginForm>) -> Response {
    let ip = match p {
        Principal::Visitor(ip) => ip,
        Principal::Remote(_) => return see_other("/ui/"),
        Principal::Local => return detail(StatusCode::NOT_FOUND, "sign-in is for the network address of remote access"),
    };
    let r = s.remote.clone();
    if let Err(wait) = r.limiter.check(ip, Instant::now()) {
        return see_other(&format!("/remote/login?e=wait&s={}", wait.as_secs().max(1)));
    }
    let Some(hash) = r.password() else { return see_other("/remote/login?e=off") };
    let Ok(_slot) = r.verifying.try_acquire() else { return see_other("/remote/login?e=busy") };
    let ok = tokio::task::spawn_blocking(move || password::verify(&hash, &f.password)).await.unwrap_or(false);
    if !ok {
        r.limiter.failed(ip, Instant::now());
        eprintln!("remote access: wrong password from {ip}");
        return see_other("/remote/login?e=wrong");
    }
    r.limiter.succeeded(ip);
    let agent = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or("");
    let token = r.sessions.create(ip, agent, now_ms());
    eprintln!("remote access: {ip} signed in");
    let cookie = format!("{COOKIE}={token}; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age={}", sessions::MAX_AGE_MS / 1000);
    (StatusCode::SEE_OTHER, [(header::LOCATION, "/ui/".to_string()), (header::SET_COOKIE, cookie)]).into_response()
}

async fn logout(State(s): State<S>, p: Principal) -> Response {
    match p {
        Principal::Remote(v) => {
            s.remote.sessions.end_view(&v);
            eprintln!("remote access: {} signed out", v.address);
            let clear = format!("{COOKIE}=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0");
            (StatusCode::SEE_OTHER, [(header::LOCATION, "/remote/login".to_string()), (header::SET_COOKIE, clear)]).into_response()
        }
        _ => see_other("/ui/"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_session_cookie_is_found_among_others() {
        let mut h = HeaderMap::new();
        h.append(header::COOKIE, HeaderValue::from_static("a=1; __Host-bm_session=abc; b=2"));
        assert_eq!(cookie_token(&h).as_deref(), Some("abc"));
        let mut h = HeaderMap::new();
        h.append(header::COOKIE, HeaderValue::from_static("__Host-bm_sessionx=abc; __Host-bm_session="));
        assert_eq!(cookie_token(&h), None);
    }

    #[test]
    fn settings_persist_without_the_password_in_clear() {
        let dir = std::env::temp_dir().join(format!("bm-remote-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let r = Remote::new(Some(dir.clone()));
        assert_eq!(r.status()["enabled"], false);
        r.set_password(password::hash_with("a long password", 1000), true).unwrap();
        let text = std::fs::read_to_string(dir.join("remote.json")).unwrap();
        assert!(!text.contains("a long password"));
        let again = Remote::new(Some(dir.clone()));
        let st = again.status();
        assert_eq!((st["enabled"].clone(), st["password_set"].clone(), st["port"].clone()), (json!(true), json!(true), json!(DEFAULT_PORT)));
        assert_eq!(st["listening"], false, "not attached to a server");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
