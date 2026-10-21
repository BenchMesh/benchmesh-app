//! Flow documents (`benchmesh.flow/v1`, contract `ai_context/AUTOMATION.md`): parsing with
//! paths for every problem, validation against the bench's [`Vocab`], CEL expressions, and
//! the list of steps that can energise an output (what a run must be armed for).

use std::collections::{BTreeMap, BTreeSet};

use cel::{Context, Program, Value as Cel};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::vocab::Vocab;

pub const SCHEMA: &str = "benchmesh.flow/v1";
/// Loops without an explicit bound stop after this many passes.
pub const DEFAULT_MAX_ITERATIONS: u64 = 10_000;
/// Sweeps longer than this are refused (a typo in `step` should not mean a million points).
pub const MAX_SWEEP_POINTS: usize = 10_000;

// ---- issues ------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Issue {
    pub path: String,
    /// `error` (the flow cannot run) or `warning`.
    pub level: &'static str,
    pub message: String,
}

fn error(path: &str, message: impl Into<String>) -> Issue {
    Issue { path: path.to_string(), level: "error", message: message.into() }
}

fn warning(path: &str, message: impl Into<String>) -> Issue {
    Issue { path: path.to_string(), level: "warning", message: message.into() }
}

pub fn has_errors(issues: &[Issue]) -> bool {
    issues.iter().any(|i| i.level == "error")
}

// ---- expressions -------------------------------------------------------------------------

/// A CEL expression. Integer literals are read as decimals (`target + 1` works when `target`
/// is 4.5: CEL itself has no int + double), and every number a flow sees is a double.
pub struct Expression {
    src: String,
    program: Program,
}

impl std::fmt::Debug for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expression({:?})", self.src)
    }
}

/// `3` → `3.0` outside strings, identifiers and existing decimals / exponents.
fn decimal_literals(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len() + 8);
    let mut i = 0;
    let mut quote: Option<char> = None;
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = quote {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            quote = Some(c);
            out.push(c);
            i += 1;
            continue;
        }
        let prev = if i == 0 { ' ' } else { chars[i - 1] };
        if c.is_ascii_digit() && !(prev.is_alphanumeric() || prev == '_' || prev == '.') {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            out.extend(&chars[start..i]);
            let next = chars.get(i).copied().unwrap_or(' ');
            // 1.5, 1e3, 0x1F, 3u stay as written
            if !(next == '.' || next == 'e' || next == 'E' || next == 'x' || next == 'X' || next == 'u' || next == 'U' || next.is_alphanumeric() || next == '_') {
                out.push_str(".0");
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

fn num(v: Cel) -> Cel {
    match v {
        Cel::Float(f) => Cel::Float(f),
        Cel::Int(i) => Cel::Float(i as f64),
        Cel::UInt(u) => Cel::Float(u as f64),
        Cel::String(s) => s.trim().parse::<f64>().map(Cel::Float).unwrap_or(Cel::Null),
        _ => Cel::Null,
    }
}

fn abs(v: Cel) -> Cel {
    match num(v) {
        Cel::Float(f) => Cel::Float(f.abs()),
        other => other,
    }
}

fn round(v: Cel, digits: Cel) -> Cel {
    let n = match num(digits) {
        Cel::Float(d) => d.clamp(0.0, 12.0) as i32,
        _ => 0,
    };
    match num(v) {
        Cel::Float(f) => {
            let k = 10f64.powi(n);
            Cel::Float((f * k).round() / k)
        }
        other => other,
    }
}

fn min(a: Cel, b: Cel) -> Cel {
    match (num(a), num(b)) {
        (Cel::Float(x), Cel::Float(y)) => Cel::Float(x.min(y)),
        _ => Cel::Null,
    }
}

fn max(a: Cel, b: Cel) -> Cel {
    match (num(a), num(b)) {
        (Cel::Float(x), Cel::Float(y)) => Cel::Float(x.max(y)),
        _ => Cel::Null,
    }
}

/// JSON → CEL value with every number a double.
fn cel_value(v: &Value) -> Value {
    match v {
        Value::Number(n) => n.as_f64().map(|f| json!(f)).unwrap_or(Value::Null),
        Value::Array(a) => Value::Array(a.iter().map(cel_value).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), cel_value(v))).collect()),
        other => other.clone(),
    }
}

impl Expression {
    pub fn compile(src: &str) -> Result<Expression, String> {
        let rewritten = decimal_literals(src);
        let program = Program::compile(&rewritten).map_err(|e| format!("cannot parse {src:?}: {e}"))?;
        Ok(Expression { src: src.to_string(), program })
    }

    pub fn source(&self) -> &str {
        &self.src
    }

    /// Variables the expression reads.
    pub fn variables(&self) -> Vec<String> {
        self.program.references().variables().into_iter().map(str::to_string).collect()
    }

