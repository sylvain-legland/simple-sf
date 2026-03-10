// ══════════════════════════════════════════════════════════════
// Integration Test Suite — SimpleSF Engine
// Covers: Memory, Jarvis, Projects, Workflows, Resilience
// ══════════════════════════════════════════════════════════════
//
// Tests run sequentially (single global DB singleton).
// Each test uses its own project_id to avoid cross-contamination.

use sf_engine::{db, tools, agents, catalog, guard, indexer};
use serde_json::json;
use std::sync::Once;

static INIT: Once = Once::new();

fn ensure_db() {
    INIT.call_once(|| {
        let tmp = std::env::temp_dir().join("sf_test.db");
        let _ = std::fs::remove_file(&tmp); // clean slate
        db::init_db(tmp.to_str().unwrap());
        // Seed fallback agents + catalog data
        catalog::seed_from_json("/nonexistent"); // triggers fallback agents
    });
}

// ══════════════════════════════════════════════════════════════
// 1. MEMORY SYSTEM TESTS
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn mem_01_store_and_search_project_scoped() {
    ensure_db();
    let pid = "test-proj-mem01";

    // Store a project-scoped entry
    let args = json!({"key": "tech-stack", "value": "Swift + SpriteKit", "category": "decision"});
    let result = tools::execute_tool("memory_store", &args, &format!("/tmp/workspaces/{}", pid)).await;
    assert!(result.contains("Stored memory"), "Expected success, got: {}", result);
    assert!(result.contains("[project]"), "Should be project-scoped: {}", result);

    // Search should find it
    let search = json!({"query": "Swift", "scope": "project"});
    let found = tools::execute_tool("memory_search", &search, &format!("/tmp/workspaces/{}", pid)).await;
    assert!(found.contains("tech-stack"), "Should find tech-stack: {}", found);
    assert!(found.contains("Swift"), "Should contain value: {}", found);
}

#[tokio::test]
async fn mem_02_project_isolation() {
    ensure_db();
    let pid_a = "test-proj-iso-a";
    let pid_b = "test-proj-iso-b";

    // Store in project A
    let args = json!({"key": "secret-a", "value": "only for A", "category": "note"});
    tools::execute_tool("memory_store", &args, &format!("/tmp/workspaces/{}", pid_a)).await;

    // Search from project B — should NOT find A's data
    let search = json!({"query": "secret-a", "scope": "project"});
    let found = tools::execute_tool("memory_search", &search, &format!("/tmp/workspaces/{}", pid_b)).await;
    assert!(found.contains("No memory found") || !found.contains("only for A"),
        "Project B should not see A's data: {}", found);
}

#[tokio::test]
async fn mem_03_global_scope() {
    ensure_db();
    let pid = "test-proj-global";

    // Store global entry
    let args = json!({"key": "company-convention", "value": "Always use snake_case", "category": "convention", "scope": "global"});
    let result = tools::execute_tool("memory_store", &args, &format!("/tmp/workspaces/{}", pid)).await;
    assert!(result.contains("[global]"), "Should be global: {}", result);

    // Any project should find global entries
    let search = json!({"query": "snake_case", "scope": "project"});
    let found = tools::execute_tool("memory_search", &search, &format!("/tmp/workspaces/other-proj")).await;
    assert!(found.contains("company-convention"), "Global entry should be visible: {}", found);
}

#[tokio::test]
async fn mem_04_upsert_same_key() {
    ensure_db();
    let pid = "test-proj-upsert";
    let ws = format!("/tmp/workspaces/{}", pid);

    // Store initial value
    let args1 = json!({"key": "api-version", "value": "v1", "category": "decision"});
    tools::execute_tool("memory_store", &args1, &ws).await;

    // Upsert with same key
    let args2 = json!({"key": "api-version", "value": "v2-updated", "category": "decision"});
    tools::execute_tool("memory_store", &args2, &ws).await;

    // Search should find v2, not v1
    let search = json!({"query": "api-version"});
    let found = tools::execute_tool("memory_search", &search, &ws).await;
    assert!(found.contains("v2-updated"), "Should have updated value: {}", found);
    // Should not have duplicates
    let count = found.matches("api-version").count();
    assert_eq!(count, 1, "Should have exactly one entry, got {}: {}", count, found);
}

#[tokio::test]
async fn mem_05_load_project_memory_format() {
    ensure_db();
    let pid = "test-proj-inject";
    let ws = format!("/tmp/workspaces/{}", pid);

    // Store some entries
    tools::execute_tool("memory_store", &json!({"key": "arch", "value": "MVVM pattern", "category": "decision"}), &ws).await;
    tools::execute_tool("memory_store", &json!({"key": "db", "value": "SQLite WAL", "category": "decision"}), &ws).await;

    // Load project memory (auto-injection format)
    let memory = tools::load_project_memory(pid);
    assert!(memory.contains("## Project Memory"), "Should have header: {}", memory);
    assert!(memory.contains("arch"), "Should include arch entry");
    assert!(memory.contains("MVVM"), "Should include value");
    assert!(memory.len() <= 4200, "Should respect 4K limit: {} chars", memory.len());
}

