//! Cross-check device definitions against protocol test vectors of independent libraries
//! (PyMeasure, QCoDeS, InstrumentKit), extracted by `scripts/libcheck/extract.py`.
//! Dev tooling, not shipped.
//!
//! ```text
//! cargo run -q -p bm-driver --example crosscheck -- [--vectors DIR] [--mapping FILE]
//!     [--devices DIR] [--out DIR] [--only id,id]
//! ```
//!
//! For every mapped definition it reports
//!  (a) replies: vector queries that match one of our commands (SCPI-normalised) are run
//!      through our parse pipeline;
//!  (b) writes: vector setters are matched against our `send` templates (params as
//!      wildcards, enum params expanded to their wire values) and the value is checked
//!      against our param spec (format, min/max);
//!  (c) our polled queries and setters no vector exercises.
//!
//! Normalisation: case-insensitive; long mnemonics shortened by the SCPI rule (first four
//! letters, three when the fourth is a vowel); optional leading `:`; numeric node suffix
//! defaulting to 1; numbers compared as numbers; `;`-joined commands split with SCPI path
//! rules. A second, looser pass drops commonly optional nodes (`SOUR`/`SENS` at the root,
//! `IMM`, `AMPL`, `LEV`, `STAT`, `DC` below it; not `SCALar`, which
//! collides with `SCALe`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use bm_devicedef::template::{self, Piece};
use bm_devicedef::{Command, DeviceDef, ValueKind};
use bm_driver::parse::{lenient_number, parse_reply};
use bm_driver::wire::format_param;
use serde::Deserialize;
use serde_json::Value;

const OPEN: char = '\u{1}';
const CLOSE: char = '\u{2}';

// ------------------------------------------------------------------------------ inputs

#[derive(Deserialize, Clone)]
struct Exchange {
    write: Option<String>,
    reply: Option<String>,
}

#[derive(Deserialize, Clone)]
struct Vector {
    library: String,
    #[allow(dead_code)]
    file: String,
    test: String,
    #[allow(dead_code)]
    instrument_class: String,
    exchanges: Vec<Exchange>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize)]
struct MapEntry {
    #[serde(default)]
    transport: Option<String>,
    vectors: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    note: Option<String>,
}

// ------------------------------------------------------------------------- normalising

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Word(String),
    Wild(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Name(String, u32),
    Wild(String),
}

#[derive(Debug, Clone)]
struct Norm {
    nodes: Vec<Node>,
    query: bool,
    args: Vec<Tok>,
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'A' | 'E' | 'I' | 'O' | 'U')
}

/// SCPI short form of a mnemonic (already upper case, without numeric suffix).
fn short_form(m: &str) -> String {
    if m.len() <= 4 || !m.chars().all(|c| c.is_ascii_alphabetic()) {
        return m.to_string();
    }
    let c: Vec<char> = m.chars().collect();
    if is_vowel(c[3]) {
        c[..3].iter().collect()
    } else {
        c[..4].iter().collect()
    }
}

fn node(raw: &str) -> Node {
    if raw.contains(OPEN) {
        let name = raw.split(OPEN).nth(1).and_then(|r| r.split(CLOSE).next()).unwrap_or("?");
        return Node::Wild(name.to_string());
    }
    let up = raw.to_ascii_uppercase();
    let digits = up.len() - up.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    let (name, suffix) = up.split_at(up.len() - digits);
    if name.is_empty() || name.starts_with('*') {
        return Node::Name(up.clone(), 1);
    }
    Node::Name(short_form(name), suffix.parse().unwrap_or(1))
}

fn split_outside_quotes(s: &str, sep: char) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut quote: Option<char> = None;
    for c in s.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == sep => {
                out.push(String::new());
                continue;
            }
            _ => {}
        }
        out.last_mut().unwrap().push(c);
    }
    out
}

