//! Replay the Phase 1 wire captures through the interpreter and compare with what the
//! Python service reported for the same bytes. No hardware needed.
//!
//! 1. every captured reply frames + parses under its definition
//! 2. every successful Python GET query gives the same value under the definition
//! 3. every status value Python published to the registry is also produced by the
//!    definition's class status (same SI dicts, same enums/bools)
//!
//! Deliberate differences (Python bugs, contract fixes) are listed with reasons.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use bm_devicedef::template::{self, command_wire_regex, Piece};
use bm_devicedef::{load_catalog, Catalog, Command, DeviceDef, Reply};
use bm_driver::framing::Framer;
use bm_driver::parse::parse_reply;
use bm_driver::status::{represent, CompiledStatus};
use serde::Deserialize;
use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

#[derive(Deserialize)]
struct Ev {
    ev: String,
    t: f64,
    #[serde(default)]
    hex: Option<String>,
}

/// One TX and every byte received until the next TX.
struct Exchange {
    t: f64,
    tx: String,
    rx: Vec<u8>,
}

fn unhex(h: &str) -> Vec<u8> {
    (0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap()).collect()
}

/// Each capture log is decoded once and shared by all tests.
fn exchanges(path: &Path) -> Arc<Vec<Exchange>> {
    static CACHE: OnceLock<Mutex<BTreeMap<PathBuf, Arc<Vec<Exchange>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(x) = cache.lock().unwrap().get(path) {
        return x.clone();
    }
    let x = Arc::new(load_exchanges(path));
    cache.lock().unwrap().insert(path.to_path_buf(), x.clone());
    x
}

fn load_exchanges(path: &Path) -> Vec<Exchange> {
    let rd = BufReader::new(flate2::read::GzDecoder::new(fs::File::open(path).unwrap()));
    let mut out: Vec<Exchange> = Vec::new();
    let mut open = false;
    for line in rd.lines() {
        let line = line.unwrap();
        let e: Ev = serde_json::from_str(&line).unwrap();
        match e.ev.as_str() {
            "tx" => {
                let b = unhex(e.hex.as_deref().unwrap());
                out.push(Exchange { t: e.t, tx: b.iter().map(|&c| c as char).collect(), rx: Vec::new() });
                open = true;
            }
            "rx" if open => out.last_mut().unwrap().rx.extend(unhex(e.hex.as_deref().unwrap())),
            "open" | "close" => open = false,
            _ => {}
        }
    }
    out
}

struct Session {
    dir: PathBuf,
    devices: Vec<(String, String)>, // (device id, driver)
}

fn sessions() -> Vec<Session> {
    let mut v = Vec::new();
    for e in fs::read_dir(repo().join("captures")).unwrap() {
        let dir = e.unwrap().path();
        let sj = dir.join("session.json");
        if !sj.exists() {
            continue;
        }
        let s: Value = serde_json::from_str(&fs::read_to_string(sj).unwrap()).unwrap();
        let devices = s["config"]["devices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| (d["id"].as_str().unwrap().to_string(), d["driver"].as_str().unwrap().to_string()))
            .collect();
        v.push(Session { dir, devices });
    }
    v.sort_by(|a, b| a.dir.cmp(&b.dir));
    v
}

fn catalog() -> Catalog {
    load_catalog(&repo().join("devices")).unwrap()
}

/// Precompiled wire regexes of one definition.
struct Matcher<'a> {
    cmds: Vec<(&'a str, &'a Command, regex::Regex)>,
}

impl<'a> Matcher<'a> {
    fn new(def: &'a DeviceDef) -> Self {
        let cmds = def
            .commands
            .iter()
            .map(|(n, c)| (n.as_str(), c, command_wire_regex(c, &def.framing.tx.terminator).unwrap()))
            .collect();
        Matcher { cmds }
    }

    /// Commands whose wire form matches `tx`, with the channel number (`{ch}`).
    fn matching(&self, tx: &str) -> Vec<(&'a str, &'a Command, u8)> {
        self.cmds.iter().filter(|(_, _, re)| re.is_match(tx)).map(|(n, c, _)| (*n, *c, channel_of(c, tx))).collect()
    }
}

fn channel_of(cmd: &Command, tx: &str) -> u8 {
    let mut prefix = String::new();
    for p in template::parse(&cmd.send).unwrap() {
        match p {
            Piece::Literal(l) => prefix.push_str(&l),
            Piece::Placeholder(n) if n == "ch" => {
                let digits: String = tx[prefix.len().min(tx.len())..].chars().take_while(|c| c.is_ascii_digit()).collect();
                return digits.parse().unwrap_or(1);
            }
            Piece::Placeholder(_) => return 1,
        }
    }
    1
}

