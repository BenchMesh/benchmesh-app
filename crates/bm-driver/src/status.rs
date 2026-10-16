//! Class status: evaluate a binding's `status` expressions over the latest poll results.

use std::collections::BTreeMap;

use bm_devicedef::{ClassBinding, Repr};
use serde_json::Value;

use crate::expr::{Expr, Vars};
use crate::si::si_value;
use crate::Error;

/// A class binding with its expressions compiled once.
#[derive(Debug)]
pub struct CompiledStatus {
    fields: Vec<(String, Expr, Repr)>,
    /// Commands read by this binding's poll groups (bound as null when missing).
    pub poll_commands: Vec<String>,
}

impl CompiledStatus {
    pub fn new(binding: &ClassBinding) -> Result<Self, Error> {
        let fields = binding
            .status
            .iter()
            .map(|(k, f)| Ok((k.clone(), Expr::compile(f.expr())?, f.repr())))
            .collect::<Result<_, Error>>()?;
        let poll_commands = binding.poll.iter().flat_map(|g| g.read.iter().cloned()).collect();
        Ok(CompiledStatus { fields, poll_commands })
    }

    /// Status dict. A failing expression yields null for that key (unknown right now),
    /// never a fabricated value.
    pub fn evaluate(
        &self,
        results: &BTreeMap<String, Value>,
        state: &BTreeMap<String, Value>,
        ch: u8,
    ) -> BTreeMap<String, Value> {
        let mut bound = results.clone();
        for c in &self.poll_commands {
            bound.entry(c.clone()).or_insert(Value::Null);
        }
        let vars = Vars { results: &bound, state, ch };
        self.fields
            .iter()
            .map(|(k, e, repr)| {
                let v = e.eval(vars).unwrap_or(Value::Null);
                (k.clone(), represent(v, *repr))
            })
            .collect()
    }
}

/// Apply the wire representation (`as: si` → SI value dict).
pub fn represent(v: Value, repr: Repr) -> Value {
    match (repr, &v) {
        (Repr::Si, Value::Number(n)) => n.as_f64().map(|x| si_value(x, None)).unwrap_or(Value::Null),
        (Repr::Si, Value::String(s)) => crate::parse::lenient_number(s).map(|x| si_value(x, Some(s))).unwrap_or(Value::Null),
        (Repr::Si, _) => Value::Null,
        _ => v,
    }
}
