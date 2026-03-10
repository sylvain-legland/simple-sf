//! Live E2E test — requires a real LLM provider (MiniMax).
//! Run with: cargo test --test live_e2e -- --ignored --nocapture --test-threads=1
//!
//! This test simulates exactly what a user does in the GUI:
//! 1. Init DB
//! 2. Configure LLM (MiniMax)
//! 3. Create a project
//! 4. Start a mission with a brief
//! 5. Poll mission status until complete or timeout
//! 6. Verify phases executed and agents produced output

use sf_engine::{db, llm, engine, catalog, executor};
use std::sync::Arc;

fn init() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let db_path = format!("{}/Library/Application Support/SimpleSF/sf_engine_test.db", home);
    let _ = std::fs::remove_file(&db_path);
    db::init_db(&db_path);
    catalog::seed_from_json(&format!("{}/_MACARON-SOFTWARE/simple-sf/data", home));
}

fn configure_minimax() -> bool {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let key_path = format!("{}/.config/factory/minimax.key", home);
    match std::fs::read_to_string(&key_path) {
        Ok(key) => {
            let key = key.trim();
            llm::configure_llm("minimax", key, "https://api.minimax.io/v1", "MiniMax-M2.5");
            eprintln!("[live-e2e] MiniMax configured (key {}...)", &key[..8]);
            true
        }
        Err(_) => {
            eprintln!("[live-e2e] No MiniMax key found at {}", key_path);
            false
        }
    }
}

