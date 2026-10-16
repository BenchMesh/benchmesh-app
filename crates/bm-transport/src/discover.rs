//! Port discovery for the configuration UI.
//! Linux: `/dev` + sysfs (no libudev needed); elsewhere: `serialport::available_ports`.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsbInfo {
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialPortInfo {
    /// Path to open (stable `/dev/serial/by-id/...` links are listed too).
    pub device: String,
    pub description: String,
    pub usb: UsbInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbTmcInfo {
    pub device: String,
    pub name: String,
    pub usb: UsbInfo,
}

fn read_trim(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Walk up from a sysfs device directory to the USB device (the one with `idVendor`).
fn usb_info(sysfs_device: &Path) -> UsbInfo {
    let mut dir: Option<PathBuf> = std::fs::canonicalize(sysfs_device).ok();
    while let Some(d) = dir {
        if d.join("idVendor").exists() {
            return UsbInfo {
                vendor_id: read_trim(&d.join("idVendor")),
                product_id: read_trim(&d.join("idProduct")),
                manufacturer: read_trim(&d.join("manufacturer")),
                product: read_trim(&d.join("product")),
                serial: read_trim(&d.join("serial")),
            };
        }
        dir = d.parent().map(Path::to_path_buf).filter(|p| p != Path::new("/sys"));
    }
    UsbInfo::default()
}

#[cfg(target_os = "linux")]
pub fn serial_ports() -> Vec<SerialPortInfo> {
    let mut out = Vec::new();
    let name_of = |p: &Path| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut add = |dev: PathBuf, target: PathBuf, alias: bool| {
        let name = name_of(&target);
        let usb = usb_info(&Path::new("/sys/class/tty").join(&name).join("device"));
        let base = usb.product.clone().unwrap_or_else(|| "Unknown Device".into());
        let description = if alias { format!("{base} (link to {name})") } else { base };
        out.push(SerialPortInfo { device: dev.to_string_lossy().to_string(), description, usb });
    };
    if let Ok(rd) = std::fs::read_dir("/dev") {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("ttyUSB") || n.starts_with("ttyACM") {
                add(e.path(), e.path(), false);
            }
        }
    }
    for dir in ["/dev/serial/by-id", "/dev/serial/by-path"] {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                if let Ok(t) = std::fs::canonicalize(e.path()) {
                    add(e.path(), t, true);
                }
            }
        }
    }
    out.sort_by(|a, b| a.device.cmp(&b.device));
    out
}

#[cfg(not(target_os = "linux"))]
pub fn serial_ports() -> Vec<SerialPortInfo> {
    let mut out: Vec<SerialPortInfo> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| {
            let usb = match &p.port_type {
                serialport::SerialPortType::UsbPort(u) => UsbInfo {
                    vendor_id: Some(format!("{:04x}", u.vid)),
                    product_id: Some(format!("{:04x}", u.pid)),
                    manufacturer: u.manufacturer.clone(),
                    product: u.product.clone(),
                    serial: u.serial_number.clone(),
                },
                _ => UsbInfo::default(),
            };
            SerialPortInfo { description: usb.product.clone().unwrap_or_else(|| "Unknown Device".into()), device: p.port_name, usb }
        })
        .collect();
    out.sort_by(|a, b| a.device.cmp(&b.device));
    out
}

/// Linux kernel USBTMC device nodes (`/dev/usbtmcN`).
pub fn usbtmc_devices() -> Vec<UsbTmcInfo> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir("/dev") {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("usbtmc") {
                let usb = usb_info(&Path::new("/sys/class/usbmisc").join(&n).join("device"));
                out.push(UsbTmcInfo { device: e.path().to_string_lossy().to_string(), name: n, usb });
            }
        }
    }
    out.sort_by(|a, b| a.device.cmp(&b.device));
    out
}
