//! One thread per device: owns the [`Engine`], reconnects with backoff, polls on schedule
//! and serves get/set requests between polls. A parked device has no link at all: requests are
//! refused and only its [`Presence`] is checked (no traffic to the instrument).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bm_devicedef::DeviceDef;

use crate::engine::{Engine, Output, StatusUpdate, Transport};
use crate::metrics::Metrics;

use crate::Error;

/// Checks an identify reply before the session starts (e.g. the instrument's serial number).
pub type Accept = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

/// One port to try: opened, identified, accepted, then the session starts.
pub struct Attempt {
    /// Where (shown in errors and reported with [`Event::Connected`]).
    pub port: String,
    pub open: Box<dyn FnOnce() -> std::io::Result<Box<dyn Transport>> + Send>,
    pub accept: Option<Accept>,
}

impl Attempt {
    /// Identify on this transport, check the reply, start the session.
    pub fn connect(self, def: Arc<DeviceDef>) -> Result<Engine, Error> {
        let mut e = (self.open)().map_err(|e| Error::Io(e.to_string())).and_then(|t| Engine::open(def, t))?;
        if let Some(accept) = &self.accept {
            accept(&e.idn).map_err(Error::WrongInstrument)?;
        }
        e.start_session()?;
        Ok(e)
    }
}

/// The ports to try, in order (called again on every reconnect, so a device that moved to
/// another port is found again). `Err` = nothing to try, with the reason.
pub type Connector = Box<dyn FnMut() -> Result<Vec<Attempt>, String> + Send>;

/// A connector for one fixed transport (tests, simulators, fixed ports).
pub fn fixed_connector<F>(port: &str, open: F) -> Connector
where
    F: FnMut() -> std::io::Result<Box<dyn Transport>> + Send + 'static,
{
    let port = port.to_string();
    let open = Arc::new(std::sync::Mutex::new(open));
    Box::new(move || {
        let open = open.clone();
        Ok(vec![Attempt { port: port.clone(), open: Box::new(move || (open.lock().unwrap())()), accept: None }])
    })
}

/// Whether a parked instrument is attached / reachable, found without talking to it (port
/// enumeration, a TCP connect): `Ok(where)` or `Err(why not)`.
pub type Presence = Box<dyn FnMut() -> Result<String, String> + Send>;

/// How a device thread starts with respect to parking.
pub struct Parking {
    pub presence: Presence,
    /// Start parked: the port is not opened until the device is unparked.
    pub parked: bool,
}

