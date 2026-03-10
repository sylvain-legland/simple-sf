use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;
use crate::{db, llm, engine, agents, ideation};
use crate::executor::{AgentEvent, EventCallback};

// Global tokio runtime
static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

fn runtime() -> &'static tokio::runtime::Runtime {
    RT.get_or_init(|| tokio::runtime::Runtime::new().unwrap())
}

// Global event callback
type SwiftCallback = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char);
static CALLBACK: std::sync::OnceLock<Mutex<Option<SwiftCallback>>> = std::sync::OnceLock::new();

fn c_str(s: &str) -> CString {
    CString::new(s).unwrap_or_else(|_| CString::new("").unwrap())
}

fn from_c(p: *const c_char) -> String {
    if p.is_null() { return String::new(); }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().to_string()
}

// ──────────────────────────────────────────
// FFI: Init
// ──────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn sf_init(db_path: *const c_char, data_dir: *const c_char) {
    let path = from_c(db_path);
    let p = if path.is_empty() { "sf_engine.db".to_string() } else { path };
    db::init_db(&p);

    let dir = from_c(data_dir);
    if !dir.is_empty() {
        db::seed_from_json(&dir);
    }
}

/// Register a callback: (agent_id, event_type, data)
#[unsafe(no_mangle)]
pub extern "C" fn sf_set_callback(cb: SwiftCallback) {
    let lock = CALLBACK.get_or_init(|| Mutex::new(None));
    *lock.lock().unwrap() = Some(cb);
}

fn emit(agent_id: &str, event_type: &str, data: &str) {
    if let Some(lock) = CALLBACK.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(cb) = *guard {
                let a = c_str(agent_id);
                let t = c_str(event_type);
                let d = c_str(data);
                unsafe { cb(a.as_ptr(), t.as_ptr(), d.as_ptr()); }
            }
        }
    }
}

// ──────────────────────────────────────────
// FFI: LLM Config
// ──────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn sf_configure_llm(
    provider: *const c_char,
    api_key: *const c_char,
    base_url: *const c_char,
    model: *const c_char,
) {
    llm::configure_llm(&from_c(provider), &from_c(api_key), &from_c(base_url), &from_c(model));
}

/// Enable/disable YOLO mode (auto-approve all gates).
#[unsafe(no_mangle)]
pub extern "C" fn sf_set_yolo(enabled: bool) {
    engine::YOLO_MODE.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

// ──────────────────────────────────────────
// FFI: Projects
// ──────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn sf_create_project(name: *const c_char, description: *const c_char, tech: *const c_char) -> *mut c_char {
    let id = uuid::Uuid::new_v4().to_string();
    let n = from_c(name);
    let d = from_c(description);
    let t = from_c(tech);
    db::with_db(|conn| {
        if let Err(e) = conn.execute(
            "INSERT INTO projects (id, name, description, tech) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![&id, &n, &d, &t],
        ) {
            eprintln!("[db] Failed to create project: {}", e);
        }
    });
    c_str(&id).into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn sf_list_projects() -> *mut c_char {
    let json = db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, tech, status, created_at FROM projects ORDER BY created_at DESC"
        ).unwrap();
        let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, String>(2)?,
                "tech": row.get::<_, String>(3)?,
                "status": row.get::<_, String>(4)?,
                "created_at": row.get::<_, String>(5)?,
            }))
        }).unwrap().filter_map(|r| r.ok()).collect();
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
    });
    c_str(&json).into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn sf_delete_project(id: *const c_char) {
    let pid = from_c(id);
    db::with_db(|conn| {
        if let Err(e) = conn.execute("DELETE FROM projects WHERE id = ?1", rusqlite::params![&pid]) {
            eprintln!("[db] Failed to delete project: {}", e);
        }
    });
}