fn arg_tok(raw: &str) -> Tok {
    let t = raw.trim();
    if t.contains(OPEN) {
        let name = t.split(OPEN).nth(1).and_then(|r| r.split(CLOSE).next()).unwrap_or("?");
        return Tok::Wild(name.to_string());
    }
    if let Some(n) = lenient_number(t) {
        return Tok::Num(n);
    }
    let unq = t.trim_matches(|c| c == '"' || c == '\'');
    let up = unq.to_ascii_uppercase();
    let parts: Vec<String> = up
        .split(':')
        .map(|p| {
            let digits = p.len() - p.trim_end_matches(|c: char| c.is_ascii_digit()).len();
            let (n, d) = p.split_at(p.len() - digits);
            format!("{}{d}", short_form(n))
        })
        .collect();
    Tok::Word(parts.join(":"))
}

/// One SCPI program message unit. `prefix` is the header path a relative unit after `;` uses.
fn normalize_unit(unit: &str) -> Norm {
    let u = unit.trim();
    let (header, args) = match u.find(|c: char| c.is_whitespace()) {
        Some(i) => (&u[..i], u[i..].trim()),
        None => (u, ""),
    };
    let query = header.ends_with('?');
    let header = header.trim_end_matches('?').trim_start_matches(':');
    let nodes = header.split(':').filter(|n| !n.is_empty()).map(node).collect();
    let args = if args.is_empty() { Vec::new() } else { split_outside_quotes(args, ',').iter().map(|a| arg_tok(a)).collect() };
    Norm { nodes, query, args }
}

/// A whole write, split on `;` with SCPI path rules: a unit after `;` without a leading `:`
/// or `*` is relative to the previous unit's path (its last node replaced).
fn normalize(write: &str) -> Vec<Norm> {
    let mut out: Vec<Norm> = Vec::new();
    for part in split_outside_quotes(write.trim(), ';') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        let mut n = normalize_unit(p);
        let relative = !p.starts_with(':') && !p.starts_with('*');
        if relative {
            if let Some(prev) = out.last() {
                let common = prev.nodes.first().is_some_and(|f| matches!(f, Node::Name(s, _) if s.starts_with('*')));
                if !common && prev.nodes.len() > 1 {
                    let mut nodes = prev.nodes[..prev.nodes.len() - 1].to_vec();
                    nodes.extend(n.nodes);
                    n.nodes = nodes;
                }
            }
        }
        out.push(n);
    }
    out
}

const OPTIONAL_ROOT: &[&str] = &["SOUR", "SENS"];
const OPTIONAL_INNER: &[&str] = &["IMM", "AMPL", "LEV", "STAT", "DC"];

fn loosen(n: &Norm) -> Norm {
    let mut nodes = Vec::new();
    for (i, nd) in n.nodes.iter().enumerate() {
        if let Node::Name(name, sfx) = nd {
            let drop = if i == 0 { n.nodes.len() > 1 && *sfx == 1 && OPTIONAL_ROOT.contains(&name.as_str()) } else { OPTIONAL_INNER.contains(&name.as_str()) };
            if drop {
                continue;
            }
        }
        nodes.push(nd.clone());
    }
    Norm { nodes, ..n.clone() }
}

#[derive(Default, Debug, Clone)]
struct Binding {
    params: BTreeMap<String, String>,
    bool_spelling: bool,
}

fn match_header(ours: &Norm, theirs: &Norm, b: &mut Binding) -> bool {
    if ours.query != theirs.query || ours.nodes.len() != theirs.nodes.len() {
        return false;
    }
    for (o, t) in ours.nodes.iter().zip(&theirs.nodes) {
        match (o, t) {
            (Node::Wild(p), Node::Name(n, s)) => {
                b.params.insert(p.clone(), if *s == 1 { n.clone() } else { format!("{n}{s}") });
            }
            (a, b2) if a == b2 => {}
            _ => return false,
        }
    }
    true
}

fn num_eq(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs())
}

fn match_args(ours: &[Tok], theirs: &[Tok], raw_theirs: &[String], b: &mut Binding) -> bool {
    if ours.len() != theirs.len() {
        return false;
    }
    for (i, (o, t)) in ours.iter().zip(theirs).enumerate() {
        match (o, t) {
            (Tok::Wild(p), _) => {
                b.params.insert(p.clone(), raw_theirs.get(i).cloned().unwrap_or_default());
            }
            (Tok::Num(x), Tok::Num(y)) if num_eq(*x, *y) => {}
            (Tok::Word(x), Tok::Word(y)) if x == y => {}
            (Tok::Word(w), Tok::Num(n)) | (Tok::Num(n), Tok::Word(w)) if (w == "ON" && *n == 1.0) || (w == "OFF" && *n == 0.0) => {
                b.bool_spelling = true;
            }
            _ => return false,
        }
    }
    true
}

