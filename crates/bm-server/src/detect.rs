//! `POST /devices/detect`: which instruments are on the attached USB ports, and the identity
//! (`match`) to configure them by (`ai_context/migration/IDENTITY.md`). Sends only the
//! definitions' identify commands, and only to ports no instrument holds.

use std::sync::Arc;

use bm_devicedef::{Catalog, DeviceDef};
use bm_driver::engine::Engine;
use bm_transport::discover::Port;
use bm_transport::identity::Identity;
use bm_transport::{lock, port_kind, Endpoint};
use serde_json::{json, Value};

use crate::config::{str_field, DeviceEntry};
use crate::devices::{identity_of, parked_of, Devices};

/// A configured instrument as Detect sees it.
pub struct Known {
    pub entry: DeviceEntry,
    /// Port and identify reply of its current connection.
    pub connected: Option<(String, String)>,
    pub model: Option<String>,
}

/// An instrument that answered on a port.
struct Found {
    driver: String,
    model: String,
    idn: String,
}

/// Why a probe found nothing.
#[derive(Debug)]
pub enum Miss {
    /// The port cannot be opened at all (busy, missing driver…): no definition can help.
    Open(String),
    /// Opened, but this probe got no (usable) answer.
    Identify(String),
}

/// Send `def`'s identify probe on `port` → the reply text (whether or not a model matches it).
pub type Probe<'a> = dyn Fn(&Port, &DeviceDef) -> Result<String, Miss> + Sync + 'a;

/// Progress of one port: `(port index, state)`, state as served by `GET /devices/detect`.
pub type Report<'a> = dyn Fn(usize, Value) + Sync + 'a;

/// Definitions worth trying on a port, most likely first.
fn candidates<'a>(port: &Port, catalog: &'a Catalog) -> Vec<&'a DeviceDef> {
    let fits = |d: &&DeviceDef| d.enabled && d.link.all().iter().any(|l| port_kind(l) == Some(port.kind));
    let mut defs: Vec<&DeviceDef> = catalog.devices.values().map(|(_, d)| d).filter(fits).collect();
    // 0: vendor and product id declared and matching, 1: vendor id only, 2: anything else
    let rank = |d: &DeviceDef| match &d.link.usbtmc {
        Some(u) => {
            let eq = |want: &Option<String>, have: &Option<String>| {
                want.as_deref().map(str::to_ascii_lowercase) == have.as_deref().map(str::to_ascii_lowercase)
            };
            match (u.vid.is_some() && eq(&u.vid, &port.usb.vendor_id), &u.pid) {
                (true, Some(_)) if eq(&u.pid, &port.usb.product_id) => 0,
                (true, Some(_)) => 2,
                (true, None) => 1,
                _ => 2,
            }
        }
        _ => 2,
    };
    defs.sort_by_key(|d| rank(d));
    defs
}

/// Definitions that send the same identify probe (link settings, framing, identify command)
/// share one exchange: the reply is matched against all their models. Order follows
/// [`candidates`].
fn probe_groups<'a>(port: &Port, catalog: &'a Catalog) -> Vec<Vec<&'a DeviceDef>> {
    let mut keys: Vec<String> = Vec::new();
    let mut groups: Vec<Vec<&DeviceDef>> = Vec::new();
    for d in candidates(port, catalog) {
        let key = d
            .for_transport(Some(port.kind.as_str()))
            .map(|r| serde_json::to_string(&(&r.link, &r.framing, r.commands.get(&r.identify))).unwrap_or_default())
            .unwrap_or_else(|_| d.id.clone());
        match keys.iter().position(|k| *k == key) {
            Some(i) => groups[i].push(d),
            None => {
                keys.push(key);
                groups.push(vec![d]);
            }
        }
    }
    groups
}

/// What a probe group sends, for the progress view: `*IDN? · 9600 8N1 · 12 definitions`.
fn probe_label(port: &Port, group: &[&DeviceDef]) -> String {
    let d = group[0];
    let cmd = d.commands.get(&d.identify);
    let what = match cmd {
        Some(c) if c.modbus.is_some() => "Modbus read".to_string(),
        Some(c) => c.send.trim().to_string(),
        None => d.identify.clone(),
    };
    let link = d.for_transport(Some(port.kind.as_str())).ok().and_then(|r| r.link.serial.clone()).map(|s| format!(" · {} {}", s.baud, s.format));
    let who = match group.len() {
        1 => d.id.clone(),
        n => format!("{} and {} more", d.id, n - 1),
    };
    format!("{what}{} · {who}", link.unwrap_or_default())
}

