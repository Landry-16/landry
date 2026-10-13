//! Landry's SQLite store: usage history (frecency with time-of-day affinity), query history,
//! snippets, notes, tasks and time tracking.

use chrono::{Local, TimeZone, Timelike};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;

const DAY: i64 = 86_400;
const KEEP_DAYS: i64 = 120;
const HALF_LIFE_DAYS: f64 = 14.0;

pub struct Db {
    conn: Connection,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Note {
    pub id: i64,
    pub body: String,
    pub updated: i64,
}

#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct Task {
    pub id: i64,
    pub text: String,
    pub due: Option<i64>,
    pub done: bool,
}

/// A timer as stored in SQLite; `kind` is "countdown", "stopwatch" or "pomodoro".
#[derive(Clone, Debug, PartialEq)]
pub struct StoredTimer {
    pub id: u32,
    pub kind: String,
    pub label: String,
    pub start: i64,
    pub length: i64,
    pub focus: bool,
    pub round: u32,
}

#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct TimeEntry {
    pub id: i64,
    pub label: String,
    pub start: i64,
    pub end: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Snippet {
    pub id: i64,
    pub name: String,
    pub keyword: String,
    pub body: String,
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn local_hour(ts: i64) -> u32 {
    Local.timestamp_opt(ts, 0).single().map(|d| d.hour()).unwrap_or(0)
}

/// Weight of one use at `ts`, seen from `now`: recent and same-hour uses count more.
pub fn weight(ts: i64, now: i64) -> f64 {
    let age_days = (now - ts).max(0) as f64 / DAY as f64;
    let decay = 0.5f64.powf(age_days / HALF_LIFE_DAYS);
    let (h, hn) = (local_hour(ts) as i32, local_hour(now) as i32);
    let dist = (h - hn).abs().min(24 - (h - hn).abs());
    decay * if dist <= 1 { 1.5 } else { 1.0 }
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Db> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS usage (item_id TEXT NOT NULL, ts INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS usage_item ON usage(item_id);
             CREATE TABLE IF NOT EXISTS queries (text TEXT PRIMARY KEY, ts INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS snippets (
                 id INTEGER PRIMARY KEY, name TEXT NOT NULL, keyword TEXT NOT NULL DEFAULT '',
                 body TEXT NOT NULL, updated INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY, body TEXT NOT NULL, updated INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS tasks (
                 id INTEGER PRIMARY KEY, text TEXT NOT NULL, due INTEGER, done INTEGER NOT NULL DEFAULT 0,
                 notified INTEGER NOT NULL DEFAULT 0, created INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS time_entries (
                 id INTEGER PRIMARY KEY, label TEXT NOT NULL, start INTEGER NOT NULL, end INTEGER);
             CREATE TABLE IF NOT EXISTS timers (
                 id INTEGER PRIMARY KEY, kind TEXT NOT NULL, label TEXT NOT NULL, start INTEGER NOT NULL,
                 length INTEGER NOT NULL, focus INTEGER NOT NULL, round INTEGER NOT NULL);",
        )?;
        let db = Db { conn };
        db.prune(now())?;
        Ok(db)
    }

    pub fn record_use(&self, item_id: &str, ts: i64) -> rusqlite::Result<()> {
        self.conn.execute("INSERT INTO usage (item_id, ts) VALUES (?1, ?2)", params![item_id, ts])?;
        Ok(())
    }

    pub fn frecency(&self, now: i64) -> rusqlite::Result<HashMap<String, f64>> {
        let mut stmt = self.conn.prepare("SELECT item_id, ts FROM usage")?;
        let mut out = HashMap::new();
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
            let (id, ts) = row?;
            *out.entry(id).or_insert(0.0) += weight(ts, now);
        }
        Ok(out)
    }

    pub fn push_query(&self, text: &str, ts: i64) -> rusqlite::Result<()> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(());
        }
        self.conn.execute(
            "INSERT INTO queries (text, ts) VALUES (?1, ?2) ON CONFLICT(text) DO UPDATE SET ts = ?2",
            params![text, ts],
        )?;
        Ok(())
    }

    /// Most recent first.
    pub fn recent_queries(&self, limit: usize) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT text FROM queries ORDER BY ts DESC, rowid DESC LIMIT ?1")?;
        let rows = stmt.query_map([limit as i64], |r| r.get(0))?;
        rows.collect()
    }

    pub fn snippets(&self) -> rusqlite::Result<Vec<Snippet>> {
        let mut stmt = self.conn.prepare("SELECT id, name, keyword, body FROM snippets ORDER BY name COLLATE NOCASE")?;
        let rows = stmt.query_map([], |r| Ok(Snippet { id: r.get(0)?, name: r.get(1)?, keyword: r.get(2)?, body: r.get(3)? }))?;
        rows.collect()
    }

    /// Insert (id 0) or update; returns the id.
    pub fn save_snippet(&self, s: &Snippet) -> rusqlite::Result<i64> {
        if s.id == 0 {
            self.conn.execute(
                "INSERT INTO snippets (name, keyword, body, updated) VALUES (?1, ?2, ?3, ?4)",
                params![s.name, s.keyword, s.body, now()],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE snippets SET name = ?1, keyword = ?2, body = ?3, updated = ?4 WHERE id = ?5",
                params![s.name, s.keyword, s.body, now(), s.id],
            )?;
            Ok(s.id)
        }
    }

    pub fn delete_snippet(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM snippets WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------- Notes ----------

    pub fn notes(&self) -> rusqlite::Result<Vec<Note>> {
        let mut stmt = self.conn.prepare("SELECT id, body, updated FROM notes ORDER BY updated DESC")?;
        let rows = stmt.query_map([], |r| Ok(Note { id: r.get(0)?, body: r.get(1)?, updated: r.get(2)? }))?;
        rows.collect()
    }

    /// Insert (id 0) or update; returns the id.
    pub fn save_note(&self, id: i64, body: &str) -> rusqlite::Result<i64> {
        if id == 0 {
            self.conn.execute("INSERT INTO notes (body, updated) VALUES (?1, ?2)", params![body, now()])?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute("UPDATE notes SET body = ?1, updated = ?2 WHERE id = ?3", params![body, now(), id])?;
            Ok(id)
        }
    }

    pub fn delete_note(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM notes WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------- Tasks ----------

    pub fn add_task(&self, text: &str, due: Option<i64>) -> rusqlite::Result<i64> {
        self.conn.execute("INSERT INTO tasks (text, due, created) VALUES (?1, ?2, ?3)", params![text, due, now()])?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Open tasks, the ones with a due date first.
    pub fn open_tasks(&self) -> rusqlite::Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, due, done FROM tasks WHERE done = 0 ORDER BY due IS NULL, due, created",
        )?;
        let rows = stmt.query_map([], |r| Ok(Task { id: r.get(0)?, text: r.get(1)?, due: r.get(2)?, done: r.get::<_, i64>(3)? != 0 }))?;
        rows.collect()
    }

    pub fn finish_task(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("UPDATE tasks SET done = 1 WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn delete_task(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Open tasks whose reminder is due and not sent yet; marks them as sent.
    pub fn take_due_reminders(&self, now: i64) -> rusqlite::Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, due, done FROM tasks WHERE done = 0 AND notified = 0 AND due IS NOT NULL AND due <= ?1",
        )?;
        let due: Vec<Task> = stmt
            .query_map([now], |r| Ok(Task { id: r.get(0)?, text: r.get(1)?, due: r.get(2)?, done: false }))?
            .collect::<rusqlite::Result<_>>()?;
        for t in &due {
            self.conn.execute("UPDATE tasks SET notified = 1 WHERE id = ?1", [t.id])?;
        }
        Ok(due)
    }

    // ---------- Timers ----------

    /// Replace the stored timers (a handful of rows, rewritten on each change).
    pub fn save_timers(&self, timers: &[StoredTimer]) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM timers", [])?;
        for t in timers {
            self.conn.execute(
                "INSERT INTO timers (id, kind, label, start, length, focus, round) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![t.id, t.kind, t.label, t.start, t.length, t.focus, t.round],
            )?;
        }
        Ok(())
    }

    pub fn load_timers(&self) -> rusqlite::Result<Vec<StoredTimer>> {
        let mut stmt = self.conn.prepare("SELECT id, kind, label, start, length, focus, round FROM timers ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok(StoredTimer { id: r.get(0)?, kind: r.get(1)?, label: r.get(2)?, start: r.get(3)?, length: r.get(4)?, focus: r.get(5)?, round: r.get(6)? })
        })?;
        rows.collect()
    }

    // ---------- Time tracking ----------

    /// Start tracking `label`, stopping whatever was running.
    pub fn start_tracking(&self, label: &str, at: i64) -> rusqlite::Result<()> {
        self.stop_tracking(at)?;
        self.conn.execute("INSERT INTO time_entries (label, start) VALUES (?1, ?2)", params![label, at])?;
        Ok(())
    }

    /// Stop the running entry; returns it.
    pub fn stop_tracking(&self, at: i64) -> rusqlite::Result<Option<TimeEntry>> {
        let running = self.running_entry()?;
        if let Some(e) = &running {
            self.conn.execute("UPDATE time_entries SET end = ?1 WHERE id = ?2", params![at, e.id])?;
        }
        Ok(running.map(|e| TimeEntry { end: Some(at), ..e }))
    }

    pub fn running_entry(&self) -> rusqlite::Result<Option<TimeEntry>> {
        let mut stmt = self.conn.prepare("SELECT id, label, start, end FROM time_entries WHERE end IS NULL ORDER BY start DESC LIMIT 1")?;
        let mut rows = stmt.query_map([], |r| Ok(TimeEntry { id: r.get(0)?, label: r.get(1)?, start: r.get(2)?, end: r.get(3)? }))?;
        rows.next().transpose()
    }

    /// Entries started at or after `since`, oldest first.
    pub fn entries_since(&self, since: i64) -> rusqlite::Result<Vec<TimeEntry>> {
        let mut stmt = self.conn.prepare("SELECT id, label, start, end FROM time_entries WHERE start >= ?1 ORDER BY start")?;
        let rows = stmt.query_map([since], |r| Ok(TimeEntry { id: r.get(0)?, label: r.get(1)?, start: r.get(2)?, end: r.get(3)? }))?;
        rows.collect()
    }

    fn prune(&self, now: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM usage WHERE ts < ?1", [now - KEEP_DAYS * DAY])?;
        self.conn.execute(
            "DELETE FROM queries WHERE text NOT IN (SELECT text FROM queries ORDER BY ts DESC LIMIT 200)",
            [],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Db {
        // Tests run in parallel: each needs its own file.
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("landry-usage-{}-{n}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        Db::open(&path).unwrap()
    }

    #[test]
    fn recent_counts_more_than_old() {
        let n = now();
        assert!(weight(n - DAY, n) > weight(n - 30 * DAY, n));
        // Same hour yesterday beats 12 hours away yesterday.
        assert!(weight(n - DAY, n) > weight(n - DAY - 12 * 3600, n));
    }

    #[test]
    fn frecency_accumulates() {
        let db = mem();
        let n = now();
        db.record_use("a", n).unwrap();
        db.record_use("a", n - DAY).unwrap();
        db.record_use("b", n - 60 * DAY).unwrap();
        let f = db.frecency(n).unwrap();
        assert!(f["a"] > f["b"]);
    }

    #[test]
    fn snippet_crud() {
        let db = mem();
        let mut s = Snippet { id: 0, name: "Signature".into(), keyword: "sig".into(), body: "Landry".into() };
        s.id = db.save_snippet(&s).unwrap();
        s.body = "Landry, développeur".into();
        assert_eq!(db.save_snippet(&s).unwrap(), s.id);
        assert_eq!(db.snippets().unwrap(), vec![s.clone()]);
        db.delete_snippet(s.id).unwrap();
        assert!(db.snippets().unwrap().is_empty());
    }

    #[test]
    fn notes_tasks_and_tracking() {
        let db = mem();
        let id = db.save_note(0, "# Idées").unwrap();
        db.save_note(id, "# Idées\n- Landry").unwrap();
        assert_eq!(db.notes().unwrap()[0].body, "# Idées\n- Landry");

        let n = now();
        let t1 = db.add_task("appeler", Some(n - 10)).unwrap();
        db.add_task("lire", None).unwrap();
        db.add_task("plus tard", Some(n + 3600)).unwrap();
        assert_eq!(db.open_tasks().unwrap()[0].text, "appeler");
        assert_eq!(db.take_due_reminders(n).unwrap().len(), 1);
        assert!(db.take_due_reminders(n).unwrap().is_empty(), "un seul rappel");
        db.finish_task(t1).unwrap();
        assert_eq!(db.open_tasks().unwrap().len(), 2);

        db.start_tracking("nacre", 1000).unwrap();
        db.start_tracking("client B", 1600).unwrap();
        assert_eq!(db.running_entry().unwrap().unwrap().label, "client B");
        assert_eq!(db.stop_tracking(2000).unwrap().unwrap().end, Some(2000));
        let e = db.entries_since(0).unwrap();
        assert_eq!((e[0].label.as_str(), e[0].end), ("nacre", Some(1600)));
        assert!(db.running_entry().unwrap().is_none());
    }

    #[test]
    fn query_history_dedups_and_orders() {
        let db = mem();
        db.push_query("vsc", 1).unwrap();
        db.push_query("term", 2).unwrap();
        db.push_query("  ", 3).unwrap();
        db.push_query("vsc", 4).unwrap();
        assert_eq!(db.recent_queries(10).unwrap(), vec!["vsc", "term"]);
    }
}