/// Frame + parse one captured reply. `Ok(None)` = the device sent nothing.
///
/// `next` = bytes received during the following exchange. The Python service sometimes
/// stopped reading early (XDM: it ended on `\r` of a `\r\n` reply), so the end of a reply
/// was recorded in the next exchange; the device sent it contiguously.
fn decode(def: &DeviceDef, cmd: &Command, rx: &[u8], next: &[u8]) -> Result<Option<Value>, String> {
    if rx.is_empty() {
        return Ok(None);
    }
    let mut f = Framer::new(&def.framing.rx, &cmd.reply);
    let framed = match f.push(rx).map_err(|e| e.to_string())? {
        Some(fr) => fr,
        None => match f.push(next).map_err(|e| e.to_string())? {
            Some(fr) => fr,
            None => f.finish().map_err(|e| e.to_string())?,
        },
    };
    parse_reply(cmd, &framed.reply).map(Some).map_err(|e| e.to_string())
}

fn next_rx(xs: &[Exchange], i: usize) -> &[u8] {
    xs.get(i + 1).map(|x| x.rx.as_slice()).unwrap_or(&[])
}

// --------------------------------------------------------------------------------------
// 1. every captured reply parses
// --------------------------------------------------------------------------------------

#[test]
fn every_captured_reply_parses() {
    let cat = catalog();
    let mut ok = 0usize;
    let mut no_reply: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for s in sessions() {
        for (id, driver) in &s.devices {
            let log = s.dir.join(format!("{id}.jsonl.gz"));
            if !log.exists() {
                continue;
            }
            let def = &cat.devices[driver].1;
            let m = Matcher::new(def);
            let xs = exchanges(&log);
            for (i, x) in xs.iter().enumerate() {
                for (name, cmd, _) in m.matching(&x.tx) {
                    if cmd.reply == Reply::None {
                        continue;
                    }
                    match decode(def, cmd, &x.rx, next_rx(&xs, i)) {
                        Ok(Some(_)) => ok += 1,
                        Ok(None) => *no_reply.entry(format!("{driver}.{name}")).or_default() += 1,
                        Err(e) => {
                            let k = format!("{driver}.{name}");
                            let entry = failures.entry(k).or_insert((0, String::new()));
                            entry.0 += 1;
                            if entry.1.is_empty() {
                                entry.1 = format!("{e} [tx {:?}]", x.tx);
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("parsed {ok} replies; no reply: {no_reply:?}");
    assert!(ok > 10_000, "too few replies parsed: {ok}");
    let unexpected: Vec<String> = failures
        .iter()
        .filter(|(k, (n, _))| !KNOWN_BAD_REPLIES.iter().any(|(kk, max, _)| kk == k && n <= max))
        .map(|(k, (n, e))| format!("{k}: {n}x, e.g. {e}"))
        .collect();
    assert!(unexpected.is_empty(), "replies that do not parse:\n{}", unexpected.join("\n"));
}

/// (driver.command, max occurrences, reason) - replies that legitimately fail to parse.
const KNOWN_BAD_REPLIES: &[(&str, usize, &str)] = &[
    // The Python service read after XDM setters and timed out during the CONF:TEMP stall,
    // so later replies were recorded one command late (a FUNCtion? exchange holding a
    // measurement, MEAS1? holding the IDN). The definition's parsers reject these foreign
    // replies instead of publishing garbage - the behaviour the Rust engine relies on to
    // detect and drain a desynced stream.
    ("owon_xdm.func", 19, "reply stream shifted by Python (read-after-setter / TEMP stall)"),
    ("owon_xdm.meas", 18, "reply stream shifted by Python (read-after-setter / TEMP stall)"),
    ("owon_xdm.idn", 2, "reply stream shifted by Python (read-after-setter / TEMP stall)"),
];

// --------------------------------------------------------------------------------------
// 2. Python GET results == definition results for the same bytes
// --------------------------------------------------------------------------------------

#[derive(Debug)]
struct ApiCall {
    t: f64,
    dur: f64,
    path: String,
    status: i64,
    body: Value,
}

fn api_records(dir: &Path) -> Vec<Value> {
    let p = dir.join("api.jsonl.gz");
    if !p.exists() {
        return Vec::new();
    }
    BufReader::new(flate2::read::GzDecoder::new(fs::File::open(p).unwrap()))
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
        .collect()
}

/// Compare two values the way the UI consumes them: SI dicts by `si` + `sci` (Python
/// emitted `sci: "0"` for zero on one code path), everything else exactly.
fn same(python: &Value, ours: &Value) -> bool {
    let num = |v: &Value| v.as_str().and_then(|s| s.trim().parse::<f64>().ok());
    match (python, ours) {
        // Contract: numeric GETs are numbers / SI dicts; Python returned the raw reply text
        // for some drivers. The UI parseFloat()s both, so equal numbers are equivalent.
        (Value::String(_), Value::Number(o)) => num(python) == o.as_f64(),
        (Value::String(_), Value::Object(o)) if o.contains_key("si") => num(python).is_some() && num(python) == num(&o["sci"]),
        // Overload: Python produced an unparseable SI dict (`number: ""`, `sci: "OL"`); the
        // definition reports null (+ OVERLOAD status flag).
        (Value::Object(p), Value::Null) if p.get("si").and_then(|s| s.get("number")) == Some(&Value::String(String::new())) => true,
        (Value::Object(p), Value::Object(o)) if p.contains_key("si") => {
            let zero = |v: &Value| v.as_str().map(|s| if s == "0" { "0.00000E+00" } else { s }).map(str::to_string);
            p.get("si") == o.get("si") && zero(&p["sci"]) == zero(&o["sci"])
        }
        _ => python == ours,
    }
}

/// (driver, operation, max occurrences, reason) - GETs whose Python result is known to differ.
const KNOWN_API_DIFFS: &[(&str, &str, usize, &str)] = &[
    ("owon_xdm", "function", 1, "wire reply shifted by Python (see KNOWN_BAD_REPLIES); Python answered from its cache"),
];

#[test]
fn python_get_results_match_definitions() {
    let cat = catalog();
    let mut compared = 0usize;
    let mut mismatches: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for s in sessions() {
        let records = api_records(&s.dir);
        let calls: Vec<ApiCall> = records
            .iter()
            .filter(|r| r["verb"] == "GET" && r["path"].as_str().is_some_and(|p| p.matches('/').count() == 5))
            .map(|r| ApiCall {
                t: r["t"].as_f64().unwrap(),
                dur: r["dur_ms"].as_f64().unwrap_or(0.0) / 1000.0,
                path: r["path"].as_str().unwrap().to_string(),
                status: r["status"].as_i64().unwrap_or(0),
                body: r["body"].clone(),
            })
            .collect();
        let mut by_dev: BTreeMap<&str, Arc<Vec<Exchange>>> = BTreeMap::new();
        for (id, _) in &s.devices {
            let log = s.dir.join(format!("{id}.jsonl.gz"));
            if log.exists() {
                by_dev.insert(id, exchanges(&log));
            }
        }
        for call in calls.iter().filter(|c| c.status == 200) {
            // /instruments/{CLS}/{dev}/{ch}/{op}
            let parts: Vec<&str> = call.path.split('/').collect();
            let (cls, dev, ch, op) = (parts[2], parts[3], parts[4].parse::<u8>().unwrap_or(1), parts[5]);
            let Some((_, driver)) = s.devices.iter().find(|(id, _)| id == dev) else { continue };
            let def = &cat.devices[driver].1;
            let Some(oper) = def.classes.get(cls).and_then(|b| b.operations.get(op)) else { continue };
            let Some(get) = &oper.get else { continue };
            let steps = get.steps();
            let cmd_name = steps.last().unwrap().command();
            let cmd = &def.commands[cmd_name];
            let re = command_wire_regex(cmd, &def.framing.tx.terminator).unwrap();
            // The exchange this API call caused: same wire command, inside the call window.
            let Some(xs) = by_dev.get(dev) else { continue };
            let Some(i) = xs.iter().rposition(|x| {
                x.t >= call.t && x.t <= call.t + call.dur + 0.05 && re.is_match(&x.tx) && channel_of(cmd, &x.tx) == ch
            }) else {
                continue; // Python answered from its cache (no wire traffic)
            };
            let x = &xs[i];
            let ours = match decode(def, cmd, &x.rx, next_rx(xs, i)) {
                Ok(Some(v)) => represent(v, oper.repr),
                Ok(None) => continue,
                Err(e) => Value::String(format!("<error: {e}>")),
            };
            let python = call.body["value"].clone();
            compared += 1;
            if !same(&python, &ours) {
                let e = mismatches.entry(format!("{driver}.{cls}.{op}")).or_insert((0, String::new()));
                e.0 += 1;
                if e.1.is_empty() {
                    e.1 = format!("python {python} vs ours {ours} (rx {:?})", String::from_utf8_lossy(&x.rx));
                }
            }
        }
    }
    eprintln!("compared {compared} GET results");
    assert!(compared > 100, "too few GET results compared: {compared}");
    let report: Vec<String> = mismatches
        .iter()
        .filter(|(k, (n, _))| {
            !KNOWN_API_DIFFS.iter().any(|(d, o, max, _)| k.starts_with(&format!("{d}.")) && k.ends_with(&format!(".{o}")) && n <= max)
        })
        .map(|(k, (n, e))| format!("{k}: {n}x, e.g. {e}"))
        .collect();
    assert!(report.is_empty(), "GET results differ from Python:\n{}", report.join("\n"));
}

// --------------------------------------------------------------------------------------
// 3. every registry status value Python published is produced by the definitions
// --------------------------------------------------------------------------------------

/// (driver, class, key, reason) - registry keys whose Python values are known to differ.
const KNOWN_STATUS_DIFFS: &[(&str, &str, &str, &str)] = &[];

#[test]
fn python_registry_values_are_reproduced() {
    let cat = catalog();
    let canon = |v: &Value| -> String {
        // SI dicts: compare what the UI shows (si) + normalised sci
        if let Some(o) = v.as_object().filter(|o| o.contains_key("si")) {
            let sci = o["sci"].as_str().map(|s| if s == "0" { "0.00000E+00" } else { s }).unwrap_or("");
            format!("SI{}|{}", o["si"], sci)
        } else {
            v.to_string()
        }
    };
    // (driver, class, ch, key) -> values
    let mut python: BTreeMap<(String, String, String, String), BTreeSet<String>> = BTreeMap::new();
    let mut ours: BTreeMap<(String, String, String, String), BTreeSet<String>> = BTreeMap::new();
    for s in sessions() {
        let drivers: BTreeMap<&str, &str> = s.devices.iter().map(|(i, d)| (i.as_str(), d.as_str())).collect();
        for r in api_records(&s.dir) {
            if r["phase"] != "passive" {
                continue;
            }
            for (dev, entry) in r["body"].as_object().unwrap() {
                let Some(driver) = drivers.get(dev.as_str()) else { continue };
                for (cls, chans) in entry.as_object().unwrap() {
                    let Some(chans) = chans.as_object() else { continue };
                    for (chk, status) in chans {
                        for (k, v) in status.as_object().into_iter().flatten() {
                            // Contract: protection flags etc. are bool; Python passed raw ON/OFF.
                            let is_bool = cat.classes.get(cls).and_then(|c| c.status.get(k)).map(|s| s.kind)
                                == Some(bm_devicedef::ValueKind::Bool);
                            let v = match (is_bool, v.as_str()) {
                                (true, Some("ON" | "1")) => Value::Bool(true),
                                (true, Some("OFF" | "0")) => Value::Bool(false),
                                _ => v.clone(),
                            };
                            python.entry((driver.to_string(), cls.clone(), chk.clone(), k.clone())).or_default().insert(canon(&v));
                        }
                    }
                }
            }
        }
        // Simulate our polling over the same wire traffic.
        for (id, driver) in &s.devices {
            let log = s.dir.join(format!("{id}.jsonl.gz"));
            if !log.exists() {
                continue;
            }
            let def = &cat.devices[driver].1;
            let m = Matcher::new(def);
            let xs = exchanges(&log);
            for (cls, binding) in &def.classes {
                let compiled = CompiledStatus::new(binding).unwrap();
                let state: BTreeMap<String, Value> =
                    def.state.iter().map(|(k, v)| (k.clone(), v.initial.clone().unwrap_or(Value::Null))).collect();
                let mut latest: BTreeMap<u8, BTreeMap<String, Value>> = BTreeMap::new();
                for (i, x) in xs.iter().enumerate() {
                    for (name, cmd, ch) in m.matching(&x.tx) {
                        if !compiled.poll_commands.iter().any(|c| c == name) {
                            continue;
                        }
                        if let Ok(Some(v)) = decode(def, cmd, &x.rx, next_rx(&xs, i)) {
                            let res = latest.entry(ch).or_default();
                            if res.get(name) == Some(&v) {
                                continue; // unchanged: same status as before
                            }
                            res.insert(name.to_string(), v);
                            for (k, v) in compiled.evaluate(res, &state, ch) {
                                ours.entry((driver.clone(), cls.clone(), format!("status_ch{ch}"), k)).or_default().insert(canon(&v));
                            }
                        }
                    }
                }
            }
        }
    }
    let mut compared = 0usize;
    let mut missing = Vec::new();
    for (key, pvals) in &python {
        let Some(ovals) = ours.get(key) else { continue }; // key not in the class contract
        if KNOWN_STATUS_DIFFS.iter().any(|(d, c, k, _)| *d == key.0 && *c == key.1 && *k == key.3) {
            continue;
        }
        for pv in pvals {
            compared += 1;
            if !ovals.contains(pv) {
                missing.push(format!("{key:?}: python {pv} not produced (ours: {:?})", ovals.iter().take(4).collect::<Vec<_>>()));
            }
        }
    }
    eprintln!("compared {compared} distinct Python registry values");
    assert!(compared > 50, "too few registry values compared: {compared}");
    assert!(missing.is_empty(), "registry values not reproduced:\n{}", missing.join("\n"));
}
