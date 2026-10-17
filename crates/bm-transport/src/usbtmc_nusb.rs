//! Userspace USBTMC over `nusb`: Windows (device bound to the generic WinUSB driver) and macOS
//! (no driver needed). Linux uses the kernel `usbtmc` driver (`/dev/usbtmcN`) instead, which
//! needs no extra permissions; a `usb:` path works there too given access to `/dev/bus/usb`.
//!
//! Port path: `usb:<vid>:<pid>:<serial>`, or `usb:<vid>:<pid>@<location>` without a serial.

use std::io;
use std::time::Duration;

use nusb::descriptors::TransferType;
use nusb::transfer::{Bulk, ControlIn, ControlType, Direction, In, Out, Recipient};
use nusb::{DeviceInfo, Endpoint, Interface, MaybeFuture};

use crate::discover::{Port, PortKind, UsbInfo};
use crate::usbtmc_proto::{Pipe, Session};

pub const PREFIX: &str = "usb:";

fn is_usbtmc(class: u8, subclass: u8) -> bool {
    class == 0xFE && subclass == 0x03
}

/// Physical port position, in the same form as `discover` reports it per OS.
fn location(d: &DeviceInfo) -> String {
    #[cfg(target_os = "linux")]
    {
        let chain: Vec<String> = d.port_chain().iter().map(u8::to_string).collect();
        format!("{}-{}", d.busnum(), chain.join("."))
    }
    #[cfg(windows)]
    {
        d.location_paths().first().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
    }
    #[cfg(target_os = "macos")]
    {
        format!("0x{:08x}", d.location_id())
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        let _ = d;
        String::new()
    }
}

pub fn path_of(vid: u16, pid: u16, serial: Option<&str>, location: &str) -> String {
    match serial.filter(|s| !s.is_empty()) {
        Some(s) => format!("{PREFIX}{vid:04x}:{pid:04x}:{s}"),
        None => format!("{PREFIX}{vid:04x}:{pid:04x}@{location}"),
    }
}

fn path(d: &DeviceInfo) -> String {
    path_of(d.vendor_id(), d.product_id(), d.serial_number(), &location(d))
}

fn devices() -> Vec<DeviceInfo> {
    nusb::list_devices().wait().map(|it| it.filter(|d| d.interfaces().any(|i| is_usbtmc(i.class(), i.subclass()))).collect()).unwrap_or_default()
}

/// The Windows driver bound to the device, if it is not one nusb can use.
#[cfg(windows)]
pub fn unusable_driver(d: &DeviceInfo) -> Option<String> {
    match d.driver() {
        Some(drv) if drv.eq_ignore_ascii_case("winusb") => None,
        Some(drv) => Some(drv.to_string()),
        None => Some(String::new()),
    }
}

/// USBTMC instruments visible to `nusb`, with the driver note (Windows) for each.
pub fn ports() -> Vec<(Port, DeviceInfo)> {
    devices()
        .into_iter()
        .map(|d| {
            let usb = UsbInfo {
                vendor_id: Some(format!("{:04x}", d.vendor_id())),
                product_id: Some(format!("{:04x}", d.product_id())),
                manufacturer: d.manufacturer_string().map(str::to_string),
                product: d.product_string().map(str::to_string),
                serial: d.serial_number().map(str::to_string).filter(|s| !s.is_empty()),
                revision: Some(format!("{:04x}", d.device_version())),
                location: Some(location(&d)).filter(|l| !l.is_empty()),
            };
            (Port { kind: PortKind::Usbtmc, path: path(&d), usb }, d)
        })
        .collect()
}

#[cfg(windows)]
fn driver_help(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "{what}. Either install a VISA (NI-VISA, Keysight IO Libraries or Rigol UltraSigma) — BenchMesh then uses it — \
             or give the instrument the generic WinUSB driver once (Zadig: select the instrument, WinUSB, Replace Driver)"
        ),
    )
}