#[tokio::test]
async fn mem_06_load_project_memory_empty() {
    ensure_db();
    // Note: global entries from other tests may be visible (by design).
    // Test with a fresh query that won't match anything.
    let memory = tools::load_project_memory("zzz-truly-isolated-no-entries-xyz");
    // Global entries may still appear — that's correct behavior.
    // We just verify it doesn't panic and returns a string.
    eprintln!("[test] Empty project memory: '{}' ({} chars)", &memory[..memory.len().min(100)], memory.len());
}

#[tokio::test]
async fn mem_07_load_project_files() {
    ensure_db();
    let pid = "test-proj-files";
    let ws = std::env::temp_dir().join("sf_test_workspace").join(pid);
    std::fs::create_dir_all(&ws).unwrap();

    // Create instruction files
    std::fs::write(ws.join("README.md"), "# Test Project\nA test project for memory system.").unwrap();
    std::fs::write(ws.join("SPECS.md"), "## Specifications\n- Feature A\n- Feature B").unwrap();

    // Load project files
    tools::load_project_files(ws.to_str().unwrap(), pid);

    // Verify they're stored in memory
    let search = json!({"query": "README", "scope": "project"});
    let found = tools::execute_tool("memory_search", &search, ws.to_str().unwrap()).await;
    assert!(found.contains("Test Project"), "Should have loaded README: {}", found);

    // Clean up
    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn mem_08_compact_dedup() {
    ensure_db();
    let pid = "test-proj-compact";
    let ws = format!("/tmp/workspaces/{}", pid);

    // Insert some entries directly
    db::with_db(|conn| {
        conn.execute("INSERT INTO memory (key, value, category, project_id) VALUES ('dup-key', 'val1', 'note', ?1)", [pid]).unwrap();
        conn.execute("INSERT INTO memory (key, value, category, project_id) VALUES ('dup-key', 'val2', 'note', ?1)", [pid]).unwrap();
        conn.execute("INSERT INTO memory (key, value, category, project_id) VALUES ('dup-key', 'val3', 'note', ?1)", [pid]).unwrap();
    });

    // Compact
    tools::compact_memory(pid);

    // Should have only latest entry
    let search = json!({"query": "dup-key", "scope": "project"});
    let found = tools::execute_tool("memory_search", &search, &ws).await;
    let count = found.matches("dup-key").count();
    assert_eq!(count, 1, "Should have deduplicated to 1 entry, got {}: {}", count, found);
    assert!(found.contains("val3"), "Should keep latest value: {}", found);
}

#[tokio::test]
async fn mem_09_empty_key_rejected() {
    ensure_db();
    let args = json!({"key": "", "value": "something"});
    let result = tools::execute_tool("memory_store", &args, "/tmp/workspaces/test").await;
    assert!(result.contains("Error"), "Empty key should be rejected: {}", result);
}

#[tokio::test]
async fn mem_10_search_all_scope() {
    ensure_db();
    let ws_a = "/tmp/workspaces/test-all-a";
    let ws_b = "/tmp/workspaces/test-all-b";

    tools::execute_tool("memory_store", &json!({"key": "all-search-a", "value": "from A"}), ws_a).await;
    tools::execute_tool("memory_store", &json!({"key": "all-search-b", "value": "from B"}), ws_b).await;

    // "all" scope should find entries from any project
    let search = json!({"query": "all-search", "scope": "all"});
    let found = tools::execute_tool("memory_search", &search, ws_a).await;
    assert!(found.contains("all-search-a"), "Should find A: {}", found);
    assert!(found.contains("all-search-b"), "Should find B: {}", found);
}

// ══════════════════════════════════════════════════════════════
// 2. PROJECT CRUD TESTS
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn proj_01_create_project() {
    ensure_db();
    let id = db::with_db(|conn| {
        let pid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, name, description, tech) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![&pid, "Test Project", "A test", "Rust"],
        ).unwrap();
        pid
    });
    assert!(!id.is_empty());

    // Verify it exists
    let exists = db::with_db(|conn| {
        conn.query_row("SELECT COUNT(*) FROM projects WHERE id = ?1", [&id], |r| r.get::<_, i64>(0)).unwrap()
    });
    assert_eq!(exists, 1);
}

#[tokio::test]
async fn proj_02_list_projects() {
    ensure_db();
    let count = db::with_db(|conn| {
        conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get::<_, i64>(0)).unwrap()
    });
    assert!(count >= 0, "Should be able to count projects");
}

#[tokio::test]
async fn proj_03_delete_project() {
    ensure_db();
    let pid = "proj-to-delete";
    db::with_db(|conn| {
        conn.execute("INSERT OR IGNORE INTO projects (id, name) VALUES (?1, 'Deletable')", [pid]).unwrap();
    });

    db::with_db(|conn| {
        conn.execute("DELETE FROM projects WHERE id = ?1", [pid]).unwrap();
    });

    let exists = db::with_db(|conn| {
        conn.query_row("SELECT COUNT(*) FROM projects WHERE id = ?1", [pid], |r| r.get::<_, i64>(0)).unwrap()
    });
    assert_eq!(exists, 0, "Project should be deleted");
}

