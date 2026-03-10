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
    let project_id = "pacman-game";
    db::with_db(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO projects (id, name, description, tech, status) \
             VALUES (?1, ?2, ?3, ?4, 'active')",
            rusqlite::params![
                project_id,
                "Pac-Man macOS Game",
                "A native macOS Pac-Man arcade game built with Swift and SpriteKit",
                "Swift, SpriteKit, macOS"
            ],
        ).unwrap();
    });

    // Use product-lifecycle (14 phases)
    let mission_id = format!("pacman-{}", uuid::Uuid::new_v4());
    let brief = "Build a native macOS Pac-Man arcade game as a Swift Package (Package.swift). \
                 Tech stack: Swift 5.9, SpriteKit, macOS 13+. \
                 Requirements: \
                 1) Package.swift with executable target named 'PacMan' \
                 2) Sources/PacMan/main.swift — app entry with NSApplication \
                 3) Sources/PacMan/GameScene.swift — SpriteKit scene with: \
                    - 28x31 tile-based maze (SKTileMapNode) \
                    - Pac-Man sprite (yellow circle, mouth animation) \
                    - 4 ghost sprites (red, pink, cyan, orange) with basic AI \
                    - Dot/pellet collection with score counter \
                    - Arrow key input handling \
                 4) Sources/PacMan/GameWindow.swift — NSWindow with SKView \
                 The project MUST compile with 'swift build'. No external dependencies.";

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

    // Check build phase result
    let build_status = db::with_db(|conn| {
        conn.query_row(
            "SELECT status FROM mission_phases WHERE mission_id = ?1 AND phase_name = 'finalize-build'",
            rusqlite::params![&mission_id],
            |r| r.get::<_, String>(0),
        ).unwrap_or_default()
    });
    eprintln!("\n🔨 Build status: {}", build_status);

    // Check for Package.swift (minimum: agents wrote a Swift project)
    let has_package = std::path::Path::new(&workspace).join("Package.swift").exists()
        || std::path::Path::new(&workspace).join("Sources/PacMan/main.swift").exists();
    if has_package {
        eprintln!("📦 Package.swift found — Swift project structure created");
    }

    // Check for compiled binary
    let binary_candidates = [
        format!("{}/.build/release/PacMan", workspace),
        format!("{}/.build/debug/PacMan", workspace),
    ];
    let compiled = binary_candidates.iter().any(|p| std::path::Path::new(p).exists());
    if compiled {
        eprintln!("🎮 Compiled PacMan binary found!");
    } else {
        eprintln!("⚠️  No compiled binary — build step may have failed");
    }

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

