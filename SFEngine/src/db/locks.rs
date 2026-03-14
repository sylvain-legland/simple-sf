// Ref: FT-SSF-026
use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AdvisoryLock {
    pub lock_id: String,
    pub holder: String,
    pub acquired_at: u64,
}

pub fn init_locks_table(db_path: &str) {
    let conn = Connection::open(db_path).expect("Failed to open DB for locks");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS advisory_locks (
            lock_id TEXT PRIMARY KEY,
            holder TEXT NOT NULL,
            acquired_at INTEGER NOT NULL
        );",
    )
    .expect("Failed to create advisory_locks table");
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn try_acquire(db_path: &str, lock_id: &str, holder: &str) -> Result<bool, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let rows = conn
        .execute(
            "INSERT OR IGNORE INTO advisory_locks (lock_id, holder, acquired_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![lock_id, holder, now_epoch()],
        )
        .map_err(|e| e.to_string())?;
    Ok(rows > 0)
}

pub fn release(db_path: &str, lock_id: &str, holder: &str) -> Result<bool, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let rows = conn
        .execute(
            "DELETE FROM advisory_locks WHERE lock_id = ?1 AND holder = ?2",
            rusqlite::params![lock_id, holder],
        )
        .map_err(|e| e.to_string())?;
    Ok(rows > 0)
}

pub fn is_locked(db_path: &str, lock_id: &str) -> Result<bool, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM advisory_locks WHERE lock_id = ?1",
            rusqlite::params![lock_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count > 0)
}