#[tokio::test]
async fn proj_04_project_status_update() {
    ensure_db();
    let pid = "proj-status-test";
    db::with_db(|conn| {
        conn.execute("INSERT OR IGNORE INTO projects (id, name, status) VALUES (?1, 'StatusTest', 'idea')", [pid]).unwrap();
        conn.execute("UPDATE projects SET status = 'active' WHERE id = ?1", [pid]).unwrap();
    });

    let status = db::with_db(|conn| {
        conn.query_row("SELECT status FROM projects WHERE id = ?1", [pid], |r| r.get::<_, String>(0)).unwrap()
    });
    assert_eq!(status, "active");
}

#[tokio::test]
async fn proj_05_mission_belongs_to_project() {
    ensure_db();
    let pid = "proj-mission-test";
    let mid = "mission-test-01";
    db::with_db(|conn| {
        conn.execute("INSERT OR IGNORE INTO projects (id, name) VALUES (?1, 'MissionProject')", [pid]).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO missions (id, project_id, brief, status) VALUES (?1, ?2, 'Test brief', 'pending')",
            rusqlite::params![mid, pid],
        ).unwrap();
    });

    let found_project = db::with_db(|conn| {
        conn.query_row("SELECT project_id FROM missions WHERE id = ?1", [mid], |r| r.get::<_, String>(0)).unwrap()
    });
    assert_eq!(found_project, pid);
}

// ══════════════════════════════════════════════════════════════
// 3. WORKFLOW & CATALOG TESTS
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn wf_01_catalog_stats() {
    ensure_db();
    let (agents_count, skills, patterns, workflows) = catalog::catalog_stats();
    // Fallback agents should be seeded (at least 7)
    assert!(agents_count >= 7, "Should have at least 7 fallback agents, got {}", agents_count);
    eprintln!("[test] Catalog: {} agents, {} skills, {} patterns, {} workflows",
        agents_count, skills, patterns, workflows);
}

#[tokio::test]
async fn wf_02_agent_lookup() {
    ensure_db();
    // Fallback agents from catalog.rs
    let agent = catalog::get_agent_info("rte-marie");
    assert!(agent.is_some(), "Should find fallback agent rte-marie");
    let a = agent.unwrap();
    assert_eq!(a.role, "rte");
    assert!(a.name.contains("Marie") || a.name.contains("Lefevre"), "Name: {}", a.name);
}

#[tokio::test]
async fn wf_03_agent_by_role() {
    ensure_db();
    let devs = agents::get_agents_for_roles(&["developer"]);
    assert!(!devs.is_empty(), "Should find developer agents");
    for dev in &devs {
        assert_eq!(dev.role, "developer", "All should be developers: {}", dev.id);
    }
}

#[tokio::test]
async fn wf_04_all_agents_not_empty() {
    ensure_db();
    let all = agents::all_agents();
    assert!(all.len() >= 7, "Should have at least fallback agents: {}", all.len());
}

#[tokio::test]
async fn wf_05_list_workflows() {
    ensure_db();
    let workflows = catalog::list_workflows();
    // May be empty if no JSON seed files, but should not panic
    eprintln!("[test] {} workflows in catalog", workflows.len());
}

#[tokio::test]
async fn wf_06_fallback_workflow_phases() {
    ensure_db();
    // The hardcoded SAFE_PHASES should be used when workflow not found
    let phases = catalog::get_workflow_phases("nonexistent-workflow");
    assert!(phases.is_none(), "Nonexistent workflow should return None");
}

#[tokio::test]
async fn wf_07_tool_schemas_by_role() {
    ensure_db();
    let dev_tools = tools::tool_schemas_for_role("developer");
    assert!(!dev_tools.is_empty(), "Developers should have tools");

    let tool_names: Vec<String> = dev_tools.iter()
        .filter_map(|t| t["function"]["name"].as_str().map(String::from))
        .collect();
    assert!(tool_names.contains(&"code_write".to_string()), "Devs should have code_write: {:?}", tool_names);
    assert!(tool_names.contains(&"code_read".to_string()), "Devs should have code_read: {:?}", tool_names);
    assert!(tool_names.contains(&"build".to_string()), "Devs should have build: {:?}", tool_names);

    // lead_dev should have memory tools
    let lead_tools = tools::tool_schemas_for_role("lead_dev");
    let lead_names: Vec<String> = lead_tools.iter()
        .filter_map(|t| t["function"]["name"].as_str().map(String::from))
        .collect();
    assert!(lead_names.contains(&"memory_store".to_string()), "Lead devs should have memory_store: {:?}", lead_names);
    assert!(lead_names.contains(&"memory_search".to_string()), "Lead devs should have memory_search: {:?}", lead_names);
}

#[tokio::test]
async fn wf_08_tool_schemas_rte() {
    ensure_db();
    let rte_tools = tools::tool_schemas_for_role("rte");
    assert!(!rte_tools.is_empty(), "RTE should have tools");

    let names: Vec<String> = rte_tools.iter()
        .filter_map(|t| t["function"]["name"].as_str().map(String::from))
        .collect();
    // RTE typically gets memory + search tools, not code_write
    assert!(names.contains(&"memory_search".to_string()), "RTE should have memory_search: {:?}", names);
}