fn raw_args(unit: &str) -> Vec<String> {
    let u = unit.trim();
    match u.find(|c: char| c.is_whitespace()) {
        Some(i) => split_outside_quotes(u[i..].trim(), ',').iter().map(|s| s.trim().to_string()).collect(),
        None => Vec::new(),
    }
}

// ------------------------------------------------------------------------ our commands

/// One concrete spelling of one of our commands: enum params and `{ch}` expanded, other
/// params as wildcards.
struct Candidate {
    cmd: String,
    text: String,
    units: Vec<Norm>,
}

fn candidates(name: &str, cmd: &Command, max_ch: u32) -> Vec<Candidate> {
    let Ok(pieces) = template::parse(&cmd.send) else { return Vec::new() };
    let mut texts = vec![String::new()];
    for p in pieces {
        let options: Vec<String> = match &p {
            Piece::Literal(l) => vec![l.clone()],
            Piece::Placeholder(ph) if ph == "ch" => (1..=max_ch).map(|c| c.to_string()).collect(),
            Piece::Placeholder(ph) => match cmd.params.get(ph) {
                Some(spec) if spec.kind == ValueKind::Enum => spec.values.values().cloned().collect(),
                _ => vec![format!("{OPEN}{ph}{CLOSE}")],
            },
        };
        texts = texts.iter().flat_map(|t| options.iter().map(move |o| format!("{t}{o}"))).take(4000).collect();
    }
    texts
        .into_iter()
        .map(|t| {
            let units = normalize(&t);
            Candidate { cmd: name.to_string(), text: t, units }
        })
        .collect()
}

fn max_channels(def: &DeviceDef) -> u32 {
    let mut n = def.classes.values().map(|c| c.channels as u32).max().unwrap_or(1);
    for m in &def.models {
        n = n.max(m.channels.values().map(|c| *c as u32).max().unwrap_or(1));
    }
    n.clamp(1, 8)
}

/// Commands the class bindings use: (polled queries, setters incl. their steps).
fn used_commands(def: &DeviceDef) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut polled = BTreeSet::new();
    let mut setters = BTreeSet::new();
    for b in def.classes.values() {
        for g in &b.poll {
            polled.extend(g.read.iter().cloned());
        }
        for op in b.operations.values() {
            if let Some(a) = &op.get {
                for s in a.steps() {
                    polled.insert(s.command().to_string());
                }
            }
            if let Some(a) = &op.set {
                for s in a.steps() {
                    setters.insert(s.command().to_string());
                }
            }
        }
    }
    (polled, setters)
}

// ------------------------------------------------------------------------------ checks

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Class {
    Equivalent,
    EquivalentLoose,
    Rejected,
    Different,
    Missing,
}

impl Class {
    fn label(self) -> &'static str {
        match self {
            Class::Equivalent => "equivalent",
            Class::EquivalentLoose => "equivalent (optional nodes)",
            Class::Rejected => "value rejected / changed",
            Class::Different => "same header, other spelling/value",
            Class::Missing => "no such command",
        }
    }
}

struct Outcome {
    class: Class,
    cmd: Option<String>,
    ours: String,
    note: String,
}

struct Family<'a> {
    def: &'a DeviceDef,
    cands: Vec<Candidate>,
    rx_strip: String,
}