/// Try the attempts in order; the first that connects wins.
fn connect_any(def: &Arc<DeviceDef>, attempts: Vec<Attempt>) -> Result<(Engine, String), Error> {
    let mut errors = Vec::new();
    for a in attempts {
        let port = a.port.clone();
        match a.connect(def.clone()) {
            Ok(e) => return Ok((e, port)),
            Err(e) => errors.push((port, e)),
        }
    }
    match errors.len() {
        0 => Err(Error::Io("no port to try".into())),
        1 => Err(errors.pop().unwrap().1),
        _ => Err(Error::Io(errors.iter().map(|(p, e)| format!("{p}: {e}")).collect::<Vec<_>>().join("; "))),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// `port`: where the device was found.
    Connected { idn: String, model: String, port: String },
    Status(StatusUpdate),
    Disconnected { reason: String },
    /// Performance metrics (same fields as the former Python `/metrics`).
    Metrics(serde_json::Value),
    /// Parked (the link is closed), and on every change of its presence: `detail` is where it
    /// is attached or why it is not found.
    Parked { present: bool, detail: String },
    /// Unparked: connecting again.
    Unparked,
}

enum Request {
    Get { class: String, op: String, ch: u8, reply: Sender<Result<Output, Error>> },
    Set { class: String, op: String, ch: u8, value: String, reply: Sender<Result<(), Error>> },
    /// Park (`true`) or unpark; answered once done.
    Park { parked: bool, reply: Sender<()> },
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
    /// How often the device thread publishes metrics.
    pub metrics_every: Duration,
    /// How often a parked device's presence is checked.
    pub presence_every: Duration,
}

impl Default for ActorConfig {
    fn default() -> Self {
        ActorConfig {
            reconnect_backoff: Duration::from_secs(2),
            max_silent_polls: 3,
            request_timeout: Duration::from_secs(10),
            metrics_every: Duration::from_secs(5),
            presence_every: Duration::from_secs(5),
        }
    }
}

pub struct DeviceHandle {
    tx: Sender<Request>,
    join: Option<JoinHandle<()>>,
    request_timeout: Duration,
    pending: Arc<AtomicUsize>,
}

impl DeviceHandle {
    pub fn spawn(name: &str, def: Arc<DeviceDef>, connector: Connector, events: Box<dyn Fn(Event) + Send>, cfg: ActorConfig) -> DeviceHandle {
        let parking = Parking { presence: Box::new(|| Err("presence unknown".into())), parked: false };
        Self::spawn_with(name, def, connector, parking, events, cfg)
    }

    /// [`DeviceHandle::spawn`] that can start parked and checks `parking.presence` while parked.
    pub fn spawn_with(
        name: &str,
        def: Arc<DeviceDef>,
        connector: Connector,
        parking: Parking,
        events: Box<dyn Fn(Event) + Send>,
        cfg: ActorConfig,
    ) -> DeviceHandle {
        let (tx, rx) = channel();
        let request_timeout = cfg.request_timeout;
        let pending = Arc::new(AtomicUsize::new(0));
        // resolved definitions (`DeviceDef::for_transport`) have exactly one link
        let transport = def.link.kinds().first().copied().unwrap_or("none");
        let metrics = Metrics::new(name, transport);
        let p = pending.clone();
        let device = Device { name: name.to_string(), def, connector, presence: parking.presence, events, cfg };
        let join = std::thread::Builder::new()
            .name(format!("dev-{name}"))
            .spawn(move || run(device, parking.parked, rx, metrics, p))
            .expect("spawn device thread");
        DeviceHandle { tx, join: Some(join), request_timeout, pending }
    }

    /// Cheap, cloneable request sender (e.g. for HTTP handlers).
    pub fn client(&self) -> DeviceClient {
        DeviceClient { tx: self.tx.clone(), request_timeout: self.request_timeout, pending: self.pending.clone() }
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
    /// Requests sent but not yet picked up by the device thread (queue depth metric).
    pending: Arc<AtomicUsize>,
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

    /// Park or unpark; returns once the device thread has done it (or after the request
    /// timeout - the request is still carried out, it is queued).
    pub fn park(&self, parked: bool) -> Result<(), Error> {
        let (reply, rx) = channel();
        self.tx.send(Request::Park { parked, reply }).map_err(|_| Error::Io("device thread stopped".into()))?;
        rx.recv_timeout(self.request_timeout).map_err(|_| Error::Timeout { partial: Vec::new() })
    }

    fn send(&self, r: Request) -> Result<(), Error> {
        self.pending.fetch_add(1, Ordering::SeqCst);
        self.tx.send(r).map_err(|_| {
            self.pending.fetch_sub(1, Ordering::SeqCst);
            Error::Io("device thread stopped".into())
        })
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

/// Answer a get/set without a connection.
fn refuse(r: Request, msg: impl Fn() -> Error) {
    match r {
        Request::Get { reply, .. } => {
            let _ = reply.send(Err(msg()));
        }
        Request::Set { reply, .. } => {
            let _ = reply.send(Err(msg()));
        }
        Request::Park { .. } | Request::Stop => {}
    }
}

fn not_connected() -> Error {
    Error::Io("device not connected".into())
}

/// What a device thread works with.
struct Device {
    name: String,
    def: Arc<DeviceDef>,
    connector: Connector,
    presence: Presence,
    events: Box<dyn Fn(Event) + Send>,
    cfg: ActorConfig,
}

/// Why waiting (backoff, parked) ended.
enum Wake {
    Stop,
    /// Park (`true`) / unpark requested; answer `reply` once done.
    Park(bool, Sender<()>),
    Elapsed,
}

/// How a connection ended.
enum End {
    Lost(String),
    Park(Sender<()>),
    Stop,
}

fn run(mut d: Device, mut parked: bool, rx: Receiver<Request>, mut metrics: Metrics, pending: Arc<AtomicUsize>) {
    let mut next_metrics = Instant::now() + d.cfg.metrics_every;
    loop {
        if parked {
            if !park(&mut d, &rx, &pending) {
                return;
            }
            parked = false;
            (d.events)(Event::Unparked);
        }
        // ---- connect -------------------------------------------------------------------
        let engine = (d.connector)().map_err(Error::NotFound).and_then(|attempts| connect_any(&d.def, attempts));
        let (mut engine, port) = match engine {
            Ok(e) => e,
            Err(err) => {
                (d.events)(Event::Disconnected { reason: err.to_string() });
                match wait(&rx, d.cfg.reconnect_backoff, &pending, not_connected) {
                    Wake::Stop => return,
                    Wake::Park(p, reply) => {
                        parked = p;
                        let _ = reply.send(());
                    }
                    Wake::Elapsed => {}
                }
                continue;
            }
        };
        (d.events)(Event::Connected { idn: engine.idn.clone(), model: engine.model.name.clone(), port });
        metrics.connected();

        // ---- serve ---------------------------------------------------------------------
        let end = loop {
            let now = Instant::now();
            if now >= next_metrics {
                metrics.update_stats(engine.stats);
                (d.events)(Event::Metrics(metrics.snapshot()));
                metrics.roll_window();
                next_metrics = now + d.cfg.metrics_every;
            }
            let due = engine.next_due().unwrap_or(now + Duration::from_secs(1)).min(next_metrics);
            let wait = due.saturating_duration_since(now);
            let received = rx.recv_timeout(wait);
            if matches!(received, Ok(Request::Get { .. } | Request::Set { .. })) {
                pending.fetch_sub(1, Ordering::SeqCst); // Stop and Park are not counted (no queue metric)
            }
            metrics.queue_sample(pending.load(Ordering::SeqCst));
            let started = Instant::now();
            match received {
                Ok(Request::Get { class, op, ch, reply }) => {
                    let r = engine.get(&class, &op, ch);
                    metrics.api_request(started.elapsed());
                    let io = matches!(r, Err(Error::Io(_)));
                    let _ = reply.send(r);
                    if io {
                        break End::Lost("transport error".to_string());
                    }
                }
                Ok(Request::Set { class, op, ch, value, reply }) => {
                    let r = engine.set(&class, &op, ch, &value);
                    metrics.api_request(started.elapsed());
                    let io = matches!(r, Err(Error::Io(_)));
                    let _ = reply.send(r);
                    if io {
                        break End::Lost("transport error".to_string());
                    }
                }
                Ok(Request::Park { parked: true, reply }) => break End::Park(reply),
                Ok(Request::Park { parked: false, reply }) => {
                    let _ = reply.send(());
                }
                Ok(Request::Stop) | Err(RecvTimeoutError::Disconnected) => break End::Stop,
                Err(RecvTimeoutError::Timeout) => match engine.poll_due() {
                    Ok(updates) => {
                        if !updates.is_empty() {
                            metrics.poll(started.elapsed());
                        }
                        for u in updates {
                            (d.events)(Event::Status(u));
                        }
                        if engine.silent_polls >= d.cfg.max_silent_polls {
                            break End::Lost(format!("no reply to {} polls", engine.silent_polls));
                        }
                    }
                    Err(e) => break End::Lost(e.to_string()),
                },
            }
        };
        match end {
            End::Stop => {
                engine.disconnect();
                (d.events)(Event::Disconnected { reason: "stopped".into() });
                return;
            }
            End::Park(reply) => {
                engine.disconnect(); // on_disconnect: the panel goes back to the user; the port is released
                parked = true;
                let _ = reply.send(());
            }
            End::Lost(reason) => {
                drop(engine); // the link is broken: no on_disconnect traffic
                (d.events)(Event::Disconnected { reason });
                match wait(&rx, d.cfg.reconnect_backoff, &pending, not_connected) {
                    Wake::Stop => return,
                    Wake::Park(p, reply) => {
                        parked = p;
                        let _ = reply.send(());
                    }
                    Wake::Elapsed => {}
                }
            }
        }
    }
}

/// Parked: refuse requests and report presence changes until unparked (`true`) or stopped.
fn park(d: &mut Device, rx: &Receiver<Request>, pending: &AtomicUsize) -> bool {
    let name = d.name.clone();
    let parked = || Error::Refused(format!("{name} is parked: unpark it to use it"));
    let mut last: Option<Result<String, String>> = None;
    loop {
        let now = (d.presence)();
        if last.as_ref() != Some(&now) {
            let (present, detail) = match &now {
                Ok(at) => (true, at.clone()),
                Err(why) => (false, why.clone()),
            };
            (d.events)(Event::Parked { present, detail });
            last = Some(now);
        }
        match wait(rx, d.cfg.presence_every, pending, parked) {
            Wake::Stop => return false,
            Wake::Park(true, reply) => {
                let _ = reply.send(());
            }
            Wake::Park(false, reply) => {
                let _ = reply.send(());
                return true;
            }
            Wake::Elapsed => {}
        }
    }
}

/// Wait `d` while refusing get/set requests with `refusal`; ends early on stop or park.
fn wait(rx: &Receiver<Request>, d: Duration, pending: &AtomicUsize, refusal: impl Fn() -> Error) -> Wake {
    let until = Instant::now() + d;
    while let Some(left) = until.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(Request::Stop) => return Wake::Stop,
            Ok(Request::Park { parked, reply }) => return Wake::Park(parked, reply),
            Ok(r) => {
                pending.fetch_sub(1, Ordering::SeqCst);
                refuse(r, &refusal)
            }
            Err(RecvTimeoutError::Timeout) => return Wake::Elapsed,
            Err(RecvTimeoutError::Disconnected) => return Wake::Stop,
        }
    }
    Wake::Elapsed
}
