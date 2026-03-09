/// ══════════════════════════════════════════════════════════════
/// PACMAN MISSION RUNNER — Full development cycle via local MLX
/// ══════════════════════════════════════════════════════════════
///
/// Runs a complete Software Factory mission to develop a Pac-Man
/// game for macOS using Swift + SpriteKit, orchestrated by agents
/// powered by a local MLX LLM (Qwen3.5-35B-A3B).
///
/// Usage: cargo run --example pacman_mission

use sf_engine::{db, llm, catalog, engine, sandbox};
use sf_engine::executor::AgentEvent;
use std::sync::Arc;
use chrono::Utc;
use rusqlite::params;

const MLX_URL: &str = "http://127.0.0.1:8800/v1";
const MLX_MODEL: &str = "mlx-community/Qwen3.5-35B-A3B-4bit";
const WORKSPACE: &str = "/tmp/pacman-dev";

const BRIEF: &str = r#"
# Mission: Jeu Pac-Man macOS natif

## Objectif
Développer un jeu Pac-Man complet et jouable pour macOS en Swift natif avec SpriteKit.

## Spécifications fonctionnelles
1. **Labyrinthe**: Grille 28×31 classique avec murs, points (pac-dots), power pellets
2. **Pac-Man**: Contrôlé par les flèches du clavier, animation bouche ouverte/fermée
3. **Fantômes**: 4 fantômes (Blinky, Pinky, Inky, Clyde) avec IA de poursuite
   - Mode chase: chaque fantôme a une stratégie différente
   - Mode scatter: retour aux coins
   - Mode frightened (bleu): après power pellet, Pac-Man peut les manger
4. **Scoring**: 10 pts par pac-dot, 50 pts par power pellet, 200/400/800/1600 pour fantômes
5. **Vies**: 3 vies, affichées en bas de l'écran
6. **Niveaux**: Vitesse augmente à chaque niveau
7. **Sons**: Effets sonores (waka-waka, manger fantôme, mort)

## Stack technique
- Swift 5.9+ / macOS 14+
- SpriteKit pour le rendu 2D
- GameplayKit pour l'IA des fantômes (GKStateMachine, pathfinding)
- SwiftUI pour le menu principal et le HUD

## Structure attendue
```
PacMan/
├── Package.swift                    # Swift Package Manager
├── Sources/
│   ├── PacMan/
│   │   ├── App.swift                # @main entry point
│   │   ├── ContentView.swift        # SwiftUI wrapper
│   │   ├── Game/
│   │   │   ├── GameScene.swift      # SpriteKit scene principale
│   │   │   ├── GameLogic.swift      # Logique de jeu séparée
│   │   │   ├── MazeBuilder.swift    # Construction du labyrinthe
│   │   │   ├── PacManNode.swift     # Sprite Pac-Man
│   │   │   ├── GhostNode.swift      # Sprites fantômes + IA
│   │   │   ├── PelletNode.swift     # Pac-dots et power pellets
│   │   │   └── ScoreManager.swift   # Gestion score/vies/niveau
│   │   └── Resources/
│   │       └── Maze.json            # Données du labyrinthe
│   └── Tests/
│       └── PacManTests/
│           ├── GameLogicTests.swift
│           └── MazeBuilderTests.swift
```

## Contraintes
- PAS de storyboard, 100% code
- PAS de flags debug (showsFPS, showsNodeCount) dans le code final
- Architecture MVVM propre avec séparation logique/vue
- Tests unitaires pour GameLogic et MazeBuilder
- Le jeu doit compiler et tourner avec `swift build && swift run`
"#;