impl<'a> Family<'a> {
    fn new(def: &'a DeviceDef, transport: Option<&str>) -> Family<'a> {
        let max_ch = max_channels(def);
        let cands = def.commands.iter().flat_map(|(n, c)| candidates(n, c, max_ch)).collect();
        let framing = def.for_transport(transport).map(|d| d.framing).unwrap_or_else(|_| def.framing.clone());
        let rx_strip = format!("{}{}\r\n", framing.rx.strip_trailing, framing.rx.terminator.clone().unwrap_or_default());
        Family { def, cands, rx_strip }
    }

    /// Best match of one vector unit (or a whole multi-unit write) among our candidates.
    fn find(&self, theirs: &[Norm], raw_units: &[String]) -> Option<(usize, Binding, bool)> {
        for loose in [false, true] {
            for (i, c) in self.cands.iter().enumerate() {
                if c.units.len() != theirs.len() {
                    continue;
                }
                let mut b = Binding::default();
                let ok = c.units.iter().zip(theirs).zip(raw_units).all(|((o, t), raw)| {
                    let (o, t) = if loose { (loosen(o), loosen(t)) } else { (o.clone(), t.clone()) };
                    match_header(&o, &t, &mut b) && match_args(&o.args, &t.args, &raw_args(raw), &mut b)
                });
                if ok {
                    return Some((i, b, loose));
                }
            }
        }
        None
    }

    /// Our spellings with the same header (loosely) but other arguments, sentinels shown as
    /// `{param}`; the command name of the first.
    fn find_header(&self, theirs: &Norm) -> Option<(String, String)> {
        let t = loosen(theirs);
        let hits: Vec<&Candidate> = self
            .cands
            .iter()
            .filter(|c| c.units.iter().any(|u| match_header(&loosen(u), &t, &mut Binding::default())))
            .collect();
        let first = hits.first()?;
        let mut texts: Vec<String> = hits.iter().map(|c| c.text.replace(OPEN, "{").replace(CLOSE, "}")).collect();
        texts.dedup();
        let more = texts.len().saturating_sub(4);
        let mut shown = texts.into_iter().take(4).collect::<Vec<_>>().join(" / ");
        if more > 0 {
            let _ = write!(shown, " / … ({more} more)");
        }
        Some((first.cmd.clone(), shown))
    }

    fn render(&self, cand: &Candidate, b: &Binding) -> (String, Vec<String>) {
        let cmd = &self.def.commands[&cand.cmd];
        let mut out = cand.text.clone();
        let mut problems = Vec::new();
        for (p, raw) in &b.params {
            let Some(spec) = cmd.params.get(p) else { continue };
            let wire = if spec.from.is_some() {
                raw.clone()
            } else {
                match format_param(p, spec, raw) {
                    Ok(w) => {
                        let same = match (lenient_number(&w), lenient_number(raw)) {
                            (Some(a), Some(b)) => num_eq(a, b),
                            _ => w.eq_ignore_ascii_case(raw),
                        };
                        if !same {
                            problems.push(format!("`{p}`: theirs {raw} → ours {w}"));
                        }
                        w
                    }
                    Err(e) => {
                        problems.push(e.to_string());
                        raw.clone()
                    }
                }
            };
            out = out.replace(&format!("{OPEN}{p}{CLOSE}"), &wire);
        }
        (out, problems)
    }

    fn check_write(&self, write: &str) -> Vec<(String, Outcome)> {
        let units = normalize(write);
        let raws: Vec<String> = split_outside_quotes(write.trim(), ';').into_iter().filter(|s| !s.trim().is_empty()).collect();
        // the whole write first (our command may itself be `A;B`), then unit by unit
        if units.len() > 1 {
            if let Some((i, b, loose)) = self.find(&units, &raws) {
                return vec![(write.to_string(), self.outcome(i, &b, loose))];
            }
        }
        units
            .iter()
            .zip(&raws)
            .map(|(u, raw)| {
                let shown = if units.len() > 1 { format!("{} (part of `{}`)", raw.trim(), write.trim()) } else { raw.trim().to_string() };
                let o = match self.find(std::slice::from_ref(u), std::slice::from_ref(raw)) {
                    Some((i, b, loose)) => self.outcome(i, &b, loose),
                    None => match self.find_header(u) {
                        Some((cmd, shown)) => Outcome { class: Class::Different, cmd: Some(cmd), ours: shown, note: String::new() },
                        None => Outcome { class: Class::Missing, cmd: None, ours: String::new(), note: String::new() },
                    },
                };
                (shown, o)
            })
            .collect()
    }

    fn outcome(&self, i: usize, b: &Binding, loose: bool) -> Outcome {
        let cand = &self.cands[i];
        let (ours, problems) = self.render(cand, b);
        let mut note = problems.join("; ");
        if b.bool_spelling {
            if !note.is_empty() {
                note.push_str("; ");
            }
            note.push_str("boolean spelled ON/OFF vs 1/0");
        }
        let class = if !problems.is_empty() {
            Class::Rejected
        } else if loose {
            Class::EquivalentLoose
        } else {
            Class::Equivalent
        };
        Outcome { class, cmd: Some(cand.cmd.clone()), ours, note }
    }

    fn decode(&self, cmd: &str, reply: &str) -> Result<Value, String> {
        let c = &self.def.commands[cmd];
        let set = self.rx_strip.as_bytes();
        // extract.py decodes bytes as latin-1: one char per byte
        let bytes: Vec<u8> = reply.chars().map(|ch| ch as u32 as u8).collect();
        let end = bytes.iter().rposition(|b| !set.contains(b)).map(|i| i + 1).unwrap_or(0);
        parse_reply(c, &bytes[..end]).map_err(|e| e.to_string())
    }
}

// ------------------------------------------------------------------------------ report

#[derive(Default)]
struct Row {
    count: usize,
    first_test: String,
    cells: Vec<String>,
}

fn md(s: &str) -> String {
    let e = s.replace('|', "\\|").replace('\n', "\\n").replace('\r', "\\r");
    if e.is_empty() {
        String::new()
    } else {
        format!("`{e}`")
    }
}

struct Summary {
    id: String,
    vectors: usize,
    queries_matched: usize,
    parse_ok: usize,
    parse_fail: usize,
    queries_missing: usize,
    writes: BTreeMap<Class, usize>,
    unseen: usize,
}

fn check_family(id: &str, def: &DeviceDef, entry: &MapEntry, vectors_dir: &Path) -> (String, Summary) {
    let fam = Family::new(def, entry.transport.as_deref());
    let mut vectors: Vec<Vector> = Vec::new();
    let mut sources = Vec::new();
    for (lib, classes) in &entry.vectors {
        for cls in classes {
            let p = vectors_dir.join(lib).join(format!("{cls}.json"));
            match fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|t| serde_json::from_str::<Vec<Vector>>(&t).map_err(|e| e.to_string())) {
                Ok(v) => {
                    sources.push(format!("{lib} `{cls}` ({} tests)", v.len()));
                    vectors.extend(v);
                }
                Err(e) => sources.push(format!("{lib} `{cls}`: **not loaded** ({e})")),
            }
        }
    }

