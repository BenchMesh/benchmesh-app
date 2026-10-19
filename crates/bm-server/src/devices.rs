//! Configured devices: one device thread each, plus the registry the UI polls.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bm_devicedef::{Catalog, DeviceDef, Model};
use bm_driver::actor::{fixed_connector, ActorConfig, Attempt, Connector, DeviceClient, DeviceHandle, Event};
use bm_driver::wirelog::WireLog;
use bm_transport::identity::Identity;
use bm_transport::Endpoint;
use serde_json::{json, Map, Value};

use crate::config::{str_field, DeviceEntry};

/// Opens a transport for a configured device. Production: real serial/USBTMC;
/// tests: simulators.
pub type ConnectorFactory = Arc<dyn Fn(&DeviceEntry, &DeviceDef) -> Connector + Send + Sync>;

/// Real ports: the entry's fixed `port`/`device`, or — with `match` — wherever the
/// instrument is plugged in (re-resolved on every reconnect).
pub fn hardware_connectors() -> ConnectorFactory {
    Arc::new(|entry: &DeviceEntry, def: &DeviceDef| {
        // `def` is resolved for the entry's transport (`Devices::start`): it has one link
        let link = def.link.single();
        let ep = Endpoint {
            path: str_field(entry, "port").or_else(|| str_field(entry, "device")).or_else(|| str_field(entry, "host")).unwrap_or_default().to_string(),
            baud: entry.get("baud").and_then(|b| b.as_u64()).map(|b| b as u32),
            format: str_field(entry, "serial").map(str::to_string),
        };
        let id = identity_of(entry);
        Box::new(move || match &link {
            Some(link) => bm_transport::attempts(link, &ep, id.as_ref()),
            None => Err("the definition offers no single link for this entry".to_string()),
        })
    })
}

/// `connector` with every transport it opens recorded in `log` (failed opens too).
fn recorded(mut connector: Connector, log: Arc<WireLog>) -> Connector {
    Box::new(move || {
        let attempts = connector().inspect_err(|e| log.info(&format!("no port to try: {e}")))?;
        Ok(attempts
            .into_iter()
            .map(|a| {
                let (log, port, open) = (log.clone(), a.port.clone(), a.open);
                Attempt {
                    port: a.port,
                    accept: a.accept,
                    open: Box::new(move || match open() {
                        Ok(t) => Ok(bm_driver::wirelog::logged(t, log, &port)),
                        Err(e) => {
                            log.info(&format!("open {port} failed: {e}"));
                            Err(e)
                        }
                    }),
                }
            })
            .collect())
    })
}

/// The entry's `match` (validated by `config::validate_entry`).
pub fn identity_of(entry: &DeviceEntry) -> Option<Identity> {
    entry.get("match").and_then(|m| serde_json::from_value(m.clone()).ok())
}

/// Simulated instruments replaying a capture session (`captures/<session>/<device-id>.jsonl.gz`):
/// UI development and demos without hardware. Setters are accepted but change nothing.
pub fn simulated_connectors(session: std::path::PathBuf) -> ConnectorFactory {
    Arc::new(move |entry: &DeviceEntry, def: &DeviceDef| {
        let id = str_field(entry, "id").unwrap_or_default().to_string();
        let path = session.join(format!("{id}.jsonl.gz"));
        let def = def.clone();
        fixed_connector(&format!("sim:{id}"), move || {
            let responder = bm_driver::replay::responder_from_capture(&def, &path)
                .map_err(|e| std::io::Error::new(e.kind(), format!("simulation for {id}: {}: {e}", path.display())))?;
            let mut t = bm_driver::sim::SimTransport::new(responder);
            t.latency = std::time::Duration::from_millis(5);
            Ok(Box::new(t) as Box<dyn bm_driver::engine::Transport>)
        })
    })
}

#[derive(Debug, Default, Clone)]
pub struct DeviceState {
    pub idn: Option<String>,
    pub model: Option<String>,
    /// Port of the current connection.
    pub port: Option<String>,
    pub connected: bool,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    /// class -> channel -> status
    pub status: BTreeMap<String, BTreeMap<u8, BTreeMap<String, Value>>>,
    /// class -> channel -> when that status was last updated (recordings skip stale values).
    pub status_at: BTreeMap<String, BTreeMap<u8, Instant>>,
    /// Latest metrics snapshot from the device thread.
    pub metrics: Option<Value>,
}

