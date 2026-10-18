//! Rendering a command to wire bytes: argument validation (enum, range, pattern),
//! printf-style number formatting, computed params, `{ch}` and `{state.*}`.

use std::collections::BTreeMap;

use bm_devicedef::template::{self, Piece};
use bm_devicedef::{Command, Param, ValueKind};
use regex::Regex;
use serde_json::Value;

use crate::parse::lenient_number;
use crate::pyfmt::fmt_g;
use crate::Error;

/// Render `cmd` with caller args (API values as strings). `computed` evaluates a param's
/// `from` expression (needs cached query results, so the engine supplies it).
pub fn render(
    cmd: &Command,
    tx_terminator: &str,
    args: &BTreeMap<String, String>,
    ch: u8,
    state: &BTreeMap<String, Value>,
    computed: &mut dyn FnMut(&str) -> Result<Value, Error>,
) -> Result<String, Error> {
    let mut out = String::new();
    for piece in template::parse(&cmd.send).map_err(Error::Definition)? {
        match piece {
            Piece::Literal(l) => out.push_str(&l),
            Piece::Placeholder(p) if p == "ch" => out.push_str(&ch.to_string()),
            Piece::Placeholder(p) if p.starts_with("state.") => {
                let v = state.get(&p["state.".len()..]).cloned().unwrap_or(Value::Null);
                out.push_str(&value_text(&v));
            }
            Piece::Placeholder(p) => {
                let spec = cmd.params.get(&p).ok_or_else(|| Error::Definition(format!("unknown param {p}")))?;
                let api_value = match &spec.from {
                    Some(expr) => value_text(&computed(expr)?),
                    None => args.get(&p).cloned().ok_or_else(|| Error::BadArgument(format!("missing value for `{p}`")))?,
                };
                out.push_str(&format_param(&p, spec, &api_value)?);
            }
        }
    }
    for k in args.keys() {
        if !cmd.params.contains_key(k) {
            return Err(Error::BadArgument(format!("unexpected argument `{k}`")));
        }
    }
    out.push_str(tx_terminator);
    Ok(out)
}

fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Validate one API value and turn it into wire text.
pub fn format_param(name: &str, spec: &Param, value: &str) -> Result<String, Error> {
    let bad = |m: String| Err(Error::BadArgument(format!("`{name}`: {m}")));
    match spec.kind {
        ValueKind::Enum => match spec.values.get(value) {
            Some(w) => Ok(w.clone()),
            None => bad(format!("{value:?} is not one of {:?}", spec.values.keys().collect::<Vec<_>>())),
        },
        ValueKind::Number | ValueKind::Integer => {
            let Some(x) = lenient_number(value) else { return bad(format!("{value:?} is not a number")) };
            if spec.min.is_some_and(|m| x < m) || spec.max.is_some_and(|m| x > m) {
                return bad(format!("{x} is outside [{}, {}]", fmt_opt(spec.min), fmt_opt(spec.max)));
            }
            if spec.kind == ValueKind::Integer && x.fract() != 0.0 {
                return bad(format!("{value:?} is not an integer"));
            }
            let x = spec.scale.map_or(x, |k| x * k);
            format_number(x, spec.format.as_deref(), spec.kind == ValueKind::Integer || spec.scale.is_some())
        }
        ValueKind::Bool => match value.to_ascii_uppercase().as_str() {
            "1" | "ON" | "TRUE" => Ok("1".into()),
            "0" | "OFF" | "FALSE" => Ok("0".into()),
            _ => bad(format!("{value:?} is not a boolean")),
        },
        ValueKind::String => {
            if let Some(p) = &spec.pattern {
                let re = Regex::new(p).map_err(|e| Error::Definition(e.to_string()))?;
                if !re.is_match(value) {
                    return bad(format!("{value:?} does not match {p}"));
                }
            }
            Ok(value.to_string())
        }
        ValueKind::Si | ValueKind::Bytes | ValueKind::List => Err(Error::Definition(format!("param `{name}` has a class-only kind"))),
    }
}

fn fmt_opt(v: Option<f64>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "-".into())
}