#[tokio::main]
async fn main() {
    eprintln!("══════════════════════════════════════════════════════════════");
    eprintln!("  🎮 PAC-MAN MISSION — Full Development Cycle");
    eprintln!("  LLM: {} (local MLX)", MLX_MODEL);
    eprintln!("  Sandbox: {}", sandbox::current_mode());
    eprintln!("══════════════════════════════════════════════════════════════\n");

    // ── 1. Initialize ──
    db::init_db("/tmp/pacman-sf.db");
    llm::configure_llm("mlx", "no-key", MLX_URL, MLX_MODEL);
    catalog::seed_from_json("/nonexistent"); // fallback agents

    // Enable YOLO mode — auto-approve all gates
    engine::YOLO_MODE.store(true, std::sync::atomic::Ordering::Relaxed);

    eprintln!("[init] DB initialized, LLM configured, agents seeded");
    eprintln!("[init] Workspace: {}", WORKSPACE);

    // Create workspace
    std::fs::create_dir_all(WORKSPACE).expect("Failed to create workspace");

    // ── 2. Create project in DB ──
    let project_id = "pacman";
    db::with_db(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO projects (id, name, description, tech, status) VALUES (?1, ?2, ?3, ?4, 'active')",
            params![project_id, "Pac-Man macOS", "Jeu Pac-Man natif macOS Swift SpriteKit", "swift,spritekit,swiftui"],
        ).unwrap();
    });

    // ── 3. Create mission ──
    let mission_id = format!("pacman-{}", Utc::now().format("%Y%m%d-%H%M%S"));
    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO missions (id, brief, project_id, status, workflow, created_at) \
             VALUES (?1, ?2, ?3, 'pending', 'safe-standard', ?4)",
            params![
                mission_id,
                BRIEF,
                project_id,
                Utc::now().to_rfc3339(),
            ],
        ).unwrap();
    });

    eprintln!("[mission] Created: {}\n", mission_id);

    // ── 4. Event callback — live output ──
    let start_time = std::time::Instant::now();
    let event_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let ec = event_count.clone();

    let on_event = move |agent_id: &str, event: AgentEvent| {
        let n = ec.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let elapsed = start_time.elapsed().as_secs();
        let mins = elapsed / 60;
        let secs = elapsed % 60;

        match &event {
            AgentEvent::Response { content } => {
                let preview = if content.len() > 200 {
                    format!("{}...", &content[..200])
                } else {
                    content.clone()
                };
                eprintln!("[{:02}:{:02}] #{:03} 💬 {} → {}", mins, secs, n, agent_id, preview);
            }
            AgentEvent::ToolCall { tool, args } => {
                let args_preview = if args.len() > 80 {
                    format!("{}...", &args[..80])
                } else {
                    args.clone()
                };
                eprintln!("[{:02}:{:02}] #{:03} 🔧 {} → {}({})", mins, secs, n, agent_id, tool, args_preview);
            }
            AgentEvent::ToolResult { tool, result } => {
                let res_preview = if result.len() > 100 {
                    format!("{}...", &result[..100])
                } else {
                    result.clone()
                };
                eprintln!("[{:02}:{:02}] #{:03} ✅ {} → {} = {}", mins, secs, n, agent_id, tool, res_preview);
            }
            AgentEvent::Reasoning { active } => {
                if *active {
                    eprintln!("[{:02}:{:02}] #{:03} 🧠 {} reasoning...", mins, secs, n, agent_id);
                } else {
                    eprintln!("[{:02}:{:02}] #{:03} 🧠 {} reasoning done", mins, secs, n, agent_id);
                }
            }
            AgentEvent::Thinking => {
                eprintln!("[{:02}:{:02}] #{:03} 💭 {} thinking...", mins, secs, n, agent_id);
            }
            AgentEvent::ResponseChunk { content } => {
                // Skip chunks in console output
                let _ = content;
            }
            AgentEvent::Error { message } => {
                eprintln!("[{:02}:{:02}] #{:03} ❌ {} → ERROR: {}", mins, secs, n, agent_id, message);
            }
        }
    };

    let on_event: Arc<dyn Fn(&str, AgentEvent) + Send + Sync> = Arc::new(on_event);

    // ── 5. Run mission ──
    eprintln!("═══════════════════════════════════════");
    eprintln!("  🚀 LAUNCHING MISSION");
    eprintln!("═══════════════════════════════════════\n");

    match engine::run_mission(&mission_id, BRIEF, WORKSPACE, &on_event).await {
        Ok(()) => {
            let elapsed = start_time.elapsed();
            let events = event_count.load(std::sync::atomic::Ordering::SeqCst);
            eprintln!("\n═══════════════════════════════════════");
            eprintln!("  ✅ MISSION COMPLETE");
            eprintln!("  Duration: {}m {}s", elapsed.as_secs() / 60, elapsed.as_secs() % 60);
            eprintln!("  Events: {}", events);
            eprintln!("═══════════════════════════════════════");
        }
        Err(e) => {
            eprintln!("\n═══════════════════════════════════════");
            eprintln!("  ❌ MISSION FAILED: {}", e);
            eprintln!("═══════════════════════════════════════");
        }
    }

    // ── 6. Show workspace results ──
    eprintln!("\n📁 Workspace contents:");
    if let Ok(entries) = std::fs::read_dir(WORKSPACE) {
        fn show_tree(path: &std::path::Path, prefix: &str) {
            if let Ok(entries) = std::fs::read_dir(path) {
                let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                entries.sort_by_key(|e| e.file_name());
                for entry in entries {
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    if name.starts_with('.') { continue; }
                    let meta = entry.metadata().ok();
                    if let Some(m) = &meta {
                        if m.is_dir() {
                            eprintln!("{}📂 {}/", prefix, name);
                            show_tree(&entry.path(), &format!("{}  ", prefix));
                        } else {
                            eprintln!("{}📄 {} ({} bytes)", prefix, name, m.len());
                        }
                    }
                }
            }
        }
        show_tree(std::path::Path::new(WORKSPACE), "  ");
    }
}