#[derive(Default)]
pub struct Registry {
    pub devices: BTreeMap<String, DeviceState>,
    /// Bumped on every change (WebSocket senders skip unchanged snapshots).
    pub version: u64,
}

impl Registry {
    /// The registry as the UI consumes it: `{dev: {IDN, CLASS: {status_chN: {...}}}}`.
    pub fn to_json(&self) -> Value {
        let mut out = Map::new();
        for (id, d) in &self.devices {
            let mut e = Map::new();
            if d.connected {
                if let Some(idn) = &d.idn {
                    e.insert("IDN".into(), json!(idn));
                }
                for (class, chans) in &d.status {
                    let m: Map<String, Value> =
                        chans.iter().map(|(ch, s)| (format!("status_ch{ch}"), json!(s))).collect();
                    e.insert(class.clone(), Value::Object(m));
                }
            }
            out.insert(id.clone(), Value::Object(e));
        }
        Value::Object(out)
    }

    /// A status value if the device is connected and that class/channel was updated within
    /// `max_age` (else `None`: never a stale value).
    pub fn fresh_value(&self, id: &str, class: &str, ch: u8, field: &str, max_age: Duration) -> Option<&Value> {
        let d = self.devices.get(id).filter(|d| d.connected)?;
        let at = d.status_at.get(class)?.get(&ch)?;
        if at.elapsed() > max_age {
            return None;
        }
        d.status.get(class)?.get(&ch)?.get(field)
    }

    fn apply(&mut self, id: &str, ev: Event) {
        let d = self.devices.entry(id.to_string()).or_default();
        match ev {
            Event::Connected { idn, model, port } => {
                d.idn = Some(idn);
                d.model = Some(model);
                d.port = Some(port);
                d.connected = true;
                d.consecutive_failures = 0;
                d.last_error = None;
            }
            Event::Status(u) => {
                d.status_at.entry(u.class.clone()).or_default().insert(u.ch, Instant::now());
                d.status.entry(u.class).or_default().insert(u.ch, u.status);
            }
            Event::Metrics(m) => {
                d.metrics = Some(m);
                return; // not a registry change (no version bump)
            }
            Event::Disconnected { reason } => {
                d.connected = false;
                d.idn = None;
                d.port = None;
                d.status.clear();
                d.status_at.clear();
                if reason != "stopped" {
                    d.consecutive_failures += 1;
                    d.last_error = Some(reason);
                }
            }
        }
        self.version += 1;
    }
}

pub struct Managed {
    pub entry: DeviceEntry,
    pub def: Arc<DeviceDef>,
    handle: DeviceHandle,
}

impl Managed {
    pub fn client(&self) -> DeviceClient {
        self.handle.client()
    }
}

pub struct Devices {
    pub catalog: Catalog,
    pub connectors: ConnectorFactory,
    pub registry: Arc<Mutex<Registry>>,
    pub managed: Mutex<Vec<(String, Managed)>>,
    pub actor: ActorConfig,
    /// Real ports exist (Detect probes them); false for simulations and tests.
    pub hardware: bool,
    /// Wire logs by device id (kept across reconnects and restarts of the same id).
    pub wire: Mutex<BTreeMap<String, Arc<WireLog>>>,
    /// The current / last Detect run (`/devices/detect`).
    pub detect: Mutex<crate::detect::Job>,
}

impl Devices {
    pub fn new(catalog: Catalog, connectors: ConnectorFactory, actor: ActorConfig) -> Self {
        Devices { catalog, connectors, registry: Default::default(), managed: Mutex::new(Vec::new()), actor, hardware: false, wire: Default::default(), detect: Default::default() }
    }

    pub fn driver_ids(&self) -> Vec<String> {
        self.catalog.devices.keys().cloned().collect()
    }

