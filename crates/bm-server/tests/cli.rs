//! The `benchmesh-server` command line: what it does not know is refused before anything starts
//! (no port is opened, no instrument is touched).

use std::process::Command;

fn server(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_benchmesh-server")).args(args).env("BENCHMESH_ONLINE", "off").output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).into(), String::from_utf8_lossy(&out.stderr).into())
}

#[test]
fn unknown_options_and_missing_values_are_refused_with_the_usage() {
    let (code, _, err) = server(&["--simulat", "captures/20261016-203514"]);
    assert_eq!(code, 2);
    assert!(err.starts_with("unknown option --simulat\n") && err.contains("usage: benchmesh-server"), "{err}");
    let (code, _, err) = server(&["--port"]);
    assert_eq!((code, err.lines().next()), (2, Some("--port needs a value")));
    let (code, _, err) = server(&["remote-password", "--port", "1"]);
    assert_eq!((code, err.lines().next()), (2, Some("unknown option --port")), "remote-password takes --data-dir only");
}

#[test]
fn help_prints_the_usage() {
    let (code, out, _) = server(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("--simulate CAPTURE") && out.contains("remote-password"), "{out}");
}