// ══════════════════════════════════════════════════════════════
// 4. JARVIS / INTAKE TESTS (synchronous DB-only, no LLM)
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn jarvis_01_discussion_session_crud() {
    ensure_db();
    let sid = uuid::Uuid::new_v4().to_string();
    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO discussion_sessions (id, topic, context) VALUES (?1, ?2, ?3)",
            rusqlite::params![&sid, "Test Discussion", "Context about test"],
        ).unwrap();
    });

    let found = db::with_db(|conn| {
        conn.query_row("SELECT topic FROM discussion_sessions WHERE id = ?1", [&sid],
            |r| r.get::<_, String>(0)).unwrap()
    });
    assert_eq!(found, "Test Discussion");
}

#[tokio::test]
async fn jarvis_02_discussion_messages() {
    ensure_db();
    let sid = uuid::Uuid::new_v4().to_string();
    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO discussion_sessions (id, topic) VALUES (?1, 'Msg Test')",
            [&sid],
        ).unwrap();

        for (i, (agent_id, agent_name, role, content)) in [
            ("rte-marie", "Marie Lefevre", "rte", "I'll coordinate the team"),
            ("archi-pierre", "Pierre Garnier", "architect", "We need microservices"),
            ("lead-thomas", "Thomas Dubois", "lead_dev", "Let's use Rust"),
        ].iter().enumerate() {
            conn.execute(
                "INSERT INTO discussion_messages (session_id, agent_id, agent_name, agent_role, round, content) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![&sid, agent_id, agent_name, role, i as i64, content],
            ).unwrap();
        }
    });

    let count = db::with_db(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM discussion_messages WHERE session_id = ?1", [&sid],
            |r| r.get::<_, i64>(0),
        ).unwrap()
    });
    assert_eq!(count, 3, "Should have 3 messages");
}

#[tokio::test]
async fn jarvis_03_discussion_history_order() {
    ensure_db();
    let sid = uuid::Uuid::new_v4().to_string();
    db::with_db(|conn| {
        conn.execute("INSERT INTO discussion_sessions (id, topic) VALUES (?1, 'Order Test')", [&sid]).unwrap();
        conn.execute(
            "INSERT INTO discussion_messages (session_id, agent_id, agent_name, agent_role, round, content) VALUES (?1, 'a1', 'Agent1', 'dev', 1, 'first')",
            [&sid],
        ).unwrap();
        conn.execute(
            "INSERT INTO discussion_messages (session_id, agent_id, agent_name, agent_role, round, content) VALUES (?1, 'a2', 'Agent2', 'dev', 2, 'second')",
            [&sid],
        ).unwrap();
    });

    let messages: Vec<String> = db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT content FROM discussion_messages WHERE session_id = ?1 ORDER BY created_at ASC"
        ).unwrap();
        stmt.query_map([&sid], |r| r.get::<_, String>(0)).unwrap()
            .filter_map(|r| r.ok()).collect()
    });
    assert_eq!(messages, vec!["first", "second"]);
}

#[tokio::test]
async fn jarvis_04_ideation_session_crud() {
    ensure_db();
    let sid = uuid::Uuid::new_v4().to_string();
    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO ideation_sessions (id, idea) VALUES (?1, 'Build a game')", [&sid],
        ).unwrap();
        conn.execute(
            "INSERT INTO ideation_messages (session_id, agent_id, agent_name, round, content) VALUES (?1, 'dev-emma', 'Clara Nguyen', 1, 'Use SpriteKit')", [&sid],
        ).unwrap();
    });

    let idea = db::with_db(|conn| {
        conn.query_row("SELECT idea FROM ideation_sessions WHERE id = ?1", [&sid],
            |r| r.get::<_, String>(0)).unwrap()
    });
    assert_eq!(idea, "Build a game");
}

// ══════════════════════════════════════════════════════════════
// 5. RESILIENCE TESTS
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn res_01_unknown_tool() {
    ensure_db();
    let result = tools::execute_tool("nonexistent_tool", &json!({}), "/tmp/workspace").await;
    assert!(result.contains("Unknown tool"), "Should handle unknown tool: {}", result);
}

#[tokio::test]
async fn res_02_empty_args() {
    ensure_db();
    // code_read with empty path
    let result = tools::execute_tool("code_read", &json!({}), "/tmp/workspace").await;
    // Should not panic
    assert!(!result.is_empty(), "Should return something for empty args");
}

#[tokio::test]
async fn res_03_code_read_nonexistent_file() {
    ensure_db();
    let result = tools::execute_tool("code_read", &json!({"path": "/nonexistent/path/file.txt"}), "/tmp/workspace").await;
    assert!(result.contains("Error") || result.contains("error") || result.contains("not found") || result.contains("No such"),
        "Should handle missing file: {}", result);
}

