//! A recordings.db written by the former Python service (SQLAlchemy) opens, reads and
//! extends unchanged.

use std::path::Path;

use bm_store::{fmt_dt, parse_dt, Store};
use serde_json::json;

#[test]
fn reads_and_extends_a_python_written_database() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/python_recordings.db");
    let copy = std::env::temp_dir().join(format!("bm-store-compat-{}.db", std::process::id()));
    std::fs::copy(&fixture, &copy).unwrap();
    let store = Store::open(&copy).unwrap();

    let all = store.list().unwrap();
    assert_eq!(all.len(), 1);
    let s = &all[0];
    assert_eq!((s.name.as_str(), s.description.as_deref()), ("python run", Some("made by SQLAlchemy")));
    assert_eq!(s.interval_seconds, 0.5);
    assert_eq!(s.channels[0]["method_name"], "voltage");
    assert_eq!(fmt_dt(&s.start_time), "2026-10-16 20:00:00.123456");
    assert_eq!(s.status(), "stopped");
    assert_eq!(s.pause_duration_seconds, 0.25);
    assert_eq!(s.data_points_count, 2);
    assert!((s.total_duration_seconds(bm_store::now()) - 1.75).abs() < 1e-9);

    let points = store.points(s.id, 0, 100).unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].measurements["tenma_PSU_1_voltage"], "4.94", "Python stored raw strings");
    assert_eq!(points[1].measurements["tenma_PSU_1_voltage"], 5.0);

    store.add_point(s.id, parse_dt("2026-10-16 20:00:02.000000").unwrap(), &json!({"tenma_PSU_1_voltage": 5.01})).unwrap();
    assert_eq!(store.series(s.id).unwrap().data_points_count, 3);
    assert_eq!(store.points(s.id, 2, 10).unwrap()[0].measurements["tenma_PSU_1_voltage"], 5.01);
    let _ = std::fs::remove_file(&copy);
}