pub fn open(path: &str) -> io::Result<Session<NusbPipe>> {
    let info = devices()
        .into_iter()
        .find(|d| self::path(d) == path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{path}: no such USBTMC device attached")))?;
    #[cfg(windows)]
    if let Some(drv) = unusable_driver(&info) {
        let what = if drv.is_empty() { "Windows has no driver for this instrument".to_string() } else { format!("the instrument uses the Windows driver \"{drv}\", not WinUSB") };
        return Err(driver_help(&what));
    }
    let number = info
        .interfaces()
        .find(|i| is_usbtmc(i.class(), i.subclass()))
        .map(|i| i.interface_number())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no USBTMC interface"))?;
    let device = info.open().wait().map_err(|e| io::Error::other(format!("open {path}: {e}")))?;
    #[cfg(target_os = "linux")]
    let iface = device.detach_and_claim_interface(number).wait();
    #[cfg(not(target_os = "linux"))]
    let iface = device.claim_interface(number).wait();
    let iface = iface.map_err(|e| io::Error::other(format!("claim USBTMC interface of {path}: {e} (another program, e.g. a VISA, may be using it)")))?;
    let desc = iface.descriptor().ok_or_else(|| io::Error::other("no interface descriptor"))?;
    let bulk = |dir: Direction| desc.endpoints().find(|e| e.transfer_type() == TransferType::Bulk && e.direction() == dir);
    let (o, i) = match (bulk(Direction::Out), bulk(Direction::In)) {
        (Some(o), Some(i)) => (o, i),
        _ => return Err(io::Error::other("USBTMC interface without bulk endpoints")),
    };
    let (out_addr, in_addr, max_packet) = (o.address(), i.address(), i.max_packet_size());
    let out = iface.endpoint::<Bulk, Out>(out_addr).map_err(io::Error::other)?;
    let inp = iface.endpoint::<Bulk, In>(in_addr).map_err(io::Error::other)?;
    Ok(Session::new(NusbPipe { iface, out, inp, in_addr }, max_packet))
}

pub struct NusbPipe {
    iface: Interface,
    out: Endpoint<Bulk, Out>,
    inp: Endpoint<Bulk, In>,
    in_addr: u8,
}

/// USBTMC class requests (Table 15) and status (Table 16).
const INITIATE_ABORT_BULK_IN: u8 = 3;
const CHECK_ABORT_BULK_IN_STATUS: u8 = 4;
const STATUS_SUCCESS: u8 = 0x01;
const STATUS_PENDING: u8 = 0x02;

impl NusbPipe {
    fn class_in(&self, request: u8, value: u16, length: u16) -> Option<Vec<u8>> {
        let req = ControlIn { control_type: ControlType::Class, recipient: Recipient::Endpoint, request, value, index: self.in_addr as u16, length };
        self.iface.control_in(req, Duration::from_millis(500)).wait().ok()
    }
}

impl Pipe for NusbPipe {
    fn bulk_out(&mut self, data: &[u8], timeout: Duration) -> io::Result<()> {
        let c = self.out.transfer_blocking(data.to_vec().into(), timeout);
        c.status.map_err(|e| io::Error::other(format!("USBTMC write: {e}")))
    }

    fn bulk_in(&mut self, max: usize, timeout: Duration) -> io::Result<Option<Vec<u8>>> {
        let buf = self.inp.allocate(max);
        self.inp.submit(buf);
        match self.inp.wait_next_complete(timeout.max(Duration::from_millis(1))) {
            Some(c) => {
                c.status.map_err(|e| io::Error::other(format!("USBTMC read: {e}")))?;
                let mut v = c.buffer.into_vec();
                v.truncate(c.actual_len);
                Ok(Some(v))
            }
            None => {
                self.inp.cancel_all();
                while self.inp.pending() > 0 {
                    if self.inp.wait_next_complete(Duration::from_secs(1)).is_none() {
                        break;
                    }
                }
                Ok(None)
            }
        }
    }

    fn abort_in(&mut self, tag: u8) {
        let started = self.class_in(INITIATE_ABORT_BULK_IN, tag as u16, 2);
        if started.as_ref().and_then(|r| r.first()) != Some(&STATUS_SUCCESS) {
            return; // nothing in flight for that tag
        }
        for _ in 0..20 {
            let Some(s) = self.class_in(CHECK_ABORT_BULK_IN_STATUS, 0, 8) else { return };
            if s.first() != Some(&STATUS_PENDING) {
                return;
            }
            if s.get(1).is_some_and(|f| f & 0x01 != 0) {
                let mp = self.inp.max_packet_size();
                let _ = self.bulk_in(mp * 64, Duration::from_millis(100)); // the device wants its data read first
            } else {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(path_of(0x1ab1, 0x044d, Some("DHO8A272907001"), "1-2"), "usb:1ab1:044d:DHO8A272907001");
        assert_eq!(path_of(0x5345, 0x1235, None, "0x14320000"), "usb:5345:1235@0x14320000");
        assert_eq!(path_of(0x5345, 0x1235, Some(""), "1-1.3"), "usb:5345:1235@1-1.3");
        assert!(is_usbtmc(0xFE, 0x03) && !is_usbtmc(0xFE, 0x01));
    }
}