    let mut replies: BTreeMap<(String, String), Row> = BTreeMap::new();
    let mut no_reply_q: BTreeMap<String, Row> = BTreeMap::new();
    let mut missing_q: BTreeMap<String, Row> = BTreeMap::new();
    let mut writes: BTreeMap<(Class, String), Row> = BTreeMap::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut s = Summary { id: id.into(), vectors: vectors.len(), queries_matched: 0, parse_ok: 0, parse_fail: 0, queries_missing: 0, writes: BTreeMap::new(), unseen: 0 };

    for v in &vectors {
        let test = format!("{} {}", v.library, v.test);
        for ex in &v.exchanges {
            let Some(w) = ex.write.as_deref() else { continue };
            let units = normalize(w);
            let is_query = units.len() == 1 && units[0].query;
            if is_query {
                let raws = vec![w.to_string()];
                let key = w.trim().to_string();
                match fam.find(&units, &raws) {
                    Some((i, _b, _loose)) => {
                        let cmd = fam.cands[i].cmd.clone();
                        seen.insert(cmd.clone());
                        match &ex.reply {
                            Some(r) => {
                                let row = replies.entry((key.clone(), r.clone())).or_default();
                                if row.count == 0 {
                                    s.queries_matched += 1;
                                    let (res, ok) = match fam.decode(&cmd, r) {
                                        Ok(val) => (format!("ok → {}", md(&val.to_string())), true),
                                        Err(e) => (format!("**parse failure**: {}", e.replace('|', "\\|")), false),
                                    };
                                    if ok {
                                        s.parse_ok += 1;
                                    } else {
                                        s.parse_fail += 1;
                                    }
                                    row.cells = vec![md(&key), md(r), md(&cmd), res];
                                    row.first_test = test.clone();
                                }
                                row.count += 1;
                            }
                            None => {
                                let row = no_reply_q.entry(key.clone()).or_default();
                                if row.count == 0 {
                                    row.cells = vec![md(&key), md(&cmd)];
                                    row.first_test = test.clone();
                                }
                                row.count += 1;
                            }
                        }
                    }
                    None => {
                        let row = missing_q.entry(key.clone()).or_default();
                        if row.count == 0 {
                            s.queries_missing += 1;
                            let hint = fam.find_header(&units[0]).map(|(c, shown)| format!("same header, other args: `{c}` {}", md(&shown))).unwrap_or_default();
                            row.cells = vec![md(&key), md(ex.reply.as_deref().unwrap_or("")), hint];
                            row.first_test = test.clone();
                        }
                        row.count += 1;
                    }
                }
                continue;
            }
            for (shown, o) in fam.check_write(w) {
                if let (Some(c), Class::Equivalent | Class::EquivalentLoose | Class::Rejected) = (&o.cmd, o.class) {
                    seen.insert(c.clone());
                }
                let row = writes.entry((o.class, shown.clone())).or_default();
                if row.count == 0 {
                    *s.writes.entry(o.class).or_default() += 1;
                    row.cells = vec![md(&shown), o.class.label().to_string(), o.cmd.as_deref().map(md).unwrap_or_default(), md(&o.ours), o.note.clone()];
                    row.first_test = test.clone();
                }
                row.count += 1;
            }
        }
    }