    pub fn eval(&self, vars: &BTreeMap<String, Value>) -> Result<Value, String> {
        let mut ctx = Context::default();
        let _ = ctx.add_function("num", num);
        let _ = ctx.add_function("abs", abs);
        let _ = ctx.add_function("round", round);
        let _ = ctx.add_function("min", min);
        let _ = ctx.add_function("max", max);
        for (k, v) in vars {
            ctx.add_variable(k.as_str(), cel_value(v)).map_err(|e| e.to_string())?;
        }
        let out = self.program.execute(&ctx).map_err(|e| format!("{}: {e}", self.src))?;
        Ok(bm_driver::expr::to_json(&out))
    }

    /// Evaluate to a boolean (a condition).
    pub fn truth(&self, vars: &BTreeMap<String, Value>) -> Result<bool, String> {
        match self.eval(vars)? {
            Value::Bool(b) => Ok(b),
            other => Err(format!("{} gave {other}, not true/false", self.src)),
        }
    }

    pub fn number(&self, vars: &BTreeMap<String, Value>) -> Result<f64, String> {
        match self.eval(vars)? {
            Value::Number(n) => n.as_f64().ok_or_else(|| format!("{} is not a number", self.src)),
            other => Err(format!("{} gave {other}, not a number", self.src)),
        }
    }
}

// ---- document ----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Param {
    pub name: String,
    pub value: Value,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}

/// An instrument channel addressed by a step.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Address {
    pub device: String,
    pub class: String,
    pub channel: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeriesSpec {
    pub device: String,
    pub class: String,
    #[serde(default = "one")]
    pub channel: u8,
    pub field: String,
    #[serde(default)]
    pub label: Option<String>,
}

fn one() -> u8 {
    1
}

