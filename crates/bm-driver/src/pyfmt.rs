//! Python-compatible number formatting (`"{:.NE}"`, `"{:.Ng}"`), needed to reproduce the
//! BenchMesh SI value dicts byte-for-byte. Rust's `{:e}` differs (`1e-4` vs `1.00000E-04`).

/// Split Rust's `{:.*e}` output into (mantissa, exponent).
fn sci_parts(x: f64, decimals: usize) -> (String, i32) {
    let s = format!("{:.*e}", decimals, x);
    let (m, e) = s.split_once('e').expect("{:e} always has an exponent");
    (m.to_string(), e.parse().expect("integer exponent"))
}

/// Python `"{:.{prec}E}".format(x)`, e.g. `fmt_e(1e-4, 5) == "1.00000E-04"`.
pub fn fmt_e(x: f64, prec: usize) -> String {
    if !x.is_finite() {
        return if x.is_nan() { "NAN".into() } else if x > 0.0 { "INF".into() } else { "-INF".into() };
    }
    let (m, e) = sci_parts(x, prec);
    format!("{m}E{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
}

/// Python `"{:.{prec}g}".format(x)` (general format, trailing zeros removed).
pub fn fmt_g(x: f64, prec: usize) -> String {
    if !x.is_finite() {
        return if x.is_nan() { "nan".into() } else if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let p = prec.max(1);
    if x == 0.0 {
        return if x.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let (m, e) = sci_parts(x, p - 1);
    if e >= -4 && (e as i64) < p as i64 {
        let decimals = (p as i64 - 1 - e as i64).max(0) as usize;
        strip_zeros(&format!("{:.*}", decimals, x))
    } else {
        format!("{}e{}{:02}", strip_zeros(&m), if e < 0 { '-' } else { '+' }, e.abs())
    }
}

fn strip_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected strings produced by CPython 3 for the same calls.
    #[test]
    fn e_format_matches_python() {
        assert_eq!(fmt_e(1e-4, 5), "1.00000E-04");
        assert_eq!(fmt_e(5.0, 5), "5.00000E+00");
        assert_eq!(fmt_e(-7.941e-05, 5), "-7.94100E-05");
        assert_eq!(fmt_e(0.0, 5), "0.00000E+00");
        assert_eq!(fmt_e(1.5e300, 2), "1.50E+300");
        assert_eq!(fmt_e(0.988, 5), "9.88000E-01");
    }

    #[test]
    fn g_format_matches_python() {
        assert_eq!(fmt_g(5.0, 6), "5");
        assert_eq!(fmt_g(100.0, 6), "100");
        assert_eq!(fmt_g(988.0, 6), "988");
        assert_eq!(fmt_g(79.41, 6), "79.41");
        assert_eq!(fmt_g(1.23456789, 6), "1.23457");
        assert_eq!(fmt_g(0.0001, 6), "0.0001");
        assert_eq!(fmt_g(0.00001, 6), "1e-05");
        assert_eq!(fmt_g(1234567.0, 6), "1.23457e+06");
        assert_eq!(fmt_g(-12.5, 6), "-12.5");
        assert_eq!(fmt_g(70000000.0, 6), "7e+07");
    }
}
