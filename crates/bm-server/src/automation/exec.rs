//! The flow executor (contract `ai_context/AUTOMATION.md`): one run at a time; `setup`, then
//! `steps`, then always `cleanup`; every `set` confirmed by the instrument, every status
//! read newer than the run's last `set`; abort interrupts waits at once; a failed or aborted
//! run puts every output it switched on back to its safe value.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bm_driver::engine::Output;
use bm_store::now_ms;
use serde_json::{json, Map, Value};
use tokio::sync::{watch, Notify};

use super::flow::{self, Address, Energize, Flow, Issue, Kind, ReadSource, RepeatBound, Step, ValueSource};
use super::vocab::{Op, Vocab};
use crate::devices::Devices;
use crate::recordings::{Recordings, MAX_AGE};

/// How long a `set` waits for the instrument to report the value (as the UI does).
pub const CONFIRM_TIMEOUT: Duration = Duration::from_secs(6);
/// How long a status read waits for an update newer than the last `set`.
pub const FRESH_TIMEOUT: Duration = Duration::from_secs(5);
/// Shortest pass of a `repeat` (a loop of reads must not spin).
pub const MIN_PASS: Duration = Duration::from_millis(100);
const EVENTS: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Running,
    Waiting,
    Cleanup,
    Finished,
    Failed,
    Aborted,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Running => "running",
            State::Waiting => "waiting",
            State::Cleanup => "cleanup",
            State::Finished => "finished",
            State::Failed => "failed",
            State::Aborted => "aborted",
        }
    }

    pub fn active(self) -> bool {
        matches!(self, State::Running | State::Waiting | State::Cleanup)
    }
}

struct Info {
    state: State,
    path: String,
    vars: BTreeMap<String, Value>,
    error: Option<String>,
    recording_id: Option<i64>,
    ended_at: Option<i64>,
    prompt: Option<String>,
    events: VecDeque<Value>,
    seq: u64,
}

pub struct Run {
    pub id: u64,
    pub flow_id: Option<String>,
    pub flow_name: String,
    /// The instruments the flow addresses (they cannot be parked while it runs).
    pub devices: BTreeSet<String>,
    pub started_at: i64,
    info: Mutex<Info>,
    /// Abort requests: 1 = abort (cleanup still runs), 2+ = also stop the cleanup.
    abort: watch::Sender<u8>,
    resume: Notify,
}

impl Run {
    fn event(&self, level: &str, path: &str, msg: impl Into<String>) {
        let msg = msg.into();
        let mut i = self.info.lock().unwrap();
        i.seq += 1;
        let e = json!({"seq": i.seq, "t": now_ms(), "level": level, "path": path, "msg": msg});
        if i.events.len() == EVENTS {
            i.events.pop_front();
        }
        i.events.push_back(e);
    }

    fn set_state(&self, s: State) {
        self.info.lock().unwrap().state = s;
    }

    pub fn state(&self) -> State {
        self.info.lock().unwrap().state
    }

    pub fn json(&self, after: u64) -> Value {
        let i = self.info.lock().unwrap();
        let events: Vec<&Value> = i.events.iter().filter(|e| e["seq"].as_u64().unwrap_or(0) > after).collect();
        json!({
            "id": self.id, "flow": {"id": self.flow_id, "name": self.flow_name}, "devices": self.devices, "state": i.state.as_str(),
            "started_at": self.started_at, "ended_at": i.ended_at, "path": i.path, "vars": i.vars,
            "error": i.error, "prompt": i.prompt, "recording_id": i.recording_id, "last_seq": i.seq, "events": events,
        })
    }
}

/// Why a part of the flow ended early.
#[derive(Debug)]
enum Halt {
    /// A `stop` step: the main part ends successfully.
    Stop,
    Fail(String),
    Abort,
}

#[derive(Debug)]
pub enum StartError {
    Busy,
    Invalid(Vec<Issue>),
    NeedsArm(Vec<Energize>),
    Params(String),
    /// Instruments the flow uses that are parked.
    Parked(Vec<String>),
}

pub struct Runner {
    devices: Arc<Devices>,
    recordings: Arc<Recordings>,
    current: Mutex<Option<Arc<Run>>>,
    next_id: Mutex<u64>,
}

impl Runner {
    pub fn new(devices: Arc<Devices>, recordings: Arc<Recordings>) -> Arc<Runner> {
        Arc::new(Runner { devices, recordings, current: Mutex::new(None), next_id: Mutex::new(1) })
    }

