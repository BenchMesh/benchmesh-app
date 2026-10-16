//! Reply parse pipeline (`commands.<name>.parse`).

use bm_devicedef::{Command, OnEmpty, ParseStep};
use regex::Regex;
use serde_json::{json, Map, Value};

use crate::framing::preview;
use crate::Error;

/// Working value: raw bytes until a step needs text or a number.
enum Cur {
    Bytes(Vec<u8>),
    Val(Value),
}

impl Cur {
    fn text(self, step: &str) -> Result<String, Error> {
        match self {
            // latin-1: every byte maps to one char, nothing is lost
            Cur::Bytes(b) => Ok(b.iter().map(|&c| c as char).collect()),
            Cur::Val(Value::String(s)) => Ok(s),
            Cur::Val(Value::Number(n)) => Ok(n.to_string()),
            Cur::Val(v) => Err(Error::Parse(format!("`{step}` needs text, got {v}"))),
        }
    }
}

/// Lenient numeric parse shared with the CEL `num()` function: trims, accepts `+1.0E-04`,
/// `05.00`, `+0.0000E-308`; rejects tokens like `OL`.
pub fn lenient_number(s: &str) -> Option<f64> {
    let t = s.trim();
    let n: f64 = t.parse().ok()?;
    n.is_finite().then_some(n)
}

/// Parse one framed reply with the command's pipeline.
pub fn parse_reply(cmd: &Command, reply: &[u8]) -> Result<Value, Error> {
    if reply.iter().all(|b| b.is_ascii_whitespace()) && !matches!(cmd.parse.first(), Some(ParseStep::Byte)) {
        return match cmd.on_empty {
            OnEmpty::Null => Ok(Value::Null),
            OnEmpty::Error => Err(Error::Parse("empty reply".into())),
        };
    }
    if reply.is_empty() {
        return Err(Error::Parse("empty reply".into()));
    }
    let mut cur = Cur::Bytes(reply.to_vec());
    for step in &cmd.parse {
        if let Cur::Val(Value::Null) = cur {
            break;
        }
        cur = apply(step, cur).map_err(|e| match e {
            Error::Parse(m) => Error::Parse(format!("{m} (reply {:?})", preview(reply))),
            other => other,
        })?;
    }
    Ok(match cur {
        Cur::Bytes(b) => Value::String(b.iter().map(|&c| c as char).collect()),
        Cur::Val(v) => v,
    })
}

