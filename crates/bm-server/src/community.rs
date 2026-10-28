//! Community status of instrument models from the BenchMesh website (schema
//! `benchmesh.status/v1`, contract: the website's `docs/APP_INTEGRATION.md` §1): which models
//! users have confirmed working, on which app versions, and which have open or fixed problem
//! reports. Public, read-only and best effort.
//!
//! A build carries the full document of its build day (`community-status.json`, refreshed by
//! `scripts/community-status.sh` before a release). At start the server asks once for what changed
//! since (`GET {BENCHMESH_ONLINE_URL}/api/v1/status?since=<its generated>`, no credentials: every
//! copy of a release asks the same URL and the CDN answers) and merges the models over it; `issues`
//! come only from that answer. Offline, a timeout or a malformed answer: the snapshot, no issues.
//!
//! `/catalog` and `/instruments` merge it per model: a model the definitions mark `awaiting` counts
//! as `confirmed` once users confirmed it, and `issue` flags a model under investigation, or fixed
//! in a version newer than this one (then whether that version is published yet, from
//! `GET /api/v1/update/latest.json`).

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use serde_json::{json, Value};

pub const DEFAULT_URL: &str = "https://benchmesh.online";
pub const SCHEMA: &str = "benchmesh.status/v1";
/// The status document of this build's day (without `issues`).
const BUNDLED: &str = include_str!("../community-status.json");
/// The app version issues are compared with.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const TIMEOUT: Duration = Duration::from_secs(8);

type Key = (String, String);

/// A parsed status document: models by (definition id, model), issues by (definition id, model or `*`).
#[derive(Debug, Default, Clone)]
pub struct Status {
    pub generated: Option<String>,
    pub models: HashMap<Key, Value>,
    pub issues: HashMap<Key, Value>,
}

#[derive(Debug)]
struct Snapshot {
    source: Option<String>,
    fetched_at: Option<String>,
    error: Option<String>,
    /// `generated` of the bundled document: the `since` of the request.
    snapshot: Option<String>,
    models: HashMap<Key, Value>,
    issues: HashMap<Key, Value>,
    /// Version of the latest published release, when an issue's fix needed it.
    latest: Option<String>,
}

#[derive(Debug)]
pub struct Community {
    snap: RwLock<Snapshot>,
}

impl Default for Community {
    /// The bundled snapshot, nothing fetched yet.
    fn default() -> Self {
        let bundled = serde_json::from_str(BUNDLED).map_err(|e| e.to_string()).and_then(|v| parse(&v)).unwrap_or_default();
        Community::with_snapshot(bundled)
    }
}

/// `{"schema": "benchmesh.status/v1", "generated", "models": {"<definition id>": {"<model>": {status, versions, reports, last}}},
///   "issues": {"<definition id>": {"<model>" | "*": {state, fixed_in?, reports, since}}}}`; unknown fields are ignored.
pub fn parse(v: &Value) -> Result<Status, String> {
    if v.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(format!("not a {SCHEMA} document"));
    }
    let families = v.get("models").and_then(Value::as_object).ok_or("no models")?;
    let entries = |families: Option<&serde_json::Map<String, Value>>, keep: &dyn Fn(&Value) -> bool| {
        let mut out = HashMap::new();
        for (driver, models) in families.into_iter().flatten() {
            for (model, entry) in models.as_object().into_iter().flatten() {
                if keep(entry) {
                    out.insert((driver.clone(), model.clone()), entry.clone());
                }
            }
        }
        out
    };
    let issue = |e: &Value| match e.get("state").and_then(Value::as_str) {
        Some("investigating") => true,
        Some("fixed") => e.get("fixed_in").and_then(Value::as_str).is_some(),
        _ => false,
    };
    Ok(Status {
        generated: v.get("generated").and_then(Value::as_str).map(str::to_string),
        models: entries(Some(families), &|e| e.get("status").and_then(Value::as_str).is_some()),
        issues: entries(v.get("issues").and_then(Value::as_object), &issue),
    })
}