    pub fn current(&self) -> Option<Arc<Run>> {
        self.current.lock().unwrap().clone()
    }

    /// Validate and start a run (in the background). Must be called inside the tokio runtime.
    pub fn start(&self, doc: &Value, flow_id: Option<String>, params: Option<&Map<String, Value>>, arm: bool) -> Result<Arc<Run>, StartError> {
        let vocab = Vocab::from_devices(&self.devices);
        let (parsed, issues, energize) = flow::check(doc, &vocab);
        let Some(parsed) = parsed.filter(|_| !flow::has_errors(&issues)) else {
            return Err(StartError::Invalid(issues));
        };
        if !energize.is_empty() && !arm {
            return Err(StartError::NeedsArm(energize));
        }
        let vars = flow::param_values(&parsed, params).map_err(StartError::Params)?;
        let mut cur = self.current.lock().unwrap();
        if cur.as_ref().is_some_and(|r| r.state().active()) {
            return Err(StartError::Busy);
        }
        // checked under `current`: parking checks the run under the same lock (`unless_used`)
        let devices = parsed.devices();
        let parked: Vec<String> = devices.iter().filter(|d| self.devices.is_parked(d)).cloned().collect();
        if !parked.is_empty() {
            return Err(StartError::Parked(parked));
        }
        let id = {
            let mut n = self.next_id.lock().unwrap();
            *n += 1;
            *n - 1
        };
        let (abort, _) = watch::channel(0u8);
        let run = Arc::new(Run {
            id,
            flow_id,
            flow_name: parsed.name.clone(),
            devices,
            started_at: now_ms(),
            info: Mutex::new(Info { state: State::Running, path: String::new(), vars: vars.clone(), error: None, recording_id: None, ended_at: None, prompt: None, events: VecDeque::new(), seq: 0 }),
            abort,
            resume: Notify::new(),
        });
        *cur = Some(run.clone());
        for w in issues.iter().filter(|i| i.level == "warning") {
            run.event("warn", &w.path, w.message.clone());
        }
        let exec = Exec {
            run: run.clone(),
            devices: self.devices.clone(),
            recordings: self.recordings.clone(),
            vocab,
            vars,
            started: Instant::now(),
            deadline: parsed.timeout_s.map(|t| Instant::now() + Duration::from_secs_f64(t)),
            last_set: None,
            energized: BTreeSet::new(),
            recording: None,
            cleanup: false,
        };
        tokio::spawn(exec.execute(parsed));
        Ok(run)
    }

    /// Run `f` unless parking `device` (`parking`) would take it from the active run (`Err`: that
    /// run's flow name). No run can start while `f` runs.
    pub fn unless_used<T>(&self, device: &str, parking: bool, f: impl FnOnce() -> T) -> Result<T, String> {
        let cur = self.current.lock().unwrap();
        match cur.as_ref() {
            Some(r) if parking && r.state().active() && r.devices.contains(device) => Err(r.flow_name.clone()),
            _ => Ok(f()),
        }
    }

    /// Abort the active run (a second abort also stops its cleanup). False: nothing running.
    pub fn abort(&self) -> bool {
        match self.current() {
            Some(r) if r.state().active() => {
                r.abort.send_modify(|n| *n = n.saturating_add(1));
                r.event("warn", "", if *r.abort.borrow() > 1 { "abort requested again: cleanup stopped" } else { "abort requested" });
                true
            }
            _ => false,
        }
    }

    /// Continue a run waiting at a `prompt`. False: no run is waiting.
    pub fn resume(&self) -> bool {
        match self.current() {
            Some(r) if r.state() == State::Waiting => {
                r.resume.notify_one();
                true
            }
            _ => false,
        }
    }
}

struct Exec {
    run: Arc<Run>,
    devices: Arc<Devices>,
    recordings: Arc<Recordings>,
    vocab: Vocab,
    vars: BTreeMap<String, Value>,
    started: Instant,
    deadline: Option<Instant>,
    /// When the run last changed an instrument (status reads must be newer).
    last_set: Option<Instant>,
    /// Energising setters the run switched on: put back to safe after a failure / abort.
    energized: BTreeSet<(Address, String)>,
    recording: Option<i64>,
    cleanup: bool,
}

type Step2<'a> = Pin<Box<dyn Future<Output = Result<(), Halt>> + Send + 'a>>;

