//! What a class operation's setter accepts, from the device definition: the value spec
//! of the caller-supplied param of its first command (type, limits, wire format, unit,
//! enum values) and whether it can energise an output. Used by `/methods` (UIs build
//! controls from it) and `/ai/context`.

use bm_devicedef::{DeviceDef, Operation, Param, Safety, ValueKind};
use serde_json::{json, Map, Value};

/// The setter's caller param and whether the command energises (None: no setter).
fn set_param<'a>(def: &'a DeviceDef, op: &Operation) -> Option<(Option<&'a Param>, bool)> {
    let cmd = op.set.as_ref()?.steps().first().and_then(|s| def.commands.get(s.command()))?;
    Some((cmd.params.values().find(|p| p.from.is_none()), cmd.safety == Safety::Energize))
}

/// `{"type": "number"|"integer"|"bool"|"string"|"enum", min?, max?, format?, unit?, values?}`
pub fn value_spec(p: &Param) -> Value {
    let kind = match p.kind {
        ValueKind::Number => "number",
        ValueKind::Integer => "integer",
        ValueKind::Bool => "bool",
        ValueKind::Enum => "enum",
        _ => "string",
    };
    let mut v = Map::new();
    v.insert("type".into(), json!(kind));
    if p.kind == ValueKind::Enum {
        v.insert("values".into(), json!(p.values.keys().collect::<Vec<_>>()));
    }
    for (k, x) in [("min", p.min), ("max", p.max)] {
        if let Some(x) = x {
            v.insert(k.into(), json!(x));
        }
    }
    for (k, x) in [("format", &p.format), ("unit", &p.unit)] {
        if let Some(x) = x {
            v.insert(k.into(), json!(x));
        }
    }
    Value::Object(v)
}

/// Value spec and energise flag of an operation's setter (None: the operation has no setter).
pub fn setter(def: &DeviceDef, op: &Operation) -> Option<(Option<Value>, bool)> {
    set_param(def, op).map(|(p, energizes)| (p.map(value_spec), energizes))
}