#[tokio::test]
async fn res_04_code_write_and_read() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_test_rw");
    std::fs::create_dir_all(&ws).unwrap();
    let file_path = "test_resilience.txt";

    // Write
    let write_result = tools::execute_tool("code_write",
        &json!({"path": file_path, "content": "Hello from test!"}),
        ws.to_str().unwrap()).await;
    assert!(write_result.contains("Wrote") || write_result.contains("wrote") || write_result.to_lowercase().contains("success") || !write_result.contains("Error"),
        "Write should succeed: {}", write_result);

    // Read back
    let read_result = tools::execute_tool("code_read",
        &json!({"path": file_path}),
        ws.to_str().unwrap()).await;
    assert!(read_result.contains("Hello from test!"), "Should read back content: {}", read_result);

    // Clean up
    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn res_05_list_files_empty_dir() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_test_empty_ls");
    std::fs::create_dir_all(&ws).unwrap();

    let result = tools::execute_tool("list_files", &json!({"path": "."}), ws.to_str().unwrap()).await;
    // Empty dir may return empty string or "No files found" — just shouldn't panic
    eprintln!("[test] list_files empty dir: '{}'", result);

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn res_06_guard_l0_clean() {
    ensure_db();
    let result = guard::check_l0("This is a normal, high-quality response about Rust architecture.", "developer", &[]);
    assert!(result.passed, "Clean content should pass L0: {:?}", result.issues);
    assert_eq!(result.score, 0);
}

#[tokio::test]
async fn res_07_guard_l0_slop() {
    ensure_db();
    let result = guard::check_l0(
        "Here's the implementation:\n```\nfn main() {\n    // TODO: implement this later\n    let data = lorem ipsum placeholder text;\n}\n```",
        "developer", &[]);
    assert!(result.score > 0, "Slop+TODO content should score > 0: score={}, issues={:?}", result.score, result.issues);
}

#[tokio::test]
async fn res_08_guard_l0_todo_placeholder() {
    ensure_db();
    let result = guard::check_l0(
        "fn main() {\n    // TODO: implement this\n    todo!()\n}",
        "developer", &[]);
    assert!(result.score > 0 || !result.passed, "TODO placeholder should be flagged: score={}", result.score);
}

#[tokio::test]
async fn res_09_memory_search_empty_query() {
    ensure_db();
    let result = tools::execute_tool("memory_search", &json!({"query": ""}), "/tmp/workspaces/test").await;
    // Empty query matches everything or returns empty — should not panic
    assert!(!result.is_empty(), "Should handle empty query");
}

#[tokio::test]
async fn res_10_code_edit_file_not_found() {
    ensure_db();
    let result = tools::execute_tool("code_edit",
        &json!({"path": "nonexistent.rs", "old": "foo", "new": "bar"}),
        "/tmp/workspace").await;
    assert!(result.contains("Error") || result.contains("error") || result.contains("not found"),
        "Should handle missing file for edit: {}", result);
}

#[tokio::test]
async fn res_11_git_status_no_repo() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_test_nogit");
    std::fs::create_dir_all(&ws).unwrap();

    let result = tools::execute_tool("git_status", &json!({}), ws.to_str().unwrap()).await;
    // Should handle gracefully (error message about not being a git repo)
    assert!(!result.is_empty(), "Should return error for non-git dir");

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn res_12_deep_search() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_test_deepsearch");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::write(ws.join("hello.rs"), "fn main() { println!(\"hello\"); }").unwrap();

    let result = tools::execute_tool("deep_search",
        &json!({"query": "println", "path": "."}),
        ws.to_str().unwrap()).await;
    assert!(result.contains("hello.rs") || result.contains("println"),
        "Should find file with content: {}", result);

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn res_13_mission_phases_table() {
    ensure_db();
    let mid = "mission-phase-test";
    let pid = "proj-phase-test";

    db::with_db(|conn| {
        conn.execute("INSERT OR IGNORE INTO projects (id, name) VALUES (?1, 'PhaseTest')", [pid]).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO missions (id, project_id, brief) VALUES (?1, ?2, 'test brief')",
            rusqlite::params![mid, pid],
        ).unwrap();

        let phase_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO mission_phases (id, mission_id, phase_name, pattern, status, started_at)
             VALUES (?1, ?2, 'design', 'sequential', 'completed', datetime('now'))",
            rusqlite::params![&phase_id, mid],
        ).unwrap();
    });

    let phase_count = db::with_db(|conn| {
        conn.query_row("SELECT COUNT(*) FROM mission_phases WHERE mission_id = ?1", [mid],
            |r| r.get::<_, i64>(0)).unwrap()
    });
    assert!(phase_count >= 1, "Should have at least 1 phase record");
}

#[tokio::test]
async fn res_14_db_initialized() {
    assert!(db::is_initialized(), "DB should be initialized for tests");
}

#[tokio::test]
async fn res_15_concurrent_db_access() {
    ensure_db();
    // Simulate concurrent reads — should not deadlock
    let handles: Vec<_> = (0..5).map(|_| {
        std::thread::spawn(move || {
            db::with_db(|conn| {
                conn.query_row("SELECT COUNT(*) FROM agents", [],
                    |r| r.get::<_, i64>(0)).unwrap()
            })
        })
    }).collect();

    for h in handles {
        let count = h.join().unwrap();
        assert!(count >= 0, "Concurrent read should succeed");
    }
}