    /// Start a device thread for a (validated) config entry.
    pub fn start(&self, entry: DeviceEntry) -> Result<(), String> {
        let id = str_field(&entry, "id").unwrap_or_default().to_string();
        let driver = str_field(&entry, "driver").unwrap_or_default();
        let def = self.catalog.devices.get(driver).ok_or(format!("unknown driver {driver:?}"))?.1.for_transport(str_field(&entry, "transport"))?;
        let def = Arc::new(def);
        let connector = recorded((self.connectors)(&entry, &def), self.wire_log(&id));
        let registry = self.registry.clone();
        let rid = id.clone();
        {
            let mut r = registry.lock().unwrap();
            r.devices.insert(id.clone(), DeviceState::default());
            r.version += 1;
        }
        let handle = DeviceHandle::spawn(
            &id,
            def.clone(),
            connector,
            Box::new(move |ev| registry.lock().unwrap().apply(&rid, ev)),
            self.actor.clone(),
        );
        self.managed.lock().unwrap().push((id, Managed { entry, def, handle }));
        Ok(())
    }

    /// Stop and forget a device (runs its on_disconnect steps).
    pub fn stop(&self, id: &str) -> bool {
        let m = {
            let mut list = self.managed.lock().unwrap();
            list.iter().position(|(i, _)| i == id).map(|p| list.remove(p).1)
        };
        let found = m.is_some();
        if let Some(m) = m {
            m.handle.stop();
        }
        let mut r = self.registry.lock().unwrap();
        r.devices.remove(id);
        r.version += 1;
        found
    }

    /// The wire log of a device id (created on first use).
    pub fn wire_log(&self, id: &str) -> Arc<WireLog> {
        self.wire.lock().unwrap().entry(id.to_string()).or_default().clone()
    }

    pub fn stop_all(&self) {
        let ids: Vec<String> = self.managed.lock().unwrap().iter().map(|(i, _)| i.clone()).collect();
        for id in ids {
            self.stop(&id);
        }
    }

    pub fn entries(&self) -> Vec<DeviceEntry> {
        self.managed.lock().unwrap().iter().map(|(_, m)| m.entry.clone()).collect()
    }

    /// (definition, client, configured/identified model) of a device.
    pub fn lookup(&self, id: &str) -> Option<(Arc<DeviceDef>, DeviceClient, DeviceEntry)> {
        self.managed.lock().unwrap().iter().find(|(i, _)| i == id).map(|(_, m)| (m.def.clone(), m.client(), m.entry.clone()))
    }

    /// The model whose overrides apply: identified model, else the configured one.
    pub fn model_of(&self, id: &str, def: &DeviceDef, entry: &DeviceEntry) -> Option<Model> {
        let name = self
            .registry
            .lock()
            .unwrap()
            .devices
            .get(id)
            .and_then(|d| d.model.clone())
            .or_else(|| str_field(entry, "model").map(str::to_string))?;
        def.models.iter().find(|m| m.name == name).cloned()
    }
}

/// Deep-merge `over` into `base` (objects merge, everything else replaces).
pub fn merge(base: &mut Value, over: &Value) {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                merge(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (b, o) => *b = o.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm_driver::engine::StatusUpdate;

    fn status(v: f64) -> Event {
        Event::Status(StatusUpdate { class: "PSU".into(), ch: 1, status: [("VOUT".to_string(), json!(v))].into_iter().collect() })
    }

    #[test]
    fn fresh_value_requires_connection_and_a_recent_update() {
        let mut r = Registry::default();
        let age = Duration::from_millis(50);
        r.apply("d", Event::Connected { idn: "X".into(), model: "M".into(), port: "p".into() });
        assert_eq!(r.fresh_value("d", "PSU", 1, "VOUT", age), None, "no status yet");
        r.apply("d", status(1.5));
        assert_eq!(r.fresh_value("d", "PSU", 1, "VOUT", age), Some(&json!(1.5)));
        assert_eq!(r.fresh_value("d", "PSU", 2, "VOUT", age), None);
        assert_eq!(r.fresh_value("d", "PSU", 1, "IOUT", age), None);
        assert_eq!(r.fresh_value("x", "PSU", 1, "VOUT", age), None);
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(r.fresh_value("d", "PSU", 1, "VOUT", age), None, "stale");
        r.apply("d", status(2.0));
        assert_eq!(r.fresh_value("d", "PSU", 1, "VOUT", age), Some(&json!(2.0)));
        r.apply("d", Event::Disconnected { reason: "io".into() });
        assert_eq!(r.fresh_value("d", "PSU", 1, "VOUT", age), None, "disconnected");
    }
}
