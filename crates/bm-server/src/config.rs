//! config.yaml: `{version: 1, devices: [{id, name, driver, model?, transport, port|device|host|match, baud?, serial?}]}`
//! (`host`: `tcpip` instruments, `name-or-address[:port]`).
//! `match` = the instrument's identity (`ai_context/migration/IDENTITY.md`); `port`/`device` is
//! then only the last known port.
//! Device entries are kept as raw JSON objects (unknown keys survive round trips).

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub path: PathBuf,
}

pub type DeviceEntry = Map<String, Value>;

impl ConfigFile {
    pub fn load(&self) -> Result<Vec<DeviceEntry>, String> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(format!("{}: {e}", self.path.display())),
        };
        let v: Value = serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", self.path.display()))?;
        Ok(v.get("devices")
            .and_then(|d| d.as_array())
            .map(|a| a.iter().filter_map(|d| d.as_object().cloned()).collect())
            .unwrap_or_default())
    }

    /// Atomic write (temp file in the same directory, then rename).
    pub fn save(&self, devices: &[DeviceEntry]) -> Result<(), String> {
        let doc = json!({"version": 1, "devices": devices});
        let text = serde_yaml_ng::to_string(&doc).map_err(|e| e.to_string())?;
        let dir = self.path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let tmp = dir.join(format!(".config_{}.yaml.tmp", std::process::id()));
        std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| format!("{}: {e}", self.path.display()))
    }
}

pub fn str_field<'a>(d: &'a DeviceEntry, k: &str) -> Option<&'a str> {
    d.get(k).and_then(|v| v.as_str())
}

/// Validate one device entry; returns a human-readable reason on failure.
pub fn validate_entry(d: &DeviceEntry, known_drivers: &[String]) -> Result<(), String> {
    let id = str_field(d, "id").unwrap_or_default();
    if id.is_empty() || id.len() > 50 || !id.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) {
        return Err(format!("invalid device id {id:?} (1-50 chars: letters, digits, - _ .)"));
    }
    let driver = str_field(d, "driver").unwrap_or_default();
    if !known_drivers.iter().any(|k| k == driver) {
        return Err(format!("unknown driver {driver:?}; available: {known_drivers:?}"));
    }
    let identified = match d.get("match") {
        None => false,
        Some(m) => {
            let id: bm_transport::identity::Identity =
                serde_json::from_value(m.clone()).map_err(|e| format!("invalid `match`: {e}"))?;
            let hex4 = |s: &str| s.len() == 4 && s.chars().all(|c| c.is_ascii_hexdigit());
            if !hex4(&id.usb.vid) || !hex4(&id.usb.pid) || id.usb.revision.as_deref().is_some_and(|r| !hex4(r)) {
                return Err("invalid `match`: usb vid, pid and revision are 4 hex digits".into());
            }
            true
        }
    };
    match str_field(d, "transport").unwrap_or("serial") {
        "serial" => {
            if !identified && str_field(d, "port").unwrap_or_default().is_empty() {
                return Err("serial device needs `port` or `match`".into());
            }
            if let Some(b) = d.get("baud") {
                const BAUDS: [u64; 10] = [300, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400];
                if !b.as_u64().is_some_and(|b| BAUDS.contains(&b)) {
                    return Err(format!("invalid baud {b}"));
                }
            }
            if let Some(s) = str_field(d, "serial") {
                let ok = s.len() == 3 && "5678".contains(&s[0..1]) && "NEO".contains(&s[1..2]) && "12".contains(&s[2..3]);
                if !ok {
                    return Err(format!("invalid serial format {s:?} (e.g. 8N1)"));
                }
            }
        }
        "usbtmc" => {
            if !identified && str_field(d, "device").unwrap_or_default().is_empty() {
                return Err("usbtmc device needs `device` or `match`".into());
            }
        }
        "tcpip" | "vxi11" => {
            let host = str_field(d, "host").unwrap_or_default().trim();
            if host.is_empty() || host.chars().any(|c| c.is_whitespace() || c == '/') {
                return Err("a LAN device needs `host` (name or address, optionally :port)".into());
            }
            if identified {
                return Err("`match` (USB identity) does not apply to a LAN device".into());
            }
        }
        t => return Err(format!("unsupported transport {t:?} (serial, usbtmc, tcpip, vxi11)")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(v: serde_json::Value) -> DeviceEntry {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn tcpip_entries_need_a_host_and_no_usb_identity() {
        let drivers = vec!["rigol_dp800".to_string()];
        let ok = entry(serde_json::json!({"id": "psu", "driver": "rigol_dp800", "transport": "tcpip", "host": "192.168.1.50:5555"}));
        assert!(validate_entry(&ok, &drivers).is_ok());
        for bad in [
            serde_json::json!({"id": "psu", "driver": "rigol_dp800", "transport": "tcpip"}),
            serde_json::json!({"id": "psu", "driver": "rigol_dp800", "transport": "tcpip", "host": "a b"}),
            serde_json::json!({"id": "psu", "driver": "rigol_dp800", "transport": "tcpip", "host": "h",
                               "match": {"usb": {"vid": "1ab1", "pid": "0e11"}}}),
        ] {
            assert!(validate_entry(&entry(bad.clone()), &drivers).is_err(), "{bad}");
        }
    }
}