/// Numeric x.y.z comparison (`0.1.10` > `0.1.9`; a `v` prefix and a suffix after the digits are ignored).
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |v: &str| -> Vec<u64> {
        v.trim().trim_start_matches('v').split('.')
            .map(|p| p.chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parts(a), parts(b));
    (0..a.len().max(b.len())).map(|i| a.get(i).unwrap_or(&0).cmp(b.get(i).unwrap_or(&0))).find(|o| o.is_ne()).unwrap_or(Ordering::Equal)
}

impl Community {
    pub fn with_snapshot(bundled: Status) -> Self {
        Community {
            snap: RwLock::new(Snapshot {
                source: None,
                fetched_at: None,
                error: None,
                snapshot: bundled.generated,
                models: bundled.models,
                issues: HashMap::new(),
                latest: None,
            }),
        }
    }

    /// The answer of the website: its models over the snapshot, its issues; an error keeps the snapshot.
    pub fn set(&self, source: &str, result: Result<Status, String>) {
        let mut s = self.snap.write().unwrap();
        s.source = Some(source.to_string());
        s.fetched_at = Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        match result {
            Ok(st) => {
                s.models.extend(st.models);
                s.issues = st.issues;
                s.error = None;
            }
            Err(e) => {
                s.issues.clear();
                s.error = Some(e)
            }
        }
    }

    /// The latest published release (`latest.json`), when known.
    pub fn set_latest(&self, version: Option<String>) {
        self.snap.write().unwrap().latest = version;
    }

    /// `{source, fetched_at, error, models, issues, snapshot}` for `/catalog` (source null before the first answer).
    pub fn summary(&self) -> Value {
        let s = self.snap.read().unwrap();
        json!({"source": s.source, "fetched_at": s.fetched_at, "error": s.error, "models": s.models.len(),
               "issues": s.issues.len(), "snapshot": s.snapshot})
    }

    /// The community entry of a model, and its status as the catalog shows it.
    pub fn annotate(&self, driver: &str, model: &str, defined: &str) -> (Option<Value>, String) {
        let s = self.snap.read().unwrap();
        let entry = s.models.get(&(driver.to_string(), model.to_string())).cloned();
        let confirmed = entry.as_ref().and_then(|e| e["status"].as_str()) == Some("confirmed");
        let support = if defined == "awaiting" && confirmed { "confirmed" } else { defined };
        (entry, support.to_string())
    }

    /// What to show about reported problems of a model with this app version (the model's own
    /// entry, else the definition's `*`): `{state: "investigating", reports, since}`, or
    /// `{state: "fixed", fixed_in, reports, since, available}` when the fix is newer than `app`
    /// (`available`: published already; null when that could not be read). None otherwise.
    pub fn issue(&self, driver: &str, model: &str, app: &str) -> Option<Value> {
        let s = self.snap.read().unwrap();
        let e = s.issues.get(&(driver.to_string(), model.to_string())).or_else(|| s.issues.get(&(driver.to_string(), "*".to_string())))?;
        let base = json!({"state": e["state"], "reports": e.get("reports"), "since": e.get("since")});
        match e["state"].as_str()? {
            "investigating" => Some(base),
            "fixed" => {
                let fixed_in = e["fixed_in"].as_str()?;
                if compare_versions(fixed_in, app).is_le() {
                    return None;
                }
                let available = s.latest.as_deref().map(|l| compare_versions(fixed_in, l).is_le());
                let mut out = base;
                out["fixed_in"] = json!(fixed_in);
                out["available"] = json!(available);
                Some(out)
            }
            _ => None,
        }
    }

    /// Whether an issue's fix is newer than `app`: only then is the latest release worth asking for.
    fn fix_ahead(&self, app: &str) -> bool {
        let s = self.snap.read().unwrap();
        s.issues.values().any(|e| e["state"] == "fixed" && e["fixed_in"].as_str().is_some_and(|f| compare_versions(f, app).is_gt()))
    }

    /// Fetch the status once (`BENCHMESH_ONLINE=off` disables it; `BENCHMESH_ONLINE_URL` points elsewhere).
    pub async fn refresh(&self) {
        if std::env::var("BENCHMESH_ONLINE").is_ok_and(|v| v == "off") {
            return;
        }
        let base = std::env::var("BENCHMESH_ONLINE_URL").unwrap_or_else(|_| DEFAULT_URL.into());
        self.refresh_from(&base, APP_VERSION).await;
    }