#[tokio::test]
#[ignore] // Only run manually with --ignored
async fn live_full_mission() {
    init();
    if !configure_minimax() {
        eprintln!("[live-e2e] SKIPPED — no LLM key");
        return;
    }

    // 1. Create project
    let project_id = "live-test-project";
    db::with_db(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO projects (id, name, description, tech, status) VALUES (?1, ?2, ?3, ?4, 'active')",
            rusqlite::params![project_id, "Live Test App", "Test project for E2E validation", "Rust"],
        ).unwrap();
    });
    eprintln!("[live-e2e] ✅ Project created: {}", project_id);

    // 2. List agents to verify catalog is loaded
    let agent_count = db::with_db(|conn| {
        conn.query_row("SELECT COUNT(*) FROM agents", [], |r| r.get::<_, i64>(0)).unwrap_or(0)
    });
    eprintln!("[live-e2e] Agents in DB: {}", agent_count);
    assert!(agent_count >= 5, "Should have loaded agents from catalog");

    // 3. Setup a small workflow (3 phases instead of 14 for speed)
    let workflow_id = "live-test-workflow";
    // Fallback agents: rte-marie, po-lucas, archi-pierre, lead-thomas, dev-emma, dev-karim, qa-sophie
    let phases_json = serde_json::json!([
        {
            "name": "Architecture",
            "pattern": "solo",
            "agent_ids": ["archi-pierre"],
            "gate": "always"
        },
        {
            "name": "Development",
            "pattern": "solo",
            "agent_ids": ["lead-thomas"],
            "gate": "always"
        },
        {
            "name": "Review",
            "pattern": "solo",
            "agent_ids": ["qa-sophie"],
            "gate": "always"
        }
    ]);
    db::with_db(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO workflows (id, name, phases_json) VALUES (?1, ?2, ?3)",
            rusqlite::params![workflow_id, "Live Test Workflow", phases_json.to_string()],
        ).unwrap();
    });
    eprintln!("[live-e2e] ✅ Workflow created: {}", workflow_id);

    // 4. Create and link mission
    let mission_id = format!("live-{}", uuid::Uuid::new_v4());
    let brief = "Create a simple Rust HTTP server with a /health endpoint that returns JSON. \
                 Use only standard library or minimal dependencies. \
                 The server should bind to port 3000.";

    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO missions (id, project_id, brief, status, workflow) VALUES (?1, ?2, ?3, 'pending', ?4)",
            rusqlite::params![&mission_id, project_id, brief, workflow_id],
        ).unwrap();
    });
    eprintln!("[live-e2e] ✅ Mission created: {}", mission_id);

    // 5. Create workspace
    let home = std::env::var("HOME").unwrap();
    let workspace = format!("{}/Library/Application Support/SimpleSF/workspaces/{}", home, mission_id);
    std::fs::create_dir_all(&workspace).unwrap();
    eprintln!("[live-e2e] ✅ Workspace: {}", workspace);

    // 6. Run mission with event capture
    let events: Arc<std::sync::Mutex<Vec<(String, String, String)>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let events_clone = events.clone();

    let callback: executor::EventCallback = Arc::new(move |agent_id, event| {
        let (etype, data) = match &event {
            executor::AgentEvent::Thinking => ("thinking".to_string(), String::new()),
            executor::AgentEvent::Response { content } => ("response".to_string(), content.clone()),
            executor::AgentEvent::ResponseChunk { content } => ("chunk".to_string(), content.clone()),
            executor::AgentEvent::ToolCall { tool, args } => ("tool_call".to_string(), format!("{}: {}", tool, args)),
            executor::AgentEvent::ToolResult { tool, result } => ("tool_result".to_string(), format!("{}: {}...", tool, &result[..result.len().min(100)])),
            executor::AgentEvent::Error { message } => ("error".to_string(), message.clone()),
            executor::AgentEvent::Reasoning { active } => ("reasoning".to_string(), active.to_string()),
        };
        if etype == "response" || etype == "tool_call" || etype == "error" {
            eprintln!("[live-e2e] {} │ {} │ {}", agent_id, etype, &data[..data.len().min(200)]);
        }
        events_clone.lock().unwrap().push((agent_id.to_string(), etype, data));
    });

    eprintln!("[live-e2e] 🚀 Starting mission...");
    let start = std::time::Instant::now();

    let result = engine::run_mission(&mission_id, brief, &workspace, &callback).await;

    let elapsed = start.elapsed();
    eprintln!("[live-e2e] ⏱  Mission completed in {:.1}s", elapsed.as_secs_f64());

    match &result {
        Ok(()) => eprintln!("[live-e2e] ✅ Mission succeeded"),
        Err(e) => eprintln!("[live-e2e] ❌ Mission failed: {}", e),
    }

    // 7. Check mission status
    let status = db::with_db(|conn| {
        conn.query_row(
            "SELECT status FROM missions WHERE id = ?1",
            rusqlite::params![&mission_id],
            |r| r.get::<_, String>(0),
        ).unwrap_or_default()
    });
    eprintln!("[live-e2e] Mission status: {}", status);

    // 8. Check phases
    let phases = db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT phase_name, status, output FROM mission_phases WHERE mission_id = ?1 ORDER BY rowid"
        ).unwrap();
        stmt.query_map(rusqlite::params![&mission_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2).unwrap_or_default(),
            ))
        }).unwrap().filter_map(|r| r.ok()).collect::<Vec<_>>()
    });

    eprintln!("\n[live-e2e] ═══ Phase Results ═══");
    for (name, status, output) in &phases {
        let preview = if output.len() > 150 { &output[..150] } else { output.as_str() };
        eprintln!("  {} │ {} │ {}", name, status, preview.replace('\n', " "));
    }

    // 9. Check events captured
    let captured = events.lock().unwrap();
    let response_count = captured.iter().filter(|(_, t, _)| t == "response").count();
    let tool_calls = captured.iter().filter(|(_, t, _)| t == "tool_call").count();
    let errors = captured.iter().filter(|(_, t, _)| t == "error").count();
    eprintln!("\n[live-e2e] Events: {} responses, {} tool_calls, {} errors (total {})",
        response_count, tool_calls, errors, captured.len());

    // 10. Assertions
    assert!(result.is_ok(), "Mission should complete without error");
    assert!(phases.len() >= 3, "Should have ≥3 phases, got {}", phases.len());
    assert!(response_count >= 3, "Should have ≥3 agent responses, got {}", response_count);

    // Check at least some phases completed
    let completed = phases.iter().filter(|(_, s, _)| s == "completed").count();
    eprintln!("[live-e2e] Completed phases: {}/{}", completed, phases.len());
    assert!(completed >= 2, "At least 2 phases should complete, got {}", completed);

    // 11. Cleanup test DB
    let db_path = format!("{}/Library/Application Support/SimpleSF/sf_engine_test.db", home);
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{}-shm", db_path));
    let _ = std::fs::remove_file(format!("{}-wal", db_path));
    let _ = std::fs::remove_dir_all(&workspace);

    eprintln!("\n[live-e2e] ✅ FULL MISSION E2E PASSED");
}