/// Step fields without the nested step lists (`then`, `else`, `steps`), which are parsed
/// separately so every problem has a path.
#[derive(Debug, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case", deny_unknown_fields)]
enum Head {
    Set { device: String, class: String, #[serde(default = "one")] channel: u8, op: String, #[serde(default)] value: Option<Value>, #[serde(default)] expr: Option<String> },
    Read { device: String, class: String, #[serde(default = "one")] channel: u8, #[serde(default)] field: Option<String>, #[serde(default)] op: Option<String>, into: String },
    Wait { #[serde(default)] seconds: Option<f64>, #[serde(default)] expr: Option<String> },
    Let { var: String, expr: String },
    If { condition: String },
    Repeat { #[serde(default)] times: Option<u64>, #[serde(default, rename = "while")] while_: Option<String>, #[serde(default)] until: Option<String>, #[serde(default)] max_iterations: Option<u64> },
    Sweep { var: String, #[serde(default)] from: Option<f64>, #[serde(default)] to: Option<f64>, #[serde(default)] step: Option<f64>, #[serde(default)] values: Option<Vec<Value>> },
    Check { condition: String, #[serde(default)] message: Option<String>, #[serde(default)] fail: Option<String> },
    Log { message: String },
    Prompt { message: String },
    Record { action: String, #[serde(default)] name: Option<String>, #[serde(default)] interval_s: Option<i64>, #[serde(default)] series: Option<Vec<SeriesSpec>> },
    Stop { #[serde(default)] message: Option<String> },
}

#[derive(Debug)]
pub enum ValueSource {
    Literal(Value),
    Expr(Expression),
}

#[derive(Debug)]
pub enum RepeatBound {
    Times(u64),
    While(Expression),
    Until(Expression),
}

#[derive(Debug)]
pub enum Kind {
    Set { at: Address, op: String, value: ValueSource },
    Read { at: Address, source: ReadSource, into: String },
    Wait { seconds: ValueSource },
    Let { var: String, expr: Expression },
    If { condition: Expression },
    Repeat { bound: RepeatBound, max_iterations: u64 },
    Sweep { var: String, values: Vec<Value> },
    Check { condition: Expression, message: String, stop: bool },
    Log { message: String },
    Prompt { message: String },
    RecordStart { name: String, interval_s: i64, series: Vec<SeriesSpec> },
    RecordStop,
    Stop { message: Option<String> },
}

#[derive(Debug, Clone)]
pub enum ReadSource {
    Field(String),
    Op(String),
}

#[derive(Debug)]
pub struct Step {
    pub path: String,
    pub kind: Kind,
    /// `then` of `if`, `steps` of `repeat` / `sweep`.
    pub body: Vec<Step>,
    /// `else` of `if`.
    pub otherwise: Vec<Step>,
}

#[derive(Debug)]
pub struct Flow {
    pub name: String,
    pub description: String,
    pub params: Vec<Param>,
    pub setup: Vec<Step>,
    pub steps: Vec<Step>,
    pub cleanup: Vec<Step>,
    pub timeout_s: Option<f64>,
}

impl Flow {
    /// Every step, depth first, in document order.
    pub fn walk(&self) -> Vec<&Step> {
        fn rec<'a>(steps: &'a [Step], out: &mut Vec<&'a Step>) {
            for s in steps {
                out.push(s);
                rec(&s.body, out);
                rec(&s.otherwise, out);
            }
        }
        let mut out = Vec::new();
        for part in [&self.setup, &self.steps, &self.cleanup] {
            rec(part, &mut out);
        }
        out
    }

    /// The instruments the flow addresses (set / read steps, recorded series).
    pub fn devices(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for s in self.walk() {
            match &s.kind {
                Kind::Set { at, .. } | Kind::Read { at, .. } => {
                    out.insert(at.device.clone());
                }
                Kind::RecordStart { series, .. } => out.extend(series.iter().map(|x| x.device.clone())),
                _ => {}
            }
        }
        out
    }
}

fn is_identifier(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic() || f == '_') && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

const RESERVED: [&str; 12] = ["true", "false", "null", "in", "as", "break", "const", "continue", "else", "for", "function", "if"];

fn check_var(path: &str, name: &str, issues: &mut Vec<Issue>) {
    if !is_identifier(name) || RESERVED.contains(&name) || name == "iteration" || name == "elapsed" {
        issues.push(error(path, format!("{name:?} cannot be a variable name (letters, digits and _; not a keyword, `iteration` or `elapsed`)")));
    }
}

fn compile(path: &str, src: &str, issues: &mut Vec<Issue>) -> Option<Expression> {
    match Expression::compile(src) {
        Ok(e) => Some(e),
        Err(m) => {
            issues.push(error(path, m));
            None
        }
    }
}

fn sweep_values(from: f64, to: f64, step: f64) -> Result<Vec<Value>, String> {
    if !(from.is_finite() && to.is_finite() && step.is_finite()) || step == 0.0 {
        return Err("`step` must be a non-zero number".into());
    }
    if (to - from) * step < 0.0 {
        return Err(format!("`step` {step} never reaches `to` {to} from {from}"));
    }
    let n = ((to - from) / step + 1e-9).floor() as usize + 1;
    if n > MAX_SWEEP_POINTS {
        return Err(format!("{n} points (max {MAX_SWEEP_POINTS})"));
    }
    // computed from the index (no accumulated rounding), cleaned to 12 significant decimals
    Ok((0..n).map(|i| json!(((from + step * i as f64) * 1e12).round() / 1e12)).collect())
}

fn parse_steps(v: Option<&Value>, path: &str, issues: &mut Vec<Issue>) -> Vec<Step> {
    let Some(v) = v else { return vec![] };
    let Some(list) = v.as_array() else {
        issues.push(error(path, "must be a list of steps"));
        return vec![];
    };
    list.iter().enumerate().filter_map(|(i, s)| parse_step(s, &format!("{path}[{i}]"), issues)).collect()
}

fn parse_step(v: &Value, path: &str, issues: &mut Vec<Issue>) -> Option<Step> {
    let Some(obj) = v.as_object() else {
        issues.push(error(path, "a step must be an object with `do`"));
        return None;
    };
    let mut head: Map<String, Value> = obj.clone();
    let then = head.remove("then");
    let otherwise = head.remove("else");
    let inner = head.remove("steps");
    let what = head.get("do").and_then(|d| d.as_str()).unwrap_or("").to_string();
    let nested_ok = match what.as_str() {
        "if" => inner.is_none(),
        "repeat" | "sweep" => then.is_none() && otherwise.is_none(),
        _ => then.is_none() && otherwise.is_none() && inner.is_none(),
    };
    if !nested_ok {
        issues.push(error(path, format!("`{what}` does not take {}", if what == "if" { "`steps` (use `then` / `else`)" } else { "nested steps here" })));
    }
    let h: Head = match serde_json::from_value(Value::Object(head)) {
        Ok(h) => h,
        Err(e) => {
            issues.push(error(path, e.to_string()));
            return None;
        }
    };
    let (body, otherwise) = if what == "if" {
        (parse_steps(then.as_ref(), &format!("{path}.then"), issues), parse_steps(otherwise.as_ref(), &format!("{path}.else"), issues))
    } else {
        (parse_steps(inner.as_ref(), &format!("{path}.steps"), issues), vec![])
    };
    let addr = |device: String, class: String, channel: u8| Address { device, class, channel };
    let kind = match h {
        Head::Set { device, class, channel, op, value, expr } => {
            let value = match (value, expr) {
                (Some(v), None) if v.is_number() || v.is_string() || v.is_boolean() => ValueSource::Literal(v),
                (Some(_), None) => {
                    issues.push(error(path, "`value` must be a number, string or true/false"));
                    return None;
                }
                (None, Some(e)) => ValueSource::Expr(compile(path, &e, issues)?),
                _ => {
                    issues.push(error(path, "give exactly one of `value` (literal) or `expr` (expression)"));
                    return None;
                }
            };
            Kind::Set { at: addr(device, class, channel), op, value }
        }
        Head::Read { device, class, channel, field, op, into } => {
            check_var(path, &into, issues);
            let source = match (field, op) {
                (Some(f), None) => ReadSource::Field(f),
                (None, Some(o)) => ReadSource::Op(o),
                _ => {
                    issues.push(error(path, "give exactly one of `field` (status key) or `op` (query)"));
                    return None;
                }
            };
            Kind::Read { at: addr(device, class, channel), source, into }
        }
        Head::Wait { seconds, expr } => match (seconds, expr) {
            (Some(s), None) if s.is_finite() && s >= 0.0 => Kind::Wait { seconds: ValueSource::Literal(json!(s)) },
            (None, Some(e)) => Kind::Wait { seconds: ValueSource::Expr(compile(path, &e, issues)?) },
            _ => {
                issues.push(error(path, "give `seconds` (≥ 0) or `expr`"));
                return None;
            }
        },
        Head::Let { var, expr } => {
            check_var(path, &var, issues);
            Kind::Let { var, expr: compile(path, &expr, issues)? }
        }
        Head::If { condition } => Kind::If { condition: compile(path, &condition, issues)? },
        Head::Repeat { times, while_, until, max_iterations } => {
            let bound = match (times, while_, until) {
                (Some(n), None, None) => RepeatBound::Times(n),
                (None, Some(w), None) => RepeatBound::While(compile(path, &w, issues)?),
                (None, None, Some(u)) => RepeatBound::Until(compile(path, &u, issues)?),
                _ => {
                    issues.push(error(path, "give exactly one of `times`, `while` or `until`"));
                    return None;
                }
            };
            Kind::Repeat { bound, max_iterations: max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS).max(1) }
        }
        Head::Sweep { var, from, to, step, values } => {
            check_var(path, &var, issues);
            let values = match (values, from, to, step) {
                (Some(v), None, None, None) if !v.is_empty() => v,
                (None, Some(f), Some(t), Some(s)) => match sweep_values(f, t, s) {
                    Ok(v) => v,
                    Err(m) => {
                        issues.push(error(path, m));
                        return None;
                    }
                },
                _ => {
                    issues.push(error(path, "give `from`, `to` and `step`, or a non-empty `values` list"));
                    return None;
                }
            };
            Kind::Sweep { var, values }
        }
        Head::Check { condition, message, fail } => {
            let stop = match fail.as_deref() {
                None | Some("stop") => true,
                Some("warn") => false,
                Some(o) => {
                    issues.push(error(path, format!("`fail` must be `stop` or `warn`, not {o:?}")));
                    true
                }
            };
            Kind::Check { message: message.unwrap_or_else(|| format!("check failed: {condition}")), condition: compile(path, &condition, issues)?, stop }
        }
        Head::Log { message } => Kind::Log { message },
        Head::Prompt { message } => Kind::Prompt { message },
        Head::Record { action, name, interval_s, series } => match action.as_str() {
            "start" => {
                let series = series.unwrap_or_default();
                if series.is_empty() {
                    issues.push(error(path, "`record` start needs `series` (what to record)"));
                }
                let interval_s = interval_s.unwrap_or(1);
                if !(1..=3600).contains(&interval_s) {
                    issues.push(error(path, "`interval_s` must be 1-3600"));
                }
                Kind::RecordStart { name: name.unwrap_or_default(), interval_s, series }
            }
            "stop" => Kind::RecordStop,
            o => {
                issues.push(error(path, format!("`action` must be `start` or `stop`, not {o:?}")));
                return None;
            }
        },
        Head::Stop { message } => Kind::Stop { message },
    };
    Some(Step { path: path.to_string(), kind, body, otherwise })
}

/// Parse a document. `None` when it is not a flow at all; otherwise the flow (possibly with
/// some steps missing) plus every problem found.
pub fn parse(doc: &Value) -> (Option<Flow>, Vec<Issue>) {
    let mut issues = Vec::new();
    let Some(o) = doc.as_object() else {
        return (None, vec![error("", "a flow must be a JSON object")]);
    };
    match o.get("schema").and_then(|v| v.as_str()) {
        Some(SCHEMA) => {}
        Some(other) => issues.push(error("schema", format!("unsupported schema {other:?} (expected {SCHEMA})"))),
        None => issues.push(error("schema", format!("missing `schema: {SCHEMA}`"))),
    }
    for k in o.keys() {
        if !["schema", "name", "description", "params", "setup", "steps", "cleanup", "timeout_s"].contains(&k.as_str()) {
            issues.push(error(k, format!("unknown field `{k}`")));
        }
    }
    let name = o.get("name").and_then(|v| v.as_str()).map(str::trim).unwrap_or("").to_string();
    if name.is_empty() || name.chars().count() > 100 {
        issues.push(error("name", "`name` must be 1-100 characters"));
    }
    let mut params = Vec::new();
    if let Some(p) = o.get("params") {
        match p.as_array() {
            None => issues.push(error("params", "must be a list")),
            Some(list) => {
                for (i, v) in list.iter().enumerate() {
                    let path = format!("params[{i}]");
                    match serde_json::from_value::<Param>(v.clone()) {
                        Ok(p) => {
                            check_var(&path, &p.name, &mut issues);
                            if !(p.value.is_number() || p.value.is_string() || p.value.is_boolean()) {
                                issues.push(error(&path, "`value` must be a number, string or true/false"));
                            }
                            if params.iter().any(|q: &Param| q.name == p.name) {
                                issues.push(error(&path, format!("parameter {:?} is defined twice", p.name)));
                            }
                            params.push(p);
                        }
                        Err(e) => issues.push(error(&path, e.to_string())),
                    }
                }
            }
        }
    }
    let timeout_s = match o.get("timeout_s") {
        None | Some(Value::Null) => None,
        Some(v) => match v.as_f64().filter(|t| *t > 0.0) {
            Some(t) => Some(t),
            None => {
                issues.push(error("timeout_s", "must be a positive number of seconds"));
                None
            }
        },
    };
    let setup = parse_steps(o.get("setup"), "setup", &mut issues);
    let steps = parse_steps(o.get("steps"), "steps", &mut issues);
    let cleanup = parse_steps(o.get("cleanup"), "cleanup", &mut issues);
    if steps.is_empty() && setup.is_empty() && !has_errors(&issues) {
        issues.push(error("steps", "the flow has no steps"));
    }
    let description = o.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
    (Some(Flow { name, description, params, setup, steps, cleanup, timeout_s }), issues)
}

/// Parameter values for a run: the flow's, overridden by `over`, checked against min/max.
pub fn param_values(flow: &Flow, over: Option<&Map<String, Value>>) -> Result<BTreeMap<String, Value>, String> {
    let mut vars = BTreeMap::new();
    for p in &flow.params {
        let v = over.and_then(|o| o.get(&p.name)).cloned().unwrap_or_else(|| p.value.clone());
        if p.value.is_number() {
            let Some(x) = v.as_f64() else { return Err(format!("parameter {} must be a number", p.name)) };
            if p.min.is_some_and(|m| x < m) || p.max.is_some_and(|m| x > m) {
                return Err(format!("parameter {} = {x} is outside [{}, {}]", p.name, p.min.map_or("-".into(), |m| m.to_string()), p.max.map_or("-".into(), |m| m.to_string())));
            }
        }
        vars.insert(p.name.clone(), v);
    }
    if let Some(o) = over {
        if let Some(k) = o.keys().find(|k| !flow.params.iter().any(|p| &p.name == *k)) {
            return Err(format!("the flow has no parameter {k:?}"));
        }
    }
    Ok(vars)
}

// ---- validation --------------------------------------------------------------------------

/// A step that can switch an output / input on.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Energize {
    pub path: String,
    pub device: String,
    pub class: String,
    pub channel: u8,
    pub op: String,
}

fn literal_matches_spec(v: &Value, spec: &Value) -> Result<(), String> {
    let text = match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    match spec["type"].as_str().unwrap_or("") {
        "enum" => {
            let values: Vec<&str> = spec["values"].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
            if values.contains(&text.as_str()) { Ok(()) } else { Err(format!("{text:?} is not one of {values:?}")) }
        }
        "number" | "integer" => {
            let x = v.as_f64().or_else(|| text.trim().parse().ok()).ok_or_else(|| format!("{text:?} is not a number"))?;
            let (min, max) = (spec["min"].as_f64(), spec["max"].as_f64());
            if min.is_some_and(|m| x < m) || max.is_some_and(|m| x > m) {
                return Err(format!("{x} is outside [{}, {}]", min.map_or("-".into(), |m| m.to_string()), max.map_or("-".into(), |m| m.to_string())));
            }
            Ok(())
        }
        "bool" => match text.to_ascii_uppercase().as_str() {
            "1" | "0" | "ON" | "OFF" | "TRUE" | "FALSE" => Ok(()),
            _ => Err(format!("{text:?} is not true/false")),
        },
        _ => Ok(()),
    }
}

/// Is a literal the safe (off) value of an energising setter?
pub fn is_safe_literal(v: &Value, safe: Option<&Value>) -> bool {
    match (v, safe) {
        (Value::String(s), Some(Value::String(off))) => s.eq_ignore_ascii_case(off),
        (Value::Bool(b), Some(Value::Bool(off))) => b == off,
        (Value::String(s), Some(Value::Bool(false))) => matches!(s.to_ascii_uppercase().as_str(), "OFF" | "0" | "FALSE"),
        (Value::Number(n), Some(Value::Bool(false))) => n.as_f64() == Some(0.0),
        _ => false,
    }
}

/// Check a parsed flow against the bench. Variables must be assigned before they are read
/// (in document order; a variable assigned in a loop or branch counts from there on).
pub fn validate(flow: &Flow, vocab: &Vocab) -> (Vec<Issue>, Vec<Energize>) {
    let mut issues = Vec::new();
    let mut energize = Vec::new();
    let mut known: BTreeSet<String> = flow.params.iter().map(|p| p.name.clone()).collect();
    known.insert("elapsed".into());
    for part in [&flow.setup, &flow.steps, &flow.cleanup] {
        check_steps(part, vocab, &mut known, 0, &mut issues, &mut energize);
    }
    (issues, energize)
}

fn check_expr(path: &str, e: &Expression, known: &BTreeSet<String>, issues: &mut Vec<Issue>) {
    for v in e.variables() {
        if !known.contains(&v) {
            issues.push(error(path, format!("{:?} uses {v:?}, which is not a parameter or a variable set before this step", e.source())));
        }
    }
}

fn check_address(path: &str, at: &Address, vocab: &Vocab, issues: &mut Vec<Issue>) -> Option<super::vocab::Class> {
    match vocab.class(&at.device, &at.class) {
        Err(m) => {
            issues.push(error(path, m));
            None
        }
        Ok(c) => {
            if at.channel == 0 || at.channel > c.channels {
                issues.push(error(path, format!("{} {} has channel{} 1-{}, not {}", at.device, at.class, if c.channels > 1 { "s" } else { "" }, c.channels, at.channel)));
            }
            match vocab.device(&at.device) {
                Some(d) if d.parked => issues.push(warning(path, format!("{} is parked: unpark it before running the flow", at.device))),
                Some(d) if !d.online => issues.push(warning(path, format!("{} is offline right now", at.device))),
                _ => {}
            }
            Some(c.clone())
        }
    }
}

fn check_steps(steps: &[Step], vocab: &Vocab, known: &mut BTreeSet<String>, loops: usize, issues: &mut Vec<Issue>, energize: &mut Vec<Energize>) {
    for s in steps {
        let p = s.path.as_str();
        match &s.kind {
            Kind::Set { at, op, value } => {
                if let ValueSource::Expr(e) = value {
                    check_expr(p, e, known, issues);
                }
                if let Some(c) = check_address(p, at, vocab, issues) {
                    match c.operations.get(op) {
                        Some(o) if o.set => {
                            if let (ValueSource::Literal(v), Some(spec)) = (value, &o.value) {
                                if let Err(m) = literal_matches_spec(v, spec) {
                                    issues.push(error(p, format!("{op}: {m}")));
                                }
                            }
                            let safe = o.safe_value();
                            let energizing = o.energizes && !matches!(value, ValueSource::Literal(v) if is_safe_literal(v, safe.as_ref()));
                            if energizing {
                                energize.push(Energize { path: s.path.clone(), device: at.device.clone(), class: at.class.clone(), channel: at.channel, op: op.clone() });
                            }
                        }
                        _ => {
                            let setters: Vec<&str> = c.operations.iter().filter(|(_, o)| o.set).map(|(k, _)| k.as_str()).collect();
                            issues.push(error(p, format!("{} {} has no setter {op:?} (setters: {})", at.device, at.class, setters.join(", "))));
                        }
                    }
                }
            }
            Kind::Read { at, source, into } => {
                if let Some(c) = check_address(p, at, vocab, issues) {
                    match source {
                        ReadSource::Field(f) if !c.fields.contains_key(f) => {
                            issues.push(error(p, format!("{} {} reports no field {f:?} (fields: {})", at.device, at.class, c.fields.keys().cloned().collect::<Vec<_>>().join(", "))));
                        }
                        ReadSource::Op(o) if !c.operations.get(o).is_some_and(|x| x.get) => {
                            let queries: Vec<&str> = c.operations.iter().filter(|(_, o)| o.get).map(|(k, _)| k.as_str()).collect();
                            issues.push(error(p, format!("{} {} has no query {o:?} (queries: {})", at.device, at.class, queries.join(", "))));
                        }
                        _ => {}
                    }
                }
                known.insert(into.clone());
            }
            Kind::Wait { seconds } => {
                if let ValueSource::Expr(e) = seconds {
                    check_expr(p, e, known, issues);
                }
            }
            Kind::Let { var, expr } => {
                check_expr(p, expr, known, issues);
                known.insert(var.clone());
            }
            Kind::If { condition } => {
                check_expr(p, condition, known, issues);
                check_steps(&s.body, vocab, known, loops, issues, energize);
                check_steps(&s.otherwise, vocab, known, loops, issues, energize);
            }
            Kind::Repeat { bound, .. } => {
                known.insert("iteration".into());
                if let RepeatBound::While(e) = bound {
                    check_expr(p, e, known, issues);
                }
                check_steps(&s.body, vocab, known, loops + 1, issues, energize);
                if let RepeatBound::Until(e) = bound {
                    check_expr(p, e, known, issues);
                }
                if loops == 0 {
                    known.remove("iteration");
                }
            }
            Kind::Sweep { var, .. } => {
                known.insert(var.clone());
                check_steps(&s.body, vocab, known, loops, issues, energize);
            }
            Kind::Check { condition, .. } => check_expr(p, condition, known, issues),
            Kind::RecordStart { series, .. } => {
                for (i, x) in series.iter().enumerate() {
                    let at = Address { device: x.device.clone(), class: x.class.clone(), channel: x.channel };
                    let sp = format!("{p}.series[{i}]");
                    if let Some(c) = check_address(&sp, &at, vocab, issues) {
                        match c.fields.get(&x.field) {
                            None => issues.push(error(&sp, format!("{} {} reports no field {:?}", at.device, at.class, x.field))),
                            Some(f) if !matches!(f.kind.as_str(), "si" | "number" | "integer") => {
                                issues.push(error(&sp, format!("{:?} is not numeric ({}) and cannot be recorded", x.field, f.kind)))
                            }
                            _ => {}
                        }
                    }
                }
            }
            Kind::Log { .. } | Kind::Prompt { .. } | Kind::RecordStop | Kind::Stop { .. } => {}
        }
    }
}

/// Parse and validate in one go (what the API, the assistant and MCP call).
pub fn check(doc: &Value, vocab: &Vocab) -> (Option<Flow>, Vec<Issue>, Vec<Energize>) {
    let (flow, mut issues) = parse(doc);
    let Some(flow) = flow else { return (None, issues, vec![]) };
    let (more, energize) = validate(&flow, vocab);
    issues.extend(more);
    (Some(flow), issues, energize)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::vocab::{Class, Device, Field, Op};
    use super::*;

    pub(crate) fn vocab() -> Vocab {
        let num = |min: f64, max: f64| Some(json!({"type": "number", "min": min, "max": max, "format": "%.2f", "unit": "A"}));
        let onoff = Some(json!({"type": "enum", "values": ["ON", "OFF"]}));
        let si = || Field { kind: "si".into(), values: None, description: None };
        let ell = Class {
            channels: 1,
            fields: BTreeMap::from([("VIN".into(), si()), ("IIN".into(), si()), ("INPUT".into(), Field { kind: "enum".into(), values: Some(vec!["ON".into(), "OFF".into()]), description: None })]),
            operations: BTreeMap::from([
                ("curr".into(), Op { get: true, set: true, value: num(0.0, 15.0), energizes: false, description: None }),
                ("input".into(), Op { get: true, set: true, value: onoff.clone(), energizes: true, description: None }),
            ]),
        };
        let dmm = Class { channels: 1, fields: BTreeMap::from([("MEAS".into(), si())]), operations: BTreeMap::new() };
        Vocab {
            devices: vec![
                Device { id: "load".into(), name: "Load".into(), vendor: "OWON".into(), model: None, online: true, parked: false, classes: BTreeMap::from([("ELL".into(), ell)]) },
                Device { id: "dmm".into(), name: "DMM".into(), vendor: "OWON".into(), model: None, online: false, parked: false, classes: BTreeMap::from([("DMM".into(), dmm)]) },
            ],
        }
    }

    /// The example from the design: hold a voltage by moving the load current.
    pub(crate) fn hold_flow() -> Value {
        json!({
            "schema": SCHEMA, "name": "Hold 5 V",
            "params": [{"name": "target", "value": 5, "unit": "V", "min": 0, "max": 30}, {"name": "step", "value": 0.05, "unit": "A"}],
            "setup": [{"do": "set", "device": "load", "class": "ELL", "op": "curr", "value": 0}, {"do": "set", "device": "load", "class": "ELL", "op": "input", "value": "ON"}],
            "steps": [{"do": "repeat", "times": 20, "steps": [
                {"do": "read", "device": "dmm", "class": "DMM", "field": "MEAS", "into": "v"},
                {"do": "read", "device": "load", "class": "ELL", "op": "curr", "into": "i"},
                {"do": "if", "condition": "v > target", "then": [{"do": "set", "device": "load", "class": "ELL", "op": "curr", "expr": "min(i + step, 2)"}],
                 "else": [{"do": "set", "device": "load", "class": "ELL", "op": "curr", "expr": "max(i - step, 0)"}]},
                {"do": "log", "message": "pass {iteration}: {v} V at {i} A"},
                {"do": "wait", "seconds": 1}
            ]}],
            "cleanup": [{"do": "set", "device": "load", "class": "ELL", "op": "input", "value": "OFF"}]
        })
    }

    #[test]
    fn integer_literals_become_decimals_outside_strings_and_names() {
        assert_eq!(decimal_literals("target + 1"), "target + 1.0");
        assert_eq!(decimal_literals("x2 > 1.5 && y < 1e3"), "x2 > 1.5 && y < 1e3");
        assert_eq!(decimal_literals(r#"m == "CH1" || n >= 10"#), r#"m == "CH1" || n >= 10.0"#);
        assert_eq!(decimal_literals("round(v, 2)"), "round(v, 2.0)");
    }

    #[test]
    fn expressions_mix_parameters_readings_and_literals() {
        let vars = BTreeMap::from([("target".to_string(), json!(5)), ("i".to_string(), json!(1.95))]);
        assert_eq!(Expression::compile("min(i + 0.1, 2)").unwrap().number(&vars).unwrap(), 2.0);
        assert!(Expression::compile("target > 4").unwrap().truth(&vars).unwrap());
        assert_eq!(Expression::compile("round(abs(i - target), 1)").unwrap().number(&vars).unwrap(), 3.1);
        assert!(Expression::compile("i +").is_err());
        assert!(Expression::compile("target").unwrap().truth(&vars).is_err());
    }

    #[test]
    fn a_good_flow_validates_and_lists_what_it_energises() {
        let (flow, issues, energize) = check(&hold_flow(), &vocab());
        assert!(flow.is_some());
        assert!(!has_errors(&issues), "{issues:?}");
        // the DMM is offline: a warning, not an error
        assert!(issues.iter().any(|i| i.level == "warning" && i.message.contains("offline")));
        // `input ON` energises; the cleanup's `input OFF` does not
        assert_eq!(energize.len(), 1);
        assert_eq!(energize[0].path, "setup[1]");
    }

    #[test]
    fn problems_have_paths_and_name_what_exists() {
        let doc = json!({
            "schema": SCHEMA, "name": "bad",
            "steps": [
                {"do": "set", "device": "load", "class": "ELL", "op": "volts", "value": 3},
                {"do": "set", "device": "load", "class": "ELL", "op": "curr", "value": 99},
                {"do": "read", "device": "load", "class": "ELL", "field": "VOUT", "into": "x"},
                {"do": "if", "condition": "y > 1", "then": [{"do": "wait"}]},
                {"do": "repeat", "steps": []},
                {"do": "frobnicate"},
                {"do": "set", "device": "nope", "class": "PSU", "op": "voltage", "value": 1, "channel": 1},
                {"do": "set", "device": "load", "class": "ELL", "channel": 2, "op": "curr", "value": 1}
            ]
        });
        let (_, issues, _) = check(&doc, &vocab());
        let at = |p: &str| issues.iter().filter(|i| i.path == p).map(|i| i.message.clone()).collect::<Vec<_>>().join(" | ");
        assert!(at("steps[0]").contains("no setter \"volts\" (setters: curr, input)"), "{issues:?}");
        assert!(at("steps[1]").contains("outside [0, 15]"));
        assert!(at("steps[2]").contains("no field \"VOUT\""));
        assert!(at("steps[3]").contains("\"y\""));
        assert!(at("steps[3].then[0]").contains("seconds"));
        assert!(at("steps[4]").contains("exactly one of `times`"));
        assert!(at("steps[5]").contains("unknown variant `frobnicate`"));
        assert!(at("steps[6]").contains("unknown device \"nope\" (configured: load, dmm)"));
        assert!(at("steps[7]").contains("channel 1-1, not 2"));
    }

    #[test]
    fn variables_must_be_set_before_use_and_iteration_only_in_loops() {
        let doc = json!({"schema": SCHEMA, "name": "order", "steps": [
            {"do": "log", "message": "x"},
            {"do": "let", "var": "a", "expr": "b + 1"},
            {"do": "let", "var": "b", "expr": "1"},
            {"do": "let", "var": "c", "expr": "a + b + elapsed"},
            {"do": "let", "var": "d", "expr": "iteration"},
            {"do": "repeat", "until": "iteration >= 3", "steps": [{"do": "let", "var": "e", "expr": "iteration * 2"}]}
        ]});
        let (_, issues, _) = check(&doc, &vocab());
        let errs: Vec<&str> = issues.iter().filter(|i| i.level == "error").map(|i| i.path.as_str()).collect();
        assert_eq!(errs, ["steps[1]", "steps[4]"], "{issues:?}");
    }

    #[test]
    fn sweeps_are_exact_bounded_and_go_both_ways() {
        assert_eq!(sweep_values(0.0, 0.3, 0.1).unwrap(), vec![json!(0.0), json!(0.1), json!(0.2), json!(0.3)]);
        assert_eq!(sweep_values(1.0, 0.0, -0.5).unwrap(), vec![json!(1.0), json!(0.5), json!(0.0)]);
        assert!(sweep_values(0.0, 1.0, -0.1).is_err());
        assert!(sweep_values(0.0, 1.0, 0.0).is_err());
        assert!(sweep_values(0.0, 1e6, 1e-3).is_err());
    }

    #[test]
    fn parameters_can_be_overridden_within_limits() {
        let (flow, _, _) = check(&hold_flow(), &vocab());
        let flow = flow.unwrap();
        assert_eq!(param_values(&flow, None).unwrap()["target"], json!(5));
        let over = json!({"target": 12}).as_object().unwrap().clone();
        assert_eq!(param_values(&flow, Some(&over)).unwrap()["target"], json!(12));
        let over = json!({"target": 31}).as_object().unwrap().clone();
        assert!(param_values(&flow, Some(&over)).unwrap_err().contains("outside"));
        let over = json!({"nope": 1}).as_object().unwrap().clone();
        assert!(param_values(&flow, Some(&over)).is_err());
    }

    #[test]
    fn a_document_that_is_not_a_flow_says_why() {
        let (_, issues, _) = check(&json!({"name": "x", "stepz": []}), &vocab());
        let msgs: Vec<&str> = issues.iter().map(|i| i.message.as_str()).collect();
        assert!(msgs.iter().any(|m| m.contains("missing `schema")));
        assert!(msgs.iter().any(|m| m.contains("unknown field `stepz`")));
    }
}