fn apply(step: &ParseStep, cur: Cur) -> Result<Cur, Error> {
    let s = |v: String| Ok(Cur::Val(Value::String(v)));
    match step {
        ParseStep::Trim => s(cur.text("trim")?.trim().to_string()),
        ParseStep::Unquote => {
            let t = cur.text("unquote")?;
            let u = t.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(&t);
            s(u.to_string())
        }
        ParseStep::Lower => s(cur.text("lower")?.to_lowercase()),
        ParseStep::Upper => s(cur.text("upper")?.to_uppercase()),
        ParseStep::Number => {
            let t = cur.text("number")?;
            let n = lenient_number(&t).ok_or_else(|| Error::Parse(format!("not a number: {t:?}")))?;
            Ok(Cur::Val(json!(n)))
        }
        ParseStep::Byte => match cur {
            Cur::Bytes(b) => Ok(Cur::Val(json!(b[0]))),
            Cur::Val(Value::String(t)) => {
                Ok(Cur::Val(json!(t.chars().next().map(|c| c as u32).ok_or_else(|| Error::Parse("byte: empty".into()))?)))
            }
            Cur::Val(v) => Err(Error::Parse(format!("byte: unexpected {v}"))),
        },
        ParseStep::Split { sep, index } => {
            let t = cur.text("split")?;
            let part = t.split(sep.as_str()).nth(*index).ok_or_else(|| Error::Parse(format!("split: no field {index} in {t:?}")))?;
            s(part.to_string())
        }
        ParseStep::Fields { sep, names } => {
            let t = cur.text("fields")?;
            let parts: Vec<&str> = t.split(sep.as_str()).collect();
            if parts.len() < names.len() {
                return Err(Error::Parse(format!("fields: expected {} fields, got {} in {t:?}", names.len(), parts.len())));
            }
            let m: Map<String, Value> = names.iter().zip(parts).map(|(k, v)| (k.clone(), json!(v))).collect();
            Ok(Cur::Val(Value::Object(m)))
        }
        ParseStep::Regex { pattern, group } => {
            let t = cur.text("regex")?;
            let re = Regex::new(pattern).map_err(|e| Error::Parse(e.to_string()))?;
            let c = re.captures(&t).ok_or_else(|| Error::Parse(format!("regex {pattern:?} does not match {t:?}")))?;
            s(c.get(*group).map(|m| m.as_str().to_string()).unwrap_or_default())
        }
        ParseStep::Replace { from, to } => s(cur.text("replace")?.replace(from.as_str(), to)),
        ParseStep::Map(m) => {
            let t = cur.text("map")?;
            match m.get(&t).or_else(|| m.get("_")) {
                Some(v) => s(v.clone()),
                None => Err(Error::Parse(format!("map: unexpected value {t:?}"))),
            }
        }
        ParseStep::Bits(spec) => {
            let n = match cur {
                Cur::Val(Value::Number(n)) => n.as_u64().ok_or_else(|| Error::Parse("bits: not an unsigned integer".into()))?,
                Cur::Bytes(b) => b[0] as u64,
                Cur::Val(v) => return Err(Error::Parse(format!("bits: unexpected {v}"))),
            };
            let re = Regex::new(r"^(\d+)(?:\.\.(\d+))?$").unwrap();
            let mut out = Map::new();
            for (name, range) in spec {
                let c = re.captures(range).ok_or_else(|| Error::Parse(format!("bits: bad range {range}")))?;
                let lo: u32 = c[1].parse().unwrap();
                match c.get(2) {
                    None => out.insert(name.clone(), json!((n >> lo) & 1 == 1)),
                    Some(hi) => {
                        let hi: u32 = hi.as_str().parse().unwrap();
                        let width = hi - lo + 1;
                        out.insert(name.clone(), json!((n >> lo) & ((1u64 << width) - 1)))
                    }
                };
            }
            Ok(Cur::Val(Value::Object(out)))
        }
        ParseStep::NullIf(values) => {
            let t = cur.text("null_if")?;
            Ok(if values.contains(&t) { Cur::Val(Value::Null) } else { Cur::Val(Value::String(t)) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(yaml: &str) -> Command {
        bm_devicedef::from_yaml(yaml).unwrap()
    }

    #[test]
    fn number_and_trim() {
        let c = cmd("send: x\nparse: [trim, number]");
        assert_eq!(parse_reply(&c, b"0.988 ").unwrap(), json!(0.988));
        assert_eq!(parse_reply(&c, b"+0.0000E-308").unwrap(), json!(0.0));
        assert!(parse_reply(&c, b"OL").is_err());
    }

    #[test]
    fn fields_and_null_if() {
        let c = cmd("send: x\nparse: [trim, {fields: {sep: ' ', names: [func, value]}}]");
        assert_eq!(parse_reply(&c, b"RES OL").unwrap(), json!({"func": "RES", "value": "OL"}));
        let c = cmd("send: x\nparse: [trim, {split: {sep: ' ', index: 1}}, {null_if: [OL]}, number]");
        assert_eq!(parse_reply(&c, b"RES OL").unwrap(), Value::Null);
        assert_eq!(parse_reply(&c, b"VOLT:DC +1.0000E-04").unwrap(), json!(1e-4));
    }

    #[test]
    fn bits_from_binary_byte() {
        let c = cmd("send: x\nreply: {bytes: 1}\nparse: [byte, {bits: {cv: '0', tracking: '2..3', out: '6'}}]");
        assert_eq!(parse_reply(&c, &[0x51]).unwrap(), json!({"cv": true, "tracking": 0, "out": true}));
        assert_eq!(parse_reply(&c, &[0x10]).unwrap(), json!({"cv": false, "tracking": 0, "out": false}));
    }

    #[test]
    fn unquote_map_and_empty() {
        let c = cmd("send: x\nparse: [trim, unquote, {map: {'VOLT AC': VOLT_AC}}]");
        assert_eq!(parse_reply(&c, b"\"VOLT AC\"").unwrap(), json!("VOLT_AC"));
        assert!(parse_reply(&c, b"\"FOO\"").is_err());
        assert!(parse_reply(&c, b"").is_err());
        let c = cmd("send: x\non_empty: 'null'\nparse: [trim]");
        assert_eq!(parse_reply(&c, b"").unwrap(), Value::Null);
    }
}
