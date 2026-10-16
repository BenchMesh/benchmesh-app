//! Command `send` templates: `{ch}`, `{<param>}`, `{state.<var>}`, `{value}` (step args only).

use regex::Regex;

use crate::model::{Command, ValueKind};

/// A parsed template piece.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Literal(String),
    Placeholder(String),
}

/// Split a template into literals and placeholders. `{{` / `}}` are literal braces.
pub fn parse(template: &str) -> Result<Vec<Piece>, String> {
    let mut pieces = Vec::new();
    let mut lit = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                lit.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                lit.push('}');
            }
            '{' => {
                let mut name = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(ch) => name.push(ch),
                        None => return Err(format!("unclosed `{{` in template `{template}`")),
                    }
                }
                if name.is_empty() {
                    return Err(format!("empty placeholder in template `{template}`"));
                }
                if !lit.is_empty() {
                    pieces.push(Piece::Literal(std::mem::take(&mut lit)));
                }
                pieces.push(Piece::Placeholder(name));
            }
            '}' => return Err(format!("unmatched `}}` in template `{template}`")),
            _ => lit.push(c),
        }
    }
    if !lit.is_empty() {
        pieces.push(Piece::Literal(lit));
    }
    Ok(pieces)
}

pub fn placeholders(template: &str) -> Result<Vec<String>, String> {
    Ok(parse(template)?
        .into_iter()
        .filter_map(|p| match p {
            Piece::Placeholder(n) => Some(n),
            Piece::Literal(_) => None,
        })
        .collect())
}

const NUMBER_RE: &str = r"[-+]?(?:[0-9]+\.?[0-9]*|\.[0-9]+)(?:[eE][-+]?[0-9]+)?";

/// Regex matching any wire text this template (+ tx terminator) can produce, ignoring
/// parameter types (every placeholder except `{ch}` matches anything).
pub fn wire_regex(template: &str, tx_terminator: &str) -> Result<Regex, String> {
    build_regex(template, tx_terminator, |_| None)
}

/// Like [`wire_regex`], but each param placeholder only matches what the param can
/// produce on the wire: enum → its wire values, number → a decimal/exponent number,
/// integer → digits, string → its `pattern`. Used to check captured traffic.
pub fn command_wire_regex(cmd: &Command, tx_terminator: &str) -> Result<Regex, String> {
    build_regex(&cmd.send, tx_terminator, |name| {
        let p = cmd.params.get(name)?;
        Some(match p.kind {
            ValueKind::Enum => {
                let alts: Vec<String> = p.values.values().map(|v| regex::escape(v)).collect();
                format!("(?:{})", alts.join("|"))
            }
            ValueKind::Number => NUMBER_RE.to_string(),
            ValueKind::Integer => r"[-+]?[0-9]+".to_string(),
            ValueKind::Bool => "(?:0|1|ON|OFF|true|false)".to_string(),
            _ => match &p.pattern {
                Some(re) => format!("(?:{})", re.trim_start_matches('^').trim_end_matches('$')),
                None => ".+?".to_string(),
            },
        })
    })
}

fn build_regex(template: &str, tx_terminator: &str, param: impl Fn(&str) -> Option<String>) -> Result<Regex, String> {
    let mut re = String::from("^");
    for p in parse(template)? {
        match p {
            Piece::Literal(l) => re.push_str(&regex::escape(&l)),
            Piece::Placeholder(n) if n == "ch" => re.push_str(r"\d+"),
            Piece::Placeholder(n) => re.push_str(&param(&n).unwrap_or_else(|| ".+?".to_string())),
        }
    }
    re.push_str(&regex::escape(tx_terminator));
    re.push('$');
    Regex::new(&re).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_placeholders_and_escapes() {
        assert_eq!(
            parse("VSET{ch}:{v}").unwrap(),
            vec![
                Piece::Literal("VSET".into()),
                Piece::Placeholder("ch".into()),
                Piece::Literal(":".into()),
                Piece::Placeholder("v".into())
            ]
        );
        assert_eq!(parse("A{{B}}").unwrap(), vec![Piece::Literal("A{B}".into())]);
        assert!(parse("A{b").is_err());
        assert!(parse("A}").is_err());
        assert!(parse("A{}").is_err());
    }

    #[test]
    fn wire_regex_matches_instances() {
        let re = wire_regex("VSET{ch}:{v}", "").unwrap();
        assert!(re.is_match("VSET1:5.00"));
        assert!(!re.is_match("VSET1?"));
        let re = wire_regex("MEAS:ALL:INFO?", "\r").unwrap();
        assert!(re.is_match("MEAS:ALL:INFO?\r"));
        assert!(!re.is_match("MEAS:ALL:INFO?"));
    }

    #[test]
    fn command_wire_regex_respects_param_types() {
        let cmd: Command = crate::from_yaml(
            "send: \"{on}\"\nparams: {on: {type: enum, values: {\"ON\": \"SYST:REM\", \"OFF\": \"SYST:LOC\"}}}\nreply: none\n",
        )
        .unwrap();
        let re = command_wire_regex(&cmd, "\r").unwrap();
        assert!(re.is_match("SYST:REM\r"));
        assert!(!re.is_match("SYST:REMOTE\r"));
        let cmd: Command = crate::from_yaml("send: \"VSET{ch}:{v}\"\nparams: {v: {type: number}}\n").unwrap();
        let re = command_wire_regex(&cmd, "").unwrap();
        assert!(re.is_match("VSET1:5.0") && re.is_match("VSET1:1.5e1") && !re.is_match("VSET1:abc"));
    }
}
