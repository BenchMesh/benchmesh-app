//! Per-instrument wire log: every byte written to and read from an instrument, with times, in a
//! bounded ring - what the UI's console shows and users copy as troubleshooting evidence.
//!
//! [`logged`] wraps a transport; reads are logged as the chunks the transport returns (a reply
//! may arrive in several; the UI joins consecutive ones). Opening, failing to open and closing a
//! port are logged as `info` lines.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::engine::Transport;

/// Entries kept per instrument.
pub const CAPACITY: usize = 2000;
/// Bytes kept per entry (waveforms and screenshots are cut; `len` keeps the real size).
pub const MAX_DATA: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Tx,
    Rx,
    Info,
}

impl Dir {
    pub fn as_str(self) -> &'static str {
        match self {
            Dir::Tx => "tx",
            Dir::Rx => "rx",
            Dir::Info => "info",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub seq: u64,
    /// Unix time in milliseconds.
    pub at_ms: u64,
    pub dir: Dir,
    /// The bytes (at most [`MAX_DATA`]); for `info`, UTF-8 text.
    pub data: Vec<u8>,
    /// Real length (more than `data.len()` when cut).
    pub len: usize,
}

#[derive(Debug, Default)]
struct Ring {
    next: u64,
    entries: VecDeque<Entry>,
}

#[derive(Debug, Default)]
pub struct WireLog {
    ring: Mutex<Ring>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis() as u64
}

impl WireLog {
    pub fn push(&self, dir: Dir, data: &[u8]) {
        let mut r = self.ring.lock().unwrap();
        r.next += 1;
        let entry = Entry { seq: r.next, at_ms: now_ms(), dir, data: data[..data.len().min(MAX_DATA)].to_vec(), len: data.len() };
        r.entries.push_back(entry);
        while r.entries.len() > CAPACITY {
            r.entries.pop_front();
        }
    }

    pub fn info(&self, text: &str) {
        self.push(Dir::Info, text.as_bytes());
    }

    /// Entries after `after` (0 = everything kept), and whether some were already dropped.
    pub fn since(&self, after: u64) -> (Vec<Entry>, bool) {
        let r = self.ring.lock().unwrap();
        let dropped = r.entries.front().is_some_and(|e| e.seq > after + 1) && after > 0;
        (r.entries.iter().filter(|e| e.seq > after).cloned().collect(), dropped)
    }

    pub fn clear(&self) {
        self.ring.lock().unwrap().entries.clear();
    }
}

struct Logged {
    inner: Box<dyn Transport>,
    log: Arc<WireLog>,
    port: String,
}

impl Transport for Logged {
    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.log.push(Dir::Tx, data);
        let r = self.inner.write(data);
        if let Err(e) = &r {
            self.log.info(&format!("write failed: {e}"));
        }
        r
    }

    fn read(&mut self, timeout: Duration) -> io::Result<Vec<u8>> {
        let r = self.inner.read(timeout);
        match &r {
            Ok(b) if !b.is_empty() => self.log.push(Dir::Rx, b),
            Ok(_) => {}
            Err(e) => self.log.info(&format!("read failed: {e}")),
        }
        r
    }
}

impl Drop for Logged {
    fn drop(&mut self) {
        self.log.info(&format!("closed {}", self.port));
    }
}

/// `t` with every exchange recorded in `log`.
pub fn logged(t: Box<dyn Transport>, log: Arc<WireLog>, port: &str) -> Box<dyn Transport> {
    log.info(&format!("opened {port}"));
    Box::new(Logged { inner: t, log, port: port.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::SimTransport;

    #[test]
    fn exchanges_are_recorded_in_order_and_the_ring_is_bounded() {
        let log = Arc::new(WireLog::default());
        let sim = SimTransport::new(Box::new(|w| (w == b"*IDN?\n").then(|| b"ACME,X1\n".to_vec())));
        let mut t = logged(Box::new(sim), log.clone(), "/dev/ttyUSB0");
        t.write(b"*IDN?\n").unwrap();
        assert_eq!(t.read(Duration::from_millis(50)).unwrap(), b"ACME,X1\n");
        assert!(t.read(Duration::from_millis(5)).unwrap().is_empty(), "silence is not logged");
        drop(t);
        let (e, dropped) = log.since(0);
        let shown: Vec<(&str, String)> = e.iter().map(|e| (e.dir.as_str(), String::from_utf8_lossy(&e.data).to_string())).collect();
        assert_eq!(shown, [("info", "opened /dev/ttyUSB0".into()), ("tx", "*IDN?\n".into()), ("rx", "ACME,X1\n".into()), ("info", "closed /dev/ttyUSB0".into())]);
        assert!(!dropped);
        assert_eq!(log.since(e[1].seq).0.len(), 2, "only the newer entries");

        for _ in 0..CAPACITY + 10 {
            log.push(Dir::Tx, &[0u8; MAX_DATA + 1]);
        }
        let (all, _) = log.since(0);
        assert_eq!(all.len(), CAPACITY);
        assert_eq!((all[0].data.len(), all[0].len), (MAX_DATA, MAX_DATA + 1), "cut, real length kept");
        assert!(log.since(1).1, "a client that fell behind learns entries were dropped");
    }
}
