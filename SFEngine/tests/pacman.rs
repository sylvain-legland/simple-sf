//! PACMAN — Full SAFe mission E2E with real LLM and 192 agents.
//! Run with: cargo test --test pacman -- --ignored --nocapture --test-threads=1

use sf_engine::{db, llm, engine, catalog, executor};
use std::sync::{Arc, atomic::Ordering};

fn init_with_full_catalog() {
    let db_path = format!(
        "{}/Library/Application Support/SimpleSF/sf_pacman_test.db",
        std::env::var("HOME").unwrap()
    );
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{}-shm", db_path));
    let _ = std::fs::remove_file(format!("{}-wal", db_path));
    db::init_db(&db_path);

    // Load full 192-agent catalog from SFData
    let data_dir = format!(
        "{}/_MACARON-SOFTWARE/simple-sf/SimpleSF/Resources/SFData",
        std::env::var("HOME").unwrap()
    );
    catalog::seed_from_json(&data_dir);
}

fn configure_minimax() -> bool {
    let key_path = format!(
        "{}/.config/factory/minimax.key",
        std::env::var("HOME").unwrap()
    );
    match std::fs::read_to_string(&key_path) {
        Ok(key) => {
            llm::configure_llm("minimax", key.trim(), "https://api.minimax.io/v1", "MiniMax-M2.5");
            true
        }
        Err(_) => false,
    }
}

