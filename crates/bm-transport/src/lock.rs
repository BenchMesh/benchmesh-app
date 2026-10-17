//! In-process port lock: one open transport per port, whatever the OS. Kernel USBTMC nodes
//! can be opened twice (two interleaved conversations desync the instrument), and a port
//! held by one instrument must not be probed by another's identity search or by Detect.

use std::collections::BTreeSet;
use std::io;
use std::sync::Mutex;

static HELD: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// The same port under any name (`/dev/serial/by-id/...` link → `/dev/ttyUSB1`).
fn key(path: &str) -> String {
    std::fs::canonicalize(path).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| path.to_string())
}

/// Held while a transport is open; released on drop.
#[derive(Debug)]
pub struct PortGuard(String);

impl Drop for PortGuard {
    fn drop(&mut self) {
        HELD.lock().unwrap().remove(&self.0);
    }
}

pub fn acquire(path: &str) -> io::Result<PortGuard> {
    let k = key(path);
    if !HELD.lock().unwrap().insert(k.clone()) {
        return Err(io::Error::new(io::ErrorKind::AddrInUse, format!("{path} is in use by another instrument")));
    }
    Ok(PortGuard(k))
}

/// Two names for the same port (e.g. a `/dev/serial/by-id/...` link and its target).
pub fn same_port(a: &str, b: &str) -> bool {
    key(a) == key(b)
}

pub fn is_held(path: &str) -> bool {
    HELD.lock().unwrap().contains(&key(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_holder_per_port() {
        let g = acquire("test-lock-port-a").unwrap();
        assert!(is_held("test-lock-port-a"));
        assert_eq!(acquire("test-lock-port-a").unwrap_err().kind(), io::ErrorKind::AddrInUse);
        assert!(acquire("test-lock-port-b").is_ok());
        drop(g);
        assert!(!is_held("test-lock-port-a"));
        assert!(acquire("test-lock-port-a").is_ok());
    }
}
