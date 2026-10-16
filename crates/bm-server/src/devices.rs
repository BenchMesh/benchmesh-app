//! Configured devices: one device thread each, plus the registry the UI polls.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bm_devicedef::{Catalog, DeviceDef, Model};
use bm_driver::actor::{ActorConfig, Connector, DeviceClient, DeviceHandle, Event};
use bm_transport::Endpoint;
use serde_json::{json, Map, Value};

use crate::config::{str_field, DeviceEntry};

/// Opens a transport for a configured device. Production: real serial/USBTMC;
/// tests: simulators.
pub type ConnectorFactory = Arc<dyn Fn(&DeviceEntry, &DeviceDef) -> Connector + Send + Sync>;

pub fn hardware_connectors() -> ConnectorFactory {
    Arc::new(|entry: &DeviceEntry, def: &DeviceDef| {
        let link = def.link.clone();
        let ep = Endpoint {
            path: str_field(entry, "port").or_else(|| str_field(entry, "device")).unwrap_or_default().to_string(),
            baud: entry.get("baud").and_then(|b| b.as_u64()).map(|b| b as u32),
            format: str_field(entry, "serial").map(str::to_string),
        };
        Box::new(move || bm_transport::open(&link, &ep))
    })
}

/// Simulated instruments replaying a capture session (`captures/<session>/<device-id>.jsonl.gz`):
/// UI development and demos without hardware. Setters are accepted but change nothing.
pub fn simulated_connectors(session: std::path::PathBuf) -> ConnectorFactory {
    Arc::new(move |entry: &DeviceEntry, def: &DeviceDef| {
        let id = str_field(entry, "id").unwrap_or_default().to_string();
        let path = session.join(format!("{id}.jsonl.gz"));
        let def = def.clone();
        Box::new(move || {
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
    pub connected: bool,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    /// class -> channel -> status
    pub status: BTreeMap<String, BTreeMap<u8, BTreeMap<String, Value>>>,
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

    fn apply(&mut self, id: &str, ev: Event) {
        let d = self.devices.entry(id.to_string()).or_default();
        match ev {
            Event::Connected { idn, model } => {
                d.idn = Some(idn);
                d.model = Some(model);
                d.connected = true;
                d.consecutive_failures = 0;
                d.last_error = None;
            }
            Event::Status(u) => {
                d.status.entry(u.class).or_default().insert(u.ch, u.status);
            }
            Event::Metrics(m) => {
                d.metrics = Some(m);
                return; // not a registry change (no version bump)
            }
            Event::Disconnected { reason } => {
                d.connected = false;
                d.idn = None;
                d.status.clear();
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
}

impl Devices {
    pub fn new(catalog: Catalog, connectors: ConnectorFactory, actor: ActorConfig) -> Self {
        Devices { catalog, connectors, registry: Default::default(), managed: Mutex::new(Vec::new()), actor }
    }

    pub fn driver_ids(&self) -> Vec<String> {
        self.catalog.devices.keys().cloned().collect()
    }

    /// Start a device thread for a (validated) config entry.
    pub fn start(&self, entry: DeviceEntry) -> Result<(), String> {
        let id = str_field(&entry, "id").unwrap_or_default().to_string();
        let driver = str_field(&entry, "driver").unwrap_or_default();
        let def = Arc::new(self.catalog.devices.get(driver).ok_or(format!("unknown driver {driver:?}"))?.1.clone());
        let connector = (self.connectors)(&entry, &def);
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
