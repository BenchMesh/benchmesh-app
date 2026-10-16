//! Physical transports for the BenchMesh engine.
//!
//! - [`SerialTransport`]: RS-232 / USB-serial via `serialport` (Linux, macOS, Windows).
//! - [`UsbTmcTransport`]: Linux kernel `usbtmc` driver (`/dev/usbtmcN`); one `read()`
//!   returns one whole device message — never split a message into several reads
//!   (split reads strand data on the device and desync the USBTMC tags; see the DGE
//!   screenshot capture).

use std::io::{self, Read, Write};
use std::time::Duration;

use bm_devicedef::{Flow, LineState, Link, ModemControl, SerialLink};
use bm_driver::engine::Transport;

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

/// Open the transport a definition's `link` describes.
pub fn open(link: &Link, ep: &Endpoint) -> io::Result<Box<dyn Transport>> {
    match link {
        Link::Serial(s) => Ok(Box::new(SerialTransport::open(s, ep)?)),
        #[cfg(target_os = "linux")]
        Link::Usbtmc(_) => Ok(Box::new(UsbTmcTransport::open(&ep.path)?)),
        #[cfg(not(target_os = "linux"))]
        Link::Usbtmc(_) => Err(io::Error::new(io::ErrorKind::Unsupported, "USBTMC is only implemented on Linux so far")),
        Link::Tcp(_) => Err(io::Error::new(io::ErrorKind::Unsupported, "TCP transport not implemented yet")),
    }
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