/// Number from a registry / query value: SI dict → `val`, number → itself.
pub fn numeric(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Object(o) => match o.get("val") {
            Some(Value::String(s)) => s.trim().parse().ok(),
            Some(Value::Number(n)) => n.as_f64(),
            _ => None,
        },
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|x: &f64| x.is_finite())
}

/// Decimals of a value spec's `%.Nf` format.
fn decimals(spec: Option<&Value>) -> Option<i32> {
    let f = spec?.get("format")?.as_str()?;
    let n = f.strip_prefix("%.")?.strip_suffix('f')?;
    n.parse().ok()
}

/// Does a reported value equal the requested one? Numbers within half a unit of the last
/// decimal of the setter's format (else 1e-6 relative); words ignoring case; ON/1/true alike.
pub fn reported_equals(reported: &Value, requested: &Value, spec: Option<&Value>) -> bool {
    let truthy = |v: &Value| match v {
        Value::Bool(b) => Some(*b),
        Value::String(s) => match s.trim().to_ascii_uppercase().as_str() {
            "ON" | "1" | "TRUE" => Some(true),
            "OFF" | "0" | "FALSE" => Some(false),
            _ => None,
        },
        Value::Number(n) => n.as_f64().map(|x| x != 0.0),
        _ => None,
    };
    let kind = spec.and_then(|s| s["type"].as_str()).unwrap_or("");
    if kind == "number" || kind == "integer" || (requested.is_number() && kind != "enum" && kind != "bool") {
        let (Some(a), Some(b)) = (numeric(reported), numeric(requested)) else { return false };
        let tol = match decimals(spec) {
            Some(d) => 0.5 * 10f64.powi(-d) + 1e-9,
            None => (b.abs() * 1e-6).max(1e-9),
        };
        return (a - b).abs() <= tol;
    }
    if let (Value::String(a), Value::String(b)) = (reported, requested) {
        if a.trim().eq_ignore_ascii_case(b.trim()) {
            return true;
        }
    }
    matches!((truthy(reported), truthy(requested)), (Some(a), Some(b)) if a == b)
}