// ──────────────────────────────────────────
// FFI: Missions
// ──────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn sf_start_mission(project_id: *const c_char, brief: *const c_char) -> *mut c_char {
    let pid = from_c(project_id);
    let b = from_c(brief);
    let mission_id = uuid::Uuid::new_v4().to_string();
    let mid = mission_id.clone();

    db::with_db(|conn| {
        if let Err(e) = conn.execute(
            "INSERT INTO missions (id, project_id, brief, status) VALUES (?1, ?2, ?3, 'pending')",
            rusqlite::params![&mission_id, &pid, &b],
        ) {
            eprintln!("[db] Failed to create mission: {}", e);
        }
        if let Err(e) = conn.execute(
            "UPDATE projects SET status = 'active' WHERE id = ?1",
            rusqlite::params![&pid],
        ) {
            eprintln!("[db] Failed to update project status: {}", e);
        }
    });

    // Create workspace dir
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let workspace = format!("{}/Library/Application Support/SimpleSF/workspaces/{}", home, mission_id);
    std::fs::create_dir_all(&workspace).ok();

    // Run mission in background
    let brief_clone = b.clone();
    let ws_clone = workspace.clone();
    let mid_clone = mid.clone();

    runtime().spawn(async move {
        let callback: EventCallback = std::sync::Arc::new(|agent_id: &str, event: AgentEvent| {
            match event {
                AgentEvent::Thinking => emit(agent_id, "thinking", ""),
                AgentEvent::Reasoning { active } => emit(agent_id, "reasoning", if active { "start" } else { "end" }),
                AgentEvent::ToolCall { tool, args } => emit(agent_id, "tool_call", &format!("{}|{}", tool, args)),
                AgentEvent::ToolResult { tool, result } => emit(agent_id, "tool_result", &format!("{}|{}", tool, result)),
                AgentEvent::Response { content } => emit(agent_id, "response", &content),
                AgentEvent::ResponseChunk { content } => emit(agent_id, "response_chunk", &content),
                AgentEvent::Error { message } => emit(agent_id, "error", &message),
            }
        });

        if let Err(e) = engine::run_mission(&mid_clone, &brief_clone, &ws_clone, &callback).await {
            emit("engine", "error", &e);
        }
        emit("engine", "mission_complete", &mid_clone);
    });

    c_str(&mid).into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn sf_mission_status(mission_id: *const c_char) -> *mut c_char {
    let mid = from_c(mission_id);
    let json = db::with_db(|conn| {
        let mission: serde_json::Value = conn.query_row(
            "SELECT id, project_id, brief, status, created_at FROM missions WHERE id = ?1",
            rusqlite::params![&mid],
            |row| Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "project_id": row.get::<_, String>(1)?,
                "brief": row.get::<_, String>(2)?,
                "status": row.get::<_, String>(3)?,
                "created_at": row.get::<_, String>(4)?,
            }))
        ).unwrap_or(serde_json::json!(null));

        let mut stmt = conn.prepare(
            "SELECT id, phase_name, pattern, status, agent_ids, output, started_at, completed_at, phase_type, iteration, max_iterations FROM mission_phases WHERE mission_id = ?1 ORDER BY rowid"
        ).unwrap();
        let phases: Vec<serde_json::Value> = stmt.query_map(rusqlite::params![&mid], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "phase_name": row.get::<_, String>(1)?,
                "pattern": row.get::<_, String>(2)?,
                "status": row.get::<_, String>(3)?,
                "agent_ids": row.get::<_, String>(4)?,
                "output": row.get::<_, String>(5).unwrap_or_default(),
                "started_at": row.get::<_, Option<String>>(6)?,
                "completed_at": row.get::<_, Option<String>>(7)?,
                "phase_type": row.get::<_, String>(8).unwrap_or_else(|_| "once".into()),
                "iteration": row.get::<_, i64>(9).unwrap_or(1),
                "max_iterations": row.get::<_, i64>(10).unwrap_or(1),
            }))
        }).unwrap().filter_map(|r| r.ok()).collect();

        let mut msg_stmt = conn.prepare(
            "SELECT agent_name, role, content, tool_calls, created_at FROM agent_messages WHERE mission_id = ?1 ORDER BY id DESC LIMIT 50"
        ).unwrap();
        let messages: Vec<serde_json::Value> = msg_stmt.query_map(rusqlite::params![&mid], |row| {
            Ok(serde_json::json!({
                "agent_name": row.get::<_, String>(0)?,
                "role": row.get::<_, String>(1)?,
                "content": row.get::<_, String>(2)?,
                "tool_calls": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, String>(4)?,
            }))
        }).unwrap().filter_map(|r| r.ok()).collect();

        serde_json::json!({
            "mission": mission,
            "phases": phases,
            "messages": messages,
        }).to_string()
    });
    c_str(&json).into_raw()
}

