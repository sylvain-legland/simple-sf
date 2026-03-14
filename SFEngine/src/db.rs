use rusqlite::Connection;
use std::sync::Mutex;

// Ref: FT-SSF-018
static DB: std::sync::OnceLock<Mutex<Connection>> = std::sync::OnceLock::new();

pub fn init_db(path: &str) {
    let conn = Connection::open(path).expect("Failed to open DB");
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").unwrap();
    conn.execute_batch(SCHEMA).unwrap();
    DB.set(Mutex::new(conn)).ok();
}

pub fn is_initialized() -> bool {
    DB.get().is_some()
}

pub fn with_db<F, T>(f: F) -> T
where F: FnOnce(&Connection) -> T {
    let lock = DB.get().expect("DB not initialized");
    let conn = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&conn)
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT DEFAULT '',
    tech TEXT DEFAULT '',
    status TEXT DEFAULT 'idea',
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS missions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    brief TEXT NOT NULL,
    status TEXT DEFAULT 'pending',
    workflow TEXT DEFAULT 'safe-standard',
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

-- Full agent schema matching SF platform (192 agents)
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    description TEXT DEFAULT '',
    system_prompt TEXT DEFAULT '',
    provider TEXT DEFAULT 'local',
    model TEXT DEFAULT 'default',
    temperature REAL DEFAULT 0.7,
    max_tokens INTEGER DEFAULT 4096,
    skills_json TEXT DEFAULT '[]',
    tools_json TEXT DEFAULT '[]',
    mcps_json TEXT DEFAULT '[]',
    permissions_json TEXT DEFAULT '{}',
    tags_json TEXT DEFAULT '[]',
    icon TEXT DEFAULT 'bot',
    color TEXT DEFAULT '#f78166',
    is_builtin INTEGER DEFAULT 0,
    avatar TEXT DEFAULT '',
    tagline TEXT DEFAULT '',
    persona TEXT DEFAULT '',
    motivation TEXT DEFAULT '',
    hierarchy_rank INTEGER DEFAULT 50,
    project_id TEXT DEFAULT '',
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Skills (1286 skills from SF platform)
CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT DEFAULT '',
    content TEXT DEFAULT '',
    source TEXT DEFAULT '',
    source_url TEXT DEFAULT '',
    tags_json TEXT DEFAULT '[]',
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Organizational patterns (19 patterns)
CREATE TABLE IF NOT EXISTS patterns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT DEFAULT '',
    type TEXT NOT NULL,
    agents_json TEXT DEFAULT '[]',
    edges_json TEXT DEFAULT '[]',
    config_json TEXT DEFAULT '{}',
    memory_config_json TEXT DEFAULT '{}',
    icon TEXT DEFAULT '',
    is_builtin INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Workflow templates (42 workflows)
CREATE TABLE IF NOT EXISTS workflows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT DEFAULT '',
    phases_json TEXT DEFAULT '[]',
    config_json TEXT DEFAULT '{}',
    icon TEXT DEFAULT '',
    is_builtin INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS memory (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    category TEXT DEFAULT 'note',
    project_id TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS mission_phases (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    phase_name TEXT NOT NULL,
    pattern TEXT NOT NULL,
    phase_type TEXT DEFAULT 'once',
    status TEXT DEFAULT 'pending',
    agent_ids TEXT DEFAULT '[]',
    output TEXT DEFAULT '',
    gate_result TEXT,
    iteration INTEGER DEFAULT 1,
    max_iterations INTEGER DEFAULT 1,
    on_veto TEXT,
    tickets TEXT,
    started_at TEXT,
    completed_at TEXT,
    FOREIGN KEY (mission_id) REFERENCES missions(id)
);

CREATE TABLE IF NOT EXISTS agent_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mission_id TEXT NOT NULL,
    phase_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tool_calls TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (mission_id) REFERENCES missions(id)
);

CREATE TABLE IF NOT EXISTS artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mission_id TEXT NOT NULL,
    phase_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ideation_sessions (
    id TEXT PRIMARY KEY,
    idea TEXT NOT NULL,
    status TEXT DEFAULT 'running',
    created_at TEXT DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS ideation_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    round INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES ideation_sessions(id)
);

CREATE TABLE IF NOT EXISTS discussion_sessions (
    id TEXT PRIMARY KEY,
    topic TEXT NOT NULL,
    context TEXT DEFAULT '',
    status TEXT DEFAULT 'running',
    created_at TEXT DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS discussion_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    agent_role TEXT NOT NULL,
    round INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES discussion_sessions(id)
);

-- AST-based code index for semantic search
CREATE TABLE IF NOT EXISTS code_chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace TEXT NOT NULL,
    file_path TEXT NOT NULL,
    language TEXT NOT NULL,
    chunk_type TEXT NOT NULL,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    embedding BLOB,
    file_mtime INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_code_chunks_workspace ON code_chunks(workspace);
CREATE INDEX IF NOT EXISTS idx_code_chunks_file ON code_chunks(workspace, file_path);

-- FTS5 full-text search over code chunks
CREATE VIRTUAL TABLE IF NOT EXISTS code_chunks_fts USING fts5(
    name, content
);
";

/// Seed all SF platform data from bundled JSON files
pub fn seed_from_json(data_dir: &str) {
    crate::catalog::seed_from_json(data_dir);
}
