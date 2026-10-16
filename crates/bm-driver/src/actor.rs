//! One thread per device: owns the [`Engine`], reconnects with backoff, polls on schedule
//! and serves get/set requests between polls.

use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bm_devicedef::DeviceDef;

use crate::engine::{Engine, Output, StatusUpdate, Transport};
use crate::Error;

/// Opens the device's transport (called again on every reconnect).
pub type Connector = Box<dyn FnMut() -> std::io::Result<Box<dyn Transport>> + Send>;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Connected { idn: String, model: String },
    Status(StatusUpdate),
    Disconnected { reason: String },
}

enum Request {
    Get { class: String, op: String, ch: u8, reply: Sender<Result<Output, Error>> },
    Set { class: String, op: String, ch: u8, value: String, reply: Sender<Result<(), Error>> },
    Stop,
}

#[derive(Debug, Clone)]
pub struct ActorConfig {
    /// Wait before reconnecting after a failure.
    pub reconnect_backoff: Duration,
    /// Polls in a row with no answer at all before the link is considered lost.
    pub max_silent_polls: u32,
    /// How long a caller waits for a request to be served.
    pub request_timeout: Duration,
}

impl Default for ActorConfig {
    fn default() -> Self {
        ActorConfig { reconnect_backoff: Duration::from_secs(2), max_silent_polls: 3, request_timeout: Duration::from_secs(10) }
    }
}

pub struct DeviceHandle {
    tx: Sender<Request>,
    join: Option<JoinHandle<()>>,
    request_timeout: Duration,
}

impl DeviceHandle {
    pub fn spawn(
        name: &str,
        def: Arc<DeviceDef>,
        connector: Connector,
        events: Box<dyn Fn(Event) + Send>,
        cfg: ActorConfig,
    ) -> DeviceHandle {
        let (tx, rx) = channel();
        let request_timeout = cfg.request_timeout;
        let join = std::thread::Builder::new()
            .name(format!("dev-{name}"))
            .spawn(move || run(def, connector, events, rx, cfg))
            .expect("spawn device thread");
        DeviceHandle { tx, join: Some(join), request_timeout }
    }

    /// Cheap, cloneable request sender (e.g. for HTTP handlers).
    pub fn client(&self) -> DeviceClient {
        DeviceClient { tx: self.tx.clone(), request_timeout: self.request_timeout }
    }

    pub fn get(&self, class: &str, op: &str, ch: u8) -> Result<Output, Error> {
        self.client().get(class, op, ch)
    }

    pub fn set(&self, class: &str, op: &str, ch: u8, value: &str) -> Result<(), Error> {
        self.client().set(class, op, ch, value)
    }

    /// Stop the thread (runs `on_disconnect`) and wait for it.
    pub fn stop(mut self) {
        let _ = self.tx.send(Request::Stop);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// Sends requests to a device thread; blocks until served (or the request timeout).
#[derive(Clone)]
pub struct DeviceClient {
    tx: Sender<Request>,
    request_timeout: Duration,
}

impl DeviceClient {
    pub fn get(&self, class: &str, op: &str, ch: u8) -> Result<Output, Error> {
        let (reply, rx) = channel();
        self.send(Request::Get { class: class.into(), op: op.into(), ch, reply })?;
        rx.recv_timeout(self.request_timeout).map_err(|_| Error::Timeout { partial: Vec::new() })?
    }

    pub fn set(&self, class: &str, op: &str, ch: u8, value: &str) -> Result<(), Error> {
        let (reply, rx) = channel();
        self.send(Request::Set { class: class.into(), op: op.into(), ch, value: value.into(), reply })?;
        rx.recv_timeout(self.request_timeout).map_err(|_| Error::Timeout { partial: Vec::new() })?
    }

    fn send(&self, r: Request) -> Result<(), Error> {
        self.tx.send(r).map_err(|_| Error::Io("device thread stopped".into()))
    }
}

impl Drop for DeviceHandle {
    fn drop(&mut self) {
        if let Some(j) = self.join.take() {
            let _ = self.tx.send(Request::Stop);
            let _ = j.join();
        }
    }
}

/// Answer a request while there is no connection.
fn refuse(r: Request) {
    let msg = || Error::Io("device not connected".into());
    match r {
        Request::Get { reply, .. } => {
            let _ = reply.send(Err(msg()));
        }
        Request::Set { reply, .. } => {
            let _ = reply.send(Err(msg()));
        }
        Request::Stop => {}
    }
}

fn run(def: Arc<DeviceDef>, mut connector: Connector, events: Box<dyn Fn(Event) + Send>, rx: Receiver<Request>, cfg: ActorConfig) {
    loop {
        // ---- connect -------------------------------------------------------------------
        let engine = connector()
            .map_err(|e| Error::Io(e.to_string()))
            .and_then(|t| Engine::connect(def.clone(), t));
        let mut engine = match engine {
            Ok(e) => e,
            Err(err) => {
                events(Event::Disconnected { reason: err.to_string() });
                if backoff(&rx, cfg.reconnect_backoff) {
                    return;
                }
                continue;
            }
        };
        events(Event::Connected { idn: engine.idn.clone(), model: engine.model.name.clone() });

        // ---- serve ---------------------------------------------------------------------
        let reason = loop {
            let wait = engine.next_due().map(|t| t.saturating_duration_since(Instant::now())).unwrap_or(Duration::from_secs(1));
            match rx.recv_timeout(wait) {
                Ok(Request::Get { class, op, ch, reply }) => {
                    let r = engine.get(&class, &op, ch);
                    let io = matches!(r, Err(Error::Io(_)));
                    let _ = reply.send(r);
                    if io {
                        break "transport error".to_string();
                    }
                }
                Ok(Request::Set { class, op, ch, value, reply }) => {
                    let r = engine.set(&class, &op, ch, &value);
                    let io = matches!(r, Err(Error::Io(_)));
                    let _ = reply.send(r);
                    if io {
                        break "transport error".to_string();
                    }
                }
                Ok(Request::Stop) | Err(RecvTimeoutError::Disconnected) => {
                    engine.disconnect();
                    events(Event::Disconnected { reason: "stopped".into() });
                    return;
                }
                Err(RecvTimeoutError::Timeout) => match engine.poll_due() {
                    Ok(updates) => {
                        for u in updates {
                            events(Event::Status(u));
                        }
                        if engine.silent_polls >= cfg.max_silent_polls {
                            break format!("no reply to {} polls", engine.silent_polls);
                        }
                    }
                    Err(e) => break e.to_string(),
                },
            }
        };
        drop(engine); // the link is broken: no on_disconnect traffic
        events(Event::Disconnected { reason });
        if backoff(&rx, cfg.reconnect_backoff) {
            return;
        }
    }
}

/// Wait before reconnecting while refusing requests; true = stop requested.
fn backoff(rx: &Receiver<Request>, d: Duration) -> bool {
    let until = Instant::now() + d;
    while let Some(left) = until.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(Request::Stop) => return true,
            Ok(r) => refuse(r),
            Err(RecvTimeoutError::Timeout) => return false,
            Err(RecvTimeoutError::Disconnected) => return true,
        }
    }
    false
}
