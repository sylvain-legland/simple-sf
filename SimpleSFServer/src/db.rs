use rusqlite::{Connection, Result, params};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Db(Arc<Mutex<Connection>>);

impl Db {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Db(Arc::new(Mutex::new(conn))))
    }

    pub fn with<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&Connection) -> anyhow::Result<T>,
    {
        let conn = self.0.lock().map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        f(&conn)
    }

    pub fn migrate(&self) -> anyhow::Result<()> {
        self.with(|conn| {
            conn.execute_batch(SCHEMA)?;
            Ok(())
        })
    }
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id          TEXT PRIMARY KEY,
    email       TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    role        TEXT NOT NULL DEFAULT 'user',
    password_hash TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS projects (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    tech_stack   TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'idle',
    progress     REAL NOT NULL DEFAULT 0.0,
    current_phase TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS chat_sessions (
    id          TEXT PRIMARY KEY,
    project_id  TEXT,
    title       TEXT NOT NULL DEFAULT 'Chat',
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS chat_messages (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    role        TEXT NOT NULL,
    content     TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ideation_sessions (
    id          TEXT PRIMARY KEY,
    topic       TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',
    agents      TEXT NOT NULL DEFAULT '[]',
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ideation_messages (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES ideation_sessions(id) ON DELETE CASCADE,
    agent       TEXT NOT NULL,
    content     TEXT NOT NULL,
    turn        INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS llm_providers (
    name        TEXT PRIMARY KEY,
    api_key     TEXT NOT NULL DEFAULT '',
    enabled     INTEGER NOT NULL DEFAULT 0
);

-- Seed demo user if missing
INSERT OR IGNORE INTO users (id, email, display_name, role, password_hash)
VALUES ('demo-user-001', 'admin@demo.local', 'Demo Admin', 'admin',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uHWi');
"#;
