//! Linux: `/dev` + sysfs (readable by any user; no libudev needed).

use std::path::{Path, PathBuf};

use super::{hex4, text, Port, PortKind, SerialPortInfo, UsbInfo};

fn read_trim(p: &Path) -> Option<String> {
    text(std::fs::read_to_string(p).ok())
}

/// Walk up from a sysfs device directory to the USB device (the one with `idVendor`).
fn usb_info(sysfs_device: &Path) -> Option<UsbInfo> {
    let mut dir: Option<PathBuf> = std::fs::canonicalize(sysfs_device).ok();
    while let Some(d) = dir {
        if d.join("idVendor").exists() {
            return Some(UsbInfo {
                vendor_id: read_trim(&d.join("idVendor")).and_then(|v| hex4(&v)),
                product_id: read_trim(&d.join("idProduct")).and_then(|v| hex4(&v)),
                manufacturer: read_trim(&d.join("manufacturer")),
                product: read_trim(&d.join("product")),
                serial: read_trim(&d.join("serial")),
                revision: read_trim(&d.join("bcdDevice")).and_then(|v| hex4(&v)),
                // The USB device's sysfs name is its bus + port chain, e.g. `1-1.3.2.2.4`.
                location: d.file_name().map(|n| n.to_string_lossy().to_string()),
            });
        }
        dir = d.parent().map(Path::to_path_buf).filter(|p| p != Path::new("/sys"));
    }
    None
}

fn dev_names(prefixes: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir("/dev")
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    v.retain(|n| prefixes.iter().any(|p| n.starts_with(p)));
    v
}

pub fn ports() -> Vec<Port> {
    let serial = dev_names(&["ttyUSB", "ttyACM"]).into_iter().filter_map(|n| {
        let usb = usb_info(&Path::new("/sys/class/tty").join(&n).join("device"))?;
        Some(Port { kind: PortKind::Serial, path: format!("/dev/{n}"), usb })
    });
    let tmc = dev_names(&["usbtmc"]).into_iter().filter_map(|n| {
        let usb = usb_info(&Path::new("/sys/class/usbmisc").join(&n).join("device"))?;
        Some(Port { kind: PortKind::Usbtmc, path: format!("/dev/{n}"), usb })
    });
    serial.chain(tmc).collect()
}

/// Serial ports for the manual port picker, including the stable `/dev/serial/by-*` links.
pub fn serial_ports() -> Vec<SerialPortInfo> {
    let mut out = Vec::new();
    let mut add = |dev: PathBuf, name: String, alias: bool| {
        let usb = usb_info(&Path::new("/sys/class/tty").join(&name).join("device")).unwrap_or_default();
        let base = usb.product.clone().unwrap_or_else(|| "Unknown Device".into());
        let description = if alias { format!("{base} (link to {name})") } else { base };
        out.push(SerialPortInfo { device: dev.to_string_lossy().to_string(), description, usb });
    };
    for n in dev_names(&["ttyUSB", "ttyACM"]) {
        add(Path::new("/dev").join(&n), n, false);
    }
    for dir in ["/dev/serial/by-id", "/dev/serial/by-path"] {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                if let Some(name) = std::fs::canonicalize(e.path()).ok().and_then(|t| t.file_name().map(|n| n.to_string_lossy().to_string())) {
                    add(e.path(), name, true);
                }
            }
        }
    }
    out.sort_by(|a, b| a.device.cmp(&b.device));
    out
}
