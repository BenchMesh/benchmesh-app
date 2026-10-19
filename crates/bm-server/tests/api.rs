//! HTTP/WS API tests against simulated instruments (wire shapes from the former Python API).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use bm_devicedef::{load_catalog, DeviceDef};
use bm_driver::actor::ActorConfig;
use bm_driver::engine::Transport;
use bm_driver::sim::SimTransport;
use bm_server::config::{ConfigFile, DeviceEntry};
use bm_server::devices::{ConnectorFactory, Devices};
use bm_server::{router, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn text(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

fn tenma() -> SimTransport {
    let st = Arc::new(Mutex::new((4.94f64, 0.988f64, false)));
    SimTransport::new(Box::new(move |w: &[u8]| {
        let c = text(w);
        let mut s = st.lock().unwrap();
        match c.as_str() {
            "*IDN?" => Some(b"TENMA 72-2540 V2.1".to_vec()),
            "VSET1?" => Some(format!("{:05.2}", s.0).into_bytes()),
            "ISET1?" => Some(format!("{:.3} ", s.1).into_bytes()),
            "VOUT1?" => Some(format!("{:05.2}", if s.2 { s.0 } else { 0.0 }).into_bytes()),
            "IOUT1?" => Some(b"0.000".to_vec()),
            "STATUS?" => Some(vec![if s.2 { 0x51 } else { 0x10 }]),
            _ => {
                if let Some(v) = c.strip_prefix("VSET1:") {
                    s.0 = v.parse().unwrap();
                }
                if c == "OUT1" || c == "OUT0" {
                    s.2 = c == "OUT1";
                }
                None
            }
        }
    }))
}

fn oel() -> SimTransport {
    SimTransport::new(Box::new(|w: &[u8]| match text(w).trim_end() {
        "*IDN?" => Some(b"OWON,OEL1515,25130087,FV:V1.1.0\n".to_vec()),
        "SYSTem:REMote?" => Some(b"1\n".to_vec()),
        "FUNC?" => Some(b"current\n".to_vec()),
        "MEAS:ALL:INFO?" => Some(b"0.000,0.000,0.000,OFF,OFF,OFF\n".to_vec()),
        "INP?" => Some(b"OFF\n".to_vec()),
        _ => None,
    }))
}

fn sim_connectors() -> ConnectorFactory {
    Arc::new(|entry: &DeviceEntry, _def: &DeviceDef| {
        let driver = entry["driver"].as_str().unwrap().to_string();
        bm_driver::actor::fixed_connector("sim", move || {
            let t: Box<dyn Transport> = match driver.as_str() {
                "tenma_72" => Box::new(tenma()),
                "owon_oel" => Box::new(oel()),
                d => panic!("no simulator for {d}"),
            };
            Ok(t)
        })
    })
}

struct Fixture {
    state: Arc<AppState>,
    dir: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.state.recordings.shutdown();
        self.state.devices.stop_all();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn fixture(name: &str) -> Fixture {
    let dir = std::env::temp_dir().join(format!("bm-server-test-{name}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("ui")).unwrap();
    std::fs::write(dir.join("ui/index.html"), "<html>BenchMesh UI</html>").unwrap();
    let cfg = ConfigFile { path: dir.join("config.yaml") };
    let entries: Vec<DeviceEntry> = vec![
        json!({"id": "t1", "name": "Tenma", "driver": "tenma_72", "transport": "serial", "port": "/dev/null", "baud": 9600}),
        json!({"id": "l1", "name": "Load", "driver": "owon_oel", "transport": "serial", "port": "/dev/null"}),
    ]
    .into_iter()
    .map(|v| v.as_object().unwrap().clone())
    .collect();
    cfg.save(&entries).unwrap();
    let catalog = load_catalog(&repo().join("devices")).unwrap();
    let actor = ActorConfig { metrics_every: Duration::from_millis(200), ..Default::default() };
    let devices = Arc::new(Devices::new(catalog, sim_connectors(), actor));
    for e in cfg.load().unwrap() {
        devices.start(e).unwrap();
    }
    let recordings = bm_server::recordings::Recordings::new(Arc::new(bm_store::Store::in_memory().unwrap()), devices.clone());
    let state = Arc::new(AppState {
        devices,
        recordings,
        config: cfg,
        ui_dir: Some(dir.join("ui")),
        ws_interval: Duration::from_millis(50),
        metrics_interval: Duration::from_millis(100),
        config_lock: Default::default(),
        community: Default::default(),
    });
    let f = Fixture { state, dir };
    // wait until both devices are connected and polled
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let reg = f.state.devices.registry.lock().unwrap().to_json();
        if reg["t1"]["PSU"]["status_ch1"].is_object() && reg["l1"]["ELL"]["status_ch1"].is_object() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    f
}

async fn call(f: &Fixture, method: &str, uri: &str, body: Option<Value>, etag: Option<&str>) -> (StatusCode, Value, Option<String>) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = etag {
        req = req.header(header::IF_NONE_MATCH, t);
    }
    let req = match body {
        Some(b) => req.header(header::CONTENT_TYPE, "application/json").body(Body::from(b.to_string())).unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let resp = router(f.state.clone()).oneshot(req).await.unwrap();
    let status = resp.status();
    let tag = resp.headers().get(header::ETAG).map(|v| v.to_str().unwrap().to_string());
    let bytes = axum::body::to_bytes(resp.into_body(), 10 << 20).await.unwrap();
    let v = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()));
    (status, v, tag)
}

#[tokio::test(flavor = "multi_thread")]
async fn instruments_list_matches_python_shape_and_supports_etag() {
    let f = fixture("list");
    let (code, v, tag) = call(&f, "GET", "/instruments", None, None).await;
    assert_eq!(code, StatusCode::OK);
    let t1 = v.as_array().unwrap().iter().find(|i| i["id"] == "t1").unwrap();
    assert_eq!(t1["name"], "Tenma");
    assert_eq!(t1["IDN"], "TENMA 72-2540 V2.1");
    assert_eq!(t1["health"]["status"], "healthy");
    assert_eq!(t1["health"]["is_alive"], true);
    assert_eq!(t1["classes"], json!([{"class": "PSU", "channels": ["/instruments/PSU/t1/1"], "ui_component": "GenericPSU"}]));
    let (code, _, _) = call(&f, "GET", "/instruments", None, tag.as_deref()).await;
    assert_eq!(code, StatusCode::NOT_MODIFIED);
    let (code, v, _) = call(&f, "GET", "/instruments/ELL", None, None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(call(&f, "GET", "/instruments/DMM", None, None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_and_post_operations() {
    let f = fixture("ops");
    let (code, v, _) = call(&f, "GET", "/instruments/PSU/t1/1/voltage", None, None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(v["path"], "/instruments/PSU/t1/1/voltage");
    assert_eq!(v["value"]["si"]["number"], "4.94");
    let (code, _, _) = call(&f, "POST", "/instruments/PSU/t1/1/voltage/5", None, None).await;
    assert_eq!(code, StatusCode::NO_CONTENT);
    let (_, v, _) = call(&f, "GET", "/instruments/PSU/t1/1/voltage", None, None).await;
    assert_eq!(v["value"]["si"]["number"], "5");

    let (code, v, _) = call(&f, "POST", "/instruments/PSU/t1/1/voltage/99", None, None).await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(v["detail"].as_str().unwrap().contains("outside"), "{v}");
    let (code, v, _) = call(&f, "POST", "/instruments/ELL/l1/1/volt/10", None, None).await;
    assert_eq!(code, StatusCode::CONFLICT);
    assert!(v["detail"].as_str().unwrap().contains("MODE=VOLT"), "{v}");
    assert_eq!(call(&f, "GET", "/instruments/PSU/t1/1/nope", None, None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&f, "GET", "/instruments/PSU/zz/1/voltage", None, None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&f, "GET", "/instruments/PSU/t1/0/voltage", None, None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&f, "GET", "/instruments/PSU/t1/2/voltage", None, None).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&f, "GET", "/instruments/DMM/t1/1/voltage", None, None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn features_methods_and_drivers() {
    let f = fixture("meta");
    let (code, v, _) = call(&f, "GET", "/instruments/PSU/t1", None, None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(v["channels"], 1);
    assert_eq!(v["lock"], true);
    assert_eq!(v["absolute_limits"]["voltage"]["max"], 30.0, "72-2540 limits: {v}");
    let (_, v, _) = call(&f, "GET", "/instruments/PSU/t1/methods", None, None).await;
    let names: Vec<(String, String)> = v["methods"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| (m["http_method"].as_str().unwrap().into(), m["name"].as_str().unwrap().into()))
        .collect();
    assert!(names.contains(&("GET".into(), "voltage".into())) && names.contains(&("POST".into(), "voltage".into())));
    // Setters carry the definition's value spec so UIs can build controls without device code.
    let post = |name: &str| v["methods"].as_array().unwrap().iter().find(|m| m["http_method"] == "POST" && m["name"] == name).cloned().unwrap();
    assert_eq!(post("voltage")["value"], json!({"type": "number", "min": 0.0, "max": 60.0, "format": "%.2f", "unit": "V"}));
    assert_eq!(post("current")["value"]["format"], "%.3f");
    assert_eq!(post("output")["value"], json!({"type": "enum", "values": ["OFF", "ON"]}));
    assert_eq!(post("output")["energizes"], true);
    assert!(post("voltage").get("energizes").is_none());
    let (_, v, _) = call(&f, "GET", "/instruments/ELL/l1/methods", None, None).await;
    let input = v["methods"].as_array().unwrap().iter().find(|m| m["http_method"] == "POST" && m["name"] == "input").unwrap();
    assert_eq!(input["energizes"], true, "load input energizes: {input}");
    let (_, v, _) = call(&f, "GET", "/drivers", None, None).await;
    assert_eq!(v["owon_dge"]["supported_transports"], json!(["usbtmc"]));
    assert_eq!(v["tenma_72"]["vendor"], "TENMA");
    let (_, v, _) = call(&f, "GET", "/drivers/owon_xdm", None, None).await;
    assert!(v.as_array().unwrap().contains(&json!("XDM1241")));
    let (_, v, _) = call(&f, "GET", "/status", None, None).await;
    assert_eq!(v, json!({"devices_total": 2, "connected": 2, "disconnected": 0}));
}

#[tokio::test(flavor = "multi_thread")]
async fn device_config_crud_persists_and_validates() {
    let f = fixture("crud");
    let new = json!({"id": "l2", "name": "Load 2", "driver": "owon_oel", "transport": "serial", "port": "/dev/null", "baud": 115200});
    assert_eq!(call(&f, "POST", "/devices", Some(new.clone()), None).await.0, StatusCode::OK);
    assert_eq!(call(&f, "POST", "/devices", Some(new.clone()), None).await.0, StatusCode::CONFLICT);
    let bad = json!({"id": "x", "driver": "nope", "port": "/dev/null"});
    let (code, v, _) = call(&f, "POST", "/devices", Some(bad), None).await;
    assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(v["detail"].as_str().unwrap().contains("unknown driver"));
    let saved = std::fs::read_to_string(&f.state.config.path).unwrap();
    assert!(saved.contains("l2") && saved.contains("Load 2"));
    let mut upd = new.clone();
    upd["name"] = json!("Renamed");
    assert_eq!(call(&f, "PUT", "/devices/other", Some(upd.clone()), None).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&f, "PUT", "/devices/l2", Some(upd), None).await.0, StatusCode::OK);
    let (_, v, _) = call(&f, "GET", "/config", None, None).await;
    assert!(v["devices"].as_array().unwrap().iter().any(|d| d["name"] == "Renamed"));
    assert_eq!(call(&f, "DELETE", "/devices/l2", None, None).await.0, StatusCode::OK);
    assert_eq!(call(&f, "DELETE", "/devices/l2", None, None).await.0, StatusCode::NOT_FOUND);
    assert!(!std::fs::read_to_string(&f.state.config.path).unwrap().contains("l2"));
    // usbtmc entries keep their `device` field (the Python POST /config dropped it)
    let all = json!({"devices": [{"id": "g1", "name": "Gen", "driver": "owon_dge", "transport": "usbtmc", "device": "/dev/usbtmc9"}]});
    let (code, v, _) = call(&f, "POST", "/config", Some(all), None).await;
    assert_eq!(code, StatusCode::OK, "{v}");
    assert!(std::fs::read_to_string(&f.state.config.path).unwrap().contains("/dev/usbtmc9"));
}

#[tokio::test(flavor = "multi_thread")]
async fn devices_can_be_configured_by_identity_instead_of_a_port() {
    let f = fixture("identity");
    let xdm = json!({"id": "x1", "name": "XDM", "driver": "owon_oel", "transport": "serial",
                     "match": {"usb": {"vid": "1a86", "pid": "7523", "revision": "0264"}, "idn_serial": "24512010"}});
    let (code, v, _) = call(&f, "POST", "/devices", Some(xdm), None).await;
    assert_eq!(code, StatusCode::OK, "{v}");
    assert!(std::fs::read_to_string(&f.state.config.path).unwrap().contains("idn_serial: '24512010'"));
    let scope = json!({"id": "s1", "name": "Scope", "driver": "rigol_dho800", "transport": "usbtmc",
                       "match": {"usb": {"vid": "1ab1", "pid": "044d", "serial": "DHO8A272907001"}}});
    assert_eq!(call(&f, "POST", "/devices", Some(scope), None).await.0, StatusCode::OK);
    for (bad, why) in [
        (json!({"usb": {"vid": "1a86"}}), "invalid `match`"),
        (json!({"usb": {"vid": "1a86", "pid": "7523", "revision": "26"}}), "4 hex digits"),
        (json!({"usb": {"vid": "1a86", "pid": "7523"}, "colour": "red"}), "unknown field"),
    ] {
        let e = json!({"id": "bad", "name": "B", "driver": "owon_oel", "transport": "serial", "match": bad});
        let (code, v, _) = call(&f, "POST", "/devices", Some(e), None).await;
        assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(v["detail"].as_str().unwrap().contains(why), "{v}");
    }
    let (code, v, _) = call(&f, "POST", "/devices", Some(json!({"id": "np", "name": "N", "driver": "owon_oel", "transport": "serial"})), None).await;
    assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(v["detail"].as_str().unwrap().contains("`port` or `match`"));
}

#[tokio::test(flavor = "multi_thread")]
async fn detect_lists_nothing_without_real_ports() {
    let f = fixture("detect");
    let (code, v, _) = call(&f, "POST", "/devices/detect", None, None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(v["ports"], json!([]), "simulated benches never probe hardware");
    assert_eq!(v["running"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_wire_log_shows_what_went_to_and_from_an_instrument() {
    let f = fixture("wire");
    // the simulated Tenma has connected and been polled once the registry knows its IDN
    let mut seen = json!(null);
    for _ in 0..100 {
        let (code, v, _) = call(&f, "GET", "/devices/t1/wire", None, None).await;
        assert_eq!(code, StatusCode::OK);
        if v["entries"].as_array().unwrap().iter().any(|e| e["dir"] == "rx") {
            seen = v;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let entries = seen["entries"].as_array().expect("rx entries within 5 s");
    assert_eq!(entries[0]["dir"], "info");
    assert!(entries[0]["data"].as_str().unwrap().starts_with("opened "));
    assert!(entries.iter().any(|e| e["dir"] == "tx" && e["data"] == "*IDN?"), "identify sent");
    let next = seen["next"].as_u64().unwrap();
    let (_, later, _) = call(&f, "GET", &format!("/devices/t1/wire?after={next}"), None, None).await;
    assert!(later["entries"].as_array().unwrap().iter().all(|e| e["seq"].as_u64().unwrap() > next));
    let (code, _, _) = call(&f, "DELETE", "/devices/t1/wire", None, None).await;
    assert_eq!(code, StatusCode::NO_CONTENT);
    let (code, _, _) = call(&f, "GET", "/devices/nope/wire", None, None).await;
    assert_eq!(code, StatusCode::NOT_FOUND);
    let (code, _, _) = call(&f, "GET", "/devices/t1/wire?after=x", None, None).await;
    assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread")]
async fn catalog_lists_families_with_model_support() {
    let f = fixture("catalog");
    let (code, v, _) = call(&f, "GET", "/catalog", None, None).await;
    assert_eq!(code, StatusCode::OK);
    let tenma = v["families"].as_array().unwrap().iter().find(|f| f["driver"] == "tenma_72").unwrap();
    assert_eq!(tenma["classes"], json!(["PSU"]));
    assert_eq!(tenma["transports"], json!(["serial"]));
    let m = tenma["models"].as_array().unwrap();
    assert_eq!(m.iter().find(|m| m["name"] == "72-2540").unwrap()["support"], "tested");
    assert_eq!(m.iter().find(|m| m["name"] == "72-2545").unwrap()["support"], "awaiting");
    assert!(v["models"]["tested"].as_u64().unwrap() >= 6, "the bench instruments");
}

#[tokio::test(flavor = "multi_thread")]
async fn usbtmc_driver_install_validates_the_request() {
    let f = fixture("winusb");
    let (code, _, _) = call(&f, "POST", "/usbtmc-devices/driver", Some(json!({})), None).await;
    assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY);
    let (code, v, _) = call(&f, "POST", "/usbtmc-devices/driver", Some(json!({"device": "usb:0000:0000:NOT-ATTACHED"})), None).await;
    assert_eq!(code, StatusCode::NOT_FOUND);
    assert!(v["detail"].as_str().unwrap().contains("no such USBTMC instrument"));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_is_served_with_spa_routes() {
    let f = fixture("ui");
    for p in ["/ui/", "/ui/metrics", "/ui/docs"] {
        let (code, v, _) = call(&f, "GET", p, None, None).await;
        assert_eq!(code, StatusCode::OK, "{p}");
        assert!(v.as_str().unwrap().contains("BenchMesh UI"), "{p}: {v}");
    }
    let resp = router(f.state.clone()).oneshot(Request::get("/").body(Body::empty()).unwrap()).await.unwrap();
    assert!(resp.status().is_redirection());
    let resp = router(f.state.clone()).oneshot(Request::get("/ui/").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.headers().get(header::CACHE_CONTROL).unwrap(), "no-cache", "UI must be revalidated after upgrades");
}

#[tokio::test(flavor = "multi_thread")]
async fn registry_websocket_streams_python_shape() {
    let f = fixture("ws");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(f.state.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws/registry")).await.unwrap();
    let msg: Value = serde_json::from_str(&next_text(&mut ws).await).unwrap();
    assert_eq!(msg["t1"]["IDN"], "TENMA 72-2540 V2.1");
    assert_eq!(msg["t1"]["PSU"]["status_ch1"]["OUTPUT"], "OFF");
    assert_eq!(msg["l1"]["ELL"]["status_ch1"]["MODE"], "CURR");
    // a second frame follows within the interval
    let started = Instant::now();
    next_text(&mut ws).await;
    assert!(started.elapsed() < Duration::from_millis(500));
}

async fn next_text<S>(ws: &mut S) -> String
where
    S: futures_util::Stream<Item = Result<tokio_tungstenite::tungstenite::Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    use futures_util::StreamExt;
    loop {
        match tokio::time::timeout(Duration::from_secs(3), ws.next()).await.expect("ws frame in time") {
            Some(Ok(tokio_tungstenite::tungstenite::Message::Text(t))) => return t.to_string(),
            Some(Ok(_)) => continue,
            other => panic!("websocket ended: {other:?}"),
        }
    }
}

// ---- recordings ---------------------------------------------------------------------

async fn add_series(f: &Fixture, id: i64, body: Value) -> (StatusCode, Value, Option<String>) {
    call(f, "POST", &format!("/recordings/{id}/series"), Some(body), None).await
}

async fn recording_of(f: &Fixture, id: i64) -> Value {
    let (code, v, _) = call(f, "GET", &format!("/recordings/{id}"), None, None).await;
    assert_eq!(code, StatusCode::OK, "{v}");
    v
}

async fn series_data(f: &Fixture, id: i64, query: &str) -> Value {
    let (code, v, _) = call(f, "GET", &format!("/recordings/{id}/data{query}"), None, None).await;
    assert_eq!(code, StatusCode::OK, "{v}");
    v
}

fn ts(series: &Value) -> Vec<i64> {
    series["t"].as_array().unwrap().iter().map(|t| t.as_i64().unwrap()).collect()
}

/// Wait (bounded) until `cond` holds for the recording's data.
async fn wait_data(f: &Fixture, id: i64, what: &str, cond: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let d = series_data(f, id, "").await;
        if cond(&d) {
            return d;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}: {d}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn recordings_share_one_time_domain_and_lifecycle() {
    let f = fixture("rec");
    // Tenma output on → VOUT reads the 4.94 V setpoint
    assert_eq!(call(&f, "POST", "/instruments/PSU/t1/1/output/ON", None, None).await.0, StatusCode::NO_CONTENT);

    let (code, r1, _) = call(&f, "POST", "/recordings", Some(json!({"name": " one ", "interval_s": 1})), None).await;
    assert_eq!(code, StatusCode::CREATED, "{r1}");
    assert_eq!((r1["name"].clone(), r1["state"].clone(), r1["series"].clone()), (json!("one"), json!("recording"), json!([])));
    assert_eq!((r1["description"].clone(), r1["samples"].clone(), r1["last_t"].clone()), (json!(""), json!(0), Value::Null));
    assert!(r1["started_at"].as_i64().unwrap() > 1_700_000_000_000, "ms since epoch: {r1}");
    let id1 = r1["id"].as_i64().unwrap();
    let (code, r2, _) = call(&f, "POST", "/recordings", Some(json!({"name": "two", "description": "d", "interval_s": 2})), None).await;
    assert_eq!(code, StatusCode::CREATED);
    let id2 = r2["id"].as_i64().unwrap();

    let (code, v, _) = call(&f, "POST", "/recordings", Some(json!({"name": "one", "interval_s": 1})), None).await;
    assert_eq!(code, StatusCode::CONFLICT, "{v}");
    for body in [json!({"name": "  ", "interval_s": 1}), json!({"name": "x".repeat(101), "interval_s": 1}),
                 json!({"name": "x", "interval_s": 0}), json!({"name": "x", "interval_s": 3601}),
                 json!({"name": "x", "interval_s": 1.5}), json!({"name": "x"}), json!({"interval_s": 1}), json!([1])] {
        let (code, v, _) = call(&f, "POST", "/recordings", Some(body.clone()), None).await;
        assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY, "{body} → {v}");
        assert!(v["detail"].is_string());
    }

    // series: valid (default label from the configured device name) and every invalid case
    let vout = json!({"device_id": "t1", "class": "PSU", "channel": 1, "field": "VOUT", "unit": "V"});
    let (code, s1, _) = add_series(&f, id1, vout.clone()).await;
    assert_eq!(code, StatusCode::CREATED, "{s1}");
    assert_eq!((s1["label"].clone(), s1["unit"].clone(), s1["removed_at"].clone()), (json!("Tenma · VOUT"), json!("V"), Value::Null));
    assert_eq!((s1["device_id"].clone(), s1["class"].clone(), s1["channel"].clone(), s1["field"].clone()),
               (json!("t1"), json!("PSU"), json!(1), json!("VOUT")));
    assert_eq!(add_series(&f, id1, vout.clone()).await.0, StatusCode::CONFLICT, "signal already active");
    let (code, s2, _) = add_series(&f, id2, vout.clone()).await;
    assert_eq!(code, StatusCode::CREATED);
    let (code, s3, _) = add_series(&f, id2, json!({"device_id": "l1", "class": "ELL", "channel": 1, "field": "VIN", "label": "Load V"})).await;
    assert_eq!(code, StatusCode::CREATED);
    assert_eq!((s3["label"].clone(), s3["unit"].clone()), (json!("Load V"), json!("")));
    for (body, why) in [
        (json!({"device_id": "zz", "class": "PSU", "channel": 1, "field": "VOUT"}), "unknown device"),
        (json!({"device_id": "t1", "class": "ELL", "channel": 1, "field": "VIN"}), "class not on the device"),
        (json!({"device_id": "t1", "class": "PSU", "channel": 2, "field": "VOUT"}), "channel out of range"),
        (json!({"device_id": "t1", "class": "PSU", "channel": 0, "field": "VOUT"}), "channel 0"),
        (json!({"device_id": "t1", "class": "PSU", "channel": 1, "field": "OUTPUT"}), "enum field"),
        (json!({"device_id": "t1", "class": "PSU", "channel": 1, "field": "CV"}), "bool field"),
        (json!({"device_id": "t1", "class": "PSU", "channel": 1, "field": "NOPE"}), "unknown field"),
        (json!({"device_id": "t1", "class": "PSU", "channel": 1}), "missing field"),
    ] {
        let (code, v, _) = add_series(&f, id1, body).await;
        assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY, "{why}: {v}");
    }
    assert_eq!(add_series(&f, 999, vout.clone()).await.0, StatusCode::NOT_FOUND);
    let (sid1, sid2, sid3) = (s1["id"].as_i64().unwrap(), s2["id"].as_i64().unwrap(), s3["id"].as_i64().unwrap());

    // two samples in the 2 s recording ⇒ several ticks of the 1 s one
    let d2 = wait_data(&f, id2, "two samples at interval 2", |d| d["series"][0]["t"].as_array().is_some_and(|t| t.len() >= 2)).await;
    let d1 = series_data(&f, id1, "").await;
    let t1 = ts(&d1["series"][0]);
    let t2 = ts(&d2["series"][0]);
    assert!(t1.len() >= 3, "{d1}");
    assert!(t1.iter().chain(&t2).all(|t| t % 1000 == 0), "whole seconds: {t1:?} {t2:?}");
    assert!(t2.iter().all(|t| (t / 1000) % 2 == 0), "interval 2 → even seconds: {t2:?}");
    assert!(t2.iter().all(|t| t1.contains(t)), "common time domain: {t1:?} ⊇ {t2:?}");
    assert!(t1.windows(2).all(|w| w[1] - w[0] == 1000), "one sample per second: {t1:?}");
    assert!(d1["series"][0]["v"].as_array().unwrap().iter().all(|v| v == &json!(4.94)), "{d1}");
    assert_eq!(d2["series"][1]["id"], sid3);
    let t3 = ts(&d2["series"][1]);
    assert!(!t3.is_empty() && t3.iter().all(|t| t2.contains(t)), "both series of a recording share the tick: {t2:?} {t3:?}");
    let r = recording_of(&f, id1).await;
    assert!(r["samples"].as_i64().unwrap() >= 3 && r["last_t"].as_i64().unwrap() % 1000 == 0, "{r}");

    // remove a series: no more samples, data kept
    assert_eq!(call(&f, "DELETE", &format!("/recordings/{id1}/series/{sid1}"), None, None).await.0, StatusCode::NO_CONTENT);
    assert_eq!(call(&f, "DELETE", &format!("/recordings/{id1}/series/{sid2}"), None, None).await.0, StatusCode::NOT_FOUND, "series of another recording");
    let removed_at = recording_of(&f, id1).await["series"][0]["removed_at"].as_i64().unwrap();
    let n2 = t2.len();
    wait_data(&f, id2, "another tick", |d| d["series"][0]["t"].as_array().is_some_and(|t| t.len() > n2)).await;
    let after = ts(&series_data(&f, id1, "").await["series"][0]);
    assert!(after.len() >= t1.len() && after.iter().all(|t| *t <= removed_at), "removed_at {removed_at}: {after:?}");

    // pause / resume / stop (idempotent, stop is final)
    let p1 = call(&f, "POST", &format!("/recordings/{id2}/pause"), None, None).await.1;
    assert_eq!(p1["state"], "paused");
    let p2 = call(&f, "POST", &format!("/recordings/{id2}/pause"), None, None).await.1;
    assert_eq!(p1["paused_at"], p2["paused_at"]);
    assert!(p1["paused_at"].is_i64());
    let x = call(&f, "POST", &format!("/recordings/{id2}/resume"), None, None).await.1;
    assert_eq!((x["state"].clone(), x["paused_at"].clone()), (json!("recording"), Value::Null));
    let st1 = call(&f, "POST", &format!("/recordings/{id2}/stop"), None, None).await.1;
    assert_eq!(st1["state"], "stopped");
    assert!(st1["stopped_at"].is_i64());
    let st2 = call(&f, "POST", &format!("/recordings/{id2}/stop"), None, None).await.1;
    assert_eq!(st1["stopped_at"], st2["stopped_at"]);
    assert_eq!(call(&f, "POST", &format!("/recordings/{id2}/resume"), None, None).await.1["state"], "stopped");
    assert_eq!(add_series(&f, id2, json!({"device_id": "t1", "class": "PSU", "channel": 1, "field": "IOUT"})).await.0, StatusCode::CONFLICT);
    assert_eq!(call(&f, "POST", "/recordings/999/pause", None, None).await.0, StatusCode::NOT_FOUND);

    // data: range and max_points
    let all = ts(&series_data(&f, id2, "").await["series"][0]);
    let first = all[0];
    let one = series_data(&f, id2, &format!("?from={first}&to={first}&max_points=10")).await;
    assert_eq!((one["from"].clone(), one["to"].clone(), one["recording_id"].clone()), (json!(first), json!(first), json!(id2)));
    assert_eq!(ts(&one["series"][0]), [first]);
    assert_eq!(ts(&series_data(&f, id2, &format!("?from={}", first + 1)).await["series"][0]), all[1..].to_vec());
    for q in ["?max_points=9", "?max_points=20001", "?from=x", "?from=5&to=4"] {
        assert_eq!(call(&f, "GET", &format!("/recordings/{id2}/data{q}"), None, None).await.0, StatusCode::UNPROCESSABLE_ENTITY, "{q}");
    }
    assert_eq!(call(&f, "GET", "/recordings/999/data", None, None).await.0, StatusCode::NOT_FOUND);

    // CSV
    let resp = router(f.state.clone()).oneshot(Request::get(format!("/recordings/{id2}/export")).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()[header::CONTENT_TYPE], "text/csv; charset=utf-8");
    assert!(resp.headers()[header::CONTENT_DISPOSITION].to_str().unwrap().contains("two.csv"));
    let csv = String::from_utf8(axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
    let lines: Vec<&str> = csv.split("\r\n").filter(|l| !l.is_empty()).collect();
    assert_eq!(lines[0], "time_utc,time_ms,Tenma · VOUT [V],Load V");
    assert_eq!(lines.len() - 1, all.len(), "one row per distinct t: {csv}");
    let cells: Vec<&str> = lines[1].split(',').collect();
    assert!(cells[0].ends_with(".000Z") && cells[1] == first.to_string() && cells[2] == "4.94" && cells[3] == "0", "{}", lines[1]);

    // rename / describe
    let (code, v, _) = call(&f, "PATCH", &format!("/recordings/{id2}"), Some(json!({"name": "renamed", "description": "notes"})), None).await;
    assert_eq!(code, StatusCode::OK, "{v}");
    assert_eq!((v["name"].clone(), v["description"].clone()), (json!("renamed"), json!("notes")));
    assert_eq!(call(&f, "PATCH", &format!("/recordings/{id2}"), Some(json!({"name": "one"})), None).await.0, StatusCode::CONFLICT);
    assert_eq!(call(&f, "PATCH", &format!("/recordings/{id2}"), Some(json!({"name": ""})), None).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(call(&f, "PATCH", "/recordings/999", Some(json!({"name": "z"})), None).await.0, StatusCode::NOT_FOUND);

    let (_, list, _) = call(&f, "GET", "/recordings", None, None).await;
    let names: Vec<&str> = list.as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["renamed", "one"], "newest first");

    assert_eq!(call(&f, "DELETE", &format!("/recordings/{id1}"), None, None).await.0, StatusCode::NO_CONTENT);
    assert_eq!(call(&f, "GET", &format!("/recordings/{id1}"), None, None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&f, "DELETE", &format!("/recordings/{id1}"), None, None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&f, "GET", "/recordings/abc", None, None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn recordings_websocket_snapshot_samples_changes_and_deletion() {
    let f = fixture("recws");
    let (_, old, _) = call(&f, "POST", "/recordings", Some(json!({"name": "existing", "interval_s": 60})), None).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(f.state.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws/recordings")).await.unwrap();
    let snap: Value = serde_json::from_str(&next_text(&mut ws).await).unwrap();
    assert_eq!(snap["type"], "snapshot");
    assert_eq!(snap["recordings"][0]["id"], old["id"]);

    let (_, r, _) = call(&f, "POST", "/recordings", Some(json!({"name": "live", "interval_s": 1})), None).await;
    let id = r["id"].as_i64().unwrap();
    let m: Value = serde_json::from_str(&next_text(&mut ws).await).unwrap();
    assert_eq!((m["type"].clone(), m["recording"]["id"].clone()), (json!("recording"), json!(id)));
    let (_, s, _) = call(&f, "POST", &format!("/recordings/{id}/series"),
        Some(json!({"device_id": "t1", "class": "PSU", "channel": 1, "field": "VSET"})), None).await;
    assert_eq!(s["detail"].as_str().map(|d| d.contains("not a numeric status key")), Some(true), "{s}");
    let (code, s, _) = call(&f, "POST", &format!("/recordings/{id}/series"),
        Some(json!({"device_id": "l1", "class": "ELL", "channel": 1, "field": "VIN"})), None).await;
    assert_eq!(code, StatusCode::CREATED);
    let m: Value = serde_json::from_str(&next_text(&mut ws).await).unwrap();
    assert_eq!(m["type"], "recording");
    assert_eq!(m["recording"]["series"][0]["id"], s["id"]);
    assert_eq!(m["recording"]["series"][0]["label"], "Load · VIN");

    let sid = s["id"].as_i64().unwrap().to_string();
    let samples = loop {
        let m: Value = serde_json::from_str(&next_text(&mut ws).await).unwrap();
        if m["type"] == "samples" {
            break m;
        }
    };
    assert_eq!(samples["recording_id"], id);
    assert_eq!(samples["t"].as_i64().unwrap() % 1000, 0, "{samples}");
    assert_eq!(samples["values"], json!({sid: 0.0}), "every active series, value from the registry");

    assert_eq!(call(&f, "POST", &format!("/recordings/{id}/stop"), None, None).await.0, StatusCode::OK);
    assert_eq!(call(&f, "DELETE", &format!("/recordings/{id}"), None, None).await.0, StatusCode::NO_CONTENT);
    let mut seen = Vec::new();
    loop {
        let m: Value = serde_json::from_str(&next_text(&mut ws).await).unwrap();
        seen.push(m["type"].as_str().unwrap().to_string());
        if m["type"] == "deleted" {
            assert_eq!(m["recording_id"], id);
            break;
        }
    }
    assert!(seen.contains(&"recording".to_string()), "stop announced: {seen:?}");
}

// ---- metrics, AI context, OpenAPI ---------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn metrics_endpoints_and_websocket() {
    let f = fixture("metrics");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut summary = Value::Null;
    while Instant::now() < deadline {
        let (_, v, _) = call(&f, "GET", "/metrics", None, None).await;
        if v["summary"]["t1"].is_object() && v["summary"]["l1"].is_object() {
            summary = v;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let m = &summary["summary"]["t1"];
    assert_eq!(m["device_id"], "t1", "{summary}");
    assert_eq!(m["transport_type"], "serial");
    for k in ["utilization_pct", "qps", "window_duration_s", "avg_queue_depth", "avg_poll_duration_ms"] {
        assert!(m[k].is_number(), "{k}: {m}");
    }
    assert!(summary["timestamp"].as_f64().unwrap() > 1.7e9);
    let (code, d, _) = call(&f, "GET", "/metrics/t1", None, None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(d["timestamp"].is_number() && d["qps"].is_number());
    assert_eq!(call(&f, "GET", "/metrics/zz", None, None).await.0, StatusCode::NOT_FOUND);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(f.state.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws/metrics")).await.unwrap();
    let frame: Value = serde_json::from_str(&next_text(&mut ws).await).unwrap();
    assert!(frame["t1"]["qps"].is_number(), "bare summary map: {frame}");
}

#[tokio::test(flavor = "multi_thread")]
async fn ai_context_describes_instruments_from_definitions() {
    let f = fixture("ai");
    let (code, v, _) = call(&f, "GET", "/ai/context?format=json", None, None).await;
    assert_eq!(code, StatusCode::OK);
    let t1 = v["instruments"].as_array().unwrap().iter().find(|i| i["id"] == "t1").unwrap();
    assert_eq!(t1["status"], "online");
    let ops = t1["classes"][0]["operations"].as_array().unwrap();
    let voltage = ops.iter().find(|o| o["name"] == "voltage").unwrap();
    assert_eq!((voltage["get"].clone(), voltage["set"].clone()), (json!(true), json!(true)));
    assert_eq!(voltage["value"]["max"], 60.0);
    assert_eq!(ops.iter().find(|o| o["name"] == "output").unwrap()["energizes"], true);
    let (_, v, _) = call(&f, "GET", "/ai/context?format=json&include=safety", None, None).await;
    assert!(v.get("instruments").is_none() && v["safety"].is_array());
    let (_, md, _) = call(&f, "GET", "/ai/context", None, None).await;
    let md = md.as_str().unwrap();
    assert!(md.contains("## System Overview") && md.contains("🟢") && md.contains("energizes"), "{md}");
}

#[tokio::test(flavor = "multi_thread")]
async fn openapi_and_docs() {
    let f = fixture("openapi");
    let (code, v, _) = call(&f, "GET", "/openapi.json", None, None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(v["openapi"], "3.1.0");
    assert!(v["paths"]["/recordings"]["post"].is_object());
    assert!(v["paths"]["/ws/recordings"]["get"].is_object());
    let resp = router(f.state.clone()).oneshot(Request::get("/openapi.yaml").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.headers()[header::CONTENT_TYPE], "application/x-yaml");
    let resp = router(f.state.clone()).oneshot(Request::get("/docs").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.headers()[header::LOCATION], "/ui/docs");
}

#[tokio::test(flavor = "multi_thread")]
async fn other_websites_cannot_drive_instruments_through_the_browser() {
    let f = fixture("guard");
    let send = |method: &str, uri: &str, headers: &[(&str, &str)]| {
        let mut req = Request::builder().method(method).uri(uri);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        router(f.state.clone()).oneshot(req.body(Body::empty()).unwrap())
    };
    // a page on another site posting a setter: refused before it reaches the instrument
    let r = send("POST", "/instruments/PSU/t1/1/voltage/5", &[("host", "127.0.0.1:57666"), ("origin", "https://evil.example")]).await.unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    // a WebSocket upgrade from another site (CORS never applies to WebSockets)
    let ws = [("host", "127.0.0.1:57666"), ("origin", "https://evil.example"), ("connection", "upgrade"), ("upgrade", "websocket"), ("sec-websocket-version", "13"), ("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")];
    assert_eq!(send("GET", "/ws/registry", &ws).await.unwrap().status(), StatusCode::FORBIDDEN);
    // DNS rebinding: the attacker's own name pointed at 127.0.0.1
    let r = send("GET", "/instruments", &[("host", "evil.example:57666"), ("origin", "http://evil.example:57666")]).await.unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    // no CORS grant for other origins
    let r = send("GET", "/instruments", &[("host", "127.0.0.1:57666")]).await.unwrap();
    assert!(r.headers().get("access-control-allow-origin").is_none());
    // the UI served by this server, and scripts without an Origin, work
    let r = send("POST", "/instruments/PSU/t1/1/voltage/5", &[("host", "127.0.0.1:57666"), ("origin", "http://127.0.0.1:57666")]).await.unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    assert_eq!(send("GET", "/instruments", &[("host", "localhost:57666")]).await.unwrap().status(), StatusCode::OK);
}
