//! Recordings database (v2, see `ai_context/migration/RECORDINGS.md`). All times are
//! integer milliseconds since the Unix epoch (UTC).
//!
//! ```sql
//! recordings(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, description TEXT,
//!            interval_s INTEGER NOT NULL, state TEXT NOT NULL,
//!            started_at INTEGER NOT NULL, stopped_at INTEGER, paused_at INTEGER)
//! series(id INTEGER PRIMARY KEY, recording_id INTEGER NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
//!        device_id TEXT NOT NULL, class TEXT NOT NULL, channel INTEGER NOT NULL, field TEXT NOT NULL,
//!        label TEXT NOT NULL, unit TEXT NOT NULL, added_at INTEGER NOT NULL, removed_at INTEGER)
//! samples(series_id INTEGER NOT NULL REFERENCES series(id) ON DELETE CASCADE,
//!         t INTEGER NOT NULL, v REAL NOT NULL, PRIMARY KEY (series_id, t)) WITHOUT ROWID
//! ```
//! WAL mode, foreign keys on. The v1 tables of the former Python service are dropped.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

const SCHEMA: &str = "
DROP TABLE IF EXISTS data_points;
DROP TABLE IF EXISTS recording_series;
CREATE TABLE IF NOT EXISTS recordings (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    interval_s INTEGER NOT NULL,
    state TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    stopped_at INTEGER,
    paused_at INTEGER
);
CREATE TABLE IF NOT EXISTS series (
    id INTEGER PRIMARY KEY,
    recording_id INTEGER NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    class TEXT NOT NULL,
    channel INTEGER NOT NULL,
    field TEXT NOT NULL,
    label TEXT NOT NULL,
    unit TEXT NOT NULL,
    added_at INTEGER NOT NULL,
    removed_at INTEGER
);
CREATE INDEX IF NOT EXISTS ix_series_recording ON series(recording_id);
CREATE TABLE IF NOT EXISTS samples (
    series_id INTEGER NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    t INTEGER NOT NULL,
    v REAL NOT NULL,
    PRIMARY KEY (series_id, t)
) WITHOUT ROWID;
";

/// Wall clock in ms since the epoch.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Recording,
    Paused,
    Stopped,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Recording => "recording",
            State::Paused => "paused",
            State::Stopped => "stopped",
        }
    }

    fn parse(s: &str) -> State {
        match s {
            "recording" => State::Recording,
            "paused" => State::Paused,
            _ => State::Stopped,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recording {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub interval_s: i64,
    pub state: State,
    pub started_at: i64,
    pub stopped_at: Option<i64>,
    pub paused_at: Option<i64>,
}

/// What a series reads: a numeric status key of one device class channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    pub device_id: String,
    pub class: String,
    pub channel: i64,
    pub field: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub id: i64,
    pub recording_id: i64,
    pub signal: Signal,
    pub label: String,
    pub unit: String,
    pub added_at: i64,
    pub removed_at: Option<i64>,
}

/// An active series of a recording that samples at a tick.
#[derive(Debug, Clone, PartialEq)]
pub struct DueSeries {
    pub recording_id: i64,
    pub series_id: i64,
    pub signal: Signal,
}

/// One CSV export row: time and a value per series (`None` = no sample).
pub type ExportRow = (i64, Vec<Option<f64>>);

/// Samples of one series in a time range: parallel time/value vectors.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Points {
    pub t: Vec<i64>,
    pub v: Vec<f64>,
}