/// `{name}` → the variable's value.
fn interpolate(msg: &str, vars: &BTreeMap<String, Value>) -> String {
    let mut out = String::new();
    let mut rest = msg;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 1..];
        match tail.find('}') {
            Some(j) if vars.contains_key(&tail[..j]) => {
                out.push_str(&show(&vars[&tail[..j]]));
                rest = &tail[j + 1..];
            }
            _ => {
                out.push('{');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A value for a log line: numbers without float noise.
pub fn show(v: &Value) -> String {
    match v {
        Value::Number(n) => match n.as_f64() {
            Some(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
            Some(f) => {
                let s = format!("{:.6}", f);
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            }
            None => n.to_string(),
        },
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Text sent to the setter: numbers rounded to the spec's decimals, words as written.
fn wire_text(v: &Value, spec: Option<&Value>) -> String {
    match v {
        Value::Number(_) => {
            let x = numeric(v).unwrap_or(0.0);
            match decimals(spec) {
                Some(d) => format!("{:.*}", d.max(0) as usize, x),
                None => show(v),
            }
        }
        Value::Bool(b) => if *b { "ON" } else { "OFF" }.to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

impl Exec {
    fn vars_now(&mut self) -> &BTreeMap<String, Value> {
        self.vars.insert("elapsed".into(), json!(self.started.elapsed().as_secs_f64()));
        &self.vars
    }

    fn publish_vars(&self) {
        self.run.info.lock().unwrap().vars = self.vars.clone();
    }

    fn info(&self, path: &str, msg: impl Into<String>) {
        self.run.event("info", path, msg);
    }

    /// Abort / timeout check between steps (the cleanup only yields to a second abort).
    fn check(&self) -> Result<(), Halt> {
        let n = *self.run.abort.borrow();
        if (!self.cleanup && n >= 1) || n >= 2 {
            return Err(Halt::Abort);
        }
        if !self.cleanup && self.deadline.is_some_and(|d| Instant::now() >= d) {
            return Err(Halt::Fail("the flow's timeout_s was reached".into()));
        }
        Ok(())
    }

    /// Sleep, interrupted by abort (and the flow timeout).
    async fn sleep(&self, d: Duration) -> Result<(), Halt> {
        let mut rx = self.run.abort.subscribe();
        let until = Instant::now() + d;
        let until = match self.deadline {
            Some(dl) if !self.cleanup && dl < until => dl,
            _ => until,
        };
        loop {
            self.check()?;
            let now = Instant::now();
            if now >= until {
                return self.check();
            }
            tokio::select! {
                _ = tokio::time::sleep(until - now) => {}
                r = rx.changed() => if r.is_err() { return Err(Halt::Abort) },
            }
        }
    }

    fn op(&self, at: &Address, op: &str) -> Option<Op> {
        self.vocab.class(&at.device, &at.class).ok()?.operations.get(op).cloned()
    }

    fn eval_value(&mut self, v: &ValueSource) -> Result<Value, Halt> {
        match v {
            ValueSource::Literal(v) => Ok(v.clone()),
            ValueSource::Expr(e) => {
                let vars = self.vars_now().clone();
                e.eval(&vars).map_err(Halt::Fail)
            }
        }
    }

    async fn device_set(&self, at: &Address, op: &str, text: String) -> Result<(), String> {
        let (_, client, _) = self.devices.lookup(&at.device).ok_or_else(|| format!("device {} is not configured", at.device))?;
        let (class, op, ch) = (at.class.clone(), op.to_string(), at.channel);
        tokio::task::spawn_blocking(move || client.set(&class, &op, ch, &text)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
    }

    async fn device_get(&self, at: &Address, op: &str) -> Result<Value, String> {
        let (_, client, _) = self.devices.lookup(&at.device).ok_or_else(|| format!("device {} is not configured", at.device))?;
        let (class, op2, ch) = (at.class.clone(), op.to_string(), at.channel);
        match tokio::task::spawn_blocking(move || client.get(&class, &op2, ch)).await.map_err(|e| e.to_string())? {
            Ok(Output::Value(v)) => Ok(v),
            Ok(Output::Bytes(_)) => Err(format!("{op} returns binary data")),
            Err(e) => Err(e.to_string()),
        }
    }

    /// A status field updated after `since` (or within MAX_AGE when nothing was set yet).
    async fn fresh_field(&self, at: &Address, field: &str, since: Option<Instant>, timeout: Duration) -> Result<Value, String> {
        let until = Instant::now() + timeout;
        loop {
            {
                let reg = self.devices.registry.lock().unwrap();
                let d = reg.devices.get(&at.device);
                let at_time = d.filter(|d| d.connected).and_then(|d| d.status_at.get(&at.class)).and_then(|m| m.get(&at.channel)).copied();
                if let Some(t) = at_time {
                    let fresh = match since {
                        Some(s) => t > s,
                        None => t.elapsed() <= MAX_AGE,
                    };
                    if fresh {
                        if let Some(v) = d.and_then(|d| d.status.get(&at.class)).and_then(|m| m.get(&at.channel)).and_then(|s| s.get(field)) {
                            return Ok(v.clone());
                        }
                        return Err(format!("{} {} does not report {field}", at.device, at.class));
                    }
                }
                if d.is_none_or(|d| !d.connected) && Instant::now() >= until {
                    return Err(format!("{} is offline", at.device));
                }
            }
            if Instant::now() >= until {
                return Err(format!("no fresh {field} from {} within {} s", at.device, timeout.as_secs()));
            }
            if let Err(Halt::Abort) = self.sleep(Duration::from_millis(50)).await {
                return Err("aborted".into());
            }
        }
    }

    /// Wait until the instrument reports `want` for this setter.
    async fn confirm(&self, at: &Address, op: &str, o: &Op, want: &Value, sent: Instant) -> Result<bool, Halt> {
        let until = Instant::now() + CONFIRM_TIMEOUT;
        let field = op.to_ascii_uppercase();
        let has_field = self.vocab.class(&at.device, &at.class).is_ok_and(|c| c.fields.contains_key(&field));
        if !o.get && !has_field {
            return Ok(false);
        }
        let mut last = Value::Null;
        loop {
            self.check()?;
            let got = if o.get {
                self.device_get(at, op).await
            } else {
                self.fresh_field(at, &field, Some(sent), until.saturating_duration_since(Instant::now())).await
            };
            match got {
                Ok(v) if reported_equals(&v, want, o.value.as_ref()) => return Ok(true),
                Ok(v) => last = v,
                Err(e) if Instant::now() >= until => return Err(Halt::Fail(format!("{op} not confirmed: {e}"))),
                Err(_) => {}
            }
            if Instant::now() >= until {
                let shown = numeric(&last).map(|x| show(&json!(x))).unwrap_or_else(|| show(&last));
                return Err(Halt::Fail(format!("{op} not confirmed: the instrument reports {shown}, not {}", show(want))));
            }
            self.sleep(Duration::from_millis(250)).await?;
        }
    }

    async fn do_set(&mut self, path: &str, at: &Address, op: &str, value: &ValueSource) -> Result<(), Halt> {
        let o = self.op(at, op).ok_or_else(|| Halt::Fail(format!("{} {} has no setter {op}", at.device, at.class)))?;
        let v = self.eval_value(value)?;
        let text = wire_text(&v, o.value.as_ref());
        let want = if v.is_number() { json!(text.parse::<f64>().unwrap_or(0.0)) } else { json!(text) };
        self.check()?;
        self.device_set(at, op, text.clone()).await.map_err(|e| Halt::Fail(format!("{} {op} = {text}: {e}", at.device)))?;
        let sent = Instant::now();
        self.last_set = Some(sent);
        if o.energizes && !flow::is_safe_literal(&json!(text), o.safe_value().as_ref()) {
            self.energized.insert((at.clone(), op.to_string()));
        }
        let confirmed = self.confirm(at, op, &o, &want, sent).await?;
        let label = format!("{} {}{} {op} = {text}", at.device, at.class, if at.channel > 1 { format!(" CH{}", at.channel) } else { String::new() });
        self.info(path, if confirmed { label } else { format!("{label} (sent; no read-back)") });
        // the reading after a change must reflect it
        self.last_set = Some(Instant::now().max(sent));
        Ok(())
    }

    async fn do_read(&mut self, path: &str, at: &Address, source: &ReadSource, into: &str) -> Result<(), Halt> {
        self.check()?;
        let (raw, what) = match source {
            ReadSource::Field(f) => (self.fresh_field(at, f, self.last_set, FRESH_TIMEOUT).await, f.clone()),
            ReadSource::Op(o) => (self.device_get(at, o).await, o.clone()),
        };
        self.check()?;
        let raw = raw.map_err(|e| Halt::Fail(format!("read {} {what}: {e}", at.device)))?;
        let v = match numeric(&raw) {
            Some(x) if !raw.is_string() || raw.as_str().is_some_and(|s| s.trim().parse::<f64>().is_ok()) => json!(x),
            _ => match raw {
                Value::Object(_) | Value::Null => return Err(Halt::Fail(format!("{} {what} has no value right now (overload or not measured?)", at.device))),
                other => other,
            },
        };
        self.info(path, format!("{into} = {} ({} {what})", show(&v), at.device));
        self.vars.insert(into.to_string(), v);
        self.publish_vars();
        Ok(())
    }

    fn run_steps<'a>(&'a mut self, steps: &'a [Step]) -> Step2<'a> {
        Box::pin(async move {
            for s in steps {
                self.check()?;
                self.run.info.lock().unwrap().path = s.path.clone();
                self.step(s).await?;
            }
            Ok(())
        })
    }

    async fn step(&mut self, s: &Step) -> Result<(), Halt> {
        let p = s.path.as_str();
        match &s.kind {
            Kind::Set { at, op, value } => self.do_set(p, at, op, value).await,
            Kind::Read { at, source, into } => self.do_read(p, at, source, into).await,
            Kind::Wait { seconds } => {
                let v = self.eval_value(seconds)?;
                let secs = numeric(&v).filter(|x| *x >= 0.0).ok_or_else(|| Halt::Fail(format!("wait: {} is not a number of seconds", show(&v))))?;
                self.sleep(Duration::from_secs_f64(secs.min(86_400.0))).await
            }
            Kind::Let { var, expr } => {
                let vars = self.vars_now().clone();
                let v = expr.eval(&vars).map_err(Halt::Fail)?;
                self.vars.insert(var.clone(), v);
                self.publish_vars();
                Ok(())
            }
            Kind::If { condition } => {
                let vars = self.vars_now().clone();
                if condition.truth(&vars).map_err(Halt::Fail)? {
                    self.run_steps(&s.body).await
                } else {
                    self.run_steps(&s.otherwise).await
                }
            }
            Kind::Repeat { bound, max_iterations } => {
                let outer = self.vars.get("iteration").cloned();
                let mut n = 0u64;
                let result = loop {
                    let limit = match bound {
                        RepeatBound::Times(t) => *t.min(max_iterations),
                        _ => *max_iterations,
                    };
                    if n >= limit {
                        if !matches!(bound, RepeatBound::Times(t) if *t <= *max_iterations) {
                            self.run.event("warn", p, format!("loop stopped after max_iterations ({limit})"));
                        }
                        break Ok(());
                    }
                    n += 1;
                    self.vars.insert("iteration".into(), json!(n));
                    if let RepeatBound::While(c) = bound {
                        let vars = self.vars_now().clone();
                        if !c.truth(&vars).map_err(Halt::Fail)? {
                            break Ok(());
                        }
                    }
                    let pass = Instant::now();
                    if let Err(h) = self.run_steps(&s.body).await {
                        break Err(h);
                    }
                    if let RepeatBound::Until(c) = bound {
                        let vars = self.vars_now().clone();
                        if c.truth(&vars).map_err(Halt::Fail)? {
                            break Ok(());
                        }
                    }
                    let spent = pass.elapsed();
                    if spent < MIN_PASS {
                        self.sleep(MIN_PASS - spent).await?;
                    }
                };
                match outer {
                    Some(v) => self.vars.insert("iteration".into(), v),
                    None => self.vars.remove("iteration"),
                };
                result
            }
            Kind::Sweep { var, values } => {
                for v in values {
                    self.vars.insert(var.clone(), v.clone());
                    self.publish_vars();
                    self.run_steps(&s.body).await?;
                }
                Ok(())
            }
            Kind::Check { condition, message, stop } => {
                let vars = self.vars_now().clone();
                if condition.truth(&vars).map_err(Halt::Fail)? {
                    return Ok(());
                }
                let msg = interpolate(message, &vars);
                if *stop {
                    Err(Halt::Fail(msg))
                } else {
                    self.run.event("warn", p, msg);
                    Ok(())
                }
            }
            Kind::Log { message } => {
                let vars = self.vars_now().clone();
                self.info(p, interpolate(message, &vars));
                Ok(())
            }
            Kind::Prompt { message } => {
                let vars = self.vars_now().clone();
                let msg = interpolate(message, &vars);
                self.run.event("info", p, format!("waiting: {msg}"));
                {
                    let mut i = self.run.info.lock().unwrap();
                    i.prompt = Some(msg);
                    i.state = State::Waiting;
                }
                let mut rx = self.run.abort.subscribe();
                let r = loop {
                    if let Err(h) = self.check() {
                        break Err(h);
                    }
                    tokio::select! {
                        _ = self.run.resume.notified() => break Ok(()),
                        r = rx.changed() => if r.is_err() { break Err(Halt::Abort) },
                    }
                };
                {
                    let mut i = self.run.info.lock().unwrap();
                    i.prompt = None;
                    i.state = if self.cleanup { State::Cleanup } else { State::Running };
                }
                if r.is_ok() {
                    self.info(p, "continued");
                }
                r
            }
            Kind::RecordStart { name, interval_s, series } => {
                if let Some(id) = self.recording.take() {
                    let _ = self.recordings.stop_id(id).await;
                }
                let name = if name.trim().is_empty() { self.run.flow_name.clone() } else { name.trim().to_string() };
                let name = format!("{name} · {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
                let series: Vec<Map<String, Value>> = series
                    .iter()
                    .map(|x| {
                        let mut m = Map::new();
                        m.insert("device_id".into(), json!(x.device));
                        m.insert("class".into(), json!(x.class));
                        m.insert("channel".into(), json!(x.channel));
                        m.insert("field".into(), json!(x.field));
                        if let Some(l) = &x.label {
                            m.insert("label".into(), json!(l));
                        }
                        m
                    })
                    .collect();
                let id = self.recordings.start_with(&name, *interval_s, &series).await.map_err(|e| Halt::Fail(format!("record: {e}")))?;
                self.recording = Some(id);
                self.run.info.lock().unwrap().recording_id = Some(id);
                self.info(p, format!("recording \"{name}\" started"));
                Ok(())
            }
            Kind::RecordStop => {
                if let Some(id) = self.recording.take() {
                    self.recordings.stop_id(id).await.map_err(|e| Halt::Fail(format!("record: {e}")))?;
                    self.info(p, "recording stopped");
                }
                Ok(())
            }
            Kind::Stop { message } => {
                if let Some(m) = message {
                    let vars = self.vars_now().clone();
                    self.info(p, interpolate(m, &vars));
                }
                Err(Halt::Stop)
            }
        }
    }

    /// Run the cleanup steps; a failing step is logged and the next one still runs.
    async fn run_cleanup(&mut self, steps: &[Step]) {
        for s in steps {
            if *self.run.abort.borrow() >= 2 {
                return;
            }
            self.run.info.lock().unwrap().path = s.path.clone();
            match self.step(s).await {
                Ok(()) | Err(Halt::Stop) => {}
                Err(Halt::Abort) => return,
                Err(Halt::Fail(m)) => self.run.event("error", &s.path, format!("cleanup: {m}")),
            }
        }
    }

    /// Outputs the run switched on → safe value (after a failure or abort).
    async fn safe_state(&mut self) {
        for (at, op) in std::mem::take(&mut self.energized) {
            let Some(safe) = self.op(&at, &op).and_then(|o| o.safe_value()) else {
                self.run.event("warn", "", format!("{} {op}: no safe value known - check the instrument", at.device));
                continue;
            };
            let text = wire_text(&safe, None);
            match self.device_set(&at, &op, text.clone()).await {
                Ok(()) => self.run.event("warn", "", format!("safe state: {} {} {op} = {text}", at.device, at.class)),
                Err(e) => self.run.event("error", "", format!("safe state: {} {op} = {text} failed: {e}", at.device)),
            }
        }
    }

    async fn execute(mut self, flow: Flow) {
        self.info("", format!("run of \"{}\" started", flow.name));
        let mut outcome = self.run_steps(&flow.setup).await;
        if outcome.is_ok() {
            outcome = self.run_steps(&flow.steps).await;
        }
        let (state, error) = match outcome {
            Ok(()) | Err(Halt::Stop) => (State::Finished, None),
            Err(Halt::Abort) => (State::Aborted, Some("aborted".to_string())),
            Err(Halt::Fail(m)) => (State::Failed, Some(m)),
        };
        if let Some(e) = &error {
            let path = self.run.info.lock().unwrap().path.clone();
            self.run.event(if state == State::Aborted { "warn" } else { "error" }, &path, e.clone());
        }
        self.cleanup = true;
        if !flow.cleanup.is_empty() {
            self.run.set_state(State::Cleanup);
            self.info("cleanup", "cleanup");
            self.run_cleanup(&flow.cleanup).await;
        }
        if state != State::Finished {
            self.safe_state().await;
        }
        if let Some(id) = self.recording.take() {
            let _ = self.recordings.stop_id(id).await;
        }
        self.publish_vars();
        let mut i = self.run.info.lock().unwrap();
        i.state = state;
        i.error = error;
        i.ended_at = Some(now_ms());
        i.path = String::new();
        drop(i);
        self.run.event(if state == State::Finished { "info" } else { "warn" }, "", format!("run {}", state.as_str()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_respects_the_setters_resolution() {
        let spec = json!({"type": "number", "format": "%.2f"});
        assert!(reported_equals(&json!({"si": {}, "val": "+4.9900E+00"}), &json!(4.99), Some(&spec)));
        assert!(reported_equals(&json!("5.00"), &json!(5.0), Some(&spec)));
        assert!(!reported_equals(&json!(5.02), &json!(5.0), Some(&spec)));
        let onoff = json!({"type": "enum", "values": ["ON", "OFF"]});
        assert!(reported_equals(&json!("on"), &json!("ON"), Some(&onoff)));
        assert!(reported_equals(&json!(true), &json!("ON"), Some(&onoff)));
        assert!(!reported_equals(&json!("OFF"), &json!("ON"), Some(&onoff)));
        assert!(reported_equals(&json!(1.0000001), &json!(1.0), None));
    }

    #[test]
    fn values_for_the_wire_and_the_log() {
        let spec = json!({"type": "number", "format": "%.3f"});
        assert_eq!(wire_text(&json!(0.30000000000000004), Some(&spec)), "0.300");
        assert_eq!(wire_text(&json!(2), None), "2");
        assert_eq!(wire_text(&json!(true), None), "ON");
        assert_eq!(show(&json!(4.990000001)), "4.99");
        let vars = BTreeMap::from([("v".to_string(), json!(4.5)), ("n".to_string(), json!(3))]);
        assert_eq!(interpolate("pass {n}: {v} V {unknown} {", &vars), "pass 3: 4.5 V {unknown} {");
    }
}
