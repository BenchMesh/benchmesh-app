//! One SCPI client connection to one instrument: reads program messages line by line, runs
//! each command through the same device thread as the REST API and answers the queries.

use std::collections::VecDeque;

use bm_driver::engine::Output;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

use super::dialect::{self, ScpiError, Target, MAX_ERRORS, MAX_LINE};
use super::Access;
use crate::automation::vocab::Device;
use crate::recordings::MAX_AGE;
use crate::S;

const READ_ONLY: &str = "Command protected; SCPI access is read only - allow control in BenchMesh: Instruments - Connectors";

struct Session {
    s: S,
    device: String,
    errors: VecDeque<ScpiError>,
}

impl Session {
    fn push(&mut self, e: ScpiError) {
        if self.errors.len() >= MAX_ERRORS {
            self.errors.pop_back();
            self.errors.push_back(ScpiError::new(-350, "Queue overflow"));
        } else {
            self.errors.push_back(e);
        }
    }

    /// Run one resolved command; `Some` reply for queries.
    async fn run(&mut self, dev: &Device, t: Target) -> Result<Option<Vec<u8>>, ScpiError> {
        let text = |s: String| Ok(Some(s.into_bytes()));
        match t {
            Target::Idn => {
                let model = dev.model.clone().unwrap_or_else(|| dev.name.clone());
                let clean = |s: &str| s.replace([',', '\n', '\r'], " ");
                text(format!("BenchMesh,{},{},{}", clean(&model), clean(&dev.id), env!("CARGO_PKG_VERSION")))
            }
            Target::Clear => {
                self.errors.clear();
                Ok(None)
            }
            Target::OpcQuery => text("1".into()),
            Target::NoOp => Ok(None),
            Target::SystemError => text(self.errors.pop_front().map_or_else(|| "0,\"No error\"".into(), |e| e.reply())),
            Target::SystemVersion => text("1999.0".into()),
            Target::Headers => text(dialect::headers(dev)),
            Target::Fetch { class, channel, field } => {
                let reg = self.s.devices.registry.lock().unwrap();
                match reg.fresh_value(&self.device, &class, channel, &field, MAX_AGE) {
                    Some(v) => text(dialect::value(v)),
                    None => Err(ScpiError::new(-230, format!("Data corrupt or stale; no fresh {field} from {} (offline?)", self.device))),
                }
            }
            Target::Query { class, channel, op } => {
                let (_, client, _) = self.s.devices.lookup(&self.device).ok_or_else(|| ScpiError::new(-241, "Hardware missing; instrument removed"))?;
                match tokio::task::spawn_blocking(move || client.get(&class, &op, channel)).await {
                    Ok(Ok(Output::Value(v))) => text(dialect::value(&v)),
                    Ok(Ok(Output::Bytes(b))) => Ok(Some(dialect::block(&b))),
                    Ok(Err(e)) => Err(dialect::driver_error(&e)),
                    Err(e) => Err(ScpiError::new(-200, format!("Execution error; {e}"))),
                }
            }
            Target::Set { class, channel, op, value } => {
                if self.s.scpi.access() != Access::Control {
                    return Err(ScpiError::new(-203, READ_ONLY));
                }
                let id = self.device.clone();
                if let Err(flow) = self.s.automation.runner.unless_used(&id, true, || ()) {
                    return Err(ScpiError::new(-221, format!("Settings conflict; {id} is used by the running flow {flow:?}")));
                }
                let (_, client, _) = self.s.devices.lookup(&id).ok_or_else(|| ScpiError::new(-241, "Hardware missing; instrument removed"))?;
                match tokio::task::spawn_blocking(move || client.set(&class, &op, channel, &value)).await {
                    Ok(Ok(())) => Ok(None),
                    Ok(Err(e)) => Err(dialect::driver_error(&e)),
                    Err(e) => Err(ScpiError::new(-200, format!("Execution error; {e}"))),
                }
            }
        }
    }

    /// One program message → the reply line (queries' answers joined by `;`), if any query.
    async fn message(&mut self, line: &str) -> Option<Vec<u8>> {
        let dev = self.s.automation.vocab().device(&self.device).cloned();
        let mut replies: Vec<Vec<u8>> = Vec::new();
        for unit in dialect::split_message(line) {
            let cmd = match dialect::parse(unit) {
                Ok(c) => c,
                Err(e) => {
                    self.push(e);
                    continue;
                }
            };
            let query = cmd.query;
            let result = match &dev {
                Some(d) => match dialect::resolve(d, &cmd) {
                    Ok(t) => self.run(d, t).await,
                    Err(e) => Err(e),
                },
                None => Err(ScpiError::new(-241, "Hardware missing; instrument removed from the configuration")),
            };
            match result {
                Ok(Some(r)) => replies.push(r),
                Ok(None) => {}
                Err(e) => {
                    self.push(e);
                    // a failed query answers an empty field instead of leaving the client waiting
                    if query {
                        replies.push(Vec::new());
                    }
                }
            }
        }
        (!replies.is_empty()).then(|| {
            let mut out = replies.join(&b';');
            out.push(b'\n');
            out
        })
    }
}

/// Serve one connection until the client closes it, sends HTTP, or access is switched off.
pub async fn serve<IO: AsyncRead + AsyncWrite + Unpin>(s: S, device: String, io: IO) {
    let (rd, mut wr) = tokio::io::split(io);
    let mut rd = BufReader::new(rd);
    let mut session = Session { s, device, errors: VecDeque::new() };
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match (&mut rd).take(MAX_LINE as u64 + 1).read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if buf.len() > MAX_LINE {
            eprintln!("scpi {}: program message longer than {MAX_LINE} bytes, connection closed", session.device);
            return;
        }
        let line: String = buf.iter().map(|&b| b as char).collect();
        let line = line.trim_end_matches(['\n', '\r']);
        if dialect::looks_like_http(line) {
            eprintln!("scpi {}: an HTTP request (a web page?) was refused and the connection closed", session.device);
            return;
        }
        if session.s.scpi.access() == Access::Off {
            return;
        }
        if let Some(reply) = session.message(line).await {
            if wr.write_all(&reply).await.and(wr.flush().await).is_err() {
                return;
            }
        }
    }
}

