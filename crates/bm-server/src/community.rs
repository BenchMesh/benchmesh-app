//! Community status of instrument models, read once at start from the BenchMesh website
//! (`GET {BENCHMESH_ONLINE_URL}/api/v1/status`, schema `benchmesh.status/v1`): which models users
//! have confirmed working, on which app versions, and which have reported issues. Read-only and
//! best effort — offline, a timeout or a malformed answer leaves the definitions' own status.
//!
//! `/catalog` merges it per model (`community`), and a model the definitions mark `awaiting`
//! counts as `confirmed` once users confirmed it.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use serde_json::{json, Value};

/// The website (temporary domain until benchmesh.online is attached).
pub const DEFAULT_URL: &str = "https://benchmesh.online";
pub const SCHEMA: &str = "benchmesh.status/v1";
const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Default)]
struct Snapshot {
    source: Option<String>,
    fetched_at: Option<String>,
    error: Option<String>,
    /// (definition id, model name) → the website's entry.
    models: HashMap<(String, String), Value>,
}

#[derive(Debug, Default)]
pub struct Community {
    snap: RwLock<Snapshot>,
}

/// `{"schema": "benchmesh.status/v1", "models": {"<definition id>": {"<model>": {status, versions, reports, last}}}}`
pub fn parse(v: &Value) -> Result<HashMap<(String, String), Value>, String> {
    if v.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(format!("not a {SCHEMA} document"));
    }
    let families = v.get("models").and_then(Value::as_object).ok_or("no models")?;
    let mut out = HashMap::new();
    for (driver, models) in families {
        for (model, entry) in models.as_object().into_iter().flatten() {
            if entry.get("status").and_then(Value::as_str).is_some() {
                out.insert((driver.clone(), model.clone()), entry.clone());
            }
        }
    }
    Ok(out)
}

impl Community {
    pub fn set(&self, source: &str, result: Result<HashMap<(String, String), Value>, String>) {
        let mut s = self.snap.write().unwrap();
        s.source = Some(source.to_string());
        s.fetched_at = Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        match result {
            Ok(m) => {
                s.models = m;
                s.error = None;
            }
            Err(e) => s.error = Some(e),
        }
    }

    /// `{source, fetched_at, error, models}` for `/catalog` (all null before the first answer).
    pub fn summary(&self) -> Value {
        let s = self.snap.read().unwrap();
        json!({"source": s.source, "fetched_at": s.fetched_at, "error": s.error, "models": s.models.len()})
    }

    /// The community entry of a model, and its status as the catalog shows it.
    pub fn annotate(&self, driver: &str, model: &str, defined: &str) -> (Option<Value>, String) {
        let s = self.snap.read().unwrap();
        let entry = s.models.get(&(driver.to_string(), model.to_string())).cloned();
        let confirmed = entry.as_ref().and_then(|e| e["status"].as_str()) == Some("confirmed");
        let support = if defined == "awaiting" && confirmed { "confirmed" } else { defined };
        (entry, support.to_string())
    }

    /// Fetch the status once (`BENCHMESH_ONLINE=off` disables it; `BENCHMESH_ONLINE_URL` points elsewhere).
    pub async fn refresh(&self) {
        if std::env::var("BENCHMESH_ONLINE").is_ok_and(|v| v == "off") {
            return;
        }
        let base = std::env::var("BENCHMESH_ONLINE_URL").unwrap_or_else(|_| DEFAULT_URL.into());
        self.refresh_from(&base).await;
    }

    pub async fn refresh_from(&self, base: &str) {
        let url = format!("{}/api/v1/status", base.trim_end_matches('/'));
        let result = fetch(&url).await;
        if let Err(e) = &result {
            eprintln!("community status from {url}: {e}");
        }
        self.set(&url, result.and_then(|v| parse(&v)));
    }
}

async fn fetch(url: &str) -> Result<Value, String> {
    // the desktop app's updater brings rustls with ring; the first installer wins
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(concat!("BenchMesh/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let r = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status()));
    }
    r.json::<Value>().await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Value {
        json!({"schema": SCHEMA, "generated": "2026-10-19T00:00:00Z", "models": {
            "rigol_dho800": {"DHO804": {"status": "confirmed", "versions": ["0.1.5"], "reports": 2, "last": "2026-10-18"}},
            "owon_xdm": {"XDM2041": {"status": "issues", "versions": ["0.1.4"], "reports": 1}, "XDM9": {"versions": []}}
        }})
    }

    #[test]
    fn confirmed_models_upgrade_awaiting_but_never_downgrade_tested() {
        let c = Community::default();
        c.set("test", parse(&doc()));
        let (entry, support) = c.annotate("rigol_dho800", "DHO804", "awaiting");
        assert_eq!(support, "confirmed");
        assert_eq!(entry.unwrap()["versions"], json!(["0.1.5"]));
        assert_eq!(c.annotate("rigol_dho800", "DHO804", "tested").1, "tested");
        assert_eq!(c.annotate("owon_xdm", "XDM2041", "awaiting").1, "awaiting", "issues do not confirm");
        assert_eq!(c.annotate("owon_xdm", "XDM9", "awaiting").0, None, "entries without a status are ignored");
        assert_eq!(c.summary()["models"], 2);
    }

    #[test]
    fn a_failed_refresh_keeps_the_last_good_answer() {
        let c = Community::default();
        c.set("test", parse(&doc()));
        c.set("test", parse(&json!({"schema": "other"})));
        assert_eq!(c.annotate("rigol_dho800", "DHO804", "awaiting").1, "confirmed");
        assert!(c.summary()["error"].as_str().unwrap().contains("benchmesh.status/v1"));
    }

    #[tokio::test]
    async fn fetched_over_http_and_unreachable_sites_are_reported() {
        let app = axum::Router::new().route("/api/v1/status", axum::routing::get(|| async { axum::Json(doc()) }));
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
        let c = Community::default();
        c.refresh_from(&format!("{base}/")).await;
        assert_eq!(c.annotate("rigol_dho800", "DHO804", "awaiting").1, "confirmed");
        assert_eq!(c.summary()["source"], format!("{base}/api/v1/status"));

        let gone = Community::default();
        gone.refresh_from("http://127.0.0.1:9").await;
        assert!(gone.summary()["error"].is_string());
        assert_eq!(gone.annotate("rigol_dho800", "DHO804", "awaiting").1, "awaiting");
    }
}