// ──────────────────────────────────────────
// FFI: Jarvis Network Discussion
// ──────────────────────────────────────────

/// Start a Jarvis intake discussion with RTE + PO (network pattern).
/// The team discusses the user's request before taking action.
/// Returns the session ID. Discussion events stream via the callback.
#[unsafe(no_mangle)]
pub extern "C" fn sf_jarvis_discuss(
    message: *const c_char,
    project_context: *const c_char,
) -> *mut c_char {
    let msg = from_c(message);
    let ctx = from_c(project_context);
    let session_id = uuid::Uuid::new_v4().to_string();
    let sid = session_id.clone();

    let msg_clone = msg.clone();
    let ctx_clone = ctx.clone();
    let sid_clone = sid.clone();

    runtime().spawn(async move {
        let callback: EventCallback = std::sync::Arc::new(|agent_id: &str, event: AgentEvent| {
            match event {
                AgentEvent::Thinking => emit(agent_id, "discuss_thinking", ""),
                AgentEvent::Reasoning { active } => emit(agent_id, "discuss_reasoning", if active { "start" } else { "end" }),
                AgentEvent::Response { content } => emit(agent_id, "discuss_response", &content),
                AgentEvent::ResponseChunk { content } => emit(agent_id, "discuss_chunk", &content),
                AgentEvent::Error { message } => emit(agent_id, "error", &message),
                _ => {}
            }
        });

        match engine::run_intake(&msg_clone, &ctx_clone, &callback).await {
            Ok(synthesis) => {
                emit("jarvis", "discuss_complete", &synthesis);
            }
            Err(e) => {
                emit("engine", "error", &format!("Discussion failed: {}", e));
            }
        }
    });

    c_str(&sid).into_raw()
}

// ──────────────────────────────────────────
// FFI: Load discussion history (for UI restore)
// ──────────────────────────────────────────