#[derive(Debug, PartialEq)]
pub enum StoreError {
    DuplicateName(String),
    NotFound(String),
    /// The request conflicts with the current state (stopped recording, signal already active).
    Conflict(String),
    Db(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::DuplicateName(n) => write!(f, "A recording named {n:?} already exists"),
            StoreError::NotFound(m) | StoreError::Conflict(m) => f.write_str(m),
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

fn not_found(id: i64) -> StoreError {
    StoreError::NotFound(format!("Recording {id} not found"))
}

fn unique_violation(e: &rusqlite::Error) -> bool {
    matches!(e, rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation)
}

const REC_COLS: &str = "id, name, COALESCE(description, ''), interval_s, state, started_at, stopped_at, paused_at";
const SERIES_COLS: &str = "id, recording_id, device_id, class, channel, field, label, unit, added_at, removed_at";

fn row_to_recording(r: &rusqlite::Row<'_>) -> rusqlite::Result<Recording> {
    Ok(Recording {
        id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        interval_s: r.get(3)?,
        state: State::parse(&r.get::<_, String>(4)?),
        started_at: r.get(5)?,
        stopped_at: r.get(6)?,
        paused_at: r.get(7)?,
    })
}

fn row_to_series(r: &rusqlite::Row<'_>) -> rusqlite::Result<Series> {
    Ok(Series {
        id: r.get(0)?,
        recording_id: r.get(1)?,
        signal: Signal { device_id: r.get(2)?, class: r.get(3)?, channel: r.get(4)?, field: r.get(5)? },
        label: r.get(6)?,
        unit: r.get(7)?,
        added_at: r.get(8)?,
        removed_at: r.get(9)?,
    })
}

/// Thread-safe handle (one connection behind a mutex).
pub struct Store {
    conn: Mutex<Connection>,
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
        // journal_mode returns a row ("wal", or "memory" for in-memory databases).
        conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Store { conn: Mutex::new(conn) })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|p| p.into_inner())
    }

    // ---- recordings -------------------------------------------------------------------

    /// New recording in state `recording`, started at `t`.
    pub fn create_recording(&self, name: &str, description: &str, interval_s: i64, t: i64) -> Result<Recording> {
        let c = self.conn();
        match c.execute(
            "INSERT INTO recordings (name, description, interval_s, state, started_at) VALUES (?1, ?2, ?3, 'recording', ?4)",
            params![name, description, interval_s, t],
        ) {
            Err(e) if unique_violation(&e) => Err(StoreError::DuplicateName(name.to_string())),
            Err(e) => Err(e.into()),
            Ok(_) => {
                let id = c.last_insert_rowid();
                drop(c);
                self.recording(id)
            }
        }
    }

    pub fn recording(&self, id: i64) -> Result<Recording> {
        self.conn()
            .query_row(&format!("SELECT {REC_COLS} FROM recordings WHERE id = ?1"), [id], row_to_recording)
            .optional()?
            .ok_or_else(|| not_found(id))
    }

    /// All recordings, newest first.
    pub fn list_recordings(&self) -> Result<Vec<Recording>> {
        let c = self.conn();
        let mut st = c.prepare(&format!("SELECT {REC_COLS} FROM recordings ORDER BY started_at DESC, id DESC"))?;
        let rows = st.query_map([], row_to_recording)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Rename and/or describe (`None` keeps the current value).
    pub fn update_recording(&self, id: i64, name: Option<&str>, description: Option<&str>) -> Result<Recording> {
        let c = self.conn();
        let r = c.execute(
            "UPDATE recordings SET name = COALESCE(?2, name), description = COALESCE(?3, description) WHERE id = ?1",
            params![id, name, description],
        );
        match r {
            Err(e) if unique_violation(&e) => Err(StoreError::DuplicateName(name.unwrap_or_default().to_string())),
            Err(e) => Err(e.into()),
            Ok(0) => Err(not_found(id)),
            Ok(_) => {
                drop(c);
                self.recording(id)
            }
        }
    }

    /// recording → paused (keeps the first pause time; no-op otherwise).
    pub fn pause(&self, id: i64, t: i64) -> Result<Recording> {
        self.transition(id, "UPDATE recordings SET state = 'paused', paused_at = ?2 WHERE id = ?1 AND state = 'recording'", t)
    }

    /// paused → recording (no-op otherwise).
    pub fn resume(&self, id: i64) -> Result<Recording> {
        self.conn().execute("UPDATE recordings SET state = 'recording', paused_at = NULL WHERE id = ?1 AND state = 'paused'", [id])?;
        self.recording(id)
    }

    /// → stopped at `t` (final; stopping again keeps the first stop time).
    pub fn stop(&self, id: i64, t: i64) -> Result<Recording> {
        self.transition(
            id,
            "UPDATE recordings SET state = 'stopped', stopped_at = ?2, paused_at = NULL WHERE id = ?1 AND state != 'stopped'",
            t,
        )
    }

    fn transition(&self, id: i64, sql: &str, t: i64) -> Result<Recording> {
        self.conn().execute(sql, params![id, t])?;
        self.recording(id)
    }

    /// Delete a recording with its series and samples.
    pub fn delete_recording(&self, id: i64) -> Result<()> {
        match self.conn().execute("DELETE FROM recordings WHERE id = ?1", [id])? {
            0 => Err(not_found(id)),
            _ => Ok(()),
        }
    }

    /// Recordings left `recording`/`paused` by a previous run are stopped at their last
    /// sample (else their start). Returns how many.
    pub fn close_orphans(&self) -> Result<usize> {
        Ok(self.conn().execute(
            "UPDATE recordings SET state = 'stopped', paused_at = NULL, stopped_at = COALESCE(
               (SELECT MAX(s.t) FROM samples s JOIN series se ON se.id = s.series_id WHERE se.recording_id = recordings.id),
               started_at)
             WHERE state != 'stopped'",
            [],
        )?)
    }

    // ---- series -----------------------------------------------------------------------

    /// Add a series (409 cases: recording stopped, signal already active in it).
    pub fn add_series(&self, recording_id: i64, signal: &Signal, label: &str, unit: &str, t: i64) -> Result<Series> {
        let mut c = self.conn();
        let tx = c.transaction()?;
        let state: Option<String> =
            tx.query_row("SELECT state FROM recordings WHERE id = ?1", [recording_id], |r| r.get(0)).optional()?;
        match state.as_deref().map(State::parse) {
            None => return Err(not_found(recording_id)),
            Some(State::Stopped) => return Err(StoreError::Conflict(format!("Recording {recording_id} is stopped"))),
            _ => {}
        }
        let active: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM series WHERE recording_id = ?1 AND device_id = ?2 AND class = ?3 \
             AND channel = ?4 AND field = ?5 AND removed_at IS NULL)",
            params![recording_id, signal.device_id, signal.class, signal.channel, signal.field],
            |r| r.get(0),
        )?;
        if active {
            return Err(StoreError::Conflict(format!(
                "{} {} ch{} {} is already recorded in recording {recording_id}",
                signal.device_id, signal.class, signal.channel, signal.field
            )));
        }
        tx.execute(
            "INSERT INTO series (recording_id, device_id, class, channel, field, label, unit, added_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![recording_id, signal.device_id, signal.class, signal.channel, signal.field, label, unit, t],
        )?;
        let id = tx.last_insert_rowid();
        let s = tx.query_row(&format!("SELECT {SERIES_COLS} FROM series WHERE id = ?1"), [id], row_to_series)?;
        tx.commit()?;
        Ok(s)
    }

    /// End a series (data kept; ending again keeps the first time).
    pub fn end_series(&self, recording_id: i64, series_id: i64, t: i64) -> Result<Series> {
        let c = self.conn();
        c.execute(
            "UPDATE series SET removed_at = COALESCE(removed_at, ?3) WHERE id = ?2 AND recording_id = ?1",
            params![recording_id, series_id, t],
        )?;
        c.query_row(
            &format!("SELECT {SERIES_COLS} FROM series WHERE id = ?2 AND recording_id = ?1"),
            params![recording_id, series_id],
            row_to_series,
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound(format!("Series {series_id} not found in recording {recording_id}")))
    }

    /// All series of a recording (active and removed), in creation order.
    pub fn series_of(&self, recording_id: i64) -> Result<Vec<Series>> {
        let c = self.conn();
        let mut st = c.prepare(&format!("SELECT {SERIES_COLS} FROM series WHERE recording_id = ?1 ORDER BY id"))?;
        let rows = st.query_map([recording_id], row_to_series)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    // ---- sampling ---------------------------------------------------------------------

    /// Active series (added at or before `t`) of recordings in state `recording` whose
    /// interval divides tick `t` (`(t / 1000) % interval_s == 0`), by recording and series.
    pub fn due_series(&self, t: i64) -> Result<Vec<DueSeries>> {
        let c = self.conn();
        let mut st = c.prepare(
            "SELECT r.id, s.id, s.device_id, s.class, s.channel, s.field FROM recordings r \
             JOIN series s ON s.recording_id = r.id \
             WHERE r.state = 'recording' AND s.removed_at IS NULL AND s.added_at <= ?1 AND (?1 / 1000) % r.interval_s = 0 \
             ORDER BY r.id, s.id",
        )?;
        let rows = st
            .query_map([t], |r| {
                Ok(DueSeries {
                    recording_id: r.get(0)?,
                    series_id: r.get(1)?,
                    signal: Signal { device_id: r.get(2)?, class: r.get(3)?, channel: r.get(4)?, field: r.get(5)? },
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Write one tick's samples in one transaction. Non-finite values, duplicates and
    /// samples for series that are no longer active in a running recording (removed,
    /// paused, deleted meanwhile) are skipped. Returns the number written.
    pub fn insert_samples(&self, t: i64, samples: &[(i64, f64)]) -> Result<usize> {
        let mut c = self.conn();
        let tx = c.transaction()?;
        let mut n = 0;
        {
            let mut st = tx.prepare(
                "INSERT OR IGNORE INTO samples (series_id, t, v) SELECT ?1, ?2, ?3 WHERE EXISTS ( \
                 SELECT 1 FROM series s JOIN recordings r ON r.id = s.recording_id \
                 WHERE s.id = ?1 AND s.removed_at IS NULL AND r.state = 'recording')",
            )?;
            for &(sid, v) in samples.iter().filter(|(_, v)| v.is_finite()) {
                n += st.execute(params![sid, t, v])?;
            }
        }
        tx.commit()?;
        Ok(n)
    }

    /// (number of samples, time of the last sample) of a recording.
    pub fn sample_stats(&self, recording_id: i64) -> Result<(i64, Option<i64>)> {
        Ok(self.conn().query_row(
            "SELECT COUNT(*), MAX(s.t) FROM samples s JOIN series se ON se.id = s.series_id WHERE se.recording_id = ?1",
            [recording_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }

    /// Samples of a series with `from <= t <= to`. More than `max_points` samples are
    /// decimated into `max_points / 2` equal time buckets keeping each bucket's minimum and
    /// maximum in time order, so peaks survive and at most `max_points` points come back.
    pub fn range(&self, series_id: i64, from: i64, to: i64, max_points: usize) -> Result<Points> {
        let c = self.conn();
        let (count, t0, t1): (i64, Option<i64>, Option<i64>) = c.query_row(
            "SELECT COUNT(*), MIN(t), MAX(t) FROM samples WHERE series_id = ?1 AND t BETWEEN ?2 AND ?3",
            params![series_id, from, to],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let mut st = c.prepare("SELECT t, v FROM samples WHERE series_id = ?1 AND t BETWEEN ?2 AND ?3 ORDER BY t")?;
        let rows = st.query_map(params![series_id, from, to], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)))?;
        let mut out = Points::default();
        let (Some(t0), Some(t1)) = (t0, t1) else { return Ok(out) };
        if count as usize <= max_points {
            for row in rows {
                let (t, v) = row?;
                out.t.push(t);
                out.v.push(v);
            }
            return Ok(out);
        }
        let buckets = (max_points / 2).max(1) as i128;
        let span = (t1 - t0) as i128 + 1;
        let bucket_of = |t: i64| ((t - t0) as i128 * buckets / span) as i64;
        type Pt = (i64, f64);
        // current bucket: (index, min (t, v), max (t, v))
        let mut cur: Option<(i64, Pt, Pt)> = None;
        let flush = |out: &mut Points, (_, lo, hi): (i64, Pt, Pt)| {
            let pts = if lo.0 == hi.0 { vec![lo] } else if lo.0 < hi.0 { vec![lo, hi] } else { vec![hi, lo] };
            for (t, v) in pts {
                out.t.push(t);
                out.v.push(v);
            }
        };
        for row in rows {
            let (t, v) = row?;
            let b = bucket_of(t);
            match &mut cur {
                Some((i, lo, hi)) if *i == b => {
                    if v < lo.1 {
                        *lo = (t, v);
                    }
                    if v > hi.1 {
                        *hi = (t, v);
                    }
                }
                _ => {
                    if let Some(prev) = cur.take() {
                        flush(&mut out, prev);
                    }
                    cur = Some((b, (t, v), (t, v)));
                }
            }
        }
        if let Some(prev) = cur {
            flush(&mut out, prev);
        }
        Ok(out)
    }

    /// Export table of a recording: its series (creation order) and one row per distinct
    /// `t` (ascending) with a value per series (`None` = no sample).
    pub fn export_rows(&self, recording_id: i64) -> Result<(Vec<Series>, Vec<ExportRow>)> {
        let series = self.series_of(recording_id)?;
        let index: std::collections::HashMap<i64, usize> = series.iter().enumerate().map(|(i, s)| (s.id, i)).collect();
        let c = self.conn();
        let mut st = c.prepare(
            "SELECT s.t, s.series_id, s.v FROM samples s JOIN series se ON se.id = s.series_id \
             WHERE se.recording_id = ?1 ORDER BY s.t, s.series_id",
        )?;
        let mut rows: Vec<ExportRow> = Vec::new();
        let it = st.query_map([recording_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, f64>(2)?)))?;
        for row in it {
            let (t, sid, v) = row?;
            if rows.last().is_none_or(|(lt, _)| *lt != t) {
                rows.push((t, vec![None; series.len()]));
            }
            if let (Some(&i), Some(last)) = (index.get(&sid), rows.last_mut()) {
                last.1[i] = Some(v);
            }
        }
        Ok((series, rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(field: &str) -> Signal {
        Signal { device_id: "t1".into(), class: "PSU".into(), channel: 1, field: field.into() }
    }

    fn store_with(interval: i64) -> (Store, Recording) {
        let s = Store::in_memory().unwrap();
        let r = s.create_recording("run", "", interval, 1_000).unwrap();
        (s, r)
    }

    #[test]
    fn create_get_list_and_unique_names() {
        let s = Store::in_memory().unwrap();
        let a = s.create_recording("a", "first", 1, 1_000).unwrap();
        assert_eq!((a.state, a.started_at, a.interval_s, a.description.as_str()), (State::Recording, 1_000, 1, "first"));
        assert_eq!(a.stopped_at, None);
        assert_eq!(s.create_recording("a", "", 1, 2_000), Err(StoreError::DuplicateName("a".into())));
        let b = s.create_recording("b", "", 5, 2_000).unwrap();
        assert_eq!(s.recording(a.id).unwrap(), a);
        let names: Vec<_> = s.list_recordings().unwrap().into_iter().map(|r| r.name).collect();
        assert_eq!(names, ["b", "a"], "newest first");
        assert!(matches!(s.recording(999), Err(StoreError::NotFound(_))));
        let _ = b;
    }

    #[test]
    fn rename_and_describe() {
        let s = Store::in_memory().unwrap();
        let a = s.create_recording("a", "", 1, 1_000).unwrap();
        s.create_recording("b", "", 1, 1_000).unwrap();
        let r = s.update_recording(a.id, Some("c"), None).unwrap();
        assert_eq!((r.name.as_str(), r.description.as_str()), ("c", ""));
        let r = s.update_recording(a.id, None, Some("notes")).unwrap();
        assert_eq!((r.name.as_str(), r.description.as_str()), ("c", "notes"));
        assert!(matches!(s.update_recording(a.id, Some("b"), None), Err(StoreError::DuplicateName(_))));
        assert!(matches!(s.update_recording(999, Some("x"), None), Err(StoreError::NotFound(_))));
    }

    #[test]
    fn state_changes_are_idempotent_and_stop_is_final() {
        let (s, r) = store_with(1);
        let p = s.pause(r.id, 2_000).unwrap();
        assert_eq!((p.state, p.paused_at), (State::Paused, Some(2_000)));
        assert_eq!(s.pause(r.id, 3_000).unwrap().paused_at, Some(2_000), "second pause keeps the first time");
        let x = s.resume(r.id).unwrap();
        assert_eq!((x.state, x.paused_at), (State::Recording, None));
        assert_eq!(s.resume(r.id).unwrap().state, State::Recording);
        s.pause(r.id, 6_000).unwrap();
        let st = s.stop(r.id, 7_000).unwrap();
        assert_eq!((st.state, st.stopped_at, st.paused_at), (State::Stopped, Some(7_000), None));
        assert_eq!(s.stop(r.id, 8_000).unwrap().stopped_at, Some(7_000));
        assert_eq!(s.resume(r.id).unwrap().state, State::Stopped, "stop is final");
        assert_eq!(s.pause(r.id, 9_000).unwrap().state, State::Stopped);
        assert!(matches!(s.pause(999, 1), Err(StoreError::NotFound(_))));
        assert!(matches!(s.resume(999), Err(StoreError::NotFound(_))));
        assert!(matches!(s.stop(999, 1), Err(StoreError::NotFound(_))));
    }

    #[test]
    fn series_add_end_and_conflicts() {
        let (s, r) = store_with(1);
        let a = s.add_series(r.id, &sig("VOUT"), "Tenma · VOUT", "V", 1_500).unwrap();
        assert_eq!((a.recording_id, a.label.as_str(), a.unit.as_str(), a.added_at, a.removed_at), (r.id, "Tenma · VOUT", "V", 1_500, None));
        assert!(matches!(s.add_series(r.id, &sig("VOUT"), "x", "", 1_600), Err(StoreError::Conflict(_))), "already active");
        s.add_series(r.id, &sig("IOUT"), "I", "A", 1_600).unwrap();
        let e = s.end_series(r.id, a.id, 2_000).unwrap();
        assert_eq!(e.removed_at, Some(2_000));
        assert_eq!(s.end_series(r.id, a.id, 3_000).unwrap().removed_at, Some(2_000));
        let again = s.add_series(r.id, &sig("VOUT"), "V", "V", 3_000).unwrap();
        assert_ne!(again.id, a.id, "re-adding creates a new series");
        assert_eq!(s.series_of(r.id).unwrap().len(), 3);
        assert!(matches!(s.end_series(r.id, 999, 1), Err(StoreError::NotFound(_))));
        let other = s.create_recording("other", "", 1, 1).unwrap();
        assert!(matches!(s.end_series(other.id, a.id, 1), Err(StoreError::NotFound(_))), "series of another recording");
        assert!(matches!(s.add_series(999, &sig("VOUT"), "x", "", 1), Err(StoreError::NotFound(_))));
        s.stop(r.id, 4_000).unwrap();
        assert!(matches!(s.add_series(r.id, &sig("POUT"), "x", "", 5_000), Err(StoreError::Conflict(_))), "stopped");
        // the same signal in another recording is a separate series
        assert!(s.add_series(other.id, &sig("VOUT"), "V", "V", 5_000).is_ok());
    }

    #[test]
    fn due_series_follow_interval_state_and_removal() {
        let s = Store::in_memory().unwrap();
        let r1 = s.create_recording("one", "", 1, 0).unwrap();
        let r2 = s.create_recording("two", "", 2, 0).unwrap();
        let a = s.add_series(r1.id, &sig("VOUT"), "a", "", 0).unwrap();
        let b = s.add_series(r2.id, &sig("VOUT"), "b", "", 0).unwrap();
        let ids = |t| s.due_series(t).unwrap().into_iter().map(|d| d.series_id).collect::<Vec<_>>();
        assert_eq!(ids(10_000), [a.id, b.id]);
        assert_eq!(ids(11_000), [a.id]);
        assert_eq!(s.due_series(10_000).unwrap()[0].signal, sig("VOUT"));
        let late = s.add_series(r1.id, &sig("IOUT"), "late", "", 10_500).unwrap();
        assert_eq!(ids(10_000), [a.id, b.id], "not sampled before it was added");
        assert_eq!(ids(11_000), [a.id, late.id]);
        s.end_series(r1.id, late.id, 11_500).unwrap();
        s.pause(r2.id, 1).unwrap();
        assert_eq!(ids(10_000), [a.id]);
        s.end_series(r1.id, a.id, 1).unwrap();
        assert!(ids(10_000).is_empty());
    }

    #[test]
    fn insert_samples_counts_and_skips() {
        let (s, r) = store_with(1);
        let a = s.add_series(r.id, &sig("VOUT"), "a", "", 0).unwrap();
        let b = s.add_series(r.id, &sig("IOUT"), "b", "", 0).unwrap();
        assert_eq!(s.sample_stats(r.id).unwrap(), (0, None));
        assert_eq!(s.insert_samples(2_000, &[(a.id, 1.0), (b.id, f64::NAN)]).unwrap(), 1, "non-finite skipped");
        assert_eq!(s.insert_samples(2_000, &[(a.id, 9.0)]).unwrap(), 0, "duplicate t ignored");
        assert_eq!(s.insert_samples(3_000, &[(a.id, 2.0), (b.id, 0.5), (999, 1.0)]).unwrap(), 2, "unknown series skipped");
        assert_eq!(s.sample_stats(r.id).unwrap(), (3, Some(3_000)));
        s.end_series(r.id, b.id, 3_500).unwrap();
        assert_eq!(s.insert_samples(4_000, &[(a.id, 3.0), (b.id, 1.0)]).unwrap(), 1, "removed series skipped");
        s.pause(r.id, 4_500).unwrap();
        assert_eq!(s.insert_samples(5_000, &[(a.id, 3.0)]).unwrap(), 0, "paused recording skipped");
        assert_eq!(s.range(a.id, 0, i64::MAX, 100).unwrap(), Points { t: vec![2_000, 3_000, 4_000], v: vec![1.0, 2.0, 3.0] });
    }

    #[test]
    fn range_filters_by_time() {
        let (s, r) = store_with(1);
        let a = s.add_series(r.id, &sig("VOUT"), "a", "", 0).unwrap();
        for i in 0..10 {
            s.insert_samples(i * 1_000, &[(a.id, i as f64)]).unwrap();
        }
        let p = s.range(a.id, 3_000, 5_000, 100).unwrap();
        assert_eq!(p.t, [3_000, 4_000, 5_000]);
        assert_eq!(p.v, [3.0, 4.0, 5.0]);
        assert_eq!(s.range(a.id, 20_000, 30_000, 100).unwrap(), Points::default());
    }

    #[test]
    fn decimation_keeps_peaks_bounded_and_ordered() {
        let (s, r) = store_with(1);
        let a = s.add_series(r.id, &sig("VOUT"), "a", "", 0).unwrap();
        let n = 10_000i64;
        let mut samples = Vec::new();
        for i in 0..n {
            let v = match i {
                1_234 => 100.0, // narrow positive spike
                7_777 => -50.0, // narrow negative spike
                _ => (i as f64 / 100.0).sin(),
            };
            samples.push((i * 1_000, v));
        }
        {
            // bulk insert, one tick per sample
            for (t, v) in &samples {
                s.insert_samples(*t, &[(a.id, *v)]).unwrap();
            }
        }
        for max in [10usize, 100, 2_000] {
            let p = s.range(a.id, 0, i64::MAX, max).unwrap();
            assert!(p.t.len() <= max, "{} > {max}", p.t.len());
            assert!(p.t.len() > max / 2, "uses the budget: {} for {max}", p.t.len());
            assert_eq!(p.t.len(), p.v.len());
            assert!(p.t.windows(2).all(|w| w[0] < w[1]), "time-ordered");
            assert!(p.v.contains(&100.0) && p.v.contains(&-50.0), "peaks survive at max_points={max}");
            assert!(p.t.contains(&1_234_000) && p.t.contains(&7_777_000));
        }
        // fewer samples than max_points: returned unchanged
        assert_eq!(s.range(a.id, 0, 4_000, 10).unwrap().t, [0, 1_000, 2_000, 3_000, 4_000]);
    }

    #[test]
    fn export_rows_have_one_row_per_t_with_gaps() {
        let (s, r) = store_with(1);
        let a = s.add_series(r.id, &sig("VOUT"), "a", "V", 0).unwrap();
        let b = s.add_series(r.id, &sig("IOUT"), "b", "A", 0).unwrap();
        s.insert_samples(1_000, &[(a.id, 1.0), (b.id, 0.1)]).unwrap();
        s.insert_samples(2_000, &[(b.id, 0.2)]).unwrap();
        s.insert_samples(3_000, &[(a.id, 3.0)]).unwrap();
        let (series, rows) = s.export_rows(r.id).unwrap();
        assert_eq!(series.iter().map(|x| x.id).collect::<Vec<_>>(), [a.id, b.id]);
        assert_eq!(rows, vec![(1_000, vec![Some(1.0), Some(0.1)]), (2_000, vec![None, Some(0.2)]), (3_000, vec![Some(3.0), None])]);
    }

    #[test]
    fn delete_cascades_to_series_and_samples() {
        let (s, r) = store_with(1);
        let a = s.add_series(r.id, &sig("VOUT"), "a", "", 0).unwrap();
        s.insert_samples(1_000, &[(a.id, 1.0)]).unwrap();
        s.delete_recording(r.id).unwrap();
        assert!(matches!(s.recording(r.id), Err(StoreError::NotFound(_))));
        assert!(s.series_of(r.id).unwrap().is_empty());
        assert_eq!(s.range(a.id, 0, i64::MAX, 100).unwrap(), Points::default(), "samples deleted by cascade");
        let left: i64 = s.conn().query_row("SELECT COUNT(*) FROM samples", [], |r| r.get(0)).unwrap();
        assert_eq!(left, 0);
        assert!(matches!(s.delete_recording(r.id), Err(StoreError::NotFound(_))));
    }

    #[test]
    fn orphans_are_stopped_at_their_last_sample_or_start() {
        let s = Store::in_memory().unwrap();
        let a = s.create_recording("with data", "", 1, 1_000).unwrap();
        let b = s.create_recording("paused, empty", "", 1, 2_000).unwrap();
        let c = s.create_recording("done", "", 1, 3_000).unwrap();
        let sa = s.add_series(a.id, &sig("VOUT"), "a", "", 1_000).unwrap();
        s.insert_samples(5_000, &[(sa.id, 1.0)]).unwrap();
        s.pause(b.id, 2_500).unwrap();
        s.stop(c.id, 4_000).unwrap();
        assert_eq!(s.close_orphans().unwrap(), 2);
        let a = s.recording(a.id).unwrap();
        assert_eq!((a.state, a.stopped_at), (State::Stopped, Some(5_000)));
        let b = s.recording(b.id).unwrap();
        assert_eq!((b.state, b.stopped_at, b.paused_at), (State::Stopped, Some(2_000), None));
        assert_eq!(s.recording(c.id).unwrap().stopped_at, Some(4_000));
        assert_eq!(s.close_orphans().unwrap(), 0);
    }

    #[test]
    fn file_database_uses_wal_drops_v1_tables_and_persists() {
        let dir = std::env::temp_dir().join(format!("bm-store-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("sub/recordings.db");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch("CREATE TABLE recording_series (id INTEGER PRIMARY KEY); CREATE TABLE data_points (id INTEGER PRIMARY KEY);")
                .unwrap();
        }
        {
            let s = Store::open(&path).unwrap();
            let mode: String = s.conn().query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
            assert_eq!(mode, "wal");
            let fk: i64 = s.conn().query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
            assert_eq!(fk, 1);
            let v1: i64 = s
                .conn()
                .query_row("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('recording_series', 'data_points')", [], |r| r.get(0))
                .unwrap();
            assert_eq!(v1, 0, "v1 tables dropped");
            s.create_recording("kept", "", 1, 1).unwrap();
        }
        assert_eq!(Store::open(&path).unwrap().list_recordings().unwrap()[0].name, "kept");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
