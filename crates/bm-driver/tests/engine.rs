//! Engine + actor behaviour against simulated devices (replayed captures and small
//! stateful simulators). No hardware needed.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bm_devicedef::{load_catalog, DeviceDef, RxMode};
use bm_driver::actor::{ActorConfig, DeviceHandle, Event};
use bm_driver::engine::{Engine, Output, Transport};
use bm_driver::sim::{SimTransport, WireLog};
use bm_driver::Error;
use serde_json::{json, Value};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn def(id: &str) -> Arc<DeviceDef> {
    let cat = load_catalog(&repo().join("devices")).unwrap();
    Arc::new(cat.devices[id].1.clone())
}

fn sim(responder: impl FnMut(&[u8]) -> Option<Vec<u8>> + Send + 'static) -> (Box<dyn Transport>, Arc<Mutex<WireLog>>) {
    let t = SimTransport::new(Box::new(responder));
    let log = t.log.clone();
    (Box::new(t), log)
}

fn text(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// Responder answering every command with the last reply it got in the setter capture
/// session (device answers reconstructed as sent: Python split XDM `\r\n` replies).
fn replay_responder(def: &DeviceDef, device_id: &str) -> impl FnMut(&[u8]) -> Option<Vec<u8>> + Send + 'static {
    let path = repo().join(format!("captures/20261016-203514/{device_id}.jsonl.gz"));
    let rd = BufReader::new(flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap()));
    let mut table: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut cur: Option<(String, Vec<u8>)> = None;
    let unhex = |h: &str| -> Vec<u8> { (0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap()).collect() };
    let rx = def.framing.rx.clone();
    let finish = |c: Option<(String, Vec<u8>)>, table: &mut BTreeMap<String, Vec<u8>>| {
        if let Some((tx, mut r)) = c {
            while r.first().is_some_and(|b| rx.skip_leading.as_bytes().contains(b)) {
                r.remove(0);
            }
            if let (RxMode::Terminator, Some(t)) = (rx.mode, rx.terminator.as_deref()) {
                if !r.is_empty() && !text(&r).contains(t) {
                    r.extend(t.as_bytes());
                }
            }
            if !r.is_empty() {
                table.insert(tx, r);
            }
        }
    };
    for line in rd.lines() {
        let e: Value = serde_json::from_str(&line.unwrap()).unwrap();
        match e["ev"].as_str().unwrap() {
            "tx" => {
                finish(cur.take(), &mut table);
                cur = Some((text(&unhex(e["hex"].as_str().unwrap())), Vec::new()));
            }
            "rx" => {
                if let Some((_, r)) = cur.as_mut() {
                    r.extend(unhex(e["hex"].as_str().unwrap()));
                }
            }
            _ => {}
        }
    }
    finish(cur, &mut table);
    move |w: &[u8]| table.get(&text(w)).cloned()
}

// ---------------------------------------------------------------------------------------

#[test]
fn every_bench_device_connects_and_polls_from_its_capture() {
    type Expect = (&'static str, u8, &'static str, Value);
    let cases: &[(&str, &str, &[Expect])] = &[
        (
            "tenma_72",
            "tenma-72-2540",
            &[("PSU", 1, "OUTPUT", json!("OFF")), ("PSU", 1, "SBY", json!(true)), ("PSU", 1, "REMOTE", json!("ON"))],
        ),
        ("owon_spm", "owon-spm3103", &[("PSU", 1, "OUTPUT", json!("OFF")), ("PSU", 1, "FAIL", json!(false))]),
        ("owon_xdm", "owon-xdm1241", &[("DMM", 1, "MODE", json!("VOLT"))]),
        (
            "owon_oel",
            "owon-oel1515",
            &[("ELL", 1, "REMOTE", json!("ON")), ("ELL", 1, "MODE", json!("CURR")), ("ELL", 1, "INPUT", json!("OFF"))],
        ),
        ("owon_dge", "owon-dge2070", &[("AWG", 1, "OUTPUT", json!("OFF")), ("AWG", 2, "OUTPUT", json!("OFF"))]),
    ];
    for (id, dev, expect) in cases {
        let d = def(id);
        let mut responder = replay_responder(&d, dev);
        // Queries the Python service never sent (new in the definitions) get a plausible value.
        let (t, log) = sim(move |w| responder(w).or_else(|| text(w).trim_end().ends_with('?').then(|| b"1.0\n".to_vec())));
        let mut e = Engine::connect(d.clone(), t).unwrap_or_else(|err| panic!("{id}: connect failed: {err}"));
        let updates = e.poll_due().unwrap();
        for (class, ch, key, want) in *expect {
            let u = updates.iter().find(|u| u.class == *class && u.ch == *ch).unwrap_or_else(|| panic!("{id}: no {class} ch{ch} update"));
            assert_eq!(&u.status[*key], want, "{id} {class} ch{ch} {key}: {:?}", u.status);
        }
        // Every required status key of the class contract is present.
        let cat = load_catalog(&repo().join("devices")).unwrap();
        for u in &updates {
            for (k, spec) in &cat.classes[&u.class].status {
                assert!(!spec.required || u.status.contains_key(k), "{id}: missing {k}");
            }
        }
        if *id == "owon_oel" {
            assert_eq!(log.lock().unwrap().texts()[1], "SYSTem:REMote\r", "OEL enters remote on connect");
        }
        e.disconnect();
    }
}