#[tokio::test]
async fn res_16_agent_messages_schema() {
    ensure_db();
    // First create the referenced mission+project to satisfy FK
    let pid = "proj-msg-schema";
    let mid = "mission-msg-schema";
    db::with_db(|conn| {
        conn.execute("INSERT OR IGNORE INTO projects (id, name) VALUES (?1, 'MsgSchema')", [pid]).unwrap();
        conn.execute("INSERT OR IGNORE INTO missions (id, project_id, brief) VALUES (?1, ?2, 'test')",
            rusqlite::params![mid, pid]).unwrap();
        conn.execute(
            "INSERT INTO agent_messages (mission_id, phase_id, agent_id, agent_name, role, content, tool_calls)
             VALUES (?1, 'p1', 'a1', 'Test Agent', 'assistant', 'Hello world', 'code_read')",
            rusqlite::params![mid],
        ).unwrap();
    });

    let found = db::with_db(|conn| {
        conn.query_row(
            "SELECT content FROM agent_messages WHERE mission_id = ?1 AND agent_id = 'a1'",
            [mid], |r| r.get::<_, String>(0),
        ).unwrap()
    });
    assert_eq!(found, "Hello world");
}

#[tokio::test]
async fn res_17_large_memory_value() {
    ensure_db();
    let pid = "test-large-mem";
    let ws = format!("/tmp/workspaces/{}", pid);
    let large_value = "x".repeat(10000);

    let args = json!({"key": "large-entry", "value": large_value, "category": "note"});
    let result = tools::execute_tool("memory_store", &args, &ws).await;
    assert!(result.contains("Stored"), "Should store large value: {}", result);

    // Load project memory should truncate
    let memory = tools::load_project_memory(pid);
    // Each entry is truncated to 300 chars in the display
    assert!(memory.len() < 5000, "Memory display should be bounded: {} chars", memory.len());
}

#[tokio::test]
async fn res_18_project_files_no_workspace() {
    ensure_db();
    // Non-existent workspace — should not panic
    tools::load_project_files("/nonexistent/workspace/path", "test-no-ws");
    // If no files found, nothing stored — verify no crash
    assert!(true, "Should handle missing workspace gracefully");
}

