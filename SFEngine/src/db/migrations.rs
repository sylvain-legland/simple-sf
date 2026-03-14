// Ref: FT-SSF-026
use rusqlite::Connection;

pub struct Migration {
    pub version: u32,
    pub name: String,
    pub up_sql: String,
    pub down_sql: String,
}

pub struct MigrationRunner {
    pub migrations: Vec<Migration>,
}

impl MigrationRunner {
    pub fn new() -> Self {
        Self {
            migrations: vec![
                Migration {
                    version: 1,
                    name: "base_schema".into(),
                    up_sql: "CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY, name TEXT NOT NULL, status TEXT DEFAULT 'idea', created_at TEXT DEFAULT (datetime('now')));
                              CREATE TABLE IF NOT EXISTS missions (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, brief TEXT NOT NULL, status TEXT DEFAULT 'pending', created_at TEXT DEFAULT (datetime('now')));
                              CREATE TABLE IF NOT EXISTS agents (id TEXT PRIMARY KEY, name TEXT NOT NULL, role TEXT NOT NULL, system_prompt TEXT DEFAULT '', created_at TEXT DEFAULT (datetime('now')));
                              CREATE TABLE IF NOT EXISTS skills (id TEXT PRIMARY KEY, name TEXT NOT NULL, content TEXT DEFAULT '', created_at TEXT DEFAULT (datetime('now')));
                              CREATE TABLE IF NOT EXISTS memory (id INTEGER PRIMARY KEY AUTOINCREMENT, key TEXT NOT NULL, value TEXT NOT NULL, category TEXT DEFAULT 'note', created_at TEXT DEFAULT (datetime('now')));".into(),
                    down_sql: "DROP TABLE IF EXISTS memory; DROP TABLE IF EXISTS skills; DROP TABLE IF EXISTS agents; DROP TABLE IF EXISTS missions; DROP TABLE IF EXISTS projects;".into(),
                },
                Migration {
                    version: 2,
                    name: "advisory_locks".into(),
                    up_sql: "CREATE TABLE IF NOT EXISTS advisory_locks (lock_id TEXT PRIMARY KEY, holder TEXT NOT NULL, acquired_at INTEGER NOT NULL);".into(),
                    down_sql: "DROP TABLE IF EXISTS advisory_locks;".into(),
                },
                Migration {
                    version: 3,
                    name: "event_log".into(),
                    up_sql: "CREATE TABLE IF NOT EXISTS event_log (id INTEGER PRIMARY KEY AUTOINCREMENT, event_type TEXT NOT NULL, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, payload TEXT DEFAULT '{}', created_at TEXT DEFAULT (datetime('now')));".into(),
                    down_sql: "DROP TABLE IF EXISTS event_log;".into(),
                },
                Migration {
                    version: 4,
                    name: "job_queue".into(),
                    up_sql: "CREATE TABLE IF NOT EXISTS job_queue (id TEXT PRIMARY KEY, job_type TEXT NOT NULL, payload TEXT DEFAULT '{}', status TEXT DEFAULT 'pending', claimed_by TEXT, error TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);".into(),
                    down_sql: "DROP TABLE IF EXISTS job_queue;".into(),
                },
                Migration {
                    version: 5,
                    name: "fts5_tables".into(),
                    up_sql: "CREATE VIRTUAL TABLE IF NOT EXISTS skills_fts USING fts5(name, content);
                              CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(key, value);".into(),
                    down_sql: "DROP TABLE IF EXISTS memory_fts; DROP TABLE IF EXISTS skills_fts;".into(),
                },
            ],
        }
    }

    pub fn current_version(&self, db_path: &str) -> u32 {
        let conn = match Connection::open(db_path) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT DEFAULT (datetime('now')));",
        )
        .ok();
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _migrations",
            [],
            |row| row.get::<_, u32>(0),
        )
        .unwrap_or(0)
    }

    pub fn migrate_up(&self, db_path: &str) -> Result<u32, String> {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT DEFAULT (datetime('now')));",
        )
        .map_err(|e| e.to_string())?;

        let current = self.current_version(db_path);
        let mut applied = current;

        for m in &self.migrations {
            if m.version > current {
                conn.execute_batch(&m.up_sql)
                    .map_err(|e| format!("Migration v{} ({}): {}", m.version, m.name, e))?;
                conn.execute(
                    "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
                    rusqlite::params![m.version, m.name],
                )
                .map_err(|e| e.to_string())?;
                applied = m.version;
            }
        }
        Ok(applied)
    }

    pub fn migrate_down(&self, db_path: &str, target: u32) -> Result<u32, String> {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        let current = self.current_version(db_path);

        for m in self.migrations.iter().rev() {
            if m.version > target && m.version <= current {
                conn.execute_batch(&m.down_sql)
                    .map_err(|e| format!("Rollback v{} ({}): {}", m.version, m.name, e))?;
                conn.execute(
                    "DELETE FROM _migrations WHERE version = ?1",
                    rusqlite::params![m.version],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        Ok(target)
    }
}
