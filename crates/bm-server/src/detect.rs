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
use crate::devices::{identity_of, Devices};

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
    /// Opened, but this definition's identify got no matching answer.
    Identify(String),
}

/// Identify reply and model name of `def`'s instrument on `port`.
pub type Probe<'a> = dyn Fn(&Port, &DeviceDef) -> Result<(String, String), Miss> + Sync + 'a;

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

fn configured_on<'a>(port: &Port, known: &'a [Known]) -> Option<&'a Known> {
    let connected = known.iter().find(|k| k.connected.as_ref().is_some_and(|(p, _)| lock::same_port(p, &port.path)));
    connected.or_else(|| {
        known.iter().find(|k| {
            k.connected.is_none()
                && str_field(&k.entry, "port").or_else(|| str_field(&k.entry, "device")).is_some_and(|p| lock::same_port(p, &port.path))
        })
    })
}

pub fn detect(ports: &[Port], catalog: &Catalog, known: &[Known], probe: &Probe<'_>) -> Vec<Value> {
    let results: Vec<Result<Option<Found>, String>> = std::thread::scope(|s| {
        let handles: Vec<_> = ports
            .iter()
            .map(|port| {
                s.spawn(move || {
                    if let Some(k) = configured_on(port, known) {
                        if let Some((_, idn)) = &k.connected {
                            let driver = str_field(&k.entry, "driver").unwrap_or_default().to_string();
                            return Ok(Some(Found { driver, model: k.model.clone().unwrap_or_default(), idn: idn.clone() }));
                        }
                    }
                    if lock::is_held(&port.path) {
                        return Err("in use by an instrument that is connecting; try again".to_string());
                    }
                    let defs = candidates(port, catalog);
                    if defs.is_empty() {
                        return Ok(None);
                    }
                    let mut last = String::new();
                    for def in defs {
                        match probe(port, def) {
                            Ok((idn, model)) => return Ok(Some(Found { driver: def.id.clone(), model, idn })),
                            Err(Miss::Open(e)) => return Err(format!("cannot open: {e}")),
                            Err(Miss::Identify(e)) => last = format!("{}: {e}", def.id),
                        }
                    }
                    Err(format!("no definition answered (last: {last})"))
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
            let k = configured_on(port, known);
            let (found, error) = match r {
                Ok(f) => (f, None),
                Err(e) => (None, Some(e)),
            };
            let identity = found.as_ref().and_then(|f| Identity::of(port, Some(&f.idn)));
            let instrument = found.as_ref().map(|f| {
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
                "certainty": identity.as_ref().map(|i| i.certainty(&all).as_str()),
                "error": error,
            })
        })
        .collect()
}

/// Real ports and real probing.
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
    let probe = |port: &Port, def: &DeviceDef| {
        // the definition as reached through this kind of port (its link and that link's framing)
        let def = def.for_transport(Some(port.kind.as_str())).map_err(Miss::Open)?;
        let link = def.link.single().ok_or_else(|| Miss::Open("no link".into()))?;
        let t = bm_transport::open(&link, &Endpoint { path: port.path.clone(), ..Endpoint::default() })
            .map_err(|e| Miss::Open(e.to_string()))?;
        let e = Engine::open(Arc::new(def), t).map_err(|e| Miss::Identify(e.to_string()))?;
        Ok((e.idn.clone(), e.model.name.clone()))
    };
    detect(&bm_transport::discover::ports(), &devices.catalog, &known, &probe)
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
            match (p.path.as_str(), d.id.as_str()) {
                ("test-detect/ttyUSB0", "owon_spm") => Ok(("OWON,SPM3103,24460763,FV:V2.1.0".to_string(), "SPM3103".to_string())),
                ("test-detect/usbtmc3", "rigol_dho800") => {
                    Ok(("RIGOL TECHNOLOGIES,DHO804,DHO8A272907001,00.01.03".to_string(), "DHO804".to_string()))
                }
                ("test-detect/usbtmc4", _) => Err(Miss::Open("Windows has no driver for this instrument. Either install a VISA …".into())),
                _ => Err(Miss::Identify("timeout".to_string())),
            }
        };
        let out = detect(&ports, &catalog(), &known, &probe);
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
        assert!(out[3]["error"].as_str().unwrap().starts_with("no definition answered"));
        assert_eq!(out[4]["error"], "cannot open: Windows has no driver for this instrument. Either install a VISA …");
        assert_eq!(tried.iter().filter(|(p, _)| p == "test-detect/usbtmc4").count(), 1, "an unopenable port is not retried per definition");
        assert!(tried.iter().filter(|(p, _)| p == "test-detect/ttyUSB9").all(|(_, d)| d != "owon_dge" && d != "rigol_dho800"), "serial ports only get serial definitions");
    }

    #[test]
    fn a_held_port_is_not_probed() {
        let ports = vec![port(PortKind::Serial, "test-detect/held", "1a86", "7523", None, "0264")];
        let _g = lock::acquire("test-detect/held").unwrap();
        let probe = |_: &Port, _: &DeviceDef| -> Result<(String, String), Miss> { panic!("probed a held port") };
        let out = detect(&ports, &catalog(), &[], &probe);
        assert!(out[0]["error"].as_str().unwrap().contains("in use"));
    }
}
