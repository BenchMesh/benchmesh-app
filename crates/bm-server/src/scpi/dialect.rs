//! The BenchMesh SCPI dialect (contract `ai_context/SCPI.md`): a program message is split into
//! commands, each command resolved against one instrument's vocabulary, and values / errors are
//! formatted the way SCPI clients (VISA, MATLAB, LabVIEW) read them. Pure: no I/O.

use serde_json::Value;

use crate::automation::exec::numeric;
use crate::automation::vocab::Device;

/// Longest program message accepted (a longer one closes the connection).
pub const MAX_LINE: usize = 64 * 1024;
/// Error queue length (SCPI-99 21.8: the last entry becomes -350 on overflow).
pub const MAX_ERRORS: usize = 20;

#[derive(Debug, Clone, PartialEq)]
pub struct ScpiError {
    pub code: i32,
    pub message: String,
}

impl ScpiError {
    pub fn new(code: i32, message: impl Into<String>) -> ScpiError {
        ScpiError { code, message: message.into() }
    }

    /// `-113,"Undefined header; ..."` as `SYSTem:ERRor?` answers it.
    pub fn reply(&self) -> String {
        format!("{},\"{}\"", self.code, self.message.replace('"', "'"))
    }
}

fn undefined(what: impl Into<String>) -> ScpiError {
    ScpiError::new(-113, format!("Undefined header; {}", what.into()))
}

/// One command of a program message: header tokens (upper case, without a leading `:`),
/// whether it is a query, and its argument text.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub header: Vec<String>,
    pub query: bool,
    pub arg: Option<String>,
}

/// What a command asks for on this instrument.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Idn,
    /// `*CLS`: clear the error queue.
    Clear,
    /// `*OPC?`: commands run in order, so everything before it is complete: `1`.
    OpcQuery,
    /// `*OPC`, `*WAI`: nothing to wait for.
    NoOp,
    SystemError,
    SystemVersion,
    Headers,
    /// Run the definition's query operation.
    Query { class: String, channel: u8, op: String },
    /// Run the definition's setter.
    Set { class: String, channel: u8, op: String, value: String },
    /// The latest polled status field (nothing sent to the instrument).
    Fetch { class: String, channel: u8, field: String },
}

/// A browser can be made to send an HTTP request to any local port; its body would otherwise
/// reach the instrument as commands (cross-protocol request). Such a connection is closed.
pub fn looks_like_http(line: &str) -> bool {
    let mut parts = line.split_ascii_whitespace();
    let method = parts.next().unwrap_or("");
    let is_method = ["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH", "CONNECT", "TRACE"].contains(&method);
    (is_method && parts.next().is_some() && parts.next().is_some_and(|v| v.starts_with("HTTP/")))
        || line.to_ascii_lowercase().starts_with("host:")
        || line.to_ascii_lowercase().starts_with("origin:")
}