    let (polled, setters) = used_commands(def);
    let unseen_q: Vec<&String> = polled.iter().filter(|c| !seen.contains(*c)).collect();
    let unseen_s: Vec<&String> = setters.iter().filter(|c| !seen.contains(*c)).collect();
    s.unseen = unseen_q.len() + unseen_s.len();

    let mut out = String::new();
    let _ = writeln!(out, "# Cross-check: `{id}`\n");
    let _ = writeln!(out, "Generated by `cargo run -q -p bm-driver --example crosscheck` from library protocol test vectors");
    let _ = writeln!(out, "(`scripts/libcheck/`). Library vectors are other people's expectations, not instrument captures:");
    let _ = writeln!(out, "every disagreement is decided against the manual (see the review section, if any, at the end).\n");
    let _ = writeln!(out, "- Definition: {} {} (`{id}`), transport `{}`", def.vendor, def.family, entry.transport.as_deref().unwrap_or("default"));
    let _ = writeln!(out, "- Vectors: {}", sources.join(", "));
    if let Some(n) = &entry.note {
        let _ = writeln!(out, "- Mapping note: {n}");
    }
    let notes: BTreeSet<&str> = vectors.iter().filter_map(|v| v.note.as_deref()).filter(|n| !n.starts_with("pairing")).collect();
    for n in notes {
        let _ = writeln!(out, "- Vector note: {n}");
    }
    let w = |c: Class| s.writes.get(&c).copied().unwrap_or(0);
    let _ = writeln!(
        out,
        "\n| replies decoded | parse failures | their queries we lack | writes equivalent | optional-node equivalent | value rejected/changed | different spelling | no such command | ours unseen |\n|---|---|---|---|---|---|---|---|---|\n| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
        s.parse_ok,
        s.parse_fail,
        s.queries_missing,
        w(Class::Equivalent),
        w(Class::EquivalentLoose),
        w(Class::Rejected),
        w(Class::Different),
        w(Class::Missing),
        s.unseen
    );