/// The model of a definition that an identify reply selects.
fn model_for<'a>(def: &'a DeviceDef, idn: &str) -> Option<&'a bm_devicedef::Model> {
    def.models.iter().find(|m| regex::Regex::new(&m.pattern).is_ok_and(|re| re.is_match(idn)))
}

fn configured_on<'a>(port: &Port, known: &'a [Known]) -> Option<&'a Known> {
    let connected = known.iter().find(|k| k.connected.as_ref().is_some_and(|(p, _)| lock::same_port(p, &port.path)));
    // a parked instrument is recognised by its identity wherever it is plugged in
    let parked = || known.iter().find(|k| parked_of(&k.entry) && identity_of(&k.entry).is_some_and(|id| id.usb.matches(&port.usb)));
    connected.or_else(parked).or_else(|| {
        known.iter().find(|k| {
            k.connected.is_none()
                && str_field(&k.entry, "port").or_else(|| str_field(&k.entry, "device")).is_some_and(|p| lock::same_port(p, &port.path))
        })
    })
}

/// Probe one port: a configured, connected instrument is reported as such; otherwise each
/// probe group in turn until a definition knows the reply.
fn probe_port(i: usize, port: &Port, catalog: &Catalog, known: &[Known], probe: &Probe<'_>, report: &Report<'_>) -> Result<Option<Found>, String> {
    if let Some(k) = configured_on(port, known) {
        if let Some((_, idn)) = &k.connected {
            let driver = str_field(&k.entry, "driver").unwrap_or_default().to_string();
            return Ok(Some(Found { driver, model: k.model.clone().unwrap_or_default(), idn: idn.clone() }));
        }
        if parked_of(&k.entry) {
            return Err(format!("{} is parked: not probed (unpark it to use it)", str_field(&k.entry, "id").unwrap_or_default()));
        }
    }
    if lock::is_held(&port.path) {
        return Err("in use by an instrument that is connecting; try again".to_string());
    }
    let groups = probe_groups(port, catalog);
    let total = groups.len();
    let mut answered: Vec<String> = Vec::new();
    let mut last = String::new();
    for (n, group) in groups.iter().enumerate() {
        report(i, json!({"state": "probing", "tried": n, "total": total, "current": probe_label(port, group)}));
        match probe(port, group[0]) {
            Ok(idn) => {
                if let Some((d, m)) = group.iter().find_map(|d| model_for(d, &idn).map(|m| (*d, m))) {
                    return Ok(Some(Found { driver: d.id.clone(), model: m.name.clone(), idn }));
                }
                if !idn.trim().is_empty() && !answered.contains(&idn) {
                    answered.push(idn);
                }
            }
            Err(Miss::Open(e)) => return Err(format!("cannot open: {e}")),
            Err(Miss::Identify(e)) => last = format!("{}: {e}", group[0].id),
        }
    }
    let defs: usize = groups.iter().map(Vec::len).sum();
    Err(match answered.first() {
        Some(idn) => format!("answered {idn:?}, which no definition knows yet (tried {total} probes for {defs} definitions)"),
        None if total == 0 => "no definition for this kind of port".to_string(),
        None => format!("no definition answered ({total} probes for {defs} definitions; last: {last})"),
    })
}