/// Minimal TENMA: no terminators, fixed-width replies, setters silent.
fn tenma(state: Arc<Mutex<(f64, f64, bool)>>) -> impl FnMut(&[u8]) -> Option<Vec<u8>> + Send + 'static {
    move |w| {
        let c = text(w);
        let mut s = state.lock().unwrap();
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
                } else if let Some(v) = c.strip_prefix("ISET1:") {
                    s.1 = v.parse().unwrap();
                } else if c == "OUT1" || c == "OUT0" {
                    s.2 = c == "OUT1";
                }
                None
            }
        }
    }
}

#[test]
fn tenma_setter_is_formatted_never_read_and_followed_by_a_gap() {
    let state = Arc::new(Mutex::new((4.94, 0.988, false)));
    let (t, log) = sim(tenma(state.clone()));
    let mut e = Engine::connect(def("tenma_72"), t).unwrap();
    e.set("PSU", "voltage", 1, "5").unwrap();
    let v = e.get("PSU", "voltage", 1).unwrap().value();
    assert_eq!(v["si"]["number"], "5");
    let log = log.lock().unwrap();
    let texts = log.texts();
    let i = texts.iter().position(|w| w == "VSET1:5.00").expect("setpoint formatted %.2f");
    assert_eq!(texts[i + 1], "VSET1?");
    let gap = log.writes[i + 1].0 - log.writes[i].0;
    assert!(gap >= Duration::from_millis(50), "only {gap:?} after a TENMA setter");
    assert_eq!(state.lock().unwrap().0, 5.0);
}

#[test]
fn tenma_poll_is_fast_and_derives_status() {
    let state = Arc::new(Mutex::new((5.0, 0.5, true)));
    let (t, log) = sim(tenma(state));
    let mut e = Engine::connect(def("tenma_72"), t).unwrap();
    let started = Instant::now();
    let u = e.poll_due().unwrap();
    assert!(started.elapsed() < Duration::from_millis(200), "poll took {:?}", started.elapsed());
    let s = &u[0].status;
    assert_eq!((s["OUTPUT"].clone(), s["CV"].clone(), s["SBY"].clone()), (json!("ON"), json!(true), json!(false)));
    assert_eq!(s["POUT"]["si"]["number"], "0");
    // 3 queries per poll (Python sent 5)
    assert_eq!(log.lock().unwrap().texts()[1..], ["VOUT1?", "IOUT1?", "STATUS?"]);
}

#[test]
fn invalid_argument_writes_nothing() {
    let (t, log) = sim(tenma(Arc::new(Mutex::new((4.94, 0.988, false)))));
    let mut e = Engine::connect(def("tenma_72"), t).unwrap();
    assert!(matches!(e.set("PSU", "voltage", 1, "61"), Err(Error::BadArgument(_))));
    assert!(matches!(e.set("PSU", "output", 1, "maybe"), Err(Error::BadArgument(_))));
    assert!(matches!(e.set("PSU", "nope", 1, "1"), Err(Error::NotFound(_))));
    assert!(matches!(e.get("PSU", "voltage", 2), Err(Error::BadArgument(_))));
    assert_eq!(log.lock().unwrap().texts(), ["*IDN?"]);
}

