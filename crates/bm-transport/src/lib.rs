//! Physical transports for the BenchMesh engine.
//!
//! - [`SerialTransport`]: RS-232 / USB-serial via `serialport` (Linux, macOS, Windows).
//! - USBTMC on Windows / macOS: userspace protocol ([`usbtmc_proto`]) over `nusb`
//!   ([`usbtmc_nusb`], WinUSB on Windows), or an installed VISA ([`visa`]). [`winusb`] gives a
//!   Windows instrument the WinUSB driver (one UAC prompt).
//! - [`UsbTmcTransport`]: Linux kernel `usbtmc` driver (`/dev/usbtmcN`); one `read()`
//!   returns one whole device message — never split a message into several reads
//!   (split reads strand data on the device and desync the USBTMC tags; see the DGE
//!   screenshot capture).
//!
//! Every open transport holds its port ([`lock`]); configured instruments are found by
//! [`identity`] on whatever port they are plugged into ([`attempts`]).

use std::io::{self, Read, Write};
use std::sync::Arc;
use std::time::Duration;

use bm_devicedef::{Flow, LineState, Link, ModemControl, SerialLink};
use bm_driver::actor::Attempt;
use bm_driver::engine::Transport;

pub mod discover;
pub mod identity;
pub mod lock;
pub mod usbtmc_nusb;
pub mod usbtmc_proto;
pub mod visa;
pub mod winusb;
#[cfg(target_os = "linux")]
mod usbtmc;
#[cfg(target_os = "linux")]
pub use usbtmc::UsbTmcTransport;

/// Where a configured device is attached (from config.yaml).
#[derive(Debug, Clone, Default)]
pub struct Endpoint {
    /// Serial port path or USBTMC device node.
    pub path: String,
    /// Baud rate override (else the definition's).
    pub baud: Option<u32>,
    /// Serial format override, e.g. `8N1`.
    pub format: Option<String>,
}

/// Open the transport a definition's `link` describes (holding the port until dropped).
pub fn open(link: &Link, ep: &Endpoint) -> io::Result<Box<dyn Transport>> {
    let guard = lock::acquire(&ep.path)?;
    let inner: Box<dyn Transport> = match link {
        Link::Serial(s) => Box::new(SerialTransport::open(s, ep)?),
        Link::Usbtmc(_) if visa::is_resource(&ep.path) => Box::new(visa::VisaTransport::open(&ep.path)?),
        Link::Usbtmc(_) if ep.path.starts_with(usbtmc_nusb::PREFIX) => Box::new(usbtmc_nusb::open(&ep.path)?),
        #[cfg(target_os = "linux")]
        Link::Usbtmc(_) => Box::new(UsbTmcTransport::open(&ep.path)?),
        #[cfg(not(target_os = "linux"))]
        Link::Usbtmc(_) => {
            let msg = format!("{}: not a USBTMC instrument path (use Detect, a `usb:` path or a VISA resource)", ep.path);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
        }
        Link::Tcp(_) => return Err(io::Error::new(io::ErrorKind::Unsupported, "TCP transport not implemented yet")),
    };
    Ok(Box::new(Held { inner, _guard: guard }))
}

/// A transport together with its port lock.
struct Held {
    inner: Box<dyn Transport>,
    _guard: lock::PortGuard,
}

impl Transport for Held {
    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.inner.write(data)
    }

    fn read(&mut self, timeout: Duration) -> io::Result<Vec<u8>> {
        self.inner.read(timeout)
    }
}

/// The kind of port a definition's link is found on.
pub fn port_kind(link: &Link) -> Option<discover::PortKind> {
    match link {
        Link::Serial(_) => Some(discover::PortKind::Serial),
        Link::Usbtmc(_) => Some(discover::PortKind::Usbtmc),
        Link::Tcp(_) => None,
    }
}