fn port_json(port: &Port, k: Option<&Known>, found: Option<&Found>, error: Option<&str>, catalog: &Catalog, all: Option<&[Identity]>) -> Value {
    let identity = found.and_then(|f| Identity::of(port, Some(&f.idn)));
    let instrument = found.map(|f| {
        let def = catalog.devices.get(&f.driver).map(|(_, d)| d);
        let support = def.and_then(|d| d.models.iter().find(|m| m.name == f.model)).map(|m| m.support.as_str());
        json!({"driver": f.driver, "model": f.model, "idn": f.idn, "support": support,
               "vendor": def.map(|d| d.vendor.clone()), "family": def.map(|d| d.family.clone()),
               "classes": def.map(|d| d.classes.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()})
    });
    let u = &port.usb;
    json!({
        "port": port.path,
        "kind": port.kind.as_str(),
        "usb": {"vid": u.vendor_id, "pid": u.product_id, "serial": u.serial, "revision": u.revision,
                "manufacturer": u.manufacturer, "product": u.product, "location": u.location},
        "configured_as": k.and_then(|k| str_field(&k.entry, "id")),
        "locked": k.is_some_and(|k| identity_of(&k.entry).is_some()),
        "instrument": instrument,
        "match": identity,
        "certainty": match (&identity, all) {
            (Some(i), Some(all)) => Some(i.certainty(all).as_str()),
            _ => None,
        },
        "error": error,
    })
}

/// Identify the instruments on `ports` (in parallel, one thread per port), reporting each
/// port's progress and, as soon as it is known, its result.
pub fn detect(ports: &[Port], catalog: &Catalog, known: &[Known], probe: &Probe<'_>, report: &Report<'_>) -> Vec<Value> {
    let results: Vec<Result<Option<Found>, String>> = std::thread::scope(|s| {
        let handles: Vec<_> = ports
            .iter()
            .enumerate()
            .map(|(i, port)| {
                s.spawn(move || {
                    let r = probe_port(i, port, catalog, known, probe, report);
                    let (f, e) = match &r {
                        Ok(f) => (f.as_ref(), None),
                        Err(e) => (None, Some(e.as_str())),
                    };
                    let mut v = port_json(port, configured_on(port, known), f, e, catalog, None);
                    v["progress"] = json!({"state": "done"});
                    report(i, v);
                    r
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("probe failed".into()))).collect()
    });

    let idn_of = |r: &Result<Option<Found>, String>| r.as_ref().ok().and_then(|f| f.as_ref()).map(|f| f.idn.clone());
    let all: Vec<Identity> = ports.iter().zip(&results).filter_map(|(p, r)| Identity::of(p, idn_of(r).as_deref())).collect();
    ports
        .iter()
        .zip(results)
        .map(|(port, r)| {
            let (found, error) = match r {
                Ok(f) => (f, None),
                Err(e) => (None, Some(e)),
            };
            port_json(port, configured_on(port, known), found.as_ref(), error.as_deref(), catalog, Some(&all))
        })
        .collect()
}

/// A Detect run as `GET /devices/detect` serves it: ports with their progress, then results.
#[derive(Debug, Default)]
pub struct Job {
    pub running: bool,
    pub started: Option<std::time::Instant>,
    pub elapsed_ms: u64,
    pub ports: Vec<Value>,
}

impl Job {
    pub fn snapshot(&self) -> Value {
        let elapsed = match (self.running, self.started) {
            (true, Some(t)) => t.elapsed().as_millis() as u64,
            _ => self.elapsed_ms,
        };
        json!({"running": self.running, "elapsed_ms": elapsed, "ports": self.ports})
    }
}

/// Shorter waits than in a session: a probe that does not answer quickly is not this instrument.
const PROBE_TIMEOUT_MS: u32 = 700;
/// A probe that has not returned by then is abandoned (a driver or device that blocks in a way
/// no read timeout catches); the port is reported and Detect goes on.
const PROBE_LIMIT: std::time::Duration = std::time::Duration::from_secs(5);

/// `probe` on its own thread, given up after `limit`.
fn bounded(probe: impl FnOnce() -> Result<String, Miss> + Send + 'static, limit: std::time::Duration) -> Result<String, Miss> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(probe());
    });
    rx.recv_timeout(limit).unwrap_or_else(|_| {
        Err(Miss::Open(format!("the port stopped responding (a probe did not return within {} s)", limit.as_secs())))
    })
}

/// One identify probe of `def` on `port`, with Detect's short timeouts.
fn probe_once(port: &Port, def: &DeviceDef) -> Result<String, Miss> {
    // the definition as reached through this kind of port (its link and that link's framing)
    let mut def = def.for_transport(Some(port.kind.as_str())).map_err(Miss::Open)?;
    def.framing.rx.timeout_ms = def.framing.rx.timeout_ms.min(PROBE_TIMEOUT_MS);
    if let Some(c) = def.commands.get_mut(&def.identify) {
        c.timeout_ms = c.timeout_ms.map(|t| t.min(PROBE_TIMEOUT_MS));
    }
    if let Some(s) = def.link.serial.as_mut() {
        s.settle_ms = s.settle_ms.min(100);
    }
    let link = def.link.single().ok_or_else(|| Miss::Open("no link".into()))?;
    let t = bm_transport::open(&link, &Endpoint { path: port.path.clone(), ..Endpoint::default() })
        .map_err(|e| Miss::Open(e.to_string()))?;
    match Engine::open(Arc::new(def), t) {
        Ok(e) => Ok(e.idn.clone()),
        // the instrument answered, just not as one of this definition's models
        Err(bm_driver::Error::UnknownModel(idn)) => Ok(idn),
        Err(e) => Err(Miss::Identify(e.to_string())),
    }
}

/// Real ports and real probing; progress goes to `devices.detect`.
pub fn run(devices: &Devices) -> Vec<Value> {
    let known: Vec<Known> = {
        let r = devices.registry.lock().unwrap();
        devices
            .entries()
            .into_iter()
            .map(|entry| {
                let st = str_field(&entry, "id").and_then(|id| r.devices.get(id));
                let connected = st.filter(|d| d.connected).and_then(|d| Some((d.port.clone()?, d.idn.clone()?)));
                Known { model: st.and_then(|d| d.model.clone()), connected, entry }
            })
            .collect()
    };
    let ports = bm_transport::discover::ports();
    {
        let mut job = devices.detect.lock().unwrap();
        job.ports = ports
            .iter()
            .map(|p| {
                let mut v = port_json(p, configured_on(p, &known), None, None, &devices.catalog, None);
                v["progress"] = json!({"state": "queued"});
                v
            })
            .collect();
    }
    let probe = |port: &Port, def: &DeviceDef| {
        let (port, def) = (port.clone(), def.clone());
        bounded(move || probe_once(&port, &def), PROBE_LIMIT)
    };
    let report = |i: usize, v: Value| {
        let mut job = devices.detect.lock().unwrap();
        if let Some(slot) = job.ports.get_mut(i) {
            if v.get("port").is_some() {
                *slot = v;
            } else {
                slot["progress"] = v;
            }
        }
    };
    detect(&ports, &devices.catalog, &known, &probe, &report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm_transport::discover::{PortKind, UsbInfo};
    use serde_json::Map;

    fn catalog() -> Catalog {
        bm_devicedef::load_catalog(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../devices"))).unwrap()
    }

    fn port(kind: PortKind, path: &str, vid: &str, pid: &str, serial: Option<&str>, rev: &str) -> Port {
        let usb = UsbInfo {
            vendor_id: Some(vid.into()),
            product_id: Some(pid.into()),
            serial: serial.map(Into::into),
            revision: Some(rev.into()),
            location: Some(format!("loc-{path}")),
            ..UsbInfo::default()
        };
        Port { kind, path: path.into(), usb }
    }

    fn entry(v: Value) -> DeviceEntry {
        v.as_object().cloned().unwrap_or_else(Map::new)
    }

    #[test]
    fn identifies_unconfigured_ports_and_reports_configured_ones_without_probing() {
        let ports = vec![
            port(PortKind::Serial, "test-detect/ttyUSB2", "1a86", "7523", None, "0264"),
            port(PortKind::Serial, "test-detect/ttyUSB0", "1a86", "7523", None, "8134"),
            port(PortKind::Usbtmc, "test-detect/usbtmc3", "1ab1", "044d", Some("DHO8A272907001"), "0404"),
            port(PortKind::Serial, "test-detect/ttyUSB9", "1a86", "7523", None, "0254"),
            port(PortKind::Usbtmc, "test-detect/usbtmc4", "1ab1", "044d", Some("DHO8A000000002"), "0404"),
        ];
        let known = vec![Known {
            entry: entry(json!({"id": "owon-xdm1241", "driver": "owon_xdm", "port": "test-detect/ttyUSB2"})),
            connected: Some(("test-detect/ttyUSB2".into(), "OWON,XDM1241,24512010,V4.3.0,3".into())),
            model: Some("XDM1241".into()),
        }];
        let tried = std::sync::Mutex::new(Vec::new());
        let probe = |p: &Port, d: &DeviceDef| {
            tried.lock().unwrap().push((p.path.clone(), d.id.clone()));
            // an instrument answers every probe it understands; here: every probe
            match (p.path.as_str(), d.id.as_str()) {
                ("test-detect/ttyUSB0", _) => Ok("OWON,SPM3103,24460763,FV:V2.1.0".to_string()),
                ("test-detect/usbtmc3", _) => Ok("RIGOL TECHNOLOGIES,DHO804,DHO8A272907001,00.01.03".to_string()),
                ("test-detect/usbtmc4", _) => Err(Miss::Open("Windows has no driver for this instrument. Either install a VISA …".into())),
                _ => Err(Miss::Identify("timeout".to_string())),
            }
        };
        let out = detect(&ports, &catalog(), &known, &probe, &|_, _| {});
        let tried = tried.into_inner().unwrap();

        assert_eq!(out[0]["configured_as"], "owon-xdm1241");
        assert_eq!(out[0]["locked"], false);
        assert_eq!(out[0]["match"]["usb"]["revision"], "0264");
        assert_eq!(out[0]["match"]["idn_serial"], "24512010");
        assert_eq!(out[0]["certainty"], "instrument");
        assert!(!tried.iter().any(|(p, _)| p == "test-detect/ttyUSB2"), "connected instrument is not probed");

        assert_eq!(out[1]["instrument"]["driver"], "owon_spm");
        assert_eq!(out[1]["instrument"]["model"], "SPM3103");
        assert_eq!(out[1]["configured_as"], Value::Null);

        assert_eq!(out[2]["instrument"]["vendor"], "RIGOL");
        assert_eq!(out[2]["match"]["usb"]["serial"], "DHO8A272907001");
        assert_eq!(out[2]["match"]["usb"].get("revision"), None, "a USB serial number makes the revision unnecessary");
        let scope_tries: Vec<&String> = tried.iter().filter(|(p, _)| p == "test-detect/usbtmc3").map(|(_, d)| d).collect();
        assert_eq!(scope_tries[0], "rigol_dho800", "the definition declaring the vendor and product id is tried first");

        assert_eq!(out[3]["instrument"], Value::Null);
        assert!(out[3]["error"].as_str().unwrap().starts_with("no definition answered"), "{}", out[3]["error"]);
        assert_eq!(out[4]["error"], "cannot open: Windows has no driver for this instrument. Either install a VISA …");
        assert_eq!(tried.iter().filter(|(p, _)| p == "test-detect/usbtmc4").count(), 1, "an unopenable port is not retried per definition");
        assert!(tried.iter().filter(|(p, _)| p == "test-detect/ttyUSB9").all(|(_, d)| d != "owon_dge" && d != "rigol_dho800"), "serial ports only get serial definitions");
    }

    #[test]
    fn definitions_sharing_a_probe_cost_one_exchange_and_progress_is_reported() {
        let cat = catalog();
        let p = port(PortKind::Serial, "test-detect/ttyUSB7", "1a86", "7523", None, "0264");
        let serial_defs = candidates(&p, &cat).len();
        let groups = probe_groups(&p, &cat);
        assert!(groups.len() < serial_defs, "{} probes for {serial_defs} definitions", groups.len());
        assert_eq!(groups.iter().map(Vec::len).sum::<usize>(), serial_defs);

        let tries = std::sync::atomic::AtomicUsize::new(0);
        let probe = |_: &Port, _: &DeviceDef| -> Result<String, Miss> {
            tries.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok("ACME,WIDGET9000,1,1.0".to_string()) // answers, but no definition knows it
        };
        let reports = std::sync::Mutex::new(Vec::new());
        let out = detect(std::slice::from_ref(&p), &cat, &[], &probe, &|i, v| reports.lock().unwrap().push((i, v)));
        assert_eq!(tries.into_inner(), groups.len());
        let e = out[0]["error"].as_str().unwrap();
        assert!(e.starts_with("answered \"ACME,WIDGET9000,1,1.0\", which no definition knows yet"), "{e}");
        let reports = reports.into_inner().unwrap();
        let probing: Vec<&Value> = reports.iter().filter(|(_, v)| v["state"] == "probing").map(|(_, v)| v).collect();
        assert_eq!(probing.len(), groups.len());
        assert_eq!(probing[0]["tried"], 0);
        assert_eq!(probing[0]["total"], groups.len());
        assert!(probing.iter().all(|v| v["current"].as_str().is_some_and(|c| !c.is_empty())));
        let (_, last) = reports.last().unwrap();
        assert_eq!(last["progress"]["state"], "done");
        assert_eq!(last["port"], "test-detect/ttyUSB7");
    }

    #[test]
    fn a_parked_instrument_is_not_probed_wherever_it_is_plugged_in() {
        let ports = vec![port(PortKind::Serial, "test-detect/ttyUSB5", "1a86", "7523", None, "0264")];
        let known = vec![Known {
            entry: entry(json!({"id": "xdm", "driver": "owon_xdm", "port": "test-detect/ttyUSB0", "parked": true,
                                "match": {"usb": {"vid": "1a86", "pid": "7523", "revision": "0264"}}})),
            connected: None,
            model: None,
        }];
        let probe = |_: &Port, _: &DeviceDef| -> Result<String, Miss> { panic!("probed a parked instrument") };
        let out = detect(&ports, &catalog(), &known, &probe, &|_, _| {});
        assert_eq!(out[0]["configured_as"], "xdm");
        assert_eq!(out[0]["error"], "xdm is parked: not probed (unpark it to use it)");
    }

    #[test]
    fn a_held_port_is_not_probed() {
        let ports = vec![port(PortKind::Serial, "test-detect/held", "1a86", "7523", None, "0264")];
        let _g = lock::acquire("test-detect/held").unwrap();
        let probe = |_: &Port, _: &DeviceDef| -> Result<String, Miss> { panic!("probed a held port") };
        let out = detect(&ports, &catalog(), &[], &probe, &|_, _| {});
        assert!(out[0]["error"].as_str().unwrap().contains("in use"));
    }
}
