//! CEL expressions over poll results, `state` and `ch`, with the BenchMesh function
//! `num(x)` (lenient number → double, or null for tokens like `OL`).

use std::collections::BTreeMap;


use cel::{Context, Program, Value as Cel};
use serde_json::{json, Map, Value};

use crate::parse::lenient_number;
use crate::Error;

/// Variables visible to an expression.
#[derive(Debug, Clone, Copy)]
pub struct Vars<'a> {
    /// Parsed command results by command name (missing results must be present as null).
    pub results: &'a BTreeMap<String, Value>,
    pub state: &'a BTreeMap<String, Value>,
    pub ch: u8,
}

pub struct Expr {
    src: String,
    program: Program,
}

impl std::fmt::Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expr({:?})", self.src)
    }
}

fn num(v: Cel) -> Cel {
    match v {
        Cel::Float(f) => Cel::Float(f),
        Cel::Int(i) => Cel::Float(i as f64),
        Cel::UInt(u) => Cel::Float(u as f64),
        Cel::String(s) => lenient_number(&s).map(Cel::Float).unwrap_or(Cel::Null),
        _ => Cel::Null,
    }
}

impl Expr {
    pub fn compile(src: &str) -> Result<Self, Error> {
        let program = Program::compile(src).map_err(|e| Error::Expr(format!("{src}: {e}")))?;
        Ok(Expr { src: src.to_string(), program })
    }

    pub fn source(&self) -> &str {
        &self.src
    }

    pub fn eval(&self, vars: Vars<'_>) -> Result<Value, Error> {
        let mut ctx = Context::default();
        ctx.add_function("num", num).expect("`num` does not clash with a CEL builtin");
        for (k, v) in vars.results {
            ctx.add_variable(k.as_str(), v).map_err(|e| Error::Expr(e.to_string()))?;
        }
        let state: Map<String, Value> = vars.state.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        ctx.add_variable("state", Value::Object(state)).map_err(|e| Error::Expr(e.to_string()))?;
        ctx.add_variable("ch", vars.ch as i64).map_err(|e| Error::Expr(e.to_string()))?;
        let out = self.program.execute(&ctx).map_err(|e| Error::Expr(format!("{}: {e}", self.src)))?;
        Ok(to_json(&out))
    }
}

pub fn to_json(v: &Cel) -> Value {
    match v {
        Cel::Null => Value::Null,
        Cel::Bool(b) => json!(b),
        Cel::Int(i) => json!(i),
        Cel::UInt(u) => json!(u),
        Cel::Float(f) if f.is_finite() => json!(f),
        Cel::Float(_) => Value::Null,
        Cel::String(s) => json!(s.as_str()),
        Cel::List(l) => Value::Array(l.iter().map(to_json).collect()),
        Cel::Map(m) => Value::Object(
            m.map
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        cel::objects::Key::String(s) => s.as_str().to_string(),
                        cel::objects::Key::Int(i) => i.to_string(),
                        cel::objects::Key::Uint(u) => u.to_string(),
                        cel::objects::Key::Bool(b) => b.to_string(),
                    };
                    (key, to_json(v))
                })
                .collect(),
        ),
        Cel::Bytes(b) => json!(String::from_utf8_lossy(b.as_ref())),
        other => json!(format!("{other:?}")),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str, results: Value) -> Value {
        let results: BTreeMap<String, Value> = results.as_object().unwrap().clone().into_iter().collect();
        let state = BTreeMap::from([("remote".to_string(), json!("OFF"))]);
        Expr::compile(src).unwrap().eval(Vars { results: &results, state: &state, ch: 1 }).unwrap()
    }

    #[test]
    fn arithmetic_ternary_and_members() {
        assert_eq!(eval("vout * iout", json!({"vout": 5.0, "iout": 0.5})), json!(2.5));
        assert_eq!(eval(r#"status.out ? "ON" : "OFF""#, json!({"status": {"out": true}})), json!("ON"));
        assert_eq!(eval("status.out && !status.ch1_cv", json!({"status": {"out": true, "ch1_cv": false}})), json!(true));
        assert_eq!(eval("state.remote", json!({})), json!("OFF"));
        assert_eq!(eval("ch", json!({})), json!(1));
    }

    #[test]
    fn num_and_null_handling() {
        assert_eq!(eval("num(info.v)", json!({"info": {"v": "5.000"}})), json!(5.0));
        assert_eq!(eval(r#"num("OL")"#, json!({})), Value::Null);
        assert_eq!(eval("info == null ? null : num(info.v)", json!({"info": null})), Value::Null);
        assert_eq!(eval(r#"{"VOLT:DC": "VOLT_DC"}[conf.func]"#, json!({"conf": {"func": "VOLT:DC"}})), json!("VOLT_DC"));
        assert_eq!(eval(r#"func in ["VOLT", "RES"]"#, json!({"func": "RES"})), json!(true));
        assert_eq!(eval("phase_q * 180.0 / 3.141592653589793", json!({"phase_q": std::f64::consts::PI})), json!(180.0));
    }
}