    let _ = writeln!(out, "## (a) Replies through our parse pipeline\n");
    let (fails, oks): (Vec<&Row>, Vec<&Row>) = replies.values().partition(|r| r.cells[3].starts_with("**"));
    if replies.is_empty() {
        let _ = writeln!(out, "_No vector query with a reply matches one of our commands._\n");
    }
    const REPLY_HEAD: &str = "| their query | their reply | our command | result | seen | first test |\n|---|---|---|---|---|---|";
    if !fails.is_empty() {
        let _ = writeln!(out, "### Parse failures\n\n{REPLY_HEAD}");
        table(&mut out, &fails);
    }
    if !oks.is_empty() {
        let _ = writeln!(out, "<details><summary>{} replies decoded</summary>\n\n{REPLY_HEAD}", oks.len());
        table(&mut out, &oks);
        let _ = writeln!(out, "</details>\n");
    }
    if !no_reply_q.is_empty() {
        let _ = writeln!(out, "Matched queries without a reply in the vectors: {}\n", no_reply_q.values().map(|r| format!("{} → {}", r.cells[0], r.cells[1])).collect::<Vec<_>>().join(", "));
    }
    if !missing_q.is_empty() {
        let rows: Vec<&Row> = missing_q.values().collect();
        let _ = writeln!(out, "### Their queries we have no command for\n\n| their query | their reply | hint | seen | first test |\n|---|---|---|---|---|");
        table(&mut out, &rows);
    }

    let _ = writeln!(out, "## (b) Their setters against our send templates\n");
    if writes.is_empty() {
        let _ = writeln!(out, "_No setter vectors._\n");
    }
    const WRITE_HEAD: &str = "| their write | verdict | our command | ours on the wire / our spellings | note | seen | first test |\n|---|---|---|---|---|---|---|";
    let group = |classes: &[Class]| -> Vec<&Row> { writes.iter().filter(|((c, _), _)| classes.contains(c)).map(|(_, r)| r).collect() };
    let disagree = group(&[Class::Rejected, Class::Different]);
    if !disagree.is_empty() {
        let _ = writeln!(out, "### Disagreements (decide with the manual)\n\n{WRITE_HEAD}");
        table(&mut out, &disagree);
    }
    let agree = group(&[Class::Equivalent, Class::EquivalentLoose]);
    if !agree.is_empty() {
        let _ = writeln!(out, "<details><summary>{} writes equivalent</summary>\n\n{WRITE_HEAD}", agree.len());
        table(&mut out, &agree);
        let _ = writeln!(out, "</details>\n");
    }
    let missing = group(&[Class::Missing]);
    if !missing.is_empty() {
        let _ = writeln!(out, "### Their setters we have no command for\n");
        let _ = writeln!(out, "{}\n", missing.iter().map(|r| format!("{} ({})", r.cells[0], r.first_test)).collect::<Vec<_>>().join(", "));
    }

    let _ = writeln!(out, "## (c) Our commands no vector exercises\n");
    let fmt = |v: &[&String]| if v.is_empty() { "none".to_string() } else { v.iter().map(|c| format!("`{c}` ({})", md(&def.commands.get(*c).map(|x| x.send.clone()).unwrap_or_default()))).collect::<Vec<_>>().join(", ") };
    let _ = writeln!(out, "- Polled / get queries: {}", fmt(&unseen_q));
    let _ = writeln!(out, "- Setters: {}", fmt(&unseen_s));
    (out, s)
}

fn table(out: &mut String, rows: &[&Row]) {
    for r in rows {
        let _ = writeln!(out, "| {} | {} | {} |", r.cells.join(" | "), r.count, r.first_test);
    }
    let _ = writeln!(out);
}