    pub async fn refresh_from(&self, base: &str, app: &str) {
        let base = base.trim_end_matches('/');
        let since = self.snap.read().unwrap().snapshot.clone();
        let url = format!("{base}/api/v1/status");
        let query: Vec<(&str, &str)> = since.as_deref().map(|s| ("since", s)).into_iter().collect();
        let result = fetch(&url, &query).await;
        if let Err(e) = &result {
            eprintln!("community status from {url}: {e}");
        }
        self.set(&url, result.and_then(|v| parse(&v)));
        if self.fix_ahead(app) {
            let latest = fetch(&format!("{base}/api/v1/update/latest.json"), &[]).await;
            if let Err(e) = &latest {
                eprintln!("latest release from {base}: {e}");
            }
            self.set_latest(latest.ok().and_then(|v| v["version"].as_str().map(str::to_string)));
        }
    }
}

/// GET a JSON document, without credentials (cacheable for everyone); redirects are followed.
async fn fetch(url: &str, query: &[(&str, &str)]) -> Result<Value, String> {
    // the desktop app's updater brings rustls with ring; the first installer wins
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(concat!("BenchMesh/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut url = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
    let r = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status()));
    }
    r.json::<Value>().await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn doc() -> Value {
        json!({"schema": SCHEMA, "generated": "2026-10-19T00:00:00Z", "models": {
            "rigol_dho800": {"DHO804": {"status": "confirmed", "versions": ["0.1.5"], "reports": 2, "last": "2026-10-18"}},
            "owon_xdm": {"XDM2041": {"status": "issues", "versions": ["0.1.4"], "reports": 1}, "XDM9": {"versions": []}}
        }})
    }

    fn issues() -> Value {
        json!({"schema": SCHEMA, "generated": "2026-10-28T00:00:00Z", "future": 1, "models": {}, "issues": {
            "owon_xdm": {"*": {"state": "investigating", "reports": 1, "since": "2026-10-27T10:00:00Z"},
                         "XDM2041": {"state": "fixed", "fixed_in": "0.1.20", "reports": 2, "since": "2026-10-26T10:00:00Z"}},
            "tenma_72": {"72-2540": {"state": "fixed", "fixed_in": "0.1.9", "reports": 1, "since": "2026-10-01T00:00:00Z"}},
            "bad": {"X": {"state": "fixed"}, "Y": {"state": "unknown"}}
        }})
    }

    #[test]
    fn the_bundled_snapshot_is_a_status_document() {
        let v: Value = serde_json::from_str(BUNDLED).unwrap();
        let st = parse(&v).unwrap();
        assert!(st.generated.is_some(), "the snapshot names its day: the since of the request");
        assert!(v.get("issues").is_none(), "issues are never bundled");
        assert_eq!(Community::default().summary()["snapshot"], json!(st.generated));
    }

    #[test]
    fn versions_compare_numerically() {
        assert!(compare_versions("0.1.10", "0.1.9").is_gt());
        assert!(compare_versions("v0.1.17", "0.1.17").is_eq());
        assert!(compare_versions("0.2", "0.1.99").is_gt());
        assert!(compare_versions("0.1.18-rc1", "0.1.18").is_eq());
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
    fn changes_merge_over_the_snapshot() {
        let c = Community::with_snapshot(parse(&doc()).unwrap());
        assert_eq!(c.annotate("rigol_dho800", "DHO804", "awaiting").1, "confirmed", "the snapshot counts before any answer");
        let changes = json!({"schema": SCHEMA, "generated": "2026-10-28T00:00:00Z", "models": {
            "owon_xdm": {"XDM2041": {"status": "confirmed", "versions": ["0.1.17"], "reports": 2}}}});
        c.set("test", parse(&changes));
        assert_eq!(c.annotate("owon_xdm", "XDM2041", "awaiting").1, "confirmed", "a changed model replaces its entry");
        assert_eq!(c.annotate("rigol_dho800", "DHO804", "awaiting").1, "confirmed", "unchanged models stay");
    }

    #[test]
    fn issues_by_model_else_every_model_of_the_definition() {
        let c = Community::default();
        c.set("test", parse(&issues()));
        let inv = c.issue("owon_xdm", "XDM1041", "0.1.17").unwrap();
        assert_eq!(inv, json!({"state": "investigating", "reports": 1, "since": "2026-10-27T10:00:00Z"}), "`*` covers every model");
        let fixed = c.issue("owon_xdm", "XDM2041", "0.1.17").unwrap();
        assert_eq!(fixed["state"], "fixed");
        assert_eq!(fixed["fixed_in"], "0.1.20");
        assert_eq!(fixed["available"], Value::Null, "the latest release was not read");
        assert_eq!(c.issue("owon_xdm", "XDM2041", "0.1.20"), None, "the fix is in this version");
        assert_eq!(c.issue("owon_xdm", "XDM2041", "0.1.21"), None, "and in newer ones");
        assert_eq!(c.issue("tenma_72", "72-2540", "0.1.17"), None);
        assert_eq!(c.issue("bad", "X", "0.1.0"), None, "a fix without a version is ignored");
        assert_eq!(c.issue("bad", "Y", "0.1.0"), None, "unknown states are ignored");
        assert_eq!(c.issue("rigol_dho800", "DHO804", "0.1.0"), None);
        assert!(c.fix_ahead("0.1.17"));
        assert!(!c.fix_ahead("0.1.20"));

        c.set_latest(Some("0.1.19".into()));
        assert_eq!(c.issue("owon_xdm", "XDM2041", "0.1.17").unwrap()["available"], false, "fixed in the next release");
        c.set_latest(Some("0.1.20".into()));
        assert_eq!(c.issue("owon_xdm", "XDM2041", "0.1.17").unwrap()["available"], true, "update");
    }

    #[test]
    fn a_failed_refresh_keeps_the_snapshot_without_issues() {
        let c = Community::default();
        c.set("test", parse(&doc()));
        c.set("test", parse(&issues()));
        c.set("test", parse(&json!({"schema": "other"})));
        assert_eq!(c.annotate("rigol_dho800", "DHO804", "awaiting").1, "confirmed");
        assert_eq!(c.issue("owon_xdm", "XDM1041", "0.1.17"), None, "no flags without an answer");
        assert!(c.summary()["error"].as_str().unwrap().contains("benchmesh.status/v1"));
    }

    #[tokio::test]
    async fn asks_for_changes_since_the_snapshot_and_the_latest_release_only_when_a_fix_is_ahead() {
        let asked: Arc<Mutex<Vec<String>>> = Default::default();
        let log = asked.clone();
        let app = axum::Router::new()
            .route("/api/v1/status", axum::routing::get(move |uri: axum::http::Uri, h: axum::http::HeaderMap| {
                let log = log.clone();
                async move {
                    assert!(h.get("authorization").is_none(), "no credentials: the answer stays cacheable");
                    log.lock().unwrap().push(uri.to_string());
                    let mut d = issues();
                    d["models"] = doc()["models"].clone();
                    axum::Json(d)
                }
            }))
            .route("/api/v1/update/latest.json", axum::routing::get(|| async { axum::Json(json!({"version": "0.1.20"})) }));
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(l, app).await.unwrap() });

        let c = Community::with_snapshot(Status { generated: Some("2026-10-19T00:00:00.000Z".into()), ..Default::default() });
        c.refresh_from(&format!("{base}/"), "0.1.17").await;
        assert_eq!(asked.lock().unwrap().as_slice(), ["/api/v1/status?since=2026-10-19T00%3A00%3A00.000Z"]);
        assert_eq!(c.annotate("rigol_dho800", "DHO804", "awaiting").1, "confirmed");
        assert_eq!(c.summary()["source"], format!("{base}/api/v1/status"));
        assert_eq!(c.issue("owon_xdm", "XDM2041", "0.1.17").unwrap()["available"], true);

        // nothing fixed ahead of this version: latest.json is not asked
        let current = Community::default();
        current.refresh_from(&base, "0.1.20").await;
        assert_eq!(current.snap.read().unwrap().latest, None);

        let gone = Community::with_snapshot(parse(&doc()).unwrap());
        gone.refresh_from("http://127.0.0.1:9", "0.1.17").await;
        assert!(gone.summary()["error"].is_string());
        assert_eq!(gone.annotate("rigol_dho800", "DHO804", "awaiting").1, "confirmed", "offline: the snapshot");
    }
}
