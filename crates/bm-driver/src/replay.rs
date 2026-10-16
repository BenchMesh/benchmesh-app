//! Simulated instruments from recorded wire captures (`captures/<session>/<device>.jsonl.gz`).
//! Each command is answered with the last reply the real device gave to it; setters are
//! accepted silently (state does not change). Used by tests and the server's `--simulate`
//! mode (UI development / demos without hardware).

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use bm_devicedef::{DeviceDef, RxMode};
use serde_json::Value;

use crate::sim::Responder;

fn text(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

fn unhex(h: &str) -> Vec<u8> {
    (0..h.len().saturating_sub(1)).step_by(2).filter_map(|i| u8::from_str_radix(&h[i..i + 2], 16).ok()).collect()
}

/// Build a responder for `def` from a capture log. Queries the capture never saw get a
/// plausible numeric reply (`1.0`) so polling does not stall on timeouts.
pub fn responder_from_capture(def: &DeviceDef, path: &Path) -> io::Result<Responder> {
    let rd = BufReader::new(flate2::read::GzDecoder::new(std::fs::File::open(path)?));
    let rx = def.framing.rx.clone();
    let terminator = match (rx.mode, rx.terminator.as_deref()) {
        (RxMode::Terminator, Some(t)) => t.to_string(),
        (RxMode::Message, _) => "\n".into(),
        _ => String::new(),
    };
    let mut table: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut cur: Option<(String, Vec<u8>)> = None;
    let finish = |c: Option<(String, Vec<u8>)>, table: &mut BTreeMap<String, Vec<u8>>| {
        if let Some((tx, mut r)) = c {
            // Python's framing split some replies across exchanges: rebuild the device reply.
            while r.first().is_some_and(|b| rx.skip_leading.as_bytes().contains(b)) {
                r.remove(0);
            }
            if !terminator.is_empty() && !r.is_empty() && !text(&r).contains(&terminator) {
                r.extend(terminator.as_bytes());
            }
            if !r.is_empty() {
                table.insert(tx, r);
            }
        }
    };
    for line in rd.lines() {
        let line = line?;
        let e: Value = serde_json::from_str(&line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        match e["ev"].as_str() {
            Some("tx") => {
                finish(cur.take(), &mut table);
                cur = Some((text(&unhex(e["hex"].as_str().unwrap_or(""))), Vec::new()));
            }
            Some("rx") => {
                if let Some((_, r)) = cur.as_mut() {
                    r.extend(unhex(e["hex"].as_str().unwrap_or("")));
                }
            }
            _ => {}
        }
    }
    finish(cur, &mut table);
    let fallback_term = terminator;
    Ok(Box::new(move |w: &[u8]| {
        let cmd = text(w);
        table.get(&cmd).cloned().or_else(|| {
            cmd.trim_end().ends_with('?').then(|| format!("1.0{fallback_term}").into_bytes())
        })
    }))
}