fn format_number(x: f64, fmt: Option<&str>, integer: bool) -> Result<String, Error> {
    match fmt {
        None if integer => Ok(format!("{}", x as i64)),
        None => Ok(format!("{x}")),
        Some("%d") => Ok(format!("{}", x.round() as i64)),
        Some(f) if f.starts_with("%0") && f.ends_with('d') => {
            let width = f[2..f.len() - 1].parse::<usize>().map_err(|_| Error::Definition(format!("unsupported format {f}")))?;
            Ok(format!("{:0width$}", x.round() as i64))
        }
        Some("%g") => Ok(fmt_g(x, 6)),
        Some(f) => {
            let decimals = f
                .strip_prefix("%.")
                .and_then(|r| r.strip_suffix('f'))
                .and_then(|d| d.parse::<usize>().ok())
                .ok_or_else(|| Error::Definition(format!("unsupported format {f}")))?;
            Ok(format!("{x:.decimals$}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(yaml: &str) -> Command {
        bm_devicedef::from_yaml(yaml).unwrap()
    }

    fn args(kv: &[(&str, &str)]) -> BTreeMap<String, String> {
        kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn no_computed(_: &str) -> Result<Value, Error> {
        panic!("not expected")
    }

    #[test]
    fn tenma_setpoint_format_and_range() {
        let c = cmd("send: 'VSET{ch}:{v}'\nparams: {v: {type: number, min: 0, max: 30, format: '%.2f'}}\nreply: none");
        let st = BTreeMap::new();
        assert_eq!(render(&c, "", &args(&[("v", "5")]), 1, &st, &mut no_computed).unwrap(), "VSET1:5.00");
        assert_eq!(render(&c, "", &args(&[("v", "12.345")]), 1, &st, &mut no_computed).unwrap(), "VSET1:12.35");
        assert!(matches!(render(&c, "", &args(&[("v", "31")]), 1, &st, &mut no_computed), Err(Error::BadArgument(_))));
        assert!(matches!(render(&c, "", &args(&[("v", "abc")]), 1, &st, &mut no_computed), Err(Error::BadArgument(_))));
    }

    #[test]
    fn scaled_zero_padded_integers() {
        // Manson HCS: VOLT127 = 12.7 V (3 digits, tenths of a volt)
        let c = cmd("send: 'VOLT{v}'\nparams: {v: {type: number, min: 1, max: 36, scale: 10, format: '%03d'}}\nreply: none");
        let st = BTreeMap::new();
        assert_eq!(render(&c, "", &args(&[("v", "12.7")]), 1, &st, &mut no_computed).unwrap(), "VOLT127");
        assert_eq!(render(&c, "", &args(&[("v", "5")]), 1, &st, &mut no_computed).unwrap(), "VOLT050");
        assert!(render(&c, "", &args(&[("v", "37")]), 1, &st, &mut no_computed).is_err(), "limits apply to the API value");
    }

    #[test]
    fn enum_maps_to_wire_and_rejects_unknown() {
        let c = cmd("send: '{on}'\nparams: {on: {type: enum, values: {'ON': SYST:REM, 'OFF': SYST:LOC}}}\nreply: none");
        let st = BTreeMap::new();
        assert_eq!(render(&c, "\r", &args(&[("on", "ON")]), 1, &st, &mut no_computed).unwrap(), "SYST:REM\r");
        assert!(render(&c, "\r", &args(&[("on", "on")]), 1, &st, &mut no_computed).is_err());
    }

    #[test]
    fn computed_param_uses_engine_callback() {
        let c = cmd(
            "send: 'CONF:{fn} {range}'\nparams:\n  fn: {type: enum, from: func, values: {RES: RES, VOLT: 'VOLT:DC'}}\n  range: {type: string, pattern: '^(AUTO|[0-9E.-]+)$'}\nreply: none",
        );
        let st = BTreeMap::new();
        let mut computed = |e: &str| -> Result<Value, Error> {
            assert_eq!(e, "func");
            Ok(Value::String("RES".into()))
        };
        assert_eq!(render(&c, "\r", &args(&[("range", "5E3")]), 1, &st, &mut computed).unwrap(), "CONF:RES 5E3\r");
        assert!(render(&c, "\r", &args(&[("range", "5k; *RST")]), 1, &st, &mut computed).is_err());
    }

    #[test]
    fn g_format_for_generators() {
        let c = cmd("send: 'FREQ {hz}'\nparams: {hz: {type: number, format: '%g'}}\nreply: none");
        let st = BTreeMap::new();
        assert_eq!(render(&c, "", &args(&[("hz", "1000")]), 1, &st, &mut no_computed).unwrap(), "FREQ 1000");
        assert_eq!(render(&c, "", &args(&[("hz", "70e6")]), 1, &st, &mut no_computed).unwrap(), "FREQ 7e+07");
    }
}
