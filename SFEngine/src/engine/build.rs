// Ref: FT-SSF-020

use crate::db;
use crate::executor::{AgentEvent, EventCallback};
use crate::tools;
use rusqlite::params;
use std::os::unix::fs::PermissionsExt;

pub(crate) async fn auto_build_check(workspace: &str) -> String {
    use std::path::Path;

    let ws = Path::new(workspace);
    let build_cmd = if ws.join("Package.swift").exists() {
        "xcrun swift build 2>&1"
    } else if ws.join("Cargo.toml").exists() {
        "cargo build 2>&1"
    } else if ws.join("package.json").exists() {
        "npm install --silent 2>&1 && npm run build --silent 2>&1"
    } else if ws.join("Makefile").exists() {
        "make 2>&1"
    } else if ws.join("go.mod").exists() {
        "go build ./... 2>&1"
    } else {
        return String::new(); // no recognized project — skip
    };

    let output = tools::execute_tool(
        "build",
        &serde_json::json!({ "command": build_cmd }),
        workspace,
    ).await;

    let success = output.contains("Build complete") || output.contains("Finished")
        || (!output.contains("error:") && !output.contains("FAILED"));

    if success {
        "BUILD OK — compilation succeeded.".into()
    } else {
        // Return first 2000 chars of error output
        let preview: String = output.chars().take(2000).collect();
        format!("BUILD FAILED:\n{}", preview)
    }
}

pub(crate) async fn finalize_build(workspace: &str, mission_id: &str, on_event: &EventCallback) {
    use std::path::Path;

    let ws = Path::new(workspace);

    // Detect project type and build command
    let build_cmd = if ws.join("Package.swift").exists() {
        // Swift package — use xcrun to avoid OpenStack swift CLI shadowing
        Some("xcrun swift build -c release 2>&1")
    } else if ws.join("Cargo.toml").exists() {
        Some("cargo build --release 2>&1")
    } else if ws.join("package.json").exists() {
        Some("npm install --silent 2>&1 && npm run build --silent 2>&1")
    } else if ws.join("Makefile").exists() {
        Some("make 2>&1")
    } else if ws.join("go.mod").exists() {
        Some("go build ./... 2>&1")
    } else if ws.join("CMakeLists.txt").exists() {
        Some("cmake -B build && cmake --build build 2>&1")
    } else {
        None
    };

    let Some(cmd) = build_cmd else {
        on_event("engine", AgentEvent::Response {
            content: "── Build: no recognized project file (Package.swift, Cargo.toml, package.json…) ──".into(),
        });
        return;
    };

    on_event("engine", AgentEvent::Response {
        content: format!("── Build: compiling project ({}) ──", cmd.split_whitespace().next().unwrap_or("?")),
    });

    let output = tools::execute_tool(
        "build",
        &serde_json::json!({ "command": cmd }),
        workspace,
    ).await;

    let success = output.contains("Build complete") || output.contains("Finished")
        || (!output.contains("error:") && !output.contains("FAILED"));
    let build_status = if success { "build_ok" } else { "build_failed" };

    // Store build result in DB
    let _ = db::with_db(|conn| {
        conn.execute(
            "INSERT INTO mission_phases (id, mission_id, phase_name, pattern, status, output, started_at, completed_at) \
             VALUES (?1, ?2, 'finalize-build', 'solo', ?3, ?4, datetime('now'), datetime('now'))",
            params![
                format!("{}-build", mission_id),
                mission_id,
                build_status,
                &output[..output.len().min(4000)],
            ],
        )
    });

    // Check for built artifacts
    let artifacts = detect_build_artifacts(workspace);

    if success {
        on_event("engine", AgentEvent::Response {
            content: format!("── Build OK ── {}", if artifacts.is_empty() { "no binary detected".into() } else { artifacts.join(", ") }),
        });
    } else {
        let preview = output.lines()
            .filter(|l| l.contains("error"))
            .take(5)
            .collect::<Vec<_>>()
            .join("\n");
        on_event("engine", AgentEvent::Error {
            message: format!("Build failed:\n{}", preview),
        });
    }
}

pub(crate) fn detect_build_artifacts(workspace: &str) -> Vec<String> {
    let mut artifacts = Vec::new();
    let ws = std::path::Path::new(workspace);

    // Swift: .build/release/ or .build/debug/
    for profile in ["release", "debug"] {
        let build_dir = ws.join(format!(".build/{}", profile));
        if build_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&build_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && !p.extension().map_or(false, |e| e == "o" || e == "d" || e == "swiftdeps") {
                        if let Ok(meta) = p.metadata() {
                            if meta.len() > 10_000 && meta.permissions().mode() & 0o111 != 0 {
                                artifacts.push(format!(".build/{}/{}", profile, p.file_name().unwrap().to_string_lossy()));
                            }
                        }
                    }
                }
            }
        }
    }

    // Rust: target/release/
    let target_release = ws.join("target/release");
    if target_release.exists() {
        if let Ok(entries) = std::fs::read_dir(&target_release) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && !p.extension().map_or(false, |e| e == "d" || e == "rmeta" || e == "rlib") {
                    if let Ok(meta) = p.metadata() {
                        if meta.len() > 10_000 && meta.permissions().mode() & 0o111 != 0 {
                            artifacts.push(format!("target/release/{}", p.file_name().unwrap().to_string_lossy()));
                        }
                    }
                }
            }
        }
    }

    // Generic: look for .app bundles
    for entry in walkdir::WalkDir::new(workspace).max_depth(4).into_iter().flatten() {
        if entry.path().extension().map_or(false, |e| e == "app") && entry.path().is_dir() {
            artifacts.push(entry.path().strip_prefix(workspace).unwrap_or(entry.path()).to_string_lossy().into());
        }
    }

    artifacts
}
