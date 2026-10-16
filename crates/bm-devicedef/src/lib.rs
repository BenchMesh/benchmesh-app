//! BenchMesh device-definition DSL: types, loading, semantic validation and JSON Schema.
//!
//! Layout on disk:
//! ```text
//! devices/
//!   classes/<CLASS>.yaml      benchmesh.class/v1  — instrument class contracts
//!   <definition-id>.yaml      benchmesh.device/v1 — one device family each
//! ```

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

pub fn load_device(path: &Path) -> Result<DeviceDef, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    from_yaml(&text).map_err(|e| format!("{}: {e}", path.display()))
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
    match yaml_files(devices_dir) {
        Ok(files) => {
            for f in files {
                match load_device(&f) {
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
