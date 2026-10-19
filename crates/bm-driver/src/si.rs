//! BenchMesh SI value dict - a faithful port of `utils/si.py` (`sci_to_value` /
//! `si_to_value` with `sig_figs=5`), because the UI renders `si.number` + `si.symbol`
//! and reads `val` for setpoints.
//!
//! ```text
//! {"si": {"number": "100", "symbol": "µ", "prefix": "micro"}, "sci": "1.00000E-04", "val": "+1.0000E-04"}
//! ```
//! `symbol`/`prefix` are omitted when there is no prefix. Known Python inconsistency not
//! reproduced: for zero, `si_to_value` emitted `sci: "0"` while `sci_to_value` emitted
//! `"0.00000E+00"`; Rust always emits the latter.

use serde_json::{json, Map, Value};

use crate::pyfmt::{fmt_e, fmt_g};

const SIG_FIGS: usize = 5;

const PREFIXES: [(i32, &str, &str); 17] = [
    (24, "Y", "yotta"),
    (21, "Z", "zetta"),
    (18, "E", "exa"),
    (15, "P", "peta"),
    (12, "T", "tera"),
    (9, "G", "giga"),
    (6, "M", "mega"),
    (3, "k", "kilo"),
    (0, "", ""),
    (-3, "m", "milli"),
    (-6, "µ", "micro"),
    (-9, "n", "nano"),
    (-12, "p", "pico"),
    (-15, "f", "femto"),
    (-18, "a", "atto"),
    (-21, "z", "zepto"),
    (-24, "y", "yocto"),
];

/// SI parts (number string, symbol, prefix name) of an already-normalised scientific
/// string - `format_scientific_to_si(sci, sig_figs=5)` in Python.
fn scientific_to_si(sci: &str) -> (String, &'static str, &'static str) {
    let num: f64 = sci.parse().unwrap_or(0.0);
    if num == 0.0 {
        return ("0".into(), "", "");
    }
    let exp = num.abs().log10().floor() as i32;
    let floor3 = exp - exp.rem_euclid(3);
    let mut si_power = PREFIXES.iter().map(|p| p.0).filter(|p| *p <= floor3).max().unwrap_or(-24);
    let pow10 = |p: i32| 10f64.powf(p as f64);
    let mut scaled = num / pow10(si_power);
    if scaled.abs() >= 1000.0 {
        si_power += 3;
        scaled = num / pow10(si_power);
    }
    if scaled.abs() < 1.0 {
        si_power -= 3;
        scaled = num / pow10(si_power);
    }
    si_power = si_power.clamp(-24, 24);
    let (_, symbol, name) = PREFIXES.iter().find(|p| p.0 == si_power).copied().unwrap_or((0, "", ""));
    // Python: digits = max(sig_figs, significant digits of the mantissa incl. trailing zeros).
    let mantissa = sci.trim_start_matches(['+', '-']).split(['E', 'e']).next().unwrap_or("");
    let sig = mantissa.replace('.', "");
    let sig_len = sig.trim_start_matches('0').len();
    let digits = SIG_FIGS.max(sig_len);
    (fmt_g(scaled, digits), symbol, name)
}

/// SI value dict for a number. `raw` is the device text (goes to `val`); without it the
/// number's shortest representation is used.
pub fn si_value(x: f64, raw: Option<&str>) -> Value {
    let sci = fmt_e(x, SIG_FIGS);
    let (number, symbol, prefix) = scientific_to_si(&sci);
    let mut si = Map::new();
    si.insert("number".into(), json!(number));
    if !symbol.is_empty() {
        si.insert("symbol".into(), json!(symbol));
    }
    if !prefix.is_empty() {
        si.insert("prefix".into(), json!(prefix));
    }
    let val = raw.map(str::to_string).unwrap_or_else(|| format!("{x}"));
    json!({"si": si, "sci": sci, "val": val})
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected dicts produced by the Python utils/si.py for the same inputs.
    #[test]
    fn matches_python_sci_to_value() {
        let v = si_value(1e-4, Some("+1.0000E-04"));
        assert_eq!(v, json!({"si": {"number": "100", "symbol": "µ", "prefix": "micro"}, "sci": "1.00000E-04", "val": "+1.0000E-04"}));
        let v = si_value(-7.941e-05, None);
        assert_eq!(v["si"], json!({"number": "-79.41", "symbol": "µ", "prefix": "micro"}));
        assert_eq!(v["sci"], "-7.94100E-05");
        let v = si_value(1.166309e-3, None);
        assert_eq!(v["si"], json!({"number": "1.16631", "symbol": "m", "prefix": "milli"}));
        assert_eq!(v["sci"], "1.16631E-03");
    }

    #[test]
    fn matches_python_si_to_value() {
        // si_to_value("05.00") -> {'si': {'number': '5'}, 'sci': '5.00000E+00', 'val': '05.00'}
        assert_eq!(si_value(5.0, Some("05.00")), json!({"si": {"number": "5"}, "sci": "5.00000E+00", "val": "05.00"}));
        // si_to_value("0.988 ") -> number '988', symbol 'm'
        assert_eq!(si_value(0.988, None)["si"], json!({"number": "988", "symbol": "m", "prefix": "milli"}));
        assert_eq!(si_value(32.0, None)["si"], json!({"number": "32"}));
        assert_eq!(si_value(4.94, None)["si"], json!({"number": "4.94"}));
    }

    #[test]
    fn zero_and_large() {
        assert_eq!(si_value(0.0, None)["si"], json!({"number": "0"}));
        assert_eq!(si_value(0.0, None)["sci"], "0.00000E+00");
        assert_eq!(si_value(70e6, None)["si"], json!({"number": "70", "symbol": "M", "prefix": "mega"}));
        assert_eq!(si_value(999999.0, None)["si"], json!({"number": "999.999", "symbol": "k", "prefix": "kilo"}));
    }
}
