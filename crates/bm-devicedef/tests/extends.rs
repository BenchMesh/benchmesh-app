//! `extends`: a definition merged over a base (`devices/bases/<id>.yaml`).

use std::fs;
use std::path::{Path, PathBuf};

use bm_devicedef::{load_catalog, load_device, validate_catalog, Severity};

fn repo_devices() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../devices")
}

/// A scratch `devices/` folder with the real class contracts.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bm-extends-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("classes")).unwrap();
    fs::create_dir_all(dir.join("bases")).unwrap();
    for f in fs::read_dir(repo_devices().join("classes")).unwrap() {
        let f = f.unwrap().path();
        fs::copy(&f, dir.join("classes").join(f.file_name().unwrap())).unwrap();
    }
    dir
}

/// owon_xdm split into a base (protocol) and a definition (identity + models).
fn split_xdm(dir: &Path) -> serde_yaml_ng::Mapping {
    let text = fs::read_to_string(repo_devices().join("owon_xdm.yaml")).unwrap();
    let mut base: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(&text).unwrap();
    let mut own = serde_yaml_ng::Mapping::new();
    for k in ["schema", "id", "vendor", "family", "version", "sources", "models"] {
        if let Some(v) = base.remove(k) {
            own.insert(k.into(), v);
        }
    }
    fs::write(dir.join("bases/scpi_xdm.yaml"), serde_yaml_ng::to_string(&base).unwrap()).unwrap();
    own.insert("extends".into(), "scpi_xdm".into());
    own
}

fn write(dir: &Path, file: &str, v: &serde_yaml_ng::Mapping) {
    fs::write(dir.join(file), serde_yaml_ng::to_string(v).unwrap()).unwrap();
}

#[test]
fn a_definition_over_its_base_equals_the_whole_definition() {
    let dir = scratch("equal");
    let own = split_xdm(&dir);
    write(&dir, "owon_xdm.yaml", &own);
    let cat = load_catalog(&dir).unwrap();
    let (_, merged) = &cat.devices["owon_xdm"];
    assert_eq!(merged.extends.as_deref(), Some("scpi_xdm"));
    let mut merged = serde_json::to_value(merged).unwrap();
    merged.as_object_mut().unwrap().remove("extends");
    let whole = serde_json::to_value(load_device(&repo_devices().join("owon_xdm.yaml")).unwrap()).unwrap();
    assert_eq!(merged, whole);
    let errors: Vec<_> = validate_catalog(&cat).into_iter().filter(|(_, i)| i.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn overrides_merge_key_by_key_and_null_removes() {
    let dir = scratch("override");
    let mut own = split_xdm(&dir);
    let over: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        r#"
framing: {rx: {timeout_ms: 900}}
commands:
  idn: {send: "*IDN?;*OPC?"}
  unused_extra: null
"#,
    )
    .unwrap();
    for (k, v) in over.as_mapping().unwrap() {
        own.insert(k.clone(), v.clone());
    }
    write(&dir, "owon_xdm.yaml", &own);
    let cat = load_catalog(&dir).unwrap();
    let (_, d) = &cat.devices["owon_xdm"];
    assert_eq!(d.framing.rx.timeout_ms, 900, "overridden");
    assert_eq!(d.framing.rx.drain_idle_ms, Some(20), "sibling keys of an override are kept");
    assert_eq!(d.commands["idn"].send, "*IDN?;*OPC?");
    assert!(!d.commands["idn"].parse.is_empty(), "the base's parse steps survive a send override");
}

#[test]
fn a_base_key_can_be_removed_with_null() {
    let dir = scratch("remove");
    let mut own = split_xdm(&dir);
    let over: serde_yaml_ng::Value = serde_yaml_ng::from_str("state: null").unwrap();
    own.insert("state".into(), over["state"].clone());
    write(&dir, "owon_xdm.yaml", &own);
    // the base's `range` state is gone, so the definition no longer validates cleanly
    let cat = load_catalog(&dir).unwrap();
    assert!(cat.devices["owon_xdm"].1.state.is_empty());
}

#[test]
fn unknown_bases_and_cycles_are_errors() {
    let dir = scratch("errors");
    let mut own = split_xdm(&dir);
    own.insert("extends".into(), "nope".into());
    write(&dir, "owon_xdm.yaml", &own);
    let e = load_catalog(&dir).unwrap_err().join("\n");
    assert!(e.contains("unknown base `nope`"), "{e}");

    fs::write(dir.join("bases/a.yaml"), "extends: b\n").unwrap();
    fs::write(dir.join("bases/b.yaml"), "extends: a\n").unwrap();
    own.insert("extends".into(), "a".into());
    write(&dir, "owon_xdm.yaml", &own);
    let e = load_catalog(&dir).unwrap_err().join("\n");
    assert!(e.contains("cycle"), "{e}");
}

#[test]
fn a_family_offers_several_links_each_with_its_own_framing() {
    let dir = scratch("links");
    let mut own = split_xdm(&dir);
    let over: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        r#"
link:
  tcp: {port: 5025, framing: {tx: {terminator: "\n"}, rx: {mode: terminator, terminator: "\n", timeout_ms: 1000}}}
"#,
    )
    .unwrap();
    own.insert("link".into(), over["link"].clone());
    write(&dir, "owon_xdm.yaml", &own);
    let cat = load_catalog(&dir).unwrap();
    let d = &cat.devices["owon_xdm"].1;
    assert_eq!(d.link.kinds(), vec!["serial", "tcpip"], "the base's serial link is kept, tcp added");

    let serial = d.for_transport(None).unwrap();
    assert_eq!(serial.link.kinds(), vec!["serial"], "no transport: serial, as configs have always meant");
    assert_eq!(serial.framing.rx.timeout_ms, 300);
    let tcp = d.for_transport(Some("tcpip")).unwrap();
    assert_eq!(tcp.link.kinds(), vec!["tcpip"]);
    assert_eq!(tcp.framing.rx.timeout_ms, 1000, "the tcp link's own framing replaces the top-level one");
    assert_eq!(tcp.framing.tx.terminator, "\n");
    assert!(d.for_transport(Some("usbtmc")).unwrap_err().contains("not offered"));
    let errors: Vec<_> = validate_catalog(&cat).into_iter().filter(|(_, i)| i.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn without_a_transport_a_family_uses_its_local_link_first() {
    let l: bm_devicedef::Links = bm_devicedef::from_yaml("{usbtmc: {vid: '1ab1'}, tcp: {port: 5555}, vxi11: {}}").unwrap();
    assert_eq!(l.choose(None).unwrap().kind(), "usbtmc", "LAN needs a host: never the default");
    let l: bm_devicedef::Links = bm_devicedef::from_yaml("{tcp: {port: 5025}, vxi11: {}}").unwrap();
    assert!(l.choose(None).unwrap_err().contains("choose a transport"));
    assert_eq!(l.choose(Some("vxi11")).unwrap().kind(), "vxi11");
}