fn oel(func: &'static str) -> impl FnMut(&[u8]) -> Option<Vec<u8>> + Send + 'static {
    move |w| match text(w).trim_end() {
        "*IDN?" => Some(b"OWON,OEL1515,25130087,FV:V1.1.0\n".to_vec()),
        "SYSTem:REMote?" => Some(b"1\n".to_vec()),
        "FUNC?" => Some(format!("{func}\n").into_bytes()),
        "MEAS:ALL:INFO?" => Some(b"0.000,0.000,0.000,OFF,OFF,OFF\n".to_vec()),
        "INP?" => Some(b"OFF\n".to_vec()),
        _ => None,
    }
}

#[test]
fn oel_mode_gated_setpoint_is_refused_with_reason() {
    let (t, log) = sim(oel("current"));
    let mut e = Engine::connect(def("owon_oel"), t).unwrap();
    match e.set("ELL", "volt", 1, "10") {
        Err(Error::Refused(m)) => assert!(m.contains("MODE=VOLT"), "{m}"),
        other => panic!("expected refusal, got {other:?}"),
    }
    e.set("ELL", "curr", 1, "0.5").unwrap();
    let texts = log.lock().unwrap().texts();
    assert!(texts.contains(&"CURR 0.500\r".to_string()));
    assert!(!texts.iter().any(|w| w.starts_with("VOLT ")), "refused setter must not be sent");
}

#[test]
fn oel_local_mode_is_not_a_link_failure() {
    let (t, _) = sim(|w: &[u8]| match text(w).trim_end() {
        "*IDN?" => Some(b"OWON,OEL1515,25130087,FV:V1.1.0\n".to_vec()),
        "SYSTem:REMote?" => Some(b"0\n".to_vec()),
        q if q.ends_with('?') => Some(b"\n".to_vec()), // local mode: empty replies
        _ => None,
    });
    let mut e = Engine::connect(def("owon_oel"), t).unwrap();
    let u = e.poll_due().unwrap();
    let s = &u[0].status;
    assert_eq!(s["REMOTE"], "OFF");
    assert_eq!(s["VIN"], Value::Null);
    assert_eq!(e.silent_polls, 0, "empty replies in local mode are answers, not silence");
}

#[test]
fn foreign_reply_is_rejected_and_the_stream_recovers() {
    let calls = Arc::new(Mutex::new(0));
    let c2 = calls.clone();
    let (t, _) = sim(move |w: &[u8]| match text(w).trim_end() {
        "*IDN?" => Some(b"OWON,XDM1241,24512010,V4.3.0,3\r\n".to_vec()),
        "MEAS1?" => {
            let mut n = c2.lock().unwrap();
            *n += 1;
            Some(if *n == 1 { b"OWON,XDM1241,24512010,V4.3.0,3\r\n".to_vec() } else { b"-7.941000E-05\r\n".to_vec() })
        }
        "FUNCtion?" => Some(b"\"VOLT\"\r\n".to_vec()),
        _ => None,
    });
    let mut e = Engine::connect(def("owon_xdm"), t).unwrap();
    assert!(matches!(e.query("meas", 1), Err(Error::Parse(_))), "IDN text is not a measurement");
    assert_eq!(e.query("meas", 1).unwrap(), json!(-7.941e-05));
}

#[test]
fn silence_gives_null_status_and_is_counted() {
    let (t, _) = sim(|w: &[u8]| (text(w) == "*IDN?").then(|| b"TENMA 72-2540 V2.1".to_vec()));
    let mut e = Engine::connect(def("tenma_72"), t).unwrap();
    let u = e.poll_due().unwrap();
    assert_eq!(u[0].status["VOUT"], Value::Null);
    assert_eq!(u[0].status["OUTPUT"], Value::Null);
    assert_eq!(e.silent_polls, 1);
}

#[test]
fn readings_are_null_while_settling_after_a_mode_change() {
    let mut d = (*def("owon_xdm")).clone();
    let m = d.commands.get_mut("mode_set").unwrap();
    m.gap_after_ms = Some(10);
    m.settle_ms = Some(200);
    let (t, log) = sim(|w: &[u8]| match text(w).trim_end() {
        "*IDN?" => Some(b"OWON,XDM1241,24512010,V4.3.0,3\r\n".to_vec()),
        "MEAS1?" => Some(b"1.000000E+00\r\n".to_vec()),
        "FUNCtion?" => Some(b"\"VOLT AC\"\r\n".to_vec()),
        _ => None,
    });
    let mut e = Engine::connect(Arc::new(d), t).unwrap();
    e.set("DMM", "mode", 1, "VOLT_AC").unwrap();
    assert!(log.lock().unwrap().texts().contains(&"CONF:VOLT:AC AUTO\r".to_string()));
    let u = e.poll_due().unwrap();
    assert_eq!(u[0].status["MEAS"], Value::Null);
    assert_eq!(u[0].status["RANGE"], "AUTO", "mode change resets the range shadow state");
    std::thread::sleep(Duration::from_millis(400));
    let u = e.poll_due().unwrap();
    assert_eq!(u[0].status["MEAS"]["si"]["number"], "1");
}

#[test]
fn dge_screenshot_is_one_ieee_block() {
    let bmp: Vec<u8> = b"BM".iter().copied().chain(std::iter::repeat_n(7u8, 998)).collect();
    let mut reply = format!("#4{:04}", bmp.len()).into_bytes();
    reply.extend(&bmp);
    reply.push(b'\n');
    let (t, _) = sim(move |w: &[u8]| match text(w).trim_end() {
        "*IDN?" => Some(b"OWON,DGE2070,25130086,SCPI:99.0 FV:V1.0.4.7.0\n".to_vec()),
        "HCOPy:SDUMp:DATA?" => Some(reply.clone()),
        _ => None,
    });
    // The definition keeps `screenshot` disabled (the Linux usbtmc driver cannot read the
    // DGE's oversized message); the engine's block handling is tested with it enabled.
    let mut d = (*def("owon_dge")).clone();
    d.classes.get_mut("AWG").unwrap().operations.insert(
        "screenshot".into(),
        bm_devicedef::Operation {
            get: Some(bm_devicedef::Action::Command("screenshot".into())),
            set: None,
            repr: bm_devicedef::Repr::Bytes,
            description: None,
        },
    );
    let mut e = Engine::connect(Arc::new(d), t).unwrap();
    match e.get("AWG", "screenshot", 1).unwrap() {
        Output::Bytes(b) => assert_eq!(b, bmp),
        other => panic!("expected bytes, got {other:?}"),
    }
}

#[test]
fn actor_publishes_status_serves_requests_and_disconnects_cleanly() {
    let opened = Arc::new(Mutex::new(Vec::<Arc<Mutex<WireLog>>>::new()));
    let o2 = opened.clone();
    let connector = Box::new(move || {
        let t = SimTransport::new(Box::new(oel("current")));
        o2.lock().unwrap().push(t.log.clone());
        Ok(Box::new(t) as Box<dyn Transport>)
    });
    let (etx, erx) = channel();
    let h = DeviceHandle::spawn("oel", def("owon_oel"), connector, Box::new(move |e| etx.send(e).unwrap()), ActorConfig::default());
    assert!(matches!(erx.recv_timeout(Duration::from_secs(2)).unwrap(), Event::Connected { model, .. } if model == "OEL1515"));
    match erx.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Status(u) => assert_eq!(u.status["MODE"], "CURR"),
        other => panic!("expected status, got {other:?}"),
    }
    assert_eq!(h.get("ELL", "input", 1).unwrap().value(), "OFF");
    assert!(matches!(h.set("ELL", "pow", 1, "1"), Err(Error::Refused(_))));
    h.stop();
    let texts = opened.lock().unwrap()[0].lock().unwrap().texts();
    assert_eq!(texts.last().unwrap(), "SYSTem:LOCal\r", "panel returned to the user on stop");
}

