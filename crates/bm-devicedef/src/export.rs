//! The public instrument list (`bm-devicedef export`, schema `benchmesh.instruments/v1`): every
//! enabled definition as the website shows it — vendor, family, models with their status,
//! classes with what can be read and set, links, manuals — plus the instrument classes.
//! No wire details (commands, framing): those stay in the definitions.

use serde_json::{json, Map, Value};

use crate::{Catalog, Links};

pub const SCHEMA: &str = "benchmesh.instruments/v1";

fn links(l: &Links) -> Value {
    let mut m = Map::new();
    if let Some(s) = &l.serial {
        m.insert("serial".into(), json!({"baud": s.baud, "format": s.format}));
    }
    if let Some(u) = &l.usbtmc {
        m.insert("usbtmc".into(), json!({"vid": u.vid, "pid": u.pid}));
    }
    if let Some(t) = &l.tcp {
        m.insert("tcpip".into(), json!({"port": t.port}));
    }
    if l.vxi11.is_some() {
        m.insert("vxi11".into(), json!({}));
    }
    Value::Object(m)
}

/// The catalog as the website's static database (`notes`: per-definition notes, e.g. from
/// `ai_context/website/instrument-notes.yaml`, keyed by definition id).
pub fn export(cat: &Catalog, notes: &Map<String, Value>) -> Value {
    let families: Vec<Value> = cat
        .devices
        .values()
        .filter(|(_, d)| d.enabled)
        .map(|(_, d)| {
            let classes: Map<String, Value> = d
                .classes
                .iter()
                .map(|(cls, b)| {
                    let ops: Vec<Value> = b
                        .operations
                        .iter()
                        .map(|(name, op)| json!({"name": name, "get": op.get.is_some(), "set": op.set.is_some(), "description": op.description}))
                        .collect();
                    (cls.clone(), json!({"channels": b.channels, "readings": b.status.keys().collect::<Vec<_>>(), "operations": ops}))
                })
                .collect();
            let models: Vec<Value> = d
                .models
                .iter()
                .map(|m| json!({"name": m.name, "support": m.support.as_str(), "note": m.note, "channels": m.channels}))
                .collect();
            let manuals: Vec<Value> = d.sources.iter().filter(|s| s.doc.is_some()).map(|s| json!({"doc": s.doc, "pages": s.pages})).collect();
            json!({
                "id": d.id, "vendor": d.vendor, "family": d.family, "version": d.version,
                "classes": classes, "links": links(&d.link), "models": models, "manuals": manuals,
                "notes": notes.get(&d.id).cloned().unwrap_or(json!([])),
            })
        })
        .collect();
    let classes: Map<String, Value> = cat
        .classes
        .values()
        .map(|c| (c.class.clone(), json!({"description": c.description, "operations": c.operations.keys().collect::<Vec<_>>()})))
        .collect();
    json!({"schema": SCHEMA, "classes": classes, "families": families})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_repository_definitions_export_without_wire_details() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../devices");
        let cat = crate::load_catalog(&dir).expect("definitions load");
        let mut notes = Map::new();
        notes.insert("tenma_72".into(), json!([{"kind": "prefer", "text": "x"}]));
        let v = export(&cat, &notes);
        assert_eq!(v["schema"], SCHEMA);
        let fams = v["families"].as_array().unwrap();
        assert_eq!(fams.len(), cat.devices.values().filter(|(_, d)| d.enabled).count());
        let tenma = fams.iter().find(|f| f["id"] == "tenma_72").unwrap();
        assert_eq!(tenma["links"]["serial"]["baud"], 9600);
        assert!(tenma["models"].as_array().unwrap().iter().any(|m| m["support"] == "tested"));
        assert_eq!(tenma["notes"][0]["kind"], "prefer");
        assert!(tenma["classes"]["PSU"]["operations"].as_array().unwrap().iter().any(|o| o["name"] == "voltage" && o["set"] == true));
        let text = v.to_string();
        assert!(!text.contains("\"commands\"") && !text.contains("\"framing\""), "no wire details");
        assert!(v["classes"]["PSU"]["description"].is_string());
    }
}
