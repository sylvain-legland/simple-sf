use rusqlite::{Connection, params};
use std::sync::Mutex;
use std::path::PathBuf;

static DB: std::sync::OnceLock<Mutex<Connection>> = std::sync::OnceLock::new();

pub fn init_db(path: &str) {
    let conn = Connection::open(path).expect("Failed to open DB");
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").unwrap();
    conn.execute_batch(SCHEMA).unwrap();
    DB.set(Mutex::new(conn)).ok();
}

pub fn with_db<F, T>(f: F) -> T
where F: FnOnce(&Connection) -> T {
    let lock = DB.get().expect("DB not initialized");
    let conn = lock.lock().unwrap();
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

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    persona TEXT DEFAULT '',
    model TEXT DEFAULT 'default'
);

CREATE TABLE IF NOT EXISTS mission_phases (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    phase_name TEXT NOT NULL,
    pattern TEXT NOT NULL,
    status TEXT DEFAULT 'pending',
    agent_ids TEXT DEFAULT '[]',
    output TEXT DEFAULT '',
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
";

// Seed default agents
pub fn seed_agents() {
    with_db(|conn| {
        let count: i64 = conn.query_row("SELECT count(*) FROM agents", [], |r| r.get(0)).unwrap_or(0);
        if count > 0 { return; }

        let agents = vec![
            ("rte-marie", "Marie Lefevre", "rte", "You are Marie, a Release Train Engineer. You coordinate the team, define sprints, assign work, and ensure delivery. You decompose briefs into phases and assign agents."),
            ("po-lucas", "Lucas Martin", "product_owner", "You are Lucas, a Product Owner. You write user stories, define acceptance criteria, prioritize the backlog, and validate deliverables from a product perspective."),
            ("lead-thomas", "Thomas Dubois", "lead_dev", "You are Thomas, a Lead Developer. You make architecture decisions, decompose features into tasks, review code, and mentor developers. You choose the right tech stack and patterns."),
            ("dev-emma", "Emma Laurent", "developer", "You are Emma, a Frontend Developer. You write clean, tested code. You implement features assigned by the Lead. You use modern frameworks and best practices."),
            ("dev-karim", "Karim Benali", "developer", "You are Karim, a Backend Developer. You implement APIs, data models, and business logic. You write robust, well-tested code with proper error handling."),
            ("qa-sophie", "Sophie Durand", "qa", "You are Sophie, a QA Engineer. You write and run tests, review code for bugs, check edge cases, validate against acceptance criteria, and ensure quality before delivery."),
        ];

        for (id, name, role, persona) in agents {
            conn.execute(
                "INSERT OR IGNORE INTO agents (id, name, role, persona) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, role, persona],
            ).unwrap();
        }
    });
}
