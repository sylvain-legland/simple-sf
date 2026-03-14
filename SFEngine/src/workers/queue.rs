// Ref: FT-SSF-026
use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn gen_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut h = DefaultHasher::new();
    now.hash(&mut h);
    format!("job-{:016x}", h.finish())
}

#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    Pending,
    Claimed,
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "claimed" => Self::Claimed,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub job_type: String,
    pub payload: String,
    pub status: JobStatus,
    pub claimed_by: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

pub struct JobQueue {
    pub db_path: String,
}

impl JobQueue {
    pub fn init(db_path: &str) -> Result<Self, String> {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS job_queue (
                id TEXT PRIMARY KEY,
                job_type TEXT NOT NULL,
                payload TEXT DEFAULT '{}',
                status TEXT DEFAULT 'pending',
                claimed_by TEXT,
                error TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            db_path: db_path.to_string(),
        })
    }

    pub fn enqueue(&self, job_type: &str, payload: &str) -> Result<String, String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        let id = gen_id();
        let now = now_epoch();
        conn.execute(
            "INSERT INTO job_queue (id, job_type, payload, status, created_at, updated_at) VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
            rusqlite::params![id, job_type, payload, now, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn claim(&self, worker_id: &str) -> Result<Option<Job>, String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        let now = now_epoch();
        // Atomic claim: update first pending row
        conn.execute(
            "UPDATE job_queue SET status = 'claimed', claimed_by = ?1, updated_at = ?2
             WHERE id = (SELECT id FROM job_queue WHERE status = 'pending' ORDER BY created_at LIMIT 1)",
            rusqlite::params![worker_id, now],
        )
        .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare("SELECT id, job_type, payload, status, claimed_by, created_at, updated_at FROM job_queue WHERE claimed_by = ?1 AND status = 'claimed' ORDER BY updated_at DESC LIMIT 1")
            .map_err(|e| e.to_string())?;

        let job = stmt
            .query_row(rusqlite::params![worker_id], |row| {
                Ok(Job {
                    id: row.get(0)?,
                    job_type: row.get(1)?,
                    payload: row.get(2)?,
                    status: JobStatus::Claimed,
                    claimed_by: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .ok();
        Ok(job)
    }

    pub fn complete(&self, job_id: &str) -> Result<(), String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE job_queue SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now_epoch(), job_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn fail(&self, job_id: &str, reason: &str) -> Result<(), String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE job_queue SET status = 'failed', error = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![reason, now_epoch(), job_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn pending_count(&self) -> Result<usize, String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM job_queue WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(count as usize)
    }
}
