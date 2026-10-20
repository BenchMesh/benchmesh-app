//! Saved flows (`<data dir>/flows/<id>.json`, the flow document as written) and the
//! automation settings (`<data dir>/automation.json`: assistant provider, model, API keys,
//! MCP access; 0600 on Unix). Without a data dir (simulations, tests) both live in memory.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use bm_store::now_ms;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub struct Flows {
    dir: Option<PathBuf>,
    mem: Mutex<BTreeMap<String, (Value, i64)>>,
}

/// Flow ids are file names: lowercase letters, digits and `-`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
        if s.len() >= 48 {
            break;
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() { "flow".into() } else { s }
}

impl Flows {
    pub fn new(dir: Option<PathBuf>) -> Flows {
        Flows { dir, mem: Mutex::new(BTreeMap::new()) }
    }

    fn path(&self, id: &str) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(format!("{id}.json")))
    }

    /// `[{id, name, description, updated_at}]`, newest first.
    pub fn list(&self) -> Vec<Value> {
        let mut out: Vec<(String, Value, i64)> = match &self.dir {
            None => self.mem.lock().unwrap().iter().map(|(k, (v, t))| (k.clone(), v.clone(), *t)).collect(),
            Some(d) => std::fs::read_dir(d)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| {
                    let p = e.path();
                    let id = p.file_stem()?.to_str()?.to_string();
                    if p.extension()? != "json" || !valid_id(&id) {
                        return None;
                    }
                    let doc: Value = serde_json::from_slice(&std::fs::read(&p).ok()?).ok()?;
                    let t = e.metadata().ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as i64;
                    Some((id, doc, t))
                })
                .collect(),
        };
        out.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        out.into_iter()
            .map(|(id, doc, t)| json!({"id": id, "name": doc["name"], "description": doc.get("description").cloned().unwrap_or(json!("")), "updated_at": t}))
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<Value> {
        if !valid_id(id) {
            return None;
        }
        match self.path(id) {
            None => self.mem.lock().unwrap().get(id).map(|(v, _)| v.clone()),
            Some(p) => serde_json::from_slice(&std::fs::read(p).ok()?).ok(),
        }
    }

    pub fn put(&self, id: &str, doc: &Value) -> Result<(), String> {
        if !valid_id(id) {
            return Err(format!("invalid flow id {id:?}"));
        }
        match self.path(id) {
            None => {
                self.mem.lock().unwrap().insert(id.to_string(), (doc.clone(), now_ms()));
                Ok(())
            }
            Some(p) => {
                std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
                let tmp = p.with_extension("json.tmp");
                std::fs::write(&tmp, serde_json::to_vec_pretty(doc).unwrap()).map_err(|e| e.to_string())?;
                std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
            }
        }
    }

    /// A new id from the flow's name (`hold-5-v`, `hold-5-v-2`, …).
    pub fn new_id(&self, name: &str) -> String {
        let base = slug(name);
        let mut id = base.clone();
        let mut n = 2;
        while self.get(&id).is_some() {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }

    pub fn delete(&self, id: &str) -> bool {
        if !valid_id(id) {
            return false;
        }
        match self.path(id) {
            None => self.mem.lock().unwrap().remove(id).is_some(),
            Some(p) => std::fs::remove_file(p).is_ok(),
        }
    }
}

// ---- settings ----------------------------------------------------------------------------

pub const PROVIDERS: [&str; 4] = ["anthropic", "openai", "openrouter", "custom"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub provider: String,
    /// Model per provider (switching provider keeps each one's choice).
    pub models: BTreeMap<String, String>,
    /// Endpoint per provider: required for `custom` (OpenAI-compatible, e.g.
    /// `http://localhost:11434/v1`), an optional override (proxy) for the others.
    pub base_urls: BTreeMap<String, String>,
    /// API key per provider.
    pub keys: BTreeMap<String, String>,
    /// MCP access: `off`, `read` (default) or `control`.
    pub mcp: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { provider: "anthropic".into(), models: BTreeMap::new(), base_urls: BTreeMap::new(), keys: BTreeMap::new(), mcp: "read".into() }
    }
}

/// `sk-ant-…a1b2`: enough to recognise a key, never enough to use it.
pub fn key_hint(k: &str) -> String {
    let k = k.trim();
    let n = k.chars().count();
    if n <= 12 {
        return format!("…{}", k.chars().skip(n.saturating_sub(2)).collect::<String>());
    }
    let head: String = k.chars().take(k.find('-').map(|i| (i + 1).min(7)).unwrap_or(3)).collect();
    let tail: String = k.chars().skip(n - 4).collect();
    format!("{head}…{tail}")
}

pub struct SettingsFile {
    path: Option<PathBuf>,
    current: Mutex<Settings>,
}

impl SettingsFile {
    pub fn open(path: Option<PathBuf>) -> SettingsFile {
        let current = path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| match serde_json::from_slice::<Settings>(&b) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("automation settings: {e} (using defaults)");
                    None
                }
            })
            .unwrap_or_default();
        SettingsFile { path, current: Mutex::new(current) }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    pub fn update(&self, f: impl FnOnce(&mut Settings)) -> Result<Settings, String> {
        let mut cur = self.current.lock().unwrap();
        let mut next = cur.clone();
        f(&mut next);
        if let Some(p) = &self.path {
            write_private(p, &serde_json::to_vec_pretty(&next).unwrap())?;
        }
        *cur = next.clone();
        Ok(next)
    }
}

/// Write a file only the user can read (API keys, password hash, TLS key).
pub(crate) fn write_private(p: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension("json.tmp");
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, p).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flows_persist_with_ids_from_their_names() {
        let dir = std::env::temp_dir().join(format!("bm-flows-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let f = Flows::new(Some(dir.clone()));
        assert_eq!(f.new_id("Hold 5 V - load!"), "hold-5-v-load");
        f.put("hold-5-v", &json!({"name": "Hold 5 V"})).unwrap();
        assert_eq!(f.new_id("Hold 5 V"), "hold-5-v-2");
        assert_eq!(f.list()[0]["name"], "Hold 5 V");
        assert!(f.get("../etc/passwd").is_none());
        assert!(f.put("../x", &json!({})).is_err());
        assert!(f.delete("hold-5-v"));
        assert!(f.list().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_are_private_and_keys_are_only_hinted() {
        let dir = std::env::temp_dir().join(format!("bm-settings-{}", std::process::id()));
        let p = dir.join("automation.json");
        let s = SettingsFile::open(Some(p.clone()));
        assert_eq!(s.get().mcp, "read");
        s.update(|x| {
            x.keys.insert("anthropic".into(), "sk-ant-api03-abcdefghijklmnop1234".into());
        })
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
        }
        assert_eq!(SettingsFile::open(Some(p)).get().keys["anthropic"], "sk-ant-api03-abcdefghijklmnop1234");
        assert_eq!(key_hint("sk-ant-api03-abcdefghijklmnop1234"), "sk-…1234");
        assert_eq!(key_hint("short"), "…rt");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
