//! Is a parked instrument there? Found without a byte to the instrument: a USB instrument by
//! port enumeration (its identity, or its fixed port), a LAN instrument by opening and closing
//! a TCP connection.

use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use bm_devicedef::Link;

use crate::discover::{self, PortKind};
use crate::identity::Identity;
use crate::{lock, port_kind, tcp_target, vxi11, Endpoint};

/// How long a LAN presence check waits for the connection.
const CONNECT: Duration = Duration::from_secs(2);

/// `Ok(where it is)` or `Err(why it is not found)`.
pub fn presence(link: &Link, ep: &Endpoint, id: Option<&Identity>) -> Result<String, String> {
    match link {
        Link::Tcp(t) => reachable(tcp_target(&ep.path, t.port)),
        Link::Vxi11(_) => reachable(vxi11::portmapper_target(&ep.path)),
        Link::Serial(_) | Link::Usbtmc(_) => {
            let kind = port_kind(link).expect("a USB link");
            match id {
                Some(id) => {
                    let ports = discover::ports();
                    let hint = Some(ep.path.as_str()).filter(|p| !p.is_empty());
                    let found = id.candidates(kind, &ports, hint).first().map(|p| p.path.clone());
                    found.ok_or_else(|| format!("not attached: no {} port matches {}", kind.as_str(), id.describe()))
                }
                None => attached(kind, &ep.path, |kind| match kind {
                    PortKind::Serial => discover::serial_ports().into_iter().map(|p| p.device).collect(),
                    PortKind::Usbtmc => discover::usbtmc_devices().into_iter().map(|d| d.device).collect(),
                }),
            }
        }
    }
}

/// A fixed port: listed by the system (`listed`), or a device node that exists.
fn attached(kind: PortKind, path: &str, listed: impl Fn(PortKind) -> Vec<String>) -> Result<String, String> {
    if path.is_empty() {
        return Err("no port configured".into());
    }
    if listed(kind).iter().any(|p| lock::same_port(p, path)) || Path::new(path).exists() {
        Ok(path.to_string())
    } else {
        Err(format!("{path} not found"))
    }
}

/// Something accepts a TCP connection at `target` (closed again at once).
fn reachable(target: io::Result<String>) -> Result<String, String> {
    let target = target.map_err(|e| e.to_string())?;
    let addrs = target.to_socket_addrs().map_err(|e| format!("{target}: {e}"))?;
    let mut last = format!("{target}: no address");
    for a in addrs {
        match TcpStream::connect_timeout(&a, CONNECT) {
            Ok(_) => return Ok(target),
            Err(e) => last = format!("{target}: {e}"),
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lan_instrument_is_present_while_it_accepts_connections() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let target = listener.local_addr().unwrap().to_string();
        assert_eq!(reachable(Ok(target.clone())), Ok(target.clone()));
        drop(listener);
        let err = reachable(Ok(target.clone())).unwrap_err();
        assert!(err.starts_with(&target), "{err}");
        assert!(reachable(Err(io::Error::new(io::ErrorKind::InvalidInput, "needs a host"))).unwrap_err().contains("needs a host"));
    }

    #[test]
    fn a_fixed_port_is_present_when_listed_or_its_node_exists() {
        let none = |_: PortKind| Vec::new();
        assert_eq!(attached(PortKind::Serial, "COM7", |_| vec!["COM7".into()]), Ok("COM7".into()), "listed by the system");
        let node = std::env::temp_dir().join(format!("bm-presence-{}", std::process::id()));
        std::fs::write(&node, b"").unwrap();
        let path = node.to_string_lossy().to_string();
        assert_eq!(attached(PortKind::Usbtmc, &path, none), Ok(path.clone()));
        std::fs::remove_file(&node).unwrap();
        assert_eq!(attached(PortKind::Usbtmc, &path, none), Err(format!("{path} not found")));
        assert_eq!(attached(PortKind::Serial, "", none), Err("no port configured".into()));
    }
}