/// Returns JSON array of the most recent discussion session's messages.
/// Format: [{"agent_id":"...","agent_name":"...","role":"...","content":"...","round":N}]
/// Safe to call before DB init — returns empty array.
#[unsafe(no_mangle)]
pub extern "C" fn sf_load_discussion_history() -> *mut c_char {
    // Safety: return empty if DB not yet initialized
    if !db::is_initialized() {
        return c_str("[]").into_raw();
    }

    let msgs: Vec<serde_json::Value> = db::with_db(|conn| {
        // Find the most recent discussion session with messages
        let session_id: Option<String> = conn.query_row(
            "SELECT ds.id FROM discussion_sessions ds \
             INNER JOIN discussion_messages dm ON dm.session_id = ds.id \
             GROUP BY ds.id ORDER BY ds.created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        ).ok();

        let sid = match session_id {
            Some(s) => s,
            None => return Ok::<Vec<_>, String>(Vec::new()),
        };

        let mut stmt = conn.prepare(
            "SELECT agent_id, agent_name, agent_role, content, round \
             FROM discussion_messages WHERE session_id = ?1 \
             ORDER BY round ASC, id ASC"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(rusqlite::params![&sid], |row| {
            Ok(serde_json::json!({
                "agent_id": row.get::<_, String>(0)?,
                "agent_name": row.get::<_, String>(1)?,
                "role": row.get::<_, String>(2)?,
                "content": row.get::<_, String>(3)?,
                "round": row.get::<_, i32>(4)?,
            }))
        }).map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
    }).unwrap_or_default();

    let json = serde_json::to_string(&msgs).unwrap_or_else(|_| "[]".into());
    c_str(&json).into_raw()
}

// ──────────────────────────────────────────
// FFI: Ideation (network discussion pattern)
// ──────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn sf_start_ideation(idea: *const c_char) -> *mut c_char {
    let idea_text = from_c(idea);
    let session_id = uuid::Uuid::new_v4().to_string();
    let sid = session_id.clone();

    db::with_db(|conn| {
        if let Err(e) = conn.execute(
            "INSERT INTO ideation_sessions (id, idea) VALUES (?1, ?2)",
            rusqlite::params![&session_id, &idea_text],
        ) {
            eprintln!("[db] Failed to create ideation session: {}", e);
        }
    });

    let idea_clone = idea_text.clone();
    let sid_clone = sid.clone();

    runtime().spawn(async move {
        let callback: EventCallback = std::sync::Arc::new(|agent_id: &str, event: AgentEvent| {
            match event {
                AgentEvent::Thinking => emit(agent_id, "thinking", ""),
                AgentEvent::Reasoning { active } => emit(agent_id, "ideation_reasoning", if active { "start" } else { "end" }),
                AgentEvent::Response { content } => emit(agent_id, "ideation_response", &content),
                AgentEvent::ResponseChunk { content } => emit(agent_id, "ideation_chunk", &content),
                AgentEvent::Error { message } => emit(agent_id, "error", &message),
                _ => {}
            }
        });

        match ideation::run_ideation(&sid_clone, &idea_clone, &callback).await {
            Ok(_) => {
                db::with_db(|conn| {
                    if let Err(e) = conn.execute(
                        "UPDATE ideation_sessions SET status = 'completed', completed_at = datetime('now') WHERE id = ?1",
                        rusqlite::params![&sid_clone],
                    ) {
                        eprintln!("[db] Failed to update ideation status: {}", e);
                    }
                });
                emit("engine", "ideation_complete", &sid_clone);
            }
            Err(e) => {
                db::with_db(|conn| {
                    if let Err(db_err) = conn.execute(
                        "UPDATE ideation_sessions SET status = 'failed', completed_at = datetime('now') WHERE id = ?1",
                        rusqlite::params![&sid_clone],
                    ) {
                        eprintln!("[db] Failed to update ideation failure: {}", db_err);
                    }
                });
                emit("engine", "error", &e);
            }
        }
    });

    c_str(&sid).into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn sf_list_agents() -> *mut c_char {
    let agents = agents::all_agents();
    let json = serde_json::to_string(&agents).unwrap_or_else(|_| "[]".into());
    c_str(&json).into_raw()
}

/// List available workflow templates (returns JSON array)
#[unsafe(no_mangle)]
pub extern "C" fn sf_list_workflows() -> *mut c_char {
    let wfs = crate::catalog::list_workflows();
    let json_arr: Vec<serde_json::Value> = wfs.iter().map(|(id, name, desc)| {
        serde_json::json!({
            "id": id,
            "name": name,
            "description": desc,
        })
    }).collect();
    let json = serde_json::to_string(&json_arr).unwrap_or_else(|_| "[]".into());
    c_str(&json).into_raw()
}

/// Run all AC bench tests. Returns JSON array of results.
#[unsafe(no_mangle)]
pub extern "C" fn sf_run_bench() -> *mut c_char {
    let result = runtime().block_on(crate::bench::run_all());
    c_str(&result).into_raw()
}

// ──────────────────────────────────────────
// FFI: Free strings
// ──────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn sf_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { let _ = CString::from_raw(s); }
    }
}