#[tokio::test]
#[ignore]
async fn pacman_full_safe_mission() {
    init_with_full_catalog();
    if !configure_minimax() {
        eprintln!("SKIP — no MiniMax key");
        return;
    }

    // Enable YOLO (auto-approve HITL checkpoints)
    engine::YOLO_MODE.store(true, Ordering::Relaxed);

    // Verify catalog loaded
    let agent_count = db::with_db(|conn| {
        conn.query_row("SELECT COUNT(*) FROM agents", [], |r| r.get::<_, i64>(0)).unwrap_or(0)
    });
    let wf_count = db::with_db(|conn| {
        conn.query_row("SELECT COUNT(*) FROM workflows", [], |r| r.get::<_, i64>(0)).unwrap_or(0)
    });
    eprintln!("\n╔══════════════════════════════════════════════╗");
    eprintln!("║  🕹️  PACMAN — Full SAFe Mission               ║");
    eprintln!("║  Agents: {:>3} │ Workflows: {:>2} │ YOLO: ON     ║", agent_count, wf_count);
    eprintln!("╚══════════════════════════════════════════════╝\n");
    assert!(agent_count >= 50, "Need full catalog, got {}", agent_count);

    // Create project
    let project_id = "pacman-test";
    db::with_db(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO projects (id, name, description, tech, status) \
             VALUES (?1, ?2, ?3, ?4, 'active')",
            rusqlite::params![
                project_id,
                "Pacman Dashboard",
                "A real-time analytics dashboard for monitoring game metrics",
                "React, TypeScript, Node.js, PostgreSQL"
            ],
        ).unwrap();
    });

    // Use product-lifecycle (14 phases)
    let mission_id = format!("pacman-{}", uuid::Uuid::new_v4());
    let brief = "Build a real-time analytics dashboard for a gaming platform. \
                 Features: live player count chart, revenue metrics cards, \
                 server health status panel, and a leaderboard table. \
                 Stack: React + TypeScript frontend, Node.js API, PostgreSQL database. \
                 Must include responsive CSS with a dark theme.";

    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO missions (id, project_id, brief, status, workflow) \
             VALUES (?1, ?2, ?3, 'pending', 'product-lifecycle')",
            rusqlite::params![&mission_id, project_id, brief],
        ).unwrap();
    });

    let home = std::env::var("HOME").unwrap();
    let workspace = format!("{}/Library/Application Support/SimpleSF/workspaces/{}", home, mission_id);
    std::fs::create_dir_all(&workspace).unwrap();

    eprintln!("📋 Brief: {}", &brief[..80]);
    eprintln!("🗂  Workspace: .../{}\n", &mission_id[..20]);

    // Event tracking
    let phase_events: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let pe = phase_events.clone();
    let agent_responses: Arc<std::sync::Mutex<Vec<(String, String)>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let ar = agent_responses.clone();

    let callback: executor::EventCallback = Arc::new(move |agent_id, event| {
        match &event {
            executor::AgentEvent::Response { content } => {
                // Show phase transitions
                if content.starts_with("──") {
                    eprintln!("  {}", content);
                    pe.lock().unwrap().push(content.clone());
                } else {
                    let preview = content.chars().take(120).collect::<String>().replace('\n', " ");
                    eprintln!("    💬 {} → {}", agent_id, preview);
                    ar.lock().unwrap().push((agent_id.to_string(), preview));
                }
            }
            executor::AgentEvent::ToolCall { tool, args: _ } => {
                eprintln!("    🔧 {} → {}", agent_id, tool);
            }
            executor::AgentEvent::Error { message } => {
                eprintln!("    ❌ {} → {}", agent_id, &message[..message.len().min(150)]);
            }
            _ => {}
        }
    });

    eprintln!("🚀 MISSION START\n");
    let start = std::time::Instant::now();

    let result = engine::run_mission(&mission_id, brief, &workspace, &callback).await;

    let elapsed = start.elapsed();
    eprintln!("\n⏱  Total: {:.0}s ({:.1} min)\n", elapsed.as_secs_f64(), elapsed.as_secs_f64() / 60.0);

    // Results
    let status = db::with_db(|conn| {
        conn.query_row("SELECT status FROM missions WHERE id = ?1",
            rusqlite::params![&mission_id], |r| r.get::<_, String>(0)).unwrap_or_default()
    });

    let phases = db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT phase_name, pattern, status, output FROM mission_phases \
             WHERE mission_id = ?1 ORDER BY rowid"
        ).unwrap();
        stmt.query_map(rusqlite::params![&mission_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?,
                r.get::<_, String>(2)?, r.get::<_, String>(3).unwrap_or_default()))
        }).unwrap().filter_map(|r| r.ok()).collect::<Vec<_>>()
    });

    let msg_count = db::with_db(|conn| {
        conn.query_row("SELECT COUNT(*) FROM agent_messages WHERE mission_id = ?1",
            rusqlite::params![&mission_id], |r| r.get::<_, i64>(0)).unwrap_or(0)
    });

    eprintln!("╔══════════════════════════════════════════════╗");
    eprintln!("║  📊 RESULTS                                   ║");
    eprintln!("╠══════════════════════════════════════════════╣");
    eprintln!("║  Status: {:40} ║", status);
    eprintln!("║  Phases: {}/{:38} ║",
        phases.iter().filter(|(_, _, s, _)| s == "completed").count(), phases.len());
    eprintln!("║  Messages: {:36} ║", msg_count);
    eprintln!("║  Duration: {:.0}s {:33} ║", elapsed.as_secs_f64(), "");
    eprintln!("╠══════════════════════════════════════════════╣");

    for (name, pattern, status, output) in &phases {
        let icon = match status.as_str() {
            "completed" => "✅",
            "failed" => "❌",
            "vetoed" => "🚫",
            _ => "⏳",
        };
        let out_preview = if output.len() > 60 {
            format!("{}...", &output[..60].replace('\n', " "))
        } else {
            output.replace('\n', " ")
        };
        eprintln!("║ {} {:20} {:12} {:8} ║", icon, name, pattern, status);
        if !out_preview.is_empty() {
            eprintln!("║    └─ {}  ║", &out_preview[..out_preview.len().min(40)]);
        }
    }
    eprintln!("╚══════════════════════════════════════════════╝");

    // Show generated files
    let files = list_files_recursive(&workspace);
    if !files.is_empty() {
        eprintln!("\n📁 Generated files:");
        for f in &files {
            let size = std::fs::metadata(f).map(|m| m.len()).unwrap_or(0);
            let rel = f.strip_prefix(&workspace).unwrap_or(f.as_str());
            eprintln!("   {} ({} bytes)", rel, size);
        }
    }

    // Assertions
    match &result {
        Ok(()) => eprintln!("\n✅ Mission completed successfully"),
        Err(e) => eprintln!("\n⚠️  Mission result: {}", e),
    }

    let completed = phases.iter().filter(|(_, _, s, _)| s == "completed").count();
    assert!(completed >= 3, "At least 3 phases should complete, got {}/{}", completed, phases.len());

    // Cleanup
    let db_path = format!("{}/Library/Application Support/SimpleSF/sf_pacman_test.db", home);
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{}-shm", db_path));
    let _ = std::fs::remove_file(format!("{}-wal", db_path));
    // Keep workspace for inspection
    eprintln!("\n🗂  Workspace preserved at: {}", workspace);
}

fn list_files_recursive(dir: &str) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(list_files_recursive(path.to_str().unwrap_or("")));
            } else {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
    files
}