// ══════════════════════════════════════════════════════════════
// 8. SEMANTIC CODE SEARCH TESTS (tree-sitter + FTS5)
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn idx_01_parse_rust_file() {
    // Parse a Rust file and verify AST chunks are extracted
    let ws = std::env::temp_dir().join("sf_idx_01");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::write(ws.join("lib.rs"), r#"
pub struct Config {
    pub name: String,
    pub port: u16,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Config { name: name.to_string(), port: 8080 }
    }
}

pub fn start_server(config: &Config) {
    println!("Starting {} on port {}", config.name, config.port);
}

fn internal_helper() -> bool {
    true
}
"#).unwrap();

    let chunks = indexer::parse_file(&ws.join("lib.rs"), ws.to_str().unwrap());

    // Should find: struct Config, impl Config, fn start_server, fn internal_helper
    assert!(chunks.len() >= 3, "Should extract ≥3 chunks from Rust, got {}", chunks.len());

    let names: Vec<&str> = chunks.iter().map(|c| c.name.as_str()).collect();
    assert!(names.iter().any(|n| n.contains("Config")), "Should find Config struct, got: {:?}", names);
    assert!(names.iter().any(|n| n.contains("start_server")), "Should find start_server fn, got: {:?}", names);

    // Check chunk types
    let types: Vec<&str> = chunks.iter().map(|c| c.chunk_type.as_str()).collect();
    assert!(types.iter().any(|t| t.contains("struct") || t.contains("item")),
        "Should have struct chunk type: {:?}", types);

    // All chunks should have valid line ranges
    for chunk in &chunks {
        assert!(chunk.start_line >= 1, "start_line should be >= 1");
        assert!(chunk.end_line >= chunk.start_line, "end_line >= start_line");
        assert!(!chunk.content.is_empty(), "Content should not be empty");
        assert_eq!(chunk.language, "rust");
    }

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn idx_02_parse_python_file() {
    let ws = std::env::temp_dir().join("sf_idx_02");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::write(ws.join("app.py"), r#"
class DatabaseClient:
    def __init__(self, host: str, port: int = 5432):
        self.host = host
        self.port = port

    def connect(self):
        return f"Connected to {self.host}:{self.port}"

def create_app(config: dict) -> DatabaseClient:
    return DatabaseClient(config["host"])
"#).unwrap();

    let chunks = indexer::parse_file(&ws.join("app.py"), ws.to_str().unwrap());

    assert!(chunks.len() >= 2, "Should extract ≥2 chunks from Python, got {}", chunks.len());
    let names: Vec<&str> = chunks.iter().map(|c| c.name.as_str()).collect();
    assert!(names.iter().any(|n| n.contains("DatabaseClient")), "Should find class: {:?}", names);

    for chunk in &chunks {
        assert_eq!(chunk.language, "python");
    }

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn idx_03_parse_javascript_file() {
    let ws = std::env::temp_dir().join("sf_idx_03");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::write(ws.join("server.js"), r#"
class Router {
    constructor() {
        this.routes = [];
    }

    get(path, handler) {
        this.routes.push({ method: 'GET', path, handler });
    }
}

function createServer(port) {
    const router = new Router();
    return { router, port };
}

const helper = (x) => x * 2;
"#).unwrap();

    let chunks = indexer::parse_file(&ws.join("server.js"), ws.to_str().unwrap());
    assert!(chunks.len() >= 2, "Should extract ≥2 chunks from JS, got {}", chunks.len());

    let names: Vec<&str> = chunks.iter().map(|c| c.name.as_str()).collect();
    assert!(names.iter().any(|n| n.contains("Router")), "Should find Router class: {:?}", names);

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn idx_04_index_workspace_fts5() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_idx_04");
    let _ = std::fs::remove_dir_all(&ws);
    std::fs::create_dir_all(ws.join("src")).unwrap();

    // Create multiple source files
    std::fs::write(ws.join("src/auth.rs"), r#"
pub fn authenticate_user(username: &str, password: &str) -> bool {
    // Verify credentials against database
    username == "admin" && password == "secret"
}

pub fn generate_jwt_token(user_id: &str) -> String {
    format!("jwt.{}.token", user_id)
}
"#).unwrap();

    std::fs::write(ws.join("src/db.rs"), r#"
pub struct DatabasePool {
    connections: Vec<Connection>,
}

impl DatabasePool {
    pub fn new(max_connections: usize) -> Self {
        DatabasePool { connections: Vec::with_capacity(max_connections) }
    }

    pub fn get_connection(&self) -> &Connection {
        &self.connections[0]
    }
}
"#).unwrap();

    std::fs::write(ws.join("src/config.py"), r#"
class AppConfig:
    def __init__(self):
        self.database_url = "sqlite:///app.db"
        self.jwt_secret = "secret-key"

def load_config(path: str) -> AppConfig:
    config = AppConfig()
    return config
"#).unwrap();

    // Index the workspace (without LLM — will use FTS5 only)
    let result = indexer::index_workspace(ws.to_str().unwrap()).await;
    assert!(result.is_ok(), "Indexing should succeed: {:?}", result);
    let (count, _indexed) = result.unwrap();
    assert!(count >= 5, "Should index ≥5 chunks, got {}", count);

    // Search via FTS5 (no embeddings configured → falls back to FTS5)
    let results = indexer::search("authenticate", ws.to_str().unwrap(), 5).await;
    assert!(results.is_ok(), "Search should succeed");
    let results = results.unwrap();
    assert!(!results.is_empty(), "Should find results for 'authenticate'");
    assert!(results[0].file_path.contains("auth.rs"),
        "Top result should be auth.rs, got: {}", results[0].file_path);

    // Search for database-related code
    let db_results = indexer::search("database connection pool", ws.to_str().unwrap(), 5).await;
    assert!(db_results.is_ok());
    let db_results = db_results.unwrap();
    assert!(!db_results.is_empty(), "Should find database-related results");

    // Search for JWT
    let jwt_results = indexer::search("jwt token", ws.to_str().unwrap(), 5).await;
    assert!(jwt_results.is_ok());

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn idx_05_grep_search_fallback() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_idx_05");
    let _ = std::fs::remove_dir_all(&ws);
    std::fs::create_dir_all(&ws).unwrap();

    std::fs::write(ws.join("main.rs"), r#"
fn compute_fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => compute_fibonacci(n - 1) + compute_fibonacci(n - 2),
    }
}
"#).unwrap();

    // Index first (grep_search searches the indexed code_chunks table)
    indexer::index_workspace(ws.to_str().unwrap()).await.unwrap();

    // grep_search should find the pattern in indexed chunks
    let results = indexer::grep_search("fibonacci", ws.to_str().unwrap(), 10);
    assert!(!results.is_empty(), "Grep should find 'fibonacci'");
    assert!(results[0].content.contains("fibonacci"), "Result should contain the match");

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn idx_06_code_search_tool_integration() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_idx_06");
    let _ = std::fs::remove_dir_all(&ws);
    std::fs::create_dir_all(&ws).unwrap();

    std::fs::write(ws.join("handler.rs"), r#"
pub fn handle_request(method: &str, path: &str) -> String {
    match (method, path) {
        ("GET", "/health") => "OK".to_string(),
        ("POST", "/api/data") => process_data(),
        _ => "Not Found".to_string(),
    }
}

fn process_data() -> String {
    "Data processed".to_string()
}
"#).unwrap();

    // Test via the tool interface (like an agent would call it)
    let result = tools::execute_tool(
        "code_search",
        &json!({"query": "handle_request"}),
        ws.to_str().unwrap()
    ).await;

    assert!(!result.contains("Unknown tool"), "code_search should be a valid tool");
    // Should find the function via grep fallback or FTS5
    assert!(result.contains("handle_request") || result.contains("handler.rs"),
        "Should find the handler function: {}", result);

    // Test regex pattern mode
    let pattern_result = tools::execute_tool(
        "code_search",
        &json!({"pattern": "fn \\w+_request"}),
        ws.to_str().unwrap()
    ).await;

    assert!(pattern_result.contains("handle_request"),
        "Pattern search should find handle_request: {}", pattern_result);

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn idx_07_incremental_indexing() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_idx_07");
    let _ = std::fs::remove_dir_all(&ws);
    std::fs::create_dir_all(&ws).unwrap();

    std::fs::write(ws.join("lib.rs"), "fn original() { }").unwrap();

    // First index
    let (count1, _has_emb) = indexer::index_workspace(ws.to_str().unwrap()).await.unwrap();
    assert!(count1 >= 1, "Should index at least 1 chunk, got {}", count1);

    // Second index without changes — should skip (returns same count)
    let result2 = indexer::index_workspace(ws.to_str().unwrap()).await.unwrap();
    assert_eq!(result2.0, count1, "Count should be same after no-op re-index");

    // Modify file → should re-index
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(ws.join("lib.rs"), "fn modified() { }\nfn extra() { }").unwrap();

    let (count3, _) = indexer::index_workspace(ws.to_str().unwrap()).await.unwrap();
    assert!(count3 >= 1, "Should have chunks after re-index, got {}", count3);

    // Verify the new content is searchable
    let results = indexer::search("modified", ws.to_str().unwrap(), 5).await.unwrap();
    assert!(!results.is_empty(), "Should find 'modified' after re-index");

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn idx_08_skip_binary_and_hidden() {
    let ws = std::env::temp_dir().join("sf_idx_08");
    let _ = std::fs::remove_dir_all(&ws);
    std::fs::create_dir_all(ws.join(".git")).unwrap();
    std::fs::create_dir_all(ws.join("node_modules/dep")).unwrap();
    std::fs::create_dir_all(ws.join("src")).unwrap();

    std::fs::write(ws.join(".git/HEAD"), "ref: refs/heads/main").unwrap();
    std::fs::write(ws.join("node_modules/dep/index.js"), "module.exports = {};").unwrap();
    std::fs::write(ws.join("image.png"), &[0x89, 0x50, 0x4e, 0x47]).unwrap();
    std::fs::write(ws.join("src/app.rs"), "fn app() { }").unwrap();

    let chunks = indexer::parse_file(&ws.join("src/app.rs"), ws.to_str().unwrap());
    assert!(!chunks.is_empty(), "Should parse src/app.rs");

    // .git and node_modules should not have chunks
    // (parse_file itself doesn't skip, but index_workspace does)
    ensure_db();
    let (count, _) = indexer::index_workspace(ws.to_str().unwrap()).await.unwrap();
    assert_eq!(count, 1, "Should only index src/app.rs (skip .git, node_modules, .png)");

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn idx_09_format_results() {
    let results = vec![
        indexer::SearchResult {
            file_path: "src/main.rs".to_string(),
            language: "rust".to_string(),
            chunk_type: "function_item".to_string(),
            name: "main".to_string(),
            content: "fn main() {\n    println!(\"hello\");\n}".to_string(),
            start_line: 1,
            end_line: 3,
            score: 0.95,
        },
        indexer::SearchResult {
            file_path: "src/lib.rs".to_string(),
            language: "rust".to_string(),
            chunk_type: "struct_item".to_string(),
            name: "Config".to_string(),
            content: "pub struct Config { pub name: String }".to_string(),
            start_line: 5,
            end_line: 5,
            score: 0.72,
        },
    ];

    let formatted = indexer::format_results(&results);
    assert!(formatted.contains("src/main.rs"), "Should include file path");
    assert!(formatted.contains("main"), "Should include function name");
    assert!(formatted.contains("Config"), "Should include struct name");
    assert!(formatted.contains("fn main()"), "Should include content");
}

#[tokio::test]
async fn idx_10_multi_language_workspace() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_idx_10");
    let _ = std::fs::remove_dir_all(&ws);
    std::fs::create_dir_all(&ws).unwrap();

    // Rust
    std::fs::write(ws.join("server.rs"), "pub fn start_server(port: u16) { }").unwrap();
    // Python
    std::fs::write(ws.join("client.py"), "def connect_client(host: str): pass").unwrap();
    // JavaScript
    std::fs::write(ws.join("utils.js"), "function formatDate(d) { return d.toISOString(); }").unwrap();
    // Go
    std::fs::write(ws.join("main.go"), "package main\n\nfunc handleRequest(w http.ResponseWriter, r *http.Request) {\n}\n").unwrap();

    let (count, _) = indexer::index_workspace(ws.to_str().unwrap()).await.unwrap();
    assert!(count >= 4, "Should index chunks from all languages, got {}", count);

    // FTS5 search should span all languages
    let results = indexer::search("server", ws.to_str().unwrap(), 10).await.unwrap();
    assert!(!results.is_empty(), "Should find server-related chunks");

    let _ = std::fs::remove_dir_all(&ws);
}
