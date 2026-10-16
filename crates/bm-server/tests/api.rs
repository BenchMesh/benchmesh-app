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
        Box::new(move || {
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
    let devices = Arc::new(Devices::new(catalog, sim_connectors(), ActorConfig::default()));
    for e in cfg.load().unwrap() {
        devices.start(e).unwrap();
    }
    let state = Arc::new(AppState {
        devices,
        config: cfg,
        ui_dir: Some(dir.join("ui")),
        ws_interval: Duration::from_millis(50),
        config_lock: Default::default(),
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
async fn ui_is_served_with_spa_routes() {
    let f = fixture("ui");
    for p in ["/ui/", "/ui/metrics", "/ui/docs"] {
        let (code, v, _) = call(&f, "GET", p, None, None).await;
        assert_eq!(code, StatusCode::OK, "{p}");
        assert!(v.as_str().unwrap().contains("BenchMesh UI"), "{p}: {v}");
    }
    let resp = router(f.state.clone()).oneshot(Request::get("/").body(Body::empty()).unwrap()).await.unwrap();
    assert!(resp.status().is_redirection());
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