// -------------------------------------------------------------------------------- main

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut vectors = root.join("../BenchMeshDOC/_vectors");
    let mut mapping = root.join("scripts/libcheck/mapping.yaml");
    let mut devices = root.join("devices");
    let mut out_dir = root.join("ai_context/migration/coverage/crosscheck");
    let mut only: Option<BTreeSet<String>> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || PathBuf::from(args.next().unwrap_or_else(|| panic!("{a} needs a value")));
        match a.as_str() {
            "--vectors" => vectors = val(),
            "--mapping" => mapping = val(),
            "--devices" => devices = val(),
            "--out" => out_dir = val(),
            "--only" => only = Some(val().to_string_lossy().split(',').map(str::to_string).collect()),
            other => {
                eprintln!("unknown argument {other}\nusage: crosscheck [--vectors DIR] [--mapping FILE] [--devices DIR] [--out DIR] [--only id,id]");
                std::process::exit(2);
            }
        }
    }
    let cat = bm_devicedef::load_catalog(&devices).unwrap_or_else(|e| panic!("devices: {}", e.join("\n")));
    let text = fs::read_to_string(&mapping).unwrap_or_else(|e| panic!("{}: {e}", mapping.display()));
    let map: BTreeMap<String, MapEntry> = bm_devicedef::from_yaml(&text).unwrap_or_else(|e| panic!("{}: {e}", mapping.display()));
    fs::create_dir_all(&out_dir).expect("create output dir");

    let mut summaries = Vec::new();
    for (id, entry) in &map {
        if only.as_ref().is_some_and(|o| !o.contains(id)) {
            continue;
        }
        let Some((_, def)) = cat.devices.get(id) else {
            eprintln!("mapping: unknown definition `{id}`");
            continue;
        };
        let (report, s) = check_family(id, def, entry, &vectors);
        let path = out_dir.join(format!("{id}.md"));
        // keep a hand-written review section (from `## Review` on) across regenerations
        let review = fs::read_to_string(&path).ok().and_then(|old| old.find("\n## Review").map(|i| old[i..].to_string())).unwrap_or_default();
        fs::write(&path, format!("{report}{review}")).expect("write report");
        println!(
            "{id:24} vectors {:4}  replies ok {:3} fail {:2}  writes eq {:3}/{:3} diff {:2} rej {:2} missing {:3}  unseen {:2}",
            s.vectors,
            s.parse_ok,
            s.parse_fail,
            s.writes.get(&Class::Equivalent).copied().unwrap_or(0),
            s.writes.get(&Class::EquivalentLoose).copied().unwrap_or(0),
            s.writes.get(&Class::Different).copied().unwrap_or(0),
            s.writes.get(&Class::Rejected).copied().unwrap_or(0),
            s.writes.get(&Class::Missing).copied().unwrap_or(0),
            s.unseen
        );
        summaries.push((s, entry.vectors.iter().map(|(l, c)| format!("{l}: {}", c.join(", "))).collect::<Vec<_>>().join("; ")));
    }
    if only.is_none() {
        let mut idx = String::from(
            "# Library cross-check summary\n\nGenerated by `cargo run -q -p bm-driver --example crosscheck` (vectors: `scripts/libcheck/README.md`).\nCounts are distinct commands/exchanges. A per-family report has a hand-written `## Review` section once its disagreements were decided against the manual.\n\n| definition | vectors from | tests | replies decoded | parse failures | their queries we lack | writes equivalent (+ optional nodes) | value rejected/changed | different spelling | no such command | ours unseen |\n|---|---|---|---|---|---|---|---|---|---|---|\n",
        );
        for (s, from) in &summaries {
            let w = |c: Class| s.writes.get(&c).copied().unwrap_or(0);
            let _ = writeln!(
                idx,
                "| [`{}`]({}.md) | {} | {} | {} | {} | {} | {} (+{}) | {} | {} | {} | {} |",
                s.id,
                s.id,
                from,
                s.vectors,
                s.parse_ok,
                s.parse_fail,
                s.queries_missing,
                w(Class::Equivalent),
                w(Class::EquivalentLoose),
                w(Class::Rejected),
                w(Class::Different),
                w(Class::Missing),
                s.unseen
            );
        }
        let path = out_dir.join("README.md");
        let tail = fs::read_to_string(&path).ok().and_then(|old| old.find("\n## Pilot").map(|i| old[i..].to_string())).unwrap_or_default();
        fs::write(path, format!("{idx}{tail}")).expect("write summary");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_forms() {
        for (l, s) in [("VOLTAGE", "VOLT"), ("LEVEL", "LEV"), ("FREQUENCY", "FREQ"), ("POWER", "POW"), ("RATE", "RATE"), ("IMPEDANCE", "IMP"), ("DCYCLE", "DCYC")] {
            assert_eq!(short_form(l), s);
        }
    }

    #[test]
    fn compound_paths() {
        let u = normalize(":SENS:VOLT:RANG:AUTO 0;:SENS:VOLT:RANG 234");
        assert_eq!(u.len(), 2);
        let u = normalize("VOLT:RANG 1;AUTO OFF");
        assert_eq!(u[1].nodes, vec![Node::Name("VOLT".into(), 1), Node::Name("AUTO".into(), 1)]);
    }
}
