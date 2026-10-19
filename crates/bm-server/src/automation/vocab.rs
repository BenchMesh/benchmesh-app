//! What a flow can address on this bench: the configured devices, their classes and channels,
//! the status fields each class reports and the operations it supports (with the setter's
//! value spec and energise flag from the definition). Built from the running devices; the
//! validator, the assistant's prompt, MCP and `GET /automation/vocabulary` all use it.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{json, Value};

use crate::config::str_field;
use crate::devices::Devices;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Field {
    /// `si` (number with unit), `number`, `integer`, `bool`, `enum`, `string`, …
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Op {
    pub get: bool,
    pub set: bool,
    /// Setter value spec (`opspec::value_spec`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub energizes: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Op {
    /// The value that leaves the output off (`OFF` / `false`), for energising setters.
    pub fn safe_value(&self) -> Option<Value> {
        let v = self.value.as_ref()?;
        match v["type"].as_str()? {
            "bool" => Some(json!(false)),
            "enum" => v["values"].as_array()?.iter().find(|x| x.as_str().is_some_and(|s| s.eq_ignore_ascii_case("OFF"))).cloned(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Class {
    pub channels: u8,
    pub fields: BTreeMap<String, Field>,
    pub operations: BTreeMap<String, Op>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub vendor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub online: bool,
    pub classes: BTreeMap<String, Class>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Vocab {
    pub devices: Vec<Device>,
}

impl Vocab {
    pub fn device(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// The class of a device, or an error message naming what exists.
    pub fn class(&self, device: &str, class: &str) -> Result<&Class, String> {
        let d = self.device(device).ok_or_else(|| {
            let ids: Vec<&str> = self.devices.iter().map(|d| d.id.as_str()).collect();
            format!("unknown device {device:?} (configured: {})", if ids.is_empty() { "none".into() } else { ids.join(", ") })
        })?;
        d.classes.get(class).ok_or_else(|| format!("device {device} has no class {class:?} (it has {})", d.classes.keys().cloned().collect::<Vec<_>>().join(", ")))
    }

    pub fn from_devices(devices: &Devices) -> Vocab {
        let reg = devices.registry.lock().unwrap().devices.clone();
        let mut out = Vec::new();
        for e in devices.entries() {
            let Some(id) = str_field(&e, "id").map(str::to_string) else { continue };
            let Some((def, _, entry)) = devices.lookup(&id) else { continue };
            let model = devices.model_of(&id, &def, &entry);
            let state = reg.get(&id);
            let mut classes = BTreeMap::new();
            for (cname, b) in &def.classes {
                let contract = devices.catalog.classes.get(cname);
                let fields = b
                    .status
                    .keys()
                    .map(|k| {
                        let spec = contract.and_then(|c| c.status.get(k));
                        let kind = spec.map(|s| format!("{:?}", s.kind).to_lowercase()).unwrap_or_else(|| "string".into());
                        let values = spec.filter(|s| !s.values.is_empty()).map(|s| s.values.clone());
                        (k.clone(), Field { kind, values, description: spec.and_then(|s| s.description.clone()) })
                    })
                    .collect();
                let operations = b
                    .operations
                    .iter()
                    .map(|(name, o)| {
                        let setter = crate::opspec::setter(&def, o);
                        let description = o.description.clone().or_else(|| contract.and_then(|c| c.operations.get(name)).and_then(|c| c.description.clone()));
                        let set = setter.is_some();
                        let (value, energizes) = setter.unwrap_or((None, false));
                        (name.clone(), Op { get: o.get.is_some(), set, value, energizes, description })
                    })
                    .collect();
                let channels = model.as_ref().and_then(|m| m.channels.get(cname).copied()).unwrap_or(b.channels);
                classes.insert(cname.clone(), Class { channels, fields, operations });
            }
            out.push(Device {
                name: str_field(&entry, "name").unwrap_or(&id).to_string(),
                vendor: def.vendor.clone(),
                model: state.and_then(|s| s.model.clone()).or_else(|| model.map(|m| m.name)),
                online: state.is_some_and(|s| s.connected),
                classes,
                id,
            });
        }
        Vocab { devices: out }
    }
}
