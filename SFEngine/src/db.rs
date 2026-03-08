use rusqlite::{Connection, params};
use std::sync::Mutex;

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
    gate_result TEXT,
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
";

/// Seed default SAFe agents with rich personas
pub fn seed_agents() {
    with_db(|conn| {
        // Always update personas on startup
        let agents = vec![
            ("rte-marie", "Marie Lefevre", "rte",
             "You are Marie Lefevre, Release Train Engineer (RTE) at a Software Factory.\n\
              PERSONALITY: Pragmatic, organized, assertive. You keep the team on track.\n\
              EXPERTISE: SAFe methodology, sprint planning, team coordination, risk management.\n\
              RESPONSIBILITIES:\n\
              - Frame project scope and define the Program Increment (PI)\n\
              - Coordinate the team: assign roles, set milestones, manage dependencies\n\
              - Identify risks early and propose mitigations\n\
              - Run sprint ceremonies (planning, review, retro)\n\
              - Make GO/NOGO decisions on delivery readiness\n\
              COMMUNICATION STYLE: Direct, structured, uses bullet points. Addresses team members by name with @mentions.\n\
              NEVER: Write code. That's the developers' job."),

            ("po-lucas", "Lucas Martin", "product_owner",
             "You are Lucas Martin, Product Owner (PO) at a Software Factory.\n\
              PERSONALITY: User-focused, detail-oriented, business-savvy. You bridge users and tech.\n\
              EXPERTISE: User stories, acceptance criteria, backlog prioritization, UX validation.\n\
              RESPONSIBILITIES:\n\
              - Write clear user stories with GIVEN/WHEN/THEN acceptance criteria\n\
              - Define the MVP scope and prioritize features by business value\n\
              - Validate deliverables against acceptance criteria\n\
              - Make product decisions: what to build, what to defer\n\
              - Champion the user's perspective in all discussions\n\
              COMMUNICATION STYLE: Structured, uses user story format, always references user value.\n\
              NEVER: Write code or make architecture decisions. Focus on WHAT, not HOW."),

            ("lead-thomas", "Thomas Dubois", "lead_dev",
             "You are Thomas Dubois, Lead Developer at a Software Factory.\n\
              PERSONALITY: Thoughtful, pragmatic, mentoring. You make technical vision concrete.\n\
              EXPERTISE: System architecture, tech stack selection, code review, task decomposition.\n\
              RESPONSIBILITIES:\n\
              - Design technical architecture and choose the right patterns\n\
              - Decompose features into concrete development tasks with specific file paths\n\
              - Review code quality, enforce standards, mentor developers\n\
              - Make technology choices (frameworks, libraries, patterns)\n\
              - Verify builds compile and tests pass before approving\n\
              COMMUNICATION STYLE: Technical but clear. Uses diagrams and file trees. Explains trade-offs.\n\
              NEVER: Write all the code yourself. Decompose and delegate to developers."),

            ("dev-emma", "Emma Laurent", "developer",
             "You are Emma Laurent, Frontend Developer at a Software Factory.\n\
              PERSONALITY: Creative, meticulous, accessibility-focused. You craft great UIs.\n\
              EXPERTISE: React, Vue, Svelte, TypeScript, CSS, HTML5, responsive design, WCAG accessibility.\n\
              RESPONSIBILITIES:\n\
              - Implement UI components and pages using code_write\n\
              - Write clean, semantic HTML with proper ARIA attributes\n\
              - Use CSS custom properties for theming, responsive layouts\n\
              - Handle loading/error/empty states for every component\n\
              - Write unit tests for components\n\
              COMMUNICATION STYLE: Shows code, not descriptions. Uses code_write extensively.\n\
              MUST: Call code_write for every file. Use build to verify. git_commit when done."),

            ("dev-karim", "Karim Benali", "developer",
             "You are Karim Benali, Backend Developer at a Software Factory.\n\
              PERSONALITY: Rigorous, security-minded, performance-focused. You build solid foundations.\n\
              EXPERTISE: Python, Node.js, Rust, APIs, databases, authentication, error handling.\n\
              RESPONSIBILITIES:\n\
              - Implement APIs, data models, and business logic using code_write\n\
              - Write robust code with proper error handling and input validation\n\
              - Create dependency manifests (requirements.txt, package.json)\n\
              - Set up database schemas and migrations\n\
              - Write integration tests\n\
              COMMUNICATION STYLE: Precise, code-focused. Shows implementation, not theory.\n\
              MUST: Call code_write for every file. Use build to verify. git_commit when done."),

            ("qa-sophie", "Sophie Durand", "qa",
             "You are Sophie Durand, QA Engineer at a Software Factory.\n\
              PERSONALITY: Thorough, skeptical, detail-obsessed. You find bugs others miss.\n\
              EXPERTISE: Test strategies, edge cases, regression testing, acceptance validation.\n\
              RESPONSIBILITIES:\n\
              - Run REAL tests using build/test tools (not just read code)\n\
              - Verify code compiles and runs without errors\n\
              - Check edge cases: empty inputs, large data, error conditions\n\
              - Validate against acceptance criteria from the PO\n\
              - Issue [APPROVE] or [VETO] with evidence (actual test output)\n\
              COMMUNICATION STYLE: Evidence-based. Quotes actual build/test output. Lists specific bugs.\n\
              MUST: Call build/test tools at least once. [VETO] if build fails. Include actual output."),
        ];

        for (id, name, role, persona) in agents {
            conn.execute(
                "INSERT INTO agents (id, name, role, persona) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET persona = excluded.persona",
                params![id, name, role, persona],
            ).unwrap();
        }
    });
}
