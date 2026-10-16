//! Simulated transport: answers commands with a responder function and records every
//! write with a timestamp. Used by tests, dry runs and the server's demo mode.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::engine::Transport;

pub type Responder = Box<dyn FnMut(&[u8]) -> Option<Vec<u8>> + Send>;

/// Everything written to the simulated device.
#[derive(Debug, Clone, Default)]
pub struct WireLog {
    pub writes: Vec<(Instant, Vec<u8>)>,
}

impl WireLog {
    pub fn texts(&self) -> Vec<String> {
        self.writes.iter().map(|(_, w)| w.iter().map(|&b| b as char).collect()).collect()
    }
}

pub struct SimTransport {
    responder: Responder,
    pending: VecDeque<Vec<u8>>,
    pub log: Arc<Mutex<WireLog>>,
    /// Device processing time before a reply becomes readable.
    pub latency: Duration,
    ready_at: Instant,
}

impl SimTransport {
    pub fn new(responder: Responder) -> Self {
        SimTransport {
            responder,
            pending: VecDeque::new(),
            log: Arc::new(Mutex::new(WireLog::default())),
            latency: Duration::ZERO,
            ready_at: Instant::now(),
        }
    }
}

impl Transport for SimTransport {
    fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.log.lock().unwrap().writes.push((Instant::now(), data.to_vec()));
        if let Some(r) = (self.responder)(data) {
            self.pending.push_back(r);
            self.ready_at = Instant::now() + self.latency;
        }
        Ok(())
    }

    fn read(&mut self, timeout: Duration) -> std::io::Result<Vec<u8>> {
        let now = Instant::now();
        if !self.pending.is_empty() && self.ready_at <= now + timeout {
            if self.ready_at > now {
                std::thread::sleep(self.ready_at - now);
            }
            return Ok(self.pending.pop_front().unwrap());
        }
        std::thread::sleep(timeout);
        Ok(Vec::new())
    }
}
