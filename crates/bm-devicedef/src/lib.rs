//! BenchMesh device-definition DSL: types, loading, semantic validation and JSON Schema.
//!
//! Layout on disk:
//! ```text
//! devices/
//!   classes/<CLASS>.yaml      benchmesh.class/v1  — instrument class contracts
//!   bases/<base-id>.yaml      partial definitions a definition can `extends` (e.g. a SCPI baseline)
//!   <definition-id>.yaml      benchmesh.device/v1 — one device family each
//! ```

pub mod export;
pub mod model;
pub mod template;
pub mod validate;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub use model::*;
pub use validate::{validate_class, validate_device, Issue, Severity};

/// All definitions and class schemas found under a `devices/` directory.
#[derive(Debug, Default)]
pub struct Catalog {
    pub devices: BTreeMap<String, (PathBuf, DeviceDef)>,
    pub classes: BTreeMap<String, ClassDef>,
}

/// Parse YAML. Enum variants with data use the single-key map form (`serial: {...}`,
/// `- split: {...}`), not YAML tags; errors keep line/column information.
pub fn from_yaml<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    let de = serde_yaml_ng::Deserializer::from_str(text);
    serde_yaml_ng::with::singleton_map_recursive::deserialize(de).map_err(|e| e.to_string())
}

/// Partial definitions by base id (file stem under `bases/`), as raw YAML.
pub type Bases = BTreeMap<String, serde_yaml_ng::Value>;

/// A definition with no bases to draw from.
pub fn load_device(path: &Path) -> Result<DeviceDef, String> {
    load_device_with(path, &Bases::new())
}

/// Parse a definition, merging it over its `extends` chain first.
pub fn load_device_with(path: &Path, bases: &Bases) -> Result<DeviceDef, String> {
    let at = |e: String| format!("{}: {e}", path.display());
    let text = fs::read_to_string(path).map_err(|e| at(e.to_string()))?;
    let raw: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).map_err(|e| at(e.to_string()))?;
    if raw.get("extends").is_none() {
        // keep line/column information in errors of definitions without a base
        return from_yaml(&text).map_err(at);
    }
    let merged = resolve(raw, bases, &mut Vec::new()).map_err(at)?;
    serde_yaml_ng::with::singleton_map_recursive::deserialize(merged).map_err(|e| at(e.to_string()))
}

/// `value` merged over its base chain. Mappings merge key by key, anything else replaces, and
/// `null` removes an inherited key (e.g. a base command the device does not have).
fn resolve(mut value: serde_yaml_ng::Value, bases: &Bases, chain: &mut Vec<String>) -> Result<serde_yaml_ng::Value, String> {
    let Some(base_id) = value.get("extends").map(|b| b.as_str().map(str::to_string)) else { return Ok(value) };
    let base_id = base_id.ok_or("`extends` must be a base id")?;
    if chain.contains(&base_id) {
        return Err(format!("`extends` cycle: {} -> {base_id}", chain.join(" -> ")));
    }
    let base = bases.get(&base_id).ok_or_else(|| format!("unknown base `{base_id}` (expected devices/bases/{base_id}.yaml)"))?;
    chain.push(base_id.clone());
    let mut merged = resolve(base.clone(), bases, chain)?;
    if let Some(m) = merged.as_mapping_mut() {
        m.remove("extends");
    }
    if let Some(m) = value.as_mapping_mut() {
        m.insert("extends".into(), base_id.into());
    }
    merge_yaml(&mut merged, value);
    Ok(merged)
}

fn merge_yaml(base: &mut serde_yaml_ng::Value, over: serde_yaml_ng::Value) {
    match (base, over) {
        (serde_yaml_ng::Value::Mapping(b), serde_yaml_ng::Value::Mapping(o)) => {
            for (k, v) in o {
                if v.is_null() {
                    b.remove(&k);
                } else if let Some(slot) = b.get_mut(&k) {
                    merge_yaml(slot, v);
                } else {
                    b.insert(k, v);
                }
            }
        }
        (b, o) => *b = o,
    }
}

/// Every `bases/*.yaml` under `devices_dir` (none when the folder does not exist).
pub fn load_bases(devices_dir: &Path) -> Result<Bases, Vec<String>> {
    let dir = devices_dir.join("bases");
    if !dir.is_dir() {
        return Ok(Bases::new());
    }
    let mut bases = Bases::new();
    let mut errors = Vec::new();
    for f in yaml_files(&dir).map_err(|e| vec![e])? {
        let id = f.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        match fs::read_to_string(&f).map_err(|e| e.to_string()).and_then(|t| serde_yaml_ng::from_str(&t).map_err(|e| e.to_string())) {
            Ok(v) => {
                bases.insert(id, v);
            }
            Err(e) => errors.push(format!("{}: {e}", f.display())),
        }
    }
    if errors.is_empty() {
        Ok(bases)
    } else {
        Err(errors)
    }
}

pub fn load_class(path: &Path) -> Result<ClassDef, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    from_yaml(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn yaml_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && matches!(p.extension().and_then(|x| x.to_str()), Some("yaml" | "yml")))
        .collect();
    out.sort();
    Ok(out)
}

/// Load every class schema and device definition. Parse errors are returned, not skipped.
pub fn load_catalog(devices_dir: &Path) -> Result<Catalog, Vec<String>> {
    let mut cat = Catalog::default();
    let mut errors = Vec::new();
    match yaml_files(&devices_dir.join("classes")) {
        Ok(files) => {
            for f in files {
                match load_class(&f) {
                    Ok(c) => {
                        cat.classes.insert(c.class.clone(), c);
                    }
                    Err(e) => errors.push(e),
                }
            }
        }
        Err(e) => errors.push(e),
    }
    let bases = load_bases(devices_dir).unwrap_or_else(|e| {
        errors.extend(e);
        Bases::new()
    });
    match yaml_files(devices_dir) {
        Ok(files) => {
            for f in files {
                match load_device_with(&f, &bases) {
                    Ok(d) => {
                        if let Some((prev, _)) = cat.devices.get(&d.id) {
                            errors.push(format!("{}: duplicate id `{}` (also {})", f.display(), d.id, prev.display()));
                        }
                        cat.devices.insert(d.id.clone(), (f, d));
                    }
                    Err(e) => errors.push(e),
                }
            }
        }
        Err(e) => errors.push(e),
    }
    if errors.is_empty() {
        Ok(cat)
    } else {
        Err(errors)
    }
}

/// Validate a whole catalog; issues are prefixed with the file they belong to.
pub fn validate_catalog(cat: &Catalog) -> Vec<(String, Issue)> {
    let mut out = Vec::new();
    for c in cat.classes.values() {
        for i in validate_class(c) {
            out.push((format!("classes/{}", c.class), i));
        }
    }
    for (path, d) in cat.devices.values() {
        for i in validate_device(d, &cat.classes) {
            out.push((path.display().to_string(), i));
        }
    }
    out
}

/// JSON Schema of the device definition format.
pub fn device_json_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(DeviceDef)).expect("schema serializes")
}

/// JSON Schema of the class schema format.
pub fn class_json_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(ClassDef)).expect("schema serializes")
}