/// Split a program message at `;` outside quotes.
pub fn split_message(line: &str) -> Vec<&str> {
    let (mut out, mut start, mut quote) = (Vec::new(), 0, None::<char>);
    for (i, c) in line.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            (None, ';') => {
                out.push(&line[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&line[start..]);
    out.into_iter().map(str::trim).filter(|s| !s.is_empty()).collect()
}

/// Parse one command (`PSU:CH2:VOLTAGE 5`, `fetch:vout?`, `*IDN?`).
pub fn parse(unit: &str) -> Result<Command, ScpiError> {
    let unit = unit.trim();
    let (head, arg) = match unit.find(|c: char| c.is_ascii_whitespace()) {
        Some(i) => (&unit[..i], Some(unquote(unit[i..].trim()))),
        None => (unit, None),
    };
    let (head, query) = match head.strip_suffix('?') {
        Some(h) => (h, true),
        None => (head, false),
    };
    let head = head.strip_prefix(':').unwrap_or(head);
    if head.is_empty() {
        return Err(ScpiError::new(-102, "Syntax error; empty header"));
    }
    let header: Vec<String> = head.split(':').map(|t| t.to_ascii_uppercase()).collect();
    if header.iter().any(|t| t.is_empty() || !t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '*')) {
        return Err(ScpiError::new(-102, format!("Syntax error; {head}")));
    }
    if query && arg.is_some() && !header[0].starts_with('*') {
        return Err(ScpiError::new(-108, "Parameter not allowed; a query takes no value"));
    }
    Ok(Command { header, query, arg: arg.filter(|a| !a.is_empty()) })
}

fn unquote(s: &str) -> String {
    for q in ['"', '\''] {
        if let Some(inner) = s.strip_prefix(q).and_then(|r| r.strip_suffix(q)) {
            return inner.to_string();
        }
    }
    s.to_string()
}

/// `CH2`, `CHAN2`, `CHANNEL2` → 2.
fn channel_token(t: &str) -> Option<u8> {
    let digits = ["CHANNEL", "CHAN", "CH"].iter().find_map(|p| t.strip_prefix(p))?;
    digits.parse().ok().filter(|c| (1..=9).contains(c))
}

fn is_word(t: &str, short: &str, long: &str) -> bool {
    t == short || t == long
}

/// What `cmd` means for `dev`.
pub fn resolve(dev: &Device, cmd: &Command) -> Result<Target, ScpiError> {
    let h: Vec<&str> = cmd.header.iter().map(String::as_str).collect();
    if h[0].starts_with('*') {
        return match (h.as_slice(), cmd.query) {
            (["*IDN"], true) => Ok(Target::Idn),
            (["*CLS"], false) => Ok(Target::Clear),
            (["*OPC"], true) => Ok(Target::OpcQuery),
            (["*OPC"] | ["*WAI"], false) => Ok(Target::NoOp),
            _ => Err(undefined(format!("{}{} is not supported", h.join(":"), if cmd.query { "?" } else { "" }))),
        };
    }
    if is_word(h[0], "SYST", "SYSTEM") {
        return match (&h[1..], cmd.query) {
            ([e] | [e, _], true) if is_word(e, "ERR", "ERROR") && h.get(2).is_none_or(|n| *n == "NEXT") => Ok(Target::SystemError),
            ([v], true) if is_word(v, "VERS", "VERSION") => Ok(Target::SystemVersion),
            ([help, heads], true) if *help == "HELP" && is_word(heads, "HEAD", "HEADERS") => Ok(Target::Headers),
            _ => Err(undefined(h.join(":"))),
        };
    }
    let mut rest = &h[..];
    let mut class = None;
    if let Some(c) = dev.classes.keys().find(|c| c.as_str() == rest[0]) {
        class = Some(c.clone());
        rest = &rest[1..];
    }
    let mut channel = 1;
    if let Some(ch) = rest.first().and_then(|t| channel_token(t)) {
        channel = ch;
        rest = &rest[1..];
    }
    let fetch = rest.first().is_some_and(|t| is_word(t, "FETC", "FETCH"));
    if fetch {
        rest = &rest[1..];
    }
    let [name] = rest else { return Err(undefined(h.join(":"))) };
    let name = name.to_ascii_lowercase();
    // the class: given, the only one, or the only class that knows the name
    let class = match class {
        Some(c) => c,
        None => {
            let knows = |c: &&String| {
                let k = &dev.classes[*c];
                if fetch { k.fields.keys().any(|f| f.eq_ignore_ascii_case(&name)) } else { k.operations.contains_key(&name) }
            };
            let all: Vec<&String> = dev.classes.keys().collect();
            let found: Vec<&String> = if all.len() == 1 { all } else { dev.classes.keys().filter(knows).collect() };
            match found.as_slice() {
                [c] => (*c).clone(),
                [] => return Err(undefined(format!("{} has no {}", dev.id, h.join(":")))),
                many => {
                    let names: Vec<&str> = many.iter().map(|c| c.as_str()).collect();
                    return Err(undefined(format!("give the class: {}:{}", names.join(" or "), h.join(":"))));
                }
            }
        }
    };
    let k = &dev.classes[&class];
    if channel > k.channels {
        return Err(ScpiError::new(-114, format!("Header suffix out of range; {class} has {} channel(s)", k.channels)));
    }
    if fetch {
        if !cmd.query {
            return Err(undefined("FETCh is a query: FETCh:<field>?"));
        }
        let Some(field) = k.fields.keys().find(|f| f.eq_ignore_ascii_case(&name)) else {
            return Err(undefined(format!("{class} reports no field {}", name.to_ascii_uppercase())));
        };
        return Ok(Target::Fetch { class, channel, field: field.clone() });
    }
    let Some(op) = k.operations.get(&name) else { return Err(undefined(format!("{class} has no operation {}", name.to_ascii_uppercase()))) };
    match (cmd.query, &cmd.arg) {
        (true, _) if op.get => Ok(Target::Query { class, channel, op: name }),
        (true, _) => Err(undefined(format!("{} cannot be queried", name.to_ascii_uppercase()))),
        (false, Some(v)) if op.set => Ok(Target::Set { class, channel, op: name, value: v.clone() }),
        (false, None) if op.set => Err(ScpiError::new(-109, format!("Missing parameter; {} <value>", name.to_ascii_uppercase()))),
        (false, _) => Err(undefined(format!("{} cannot be set", name.to_ascii_uppercase()))),
    }
}

/// Every header the instrument understands, for `SYSTem:HELP:HEADers?` (one line).
pub fn headers(dev: &Device) -> String {
    let mut out = vec!["*IDN?".to_string(), "*CLS".into(), "*OPC?".into(), "SYSTem:ERRor?".into(), "SYSTem:HELP:HEADers?".into()];
    for (class, k) in &dev.classes {
        let ch = if k.channels > 1 { ":CH<n>" } else { "" };
        for (name, op) in &k.operations {
            let n = name.to_ascii_uppercase();
            if op.get {
                out.push(format!("{class}{ch}:{n}?"));
            }
            if op.set {
                out.push(format!("{class}{ch}:{n} <value>"));
            }
        }
        for f in k.fields.keys() {
            out.push(format!("{class}{ch}:FETCh:{}?", f.to_ascii_uppercase()));
        }
    }
    out.join(",")
}

/// A number the way SCPI clients parse it (shortest form; NR3 for very large or small values).
pub fn number(x: f64) -> String {
    let a = x.abs();
    if a != 0.0 && !(1e-4..1e9).contains(&a) {
        format!("{x:E}")
    } else {
        format!("{x}")
    }
}

/// A reply value: SI values and numbers as numbers in base units, booleans 1/0, text as is.
pub fn value(v: &Value) -> String {
    match v {
        Value::Bool(b) => if *b { "1" } else { "0" }.into(),
        Value::String(s) => s.trim().to_string(),
        Value::Null => String::new(),
        other => match numeric(other) {
            Some(x) => number(x),
            None => other.to_string(),
        },
    }
}

/// Binary data as an IEEE 488.2 definite-length block: `#<digits><length><bytes>`.
pub fn block(bytes: &[u8]) -> Vec<u8> {
    let len = bytes.len().to_string();
    let mut out = format!("#{}{len}", len.len()).into_bytes();
    out.extend_from_slice(bytes);
    out
}

/// A driver error as a SCPI error.
pub fn driver_error(e: &bm_driver::Error) -> ScpiError {
    use bm_driver::Error as E;
    let (code, what) = match e {
        E::BadArgument(m) if m.contains("outside") => (-222, "Data out of range"),
        E::BadArgument(_) => (-224, "Illegal parameter value"),
        E::NotFound(_) => (-113, "Undefined header"),
        E::Refused(_) => (-221, "Settings conflict"),
        E::Io(_) | E::UnknownModel(_) | E::WrongInstrument(_) => (-241, "Hardware missing"),
        E::Timeout { .. } => (-240, "Hardware error"),
        E::Framing(_) | E::Parse(_) => (-230, "Data corrupt or stale"),
        E::Expr(_) | E::Definition(_) => (-200, "Execution error"),
    };
    let detail = match e {
        E::BadArgument(m) | E::NotFound(m) | E::Refused(m) => m.clone(),
        other => other.to_string(),
    };
    ScpiError::new(code, format!("{what}; {detail}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::vocab::{Class, Field, Op};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn op(get: bool, set: bool) -> Op {
        Op { get, set, value: None, energizes: false, description: None }
    }

    fn field() -> Field {
        Field { kind: "si".into(), values: None, description: None }
    }

    /// A two-class instrument (like the OWON SPM: PSU with two channels + DMM).
    fn dev() -> Device {
        let psu = Class {
            channels: 2,
            fields: BTreeMap::from([("VOUT".into(), field()), ("OUTPUT".into(), field())]),
            operations: BTreeMap::from([("voltage".into(), op(true, true)), ("output".into(), op(false, true)), ("mode".into(), op(true, true))]),
        };
        let dmm = Class {
            channels: 1,
            fields: BTreeMap::from([("MEAS".into(), field())]),
            operations: BTreeMap::from([("measurement".into(), op(true, false)), ("mode".into(), op(false, true))]),
        };
        Device {
            id: "spm".into(),
            name: "SPM".into(),
            vendor: "OWON".into(),
            model: Some("SPM3103".into()),
            online: true,
            parked: false,
            classes: BTreeMap::from([("PSU".into(), psu), ("DMM".into(), dmm)]),
        }
    }

    fn target(s: &str) -> Result<Target, ScpiError> {
        resolve(&dev(), &parse(s)?)
    }

    #[test]
    fn messages_split_at_semicolons_outside_quotes() {
        assert_eq!(split_message("VOLT 5; OUTPUT ON;;*OPC?"), vec!["VOLT 5", "OUTPUT ON", "*OPC?"]);
        assert_eq!(split_message("MODE \"A;B\";X?"), vec!["MODE \"A;B\"", "X?"]);
        assert!(split_message("  ").is_empty());
    }

    #[test]
    fn commands_parse_case_insensitively_with_values() {
        let c = parse(":psu:ch2:voltage 5.25").unwrap();
        assert_eq!(c, Command { header: vec!["PSU".into(), "CH2".into(), "VOLTAGE".into()], query: false, arg: Some("5.25".into()) });
        let c = parse("fetch:vout?").unwrap();
        assert!(c.query && c.arg.is_none());
        assert_eq!(parse("MODE 'VOLT DC'").unwrap().arg.as_deref(), Some("VOLT DC"));
        assert_eq!(parse("VOLT? 5").unwrap_err().code, -108);
        assert_eq!(parse("VO$LT 5").unwrap_err().code, -102);
        assert_eq!(parse(":").unwrap_err().code, -102);
    }

    #[test]
    fn the_class_is_given_the_only_one_or_the_only_one_that_knows_the_name() {
        assert_eq!(target("VOLTAGE?").unwrap(), Target::Query { class: "PSU".into(), channel: 1, op: "voltage".into() });
        assert_eq!(target("MEASUREMENT?").unwrap(), Target::Query { class: "DMM".into(), channel: 1, op: "measurement".into() });
        assert_eq!(target("FETCH:MEAS?").unwrap(), Target::Fetch { class: "DMM".into(), channel: 1, field: "MEAS".into() });
        let e = target("MODE VOLT").unwrap_err();
        assert_eq!(e.code, -113);
        assert!(e.message.contains("DMM or PSU"), "{e:?}");
        assert_eq!(target("DMM:MODE VOLT").unwrap(), Target::Set { class: "DMM".into(), channel: 1, op: "mode".into(), value: "VOLT".into() });
    }

    #[test]
    fn channels_queries_setters_and_fields_are_checked() {
        assert_eq!(target("PSU:CH2:OUTPUT ON").unwrap(), Target::Set { class: "PSU".into(), channel: 2, op: "output".into(), value: "ON".into() });
        assert_eq!(target("CHAN2:FETC:vout?").unwrap(), Target::Fetch { class: "PSU".into(), channel: 2, field: "VOUT".into() });
        assert_eq!(target("PSU:CH3:VOLTAGE?").unwrap_err().code, -114);
        assert_eq!(target("OUTPUT?").unwrap_err().code, -113, "output has no query");
        assert_eq!(target("MEASUREMENT 5").unwrap_err().code, -113, "measurement has no setter");
        assert_eq!(target("VOLTAGE").unwrap_err().code, -109);
        assert_eq!(target("FETCH:NOPE?").unwrap_err().code, -113);
        assert_eq!(target("FETCH:VOUT").unwrap_err().code, -113);
        assert_eq!(target("NOPE?").unwrap_err().code, -113);
    }

    #[test]
    fn common_and_system_commands() {
        assert_eq!(target("*idn?").unwrap(), Target::Idn);
        assert_eq!(target("*CLS").unwrap(), Target::Clear);
        assert_eq!(target("*OPC?").unwrap(), Target::OpcQuery);
        assert_eq!(target("*WAI").unwrap(), Target::NoOp);
        assert_eq!(target("*RST").unwrap_err().code, -113);
        assert_eq!(target("SYST:ERR?").unwrap(), Target::SystemError);
        assert_eq!(target("SYSTEM:ERROR:NEXT?").unwrap(), Target::SystemError);
        assert_eq!(target("SYST:VERS?").unwrap(), Target::SystemVersion);
        assert_eq!(target("SYST:HELP:HEAD?").unwrap(), Target::Headers);
    }

    #[test]
    fn headers_list_every_operation_and_field() {
        let h = headers(&dev());
        for want in ["*IDN?", "DMM:MEASUREMENT?", "DMM:FETCh:MEAS?", "PSU:CH<n>:VOLTAGE?", "PSU:CH<n>:VOLTAGE <value>", "PSU:CH<n>:OUTPUT <value>"] {
            assert!(h.split(',').any(|x| x == want), "{want} missing in {h}");
        }
        assert!(!h.contains("PSU:CH<n>:OUTPUT?"));
    }

    #[test]
    fn values_are_formatted_for_scpi_clients() {
        assert_eq!(value(&json!({"si": {"number": "4.94"}, "sci": "4.94000E+00", "val": "4.94"})), "4.94");
        assert_eq!(value(&json!({"val": "0.000001"})), "1E-6");
        assert_eq!(value(&json!(12)), "12");
        assert_eq!(value(&json!(" ON ")), "ON");
        assert_eq!(value(&json!(true)), "1");
        assert_eq!(value(&Value::Null), "");
        assert_eq!(number(2.5e9), "2.5E9");
        assert_eq!(block(b"abc"), b"#13abc".to_vec());
        assert_eq!(block(&[0u8; 12]).len(), 2 + 2 + 12);
    }

    #[test]
    fn http_requests_are_recognised() {
        assert!(looks_like_http("POST / HTTP/1.1"));
        assert!(looks_like_http("GET /x?y HTTP/1.0"));
        assert!(looks_like_http("Host: 127.0.0.1:5025"));
        assert!(!looks_like_http("VOLTAGE 5"));
        assert!(!looks_like_http("GET"));
    }

    #[test]
    fn errors_quote_safely() {
        assert_eq!(ScpiError::new(-221, "Settings conflict; say \"hi\"").reply(), "-221,\"Settings conflict; say 'hi'\"");
        let e = driver_error(&bm_driver::Error::BadArgument("5 is outside 0..3".into()));
        assert_eq!(e.code, -222);
        assert_eq!(driver_error(&bm_driver::Error::Refused("output on".into())).code, -221);
    }
}
