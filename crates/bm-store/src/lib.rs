//! Recordings database. Same SQLite schema and timestamp text format as the former Python
//! service (SQLAlchemy), so existing `~/.benchmesh/recordings.db` files keep working:
//!
//! ```sql
//! recording_series(id INTEGER PK, name VARCHAR NOT NULL UNIQUE, description TEXT,
//!   interval_seconds FLOAT NOT NULL, channels TEXT NOT NULL, start_time DATETIME NOT NULL,
//!   end_time DATETIME, paused_at DATETIME, pause_duration_seconds FLOAT,
//!   data_points_count INTEGER, created_at DATETIME, updated_at DATETIME)
//! data_points(id INTEGER PK, series_id INTEGER NOT NULL REFERENCES recording_series(id)
//!   ON DELETE CASCADE, timestamp DATETIME NOT NULL, measurements TEXT NOT NULL)
//! ```
//! Datetimes are naive UTC text `YYYY-MM-DD HH:MM:SS.ffffff`.

use std::path::Path;
use std::sync::Mutex;

use chrono::{NaiveDateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

const DT_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.6f";

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS recording_series (
    id INTEGER NOT NULL PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    description TEXT,
    interval_seconds FLOAT NOT NULL,
    channels TEXT NOT NULL,
    start_time DATETIME NOT NULL,
    end_time DATETIME,
    paused_at DATETIME,
    pause_duration_seconds FLOAT,
    data_points_count INTEGER,
    created_at DATETIME,
    updated_at DATETIME
);
CREATE TABLE IF NOT EXISTS data_points (
    id INTEGER NOT NULL PRIMARY KEY,
    series_id INTEGER NOT NULL REFERENCES recording_series(id) ON DELETE CASCADE,
    timestamp DATETIME NOT NULL,
    measurements TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_data_points_series_ts ON data_points(series_id, timestamp);
";

pub fn now() -> NaiveDateTime {
    Utc::now().naive_utc()
}

pub fn fmt_dt(t: &NaiveDateTime) -> String {
    t.format(DT_FORMAT).to_string()
}

/// Parse SQLAlchemy datetime text (with or without fractional seconds, `T` or space).
pub fn parse_dt(s: &str) -> Option<NaiveDateTime> {
    let s = s.replace('T', " ");
    NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f").ok()
}

/// API representation: ISO 8601 UTC with `Z` (the Python API omitted the zone, which the
/// browser then read as local time).
pub fn iso(t: &NaiveDateTime) -> String {
    t.and_utc().to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub interval_seconds: f64,
    /// JSON array of channel objects.
    pub channels: Value,
    pub start_time: NaiveDateTime,
    pub end_time: Option<NaiveDateTime>,
    pub paused_at: Option<NaiveDateTime>,
    pub pause_duration_seconds: f64,
    pub data_points_count: i64,
}

impl Series {
    pub fn status(&self) -> &'static str {
        if self.paused_at.is_some() {
            "paused"
        } else if self.end_time.is_some() {
            "stopped"
        } else {
            "recording"
        }
    }

    /// Recorded time excluding pauses (an open pause counts as paused).
    pub fn total_duration_seconds(&self, now: NaiveDateTime) -> f64 {
        let end = self.end_time.or(self.paused_at).unwrap_or(now);
        ((end - self.start_time).num_microseconds().unwrap_or(0) as f64 / 1e6 - self.pause_duration_seconds).max(0.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataPoint {
    pub id: i64,
    pub series_id: i64,
    pub timestamp: NaiveDateTime,
    pub measurements: Value,
}

#[derive(Debug)]
pub enum StoreError {
    DuplicateName(String),
    NotFound(i64),
    Db(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::DuplicateName(n) => write!(f, "A recording named {n:?} already exists"),
            StoreError::NotFound(id) => write!(f, "Recording {id} not found"),
            StoreError::Db(m) => write!(f, "database error: {m}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Db(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// Thread-safe handle (one connection behind a mutex: writes are small and infrequent).
pub struct Store {
    conn: Mutex<Connection>,
}

const SERIES_COLS: &str = "id, name, description, interval_seconds, channels, start_time, end_time, paused_at, \
                           COALESCE(pause_duration_seconds, 0), COALESCE(data_points_count, 0)";

fn row_to_series(r: &rusqlite::Row<'_>) -> rusqlite::Result<Series> {
    let dt = |i: usize| -> rusqlite::Result<Option<NaiveDateTime>> { Ok(r.get::<_, Option<String>>(i)?.and_then(|s| parse_dt(&s))) };
    Ok(Series {
        id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        interval_seconds: r.get(3)?,
        channels: serde_json::from_str(&r.get::<_, String>(4)?).unwrap_or(Value::Array(vec![])),
        start_time: dt(5)?.unwrap_or_default(),
        end_time: dt(6)?,
        paused_at: dt(7)?,
        pause_duration_seconds: r.get(8)?,
        data_points_count: r.get(9)?,
    })
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| StoreError::Db(format!("{}: {e}", dir.display())))?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Store> {
        conn.busy_timeout(std::time::Duration::from_secs(30))?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Store { conn: Mutex::new(conn) })
    }

    pub fn create_series(&self, name: &str, description: Option<&str>, interval_seconds: f64, channels: &Value) -> Result<Series> {
        let c = self.conn.lock().unwrap();
        let t = fmt_dt(&now());
        let r = c.execute(
            "INSERT INTO recording_series (name, description, interval_seconds, channels, start_time, \
             pause_duration_seconds, data_points_count, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?5, ?5)",
            params![name, description, interval_seconds, channels.to_string(), t],
        );
        match r {
            Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
                Err(StoreError::DuplicateName(name.to_string()))
            }
            Err(e) => Err(e.into()),
            Ok(_) => {
                let id = c.last_insert_rowid();
                drop(c);
                self.series(id)
            }
        }
    }

    pub fn series(&self, id: i64) -> Result<Series> {
        let c = self.conn.lock().unwrap();
        c.query_row(&format!("SELECT {SERIES_COLS} FROM recording_series WHERE id = ?1"), [id], row_to_series)
            .optional()?
            .ok_or(StoreError::NotFound(id))
    }

    pub fn list(&self) -> Result<Vec<Series>> {
        let c = self.conn.lock().unwrap();
        let mut st = c.prepare(&format!("SELECT {SERIES_COLS} FROM recording_series ORDER BY start_time DESC"))?;
        let rows = st.query_map([], row_to_series)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Set/clear pause and end markers. `add_pause` is added to the pause total.
    pub fn update_state(&self, id: i64, paused_at: Option<NaiveDateTime>, end_time: Option<NaiveDateTime>, add_pause: f64) -> Result<Series> {
        let c = self.conn.lock().unwrap();
        let n = c.execute(
            "UPDATE recording_series SET paused_at = ?2, end_time = ?3, \
             pause_duration_seconds = COALESCE(pause_duration_seconds, 0) + ?4, updated_at = ?5 WHERE id = ?1",
            params![id, paused_at.as_ref().map(fmt_dt), end_time.as_ref().map(fmt_dt), add_pause, fmt_dt(&now())],
        )?;
        drop(c);
        if n == 0 {
            return Err(StoreError::NotFound(id));
        }
        self.series(id)
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let c = self.conn.lock().unwrap();
        let n = c.execute("DELETE FROM recording_series WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(StoreError::NotFound(id));
        }
        Ok(())
    }

    /// Store one data point and bump the series' point count atomically.
    pub fn add_point(&self, series_id: i64, timestamp: NaiveDateTime, measurements: &Value) -> Result<DataPoint> {
        let mut c = self.conn.lock().unwrap();
        let tx = c.transaction()?;
        tx.execute(
            "INSERT INTO data_points (series_id, timestamp, measurements) VALUES (?1, ?2, ?3)",
            params![series_id, fmt_dt(&timestamp), measurements.to_string()],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "UPDATE recording_series SET data_points_count = COALESCE(data_points_count, 0) + 1 WHERE id = ?1",
            [series_id],
        )?;
        tx.commit()?;
        Ok(DataPoint { id, series_id, timestamp, measurements: measurements.clone() })
    }

    pub fn points(&self, series_id: i64, offset: i64, limit: i64) -> Result<Vec<DataPoint>> {
        let c = self.conn.lock().unwrap();
        let mut st = c.prepare(
            "SELECT id, series_id, timestamp, measurements FROM data_points WHERE series_id = ?1 \
             ORDER BY timestamp, id LIMIT ?2 OFFSET ?3",
        )?;
        let rows = st
            .query_map(params![series_id, limit, offset], |r| {
                Ok(DataPoint {
                    id: r.get(0)?,
                    series_id: r.get(1)?,
                    timestamp: parse_dt(&r.get::<_, String>(2)?).unwrap_or_default(),
                    measurements: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or(Value::Null),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Recordings left open by a previous run (no end time): close them at their last data
    /// point (or start). Returns how many were closed.
    pub fn close_orphans(&self) -> Result<usize> {
        let c = self.conn.lock().unwrap();
        let n = c.execute(
            "UPDATE recording_series SET paused_at = NULL, end_time = COALESCE( \
               (SELECT MAX(timestamp) FROM data_points WHERE series_id = recording_series.id), start_time), \
               updated_at = ?1 WHERE end_time IS NULL",
            [fmt_dt(&now())],
        )?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn series_lifecycle_and_points() {
        let s = Store::in_memory().unwrap();
        let ch = json!([{"device_id": "t1", "class_name": "PSU", "channel": 1, "method_name": "voltage"}]);
        let a = s.create_series("run 1", None, 1.0, &ch).unwrap();
        assert_eq!(a.status(), "recording");
        assert!(matches!(s.create_series("run 1", None, 1.0, &ch), Err(StoreError::DuplicateName(_))));
        let p = s.add_point(a.id, now(), &json!({"t1_PSU_1_voltage": 4.94})).unwrap();
        assert_eq!(p.series_id, a.id);
        assert_eq!(s.series(a.id).unwrap().data_points_count, 1);
        assert_eq!(s.points(a.id, 0, 10).unwrap()[0].measurements["t1_PSU_1_voltage"], 4.94);
        let paused = s.update_state(a.id, Some(now()), None, 0.0).unwrap();
        assert_eq!(paused.status(), "paused");
        let stopped = s.update_state(a.id, None, Some(now()), 2.5).unwrap();
        assert_eq!(stopped.status(), "stopped");
        assert_eq!(stopped.pause_duration_seconds, 2.5);
        s.delete(a.id).unwrap();
        assert!(matches!(s.series(a.id), Err(StoreError::NotFound(_))));
        assert!(s.points(a.id, 0, 10).unwrap().is_empty(), "points deleted by cascade");
    }

    #[test]
    fn orphans_are_closed_at_their_last_point() {
        let s = Store::in_memory().unwrap();
        let a = s.create_series("left open", None, 1.0, &json!([])).unwrap();
        let t = now();
        s.add_point(a.id, t, &json!({"x": 1})).unwrap();
        assert_eq!(s.close_orphans().unwrap(), 1);
        let a = s.series(a.id).unwrap();
        assert_eq!(a.status(), "stopped");
        assert_eq!(fmt_dt(&a.end_time.unwrap()), fmt_dt(&t));
        assert_eq!(s.close_orphans().unwrap(), 0);
    }

    #[test]
    fn timestamps_round_trip_python_format() {
        let t = parse_dt("2026-10-17 08:15:30.123456").unwrap();
        assert_eq!(fmt_dt(&t), "2026-10-17 08:15:30.123456");
        assert_eq!(iso(&t), "2026-10-17T08:15:30.123456Z");
        assert!(parse_dt("2026-10-17 08:15:30").is_some(), "SQLAlchemy omits zero microseconds");
    }
}
