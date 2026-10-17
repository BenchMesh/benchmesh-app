//! Port discovery: the configuration UI's port lists and the USB ports instruments are
//! identified on ([`ports`], see `ai_context/migration/IDENTITY.md`).
//! Linux: `/dev` + sysfs (no libudev); Windows: SetupAPI; macOS: IORegistry. No rights needed.

mod ids;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsbInfo {
    /// 4 lowercase hex digits, e.g. `1a86`.
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    /// USB serial number; `None` when the device has none.
    pub serial: Option<String>,
    /// `bcdDevice`, 4 lowercase hex digits (chip/firmware revision).
    pub revision: Option<String>,
    /// Physical port position (OS-specific text); stable while the device stays on that port.
    pub location: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortKind {
    Serial,
    Usbtmc,
}

impl PortKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PortKind::Serial => "serial",
            PortKind::Usbtmc => "usbtmc",
        }
    }
}

/// A USB-attached port an instrument can be identified on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub kind: PortKind,
    /// What to open: `/dev/ttyUSB0`, `COM5`, `/dev/cu.usbserial-110`, `/dev/usbtmc2`.
    pub path: String,
    pub usb: UsbInfo,
}

/// USB serial ports and USBTMC instruments currently attached (non-USB ports are left out).
pub fn ports() -> Vec<Port> {
    #[cfg(target_os = "linux")]
    let mut out = linux::ports();
    #[cfg(windows)]
    let mut out = windows::ports();
    #[cfg(target_os = "macos")]
    let mut out = macos::ports();
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    let mut out: Vec<Port> = Vec::new();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// `0x1A86`, `1a86`, `6790` (decimal from some APIs is converted by the caller) → `1a86`.
pub(crate) fn hex4(s: &str) -> Option<String> {
    let t = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    (!t.is_empty() && t.len() <= 4 && t.chars().all(|c| c.is_ascii_hexdigit())).then(|| format!("{:0>4}", t.to_ascii_lowercase()))
}

/// Trimmed, `None` when empty.
pub(crate) fn text(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
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

#[cfg(target_os = "linux")]
pub fn serial_ports() -> Vec<SerialPortInfo> {
    linux::serial_ports()
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
                    ..UsbInfo::default()
                },
                _ => UsbInfo::default(),
            };
            SerialPortInfo { description: usb.product.clone().unwrap_or_else(|| "Unknown Device".into()), device: p.port_name, usb }
        })
        .collect();
    out.sort_by(|a, b| a.device.cmp(&b.device));
    out
}

/// USBTMC instruments (Linux kernel device nodes `/dev/usbtmcN`; other OSes: none yet).
pub fn usbtmc_devices() -> Vec<UsbTmcInfo> {
    ports()
        .into_iter()
        .filter(|p| p.kind == PortKind::Usbtmc)
        .map(|p| UsbTmcInfo { name: p.path.rsplit('/').next().unwrap_or_default().to_string(), device: p.path, usb: p.usb })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex4_normalises() {
        assert_eq!(hex4("1A86").as_deref(), Some("1a86"));
        assert_eq!(hex4("0x264").as_deref(), Some("0264"));
        assert_eq!(hex4(" 0264\n").as_deref(), Some("0264"));
        assert_eq!(hex4(""), None);
        assert_eq!(hex4("12345"), None);
        assert_eq!(hex4("zz"), None);
    }
}
