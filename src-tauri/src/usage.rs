//! Usage history (frecency with time-of-day affinity) and query history, in SQLite.

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
             CREATE TABLE IF NOT EXISTS queries (text TEXT PRIMARY KEY, ts INTEGER NOT NULL);",
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
    fn query_history_dedups_and_orders() {
        let db = mem();
        db.push_query("vsc", 1).unwrap();
        db.push_query("term", 2).unwrap();
        db.push_query("  ", 3).unwrap();
        db.push_query("vsc", 4).unwrap();
        assert_eq!(db.recent_queries(10).unwrap(), vec!["vsc", "term"]);
    }
}