#[test]
fn actor_reconnects_after_the_link_goes_silent() {
    let opens = Arc::new(Mutex::new(0));
    let o2 = opens.clone();
    let connector = Box::new(move || {
        let n = {
            let mut n = o2.lock().unwrap();
            *n += 1;
            *n
        };
        // First connection answers identify only, then goes silent; the second is healthy.
        let state = Arc::new(Mutex::new((1.0, 1.0, false)));
        let mut healthy = tenma(state);
        let t = SimTransport::new(Box::new(move |w: &[u8]| {
            if n == 1 && text(w) != "*IDN?" {
                None
            } else {
                healthy(w)
            }
        }));
        Ok(Box::new(t) as Box<dyn Transport>)
    });
    let (etx, erx) = channel();
    let cfg = ActorConfig { reconnect_backoff: Duration::from_millis(50), max_silent_polls: 2, ..Default::default() };
    let h = DeviceHandle::spawn("tenma", def("tenma_72"), connector, Box::new(move |e| etx.send(e).unwrap()), cfg);
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut saw_disconnect = false;
    let mut reconnected = false;
    while Instant::now() < deadline && !reconnected {
        match erx.recv_timeout(Duration::from_secs(1)) {
            Ok(Event::Disconnected { .. }) => saw_disconnect = true,
            Ok(Event::Status(u)) if saw_disconnect && u.status["OUTPUT"] == "OFF" => reconnected = true,
            _ => {}
        }
    }
    assert!(saw_disconnect && reconnected, "disconnect={saw_disconnect} reconnected={reconnected}");
    assert!(*opens.lock().unwrap() >= 2);
    h.stop();
}