/// The ports to try for a configured instrument: its fixed path, or — with an identity —
/// every attached port matching it (best first), each accepted only if the identify reply
/// carries the identity's serial number. `ep.path` is then the last known port (a hint).
pub fn attempts(link: &Link, ep: &Endpoint, id: Option<&identity::Identity>) -> Result<Vec<Attempt>, String> {
    let attempt = |path: &str, accept: Option<bm_driver::actor::Accept>| {
        let (link, ep) = (link.clone(), Endpoint { path: path.to_string(), ..ep.clone() });
        Attempt { port: path.to_string(), open: Box::new(move || open(&link, &ep)), accept }
    };
    let Some(id) = id else {
        return Ok(vec![attempt(&ep.path, None)]);
    };
    let kind = port_kind(link).ok_or("identity matching needs a USB link (serial or usbtmc)")?;
    let ports = discover::ports();
    let hint = Some(ep.path.as_str()).filter(|p| !p.is_empty());
    let found = id.candidates(kind, &ports, hint);
    if found.is_empty() {
        return Err(format!("no free {} port matches {}", kind.as_str(), id.describe()));
    }
    let check = id.clone();
    let accept: bm_driver::actor::Accept = Arc::new(move |idn: &str| check.check_idn(idn));
    Ok(found.into_iter().map(|p| attempt(&p.path, Some(accept.clone()))).collect())
}

pub struct SerialTransport {
    port: Box<dyn serialport::SerialPort>,
    buf: Vec<u8>,
}

fn parse_format(f: &str) -> io::Result<(serialport::DataBits, serialport::Parity, serialport::StopBits)> {
    use serialport::{DataBits, Parity, StopBits};
    let bad = || io::Error::new(io::ErrorKind::InvalidInput, format!("invalid serial format {f:?}"));
    let mut c = f.chars();
    let bits = match c.next() {
        Some('5') => DataBits::Five,
        Some('6') => DataBits::Six,
        Some('7') => DataBits::Seven,
        Some('8') => DataBits::Eight,
        _ => return Err(bad()),
    };
    let parity = match c.next() {
        Some('N') => Parity::None,
        Some('E') => Parity::Even,
        Some('O') => Parity::Odd,
        _ => return Err(bad()),
    };
    let stop = match c.as_str() {
        "1" => StopBits::One,
        "2" => StopBits::Two,
        _ => return Err(bad()),
    };
    Ok((bits, parity, stop))
}

impl SerialTransport {
    pub fn open(link: &SerialLink, ep: &Endpoint) -> io::Result<Self> {
        let (bits, parity, stop) = parse_format(ep.format.as_deref().unwrap_or(&link.format))?;
        let flow = match link.flow {
            Flow::None => serialport::FlowControl::None,
            Flow::RtsCts => serialport::FlowControl::Hardware,
            Flow::XonXoff => serialport::FlowControl::Software,
        };
        let mut port = serialport::new(&ep.path, ep.baud.unwrap_or(link.baud))
            .data_bits(bits)
            .parity(parity)
            .stop_bits(stop)
            .flow_control(flow)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(io::Error::from)?;
        let required = link.modem_control == ModemControl::Required;
        let dtr = set_line(link.dtr, |v| port.write_data_terminal_ready(v));
        let rts = set_line(link.rts, |v| port.write_request_to_send(v));
        for r in [dtr, rts] {
            // best_effort: some adapters / pseudo-terminals have no modem-control lines
            if let (Err(e), true) = (r, required) {
                return Err(io::Error::from(e));
            }
        }
        std::thread::sleep(Duration::from_millis(link.settle_ms as u64));
        let _ = port.clear(serialport::ClearBuffer::Input);
        Ok(SerialTransport { port, buf: vec![0; 4096] })
    }
}

fn set_line(state: LineState, mut set: impl FnMut(bool) -> serialport::Result<()>) -> serialport::Result<()> {
    match state {
        LineState::Untouched => Ok(()),
        LineState::Low => set(false),
        LineState::High => set(true),
    }
}

impl Transport for SerialTransport {
    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.port.write_all(data)?;
        self.port.flush()
    }

    fn read(&mut self, timeout: Duration) -> io::Result<Vec<u8>> {
        self.port.set_timeout(timeout.max(Duration::from_millis(1))).map_err(io::Error::from)?;
        match self.port.read(&mut self.buf) {
            Ok(n) => Ok(self.buf[..n].to_vec()),
            Err(e) if e.kind() == io::ErrorKind::TimedOut => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_formats() {
        assert!(parse_format("8N1").is_ok());
        assert!(parse_format("7E2").is_ok());
        assert!(parse_format("9N1").is_err());
        assert!(parse_format("8X1").is_err());
    }
}