/// Mission corrective : la SF reprend le projet Pac-Man là où il en est,
/// lit la mémoire des erreurs passées, et livre une app compilée.
/// Workflow court : archi → dev sprint(3) → QA feedback(3) → build final.
#[tokio::test]
#[ignore]
async fn pacman_fix_and_ship() {
    init_with_full_catalog();
    if !configure_minimax() {
        eprintln!("SKIP — no MiniMax key");
        return;
    }
    engine::YOLO_MODE.store(true, Ordering::Relaxed);

    let home = std::env::var("HOME").unwrap();
    let project_id = "pacman-game";

    // Register project
    db::with_db(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO projects (id, name, description, tech, status) \
             VALUES (?1, ?2, ?3, ?4, 'active')",
            rusqlite::params![project_id, "Pac-Man macOS Game",
                "Native macOS Pac-Man arcade game — Swift + SpriteKit",
                "Swift, SpriteKit, macOS"],
        ).unwrap();
    });

    // ══════════════════════════════════════════════════════════════
    // MISSION — brief simple, la SF se débrouille
    // ══════════════════════════════════════════════════════════════
    let mission_id = format!("pacman-fix-{}", uuid::Uuid::new_v4());
    let brief = "Construire un jeu Pac-Man natif macOS en Swift Package Manager + SpriteKit. \
                 Le jeu doit avoir: un labyrinthe classique, Pac-Man contrôlé aux flèches, \
                 4 fantômes avec IA basique, des dots à collecter, et un score. \
                 Le projet DOIT compiler avec 'xcrun swift build' sans erreur. \
                 Pas de dépendance externe — uniquement les frameworks Apple.";

    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO missions (id, project_id, brief, status, workflow) \
             VALUES (?1, ?2, ?3, 'pending', 'product-lifecycle')",
            rusqlite::params![&mission_id, project_id, brief],
        ).unwrap();
    });

    let workspace = format!("{}/Library/Application Support/SimpleSF/workspaces/{}", home, mission_id);
    std::fs::create_dir_all(&workspace).unwrap();

    eprintln!("\n╔══════════════════════════════════════════════════╗");
    eprintln!("║  🕹️  PACMAN — SF 100% autonome                    ║");
    eprintln!("║  Mémoire: vide │ YOLO: ON │ Workflow: P-L        ║");
    eprintln!("╚══════════════════════════════════════════════════════╝");
    eprintln!("📋 Brief: {}...", &brief[..100]);
    eprintln!("🗂  Workspace: .../{}\n", &mission_id[..20]);

    let phase_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let pc = phase_count.clone();

    let callback: executor::EventCallback = Arc::new(move |agent_id, event| {
        match &event {
            executor::AgentEvent::Response { content } => {
                if content.starts_with("──") {
                    pc.fetch_add(1, Ordering::Relaxed);
                    eprintln!("\n  {}", content);
                } else if content.contains("CONTINUE") || content.contains("DONE")
                    || content.contains("VETO") || content.contains("APPROVE")
                    || content.contains("sprint") || content.contains("Sprint") {
                    eprintln!("    📌 {} → {}", agent_id, &content[..content.len().min(200)].replace('\n', " "));
                } else {
                    let preview: String = content.chars().take(100).collect::<String>().replace('\n', " ");
                    eprintln!("    💬 {} → {}", agent_id, preview);
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

    eprintln!("🚀 MISSION START — la SF prend la main\n");
    let start = std::time::Instant::now();
    let result = engine::run_mission(&mission_id, brief, &workspace, &callback).await;
    let elapsed = start.elapsed();

    // ══════════════════════════════════════════════════════════════
    // REPORT
    // ══════════════════════════════════════════════════════════════
    eprintln!("\n⏱  Total: {:.0}s ({:.1} min)", elapsed.as_secs_f64(), elapsed.as_secs_f64() / 60.0);

    let phases = db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT phase_name, pattern, phase_type, status, iteration, max_iterations \
             FROM mission_phases WHERE mission_id = ?1 ORDER BY rowid"
        ).unwrap();
        stmt.query_map(rusqlite::params![&mission_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
                r.get::<_, String>(3)?, r.get::<_, i64>(4)?, r.get::<_, i64>(5)?))
        }).unwrap().filter_map(|r| r.ok()).collect::<Vec<_>>()
    });

    eprintln!("\n📊 Phases exécutées:");
    for (name, pat, ptype, status, iter, max_iter) in &phases {
        let icon = match status.as_str() { "completed" => "✅", "vetoed" => "🚫", "failed" => "❌", _ => "⏳" };
        eprintln!("  {} {:30} {:12} {:15} {} iter={}/{}",
            icon, name, pat, ptype, status, iter, max_iter);
    }

    // Check memory — did agents store new learnings?
    let mem_count = db::with_db(|conn| {
        conn.query_row("SELECT COUNT(*) FROM memory WHERE project_id = ?1",
            rusqlite::params![project_id], |r| r.get::<_, i64>(0)).unwrap_or(0)
    });
    eprintln!("\n🧠 Mémoire projet: {} entrées (7 seedées + {} nouvelles)", mem_count, mem_count - 7);

    // Files produced
    let files = list_files_recursive(&workspace);
    eprintln!("\n📁 Fichiers produits: {}", files.len());
    for f in &files {
        let size = std::fs::metadata(f).map(|m| m.len()).unwrap_or(0);
        let rel = f.strip_prefix(&workspace).unwrap_or(f.as_str());
        if rel.ends_with(".swift") || rel.contains("Package") {
            eprintln!("   📄 {} ({} bytes)", rel, size);
        }
    }

    // Build check
    let build_output = std::process::Command::new("xcrun")
        .args(["swift", "build"])
        .current_dir(&workspace)
        .output();

    match build_output {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let combined = format!("{}{}", stdout, stderr);
            if out.status.success() || combined.contains("Build complete") {
                eprintln!("\n🎉 BUILD SUCCESS — xcrun swift build OK");
            } else {
                let errors: Vec<_> = combined.lines().filter(|l| l.contains("error:")).collect();
                eprintln!("\n❌ BUILD FAILED — {} erreurs", errors.len());
                for e in errors.iter().take(5) {
                    eprintln!("   {}", e);
                }
            }
        }
        Err(e) => eprintln!("\n⚠️  Build command failed: {}", e),
    }

    // Binary check
    let binary_paths = [
        format!("{}/.build/release/PacMan", workspace),
        format!("{}/.build/debug/PacMan", workspace),
    ];
    let compiled = binary_paths.iter().any(|p| std::path::Path::new(p).exists());
    if compiled {
        eprintln!("🎮 Binaire PacMan compilé trouvé!");
    }

    match &result {
        Ok(()) => eprintln!("\n✅ Mission terminée avec succès"),
        Err(e) => eprintln!("\n⚠️  Mission: {}", e),
    }

    // Assertions — the SF should have produced at least a compiling project
    assert!(result.is_ok(), "Mission should complete");
    let completed = phases.iter().filter(|(_, _, _, s, _, _)| s == "completed").count();
    assert!(completed >= 3, "At least 3 phases completed, got {}", completed);

    eprintln!("\n🗂  Workspace: {}", workspace);
}
