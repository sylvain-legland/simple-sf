// Ref: FT-SSF-019
//! Memory tools: search, store, project memory loading, compaction.

use serde_json::Value;

// ── Tool implementations ──────────────────────────────────

pub(super) fn tool_memory_search(args: &Value, project_id: &str) -> String {
    let query = args["query"].as_str().unwrap_or("").to_lowercase();
    let scope = args["scope"].as_str().unwrap_or("project");

    crate::db::with_db(|conn| {
        let pattern = format!("%{}%", query);
        let sql = match scope {
            "global" => "SELECT key, value, category, project_id FROM memory WHERE \
                         (project_id IS NULL OR project_id = '') AND \
                         (LOWER(key) LIKE ?1 OR LOWER(value) LIKE ?1 OR LOWER(category) LIKE ?1) \
                         ORDER BY created_at DESC LIMIT 20",
            "all" => "SELECT key, value, category, project_id FROM memory WHERE \
                      (LOWER(key) LIKE ?1 OR LOWER(value) LIKE ?1 OR LOWER(category) LIKE ?1) \
                      ORDER BY created_at DESC LIMIT 20",
            _ => "SELECT key, value, category, project_id FROM memory WHERE \
                  (project_id = ?2 OR project_id IS NULL) AND \
                  (LOWER(key) LIKE ?1 OR LOWER(value) LIKE ?1 OR LOWER(category) LIKE ?1) \
                  ORDER BY created_at DESC LIMIT 20",
        };

        let mut stmt = conn.prepare(sql).unwrap();
        let results: Vec<String> = if scope == "global" || scope == "all" {
            stmt.query_map(
                rusqlite::params![pattern],
                |row| {
                    let key: String = row.get(0)?;
                    let value: String = row.get(1)?;
                    let category: String = row.get(2)?;
                    let pid: Option<String> = row.get(3)?;
                    let scope_tag = if pid.as_ref().map(|s| s.is_empty()).unwrap_or(true) { "global" } else { "project" };
                    Ok(format!("[{}/{}] {}: {}", scope_tag, category, key, value))
                },
            ).unwrap().filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map(
                rusqlite::params![pattern, project_id],
                |row| {
                    let key: String = row.get(0)?;
                    let value: String = row.get(1)?;
                    let category: String = row.get(2)?;
                    let pid: Option<String> = row.get(3)?;
                    let scope_tag = if pid.as_ref().map(|s| s.is_empty()).unwrap_or(true) { "global" } else { "project" };
                    Ok(format!("[{}/{}] {}: {}", scope_tag, category, key, value))
                },
            ).unwrap().filter_map(|r| r.ok()).collect()
        };

        if results.is_empty() {
            format!("No memory found for '{}'", query)
        } else {
            results.join("\n\n")
        }
    })
}

pub(super) fn tool_memory_store(args: &Value, project_id: &str) -> String {
    let key = args["key"].as_str().unwrap_or("").to_string();
    let value = args["value"].as_str().unwrap_or("").to_string();
    let category = args["category"].as_str().unwrap_or("note").to_string();
    let scope = args["scope"].as_str().unwrap_or("project");

    if key.is_empty() || value.is_empty() {
        return "Error: key and value are required".to_string();
    }

    let pid = if scope == "global" { None } else { Some(project_id.to_string()) };

    crate::db::with_db(|conn| {
        let existing: Option<i64> = conn.query_row(
            "SELECT id FROM memory WHERE key = ?1 AND (project_id = ?2 OR (?2 IS NULL AND project_id IS NULL)) LIMIT 1",
            rusqlite::params![&key, &pid],
            |row| row.get(0),
        ).ok();

        let result = if let Some(id) = existing {
            conn.execute(
                "UPDATE memory SET value = ?1, category = ?2, created_at = datetime('now') WHERE id = ?3",
                rusqlite::params![&value, &category, id],
            )
        } else {
            conn.execute(
                "INSERT INTO memory (key, value, category, project_id) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![&key, &value, &category, &pid],
            )
        };

        match result {
            Ok(_) => format!("Stored memory '{}' [{}] in category '{}'",
                key, if scope == "global" { "global" } else { "project" }, category),
            Err(e) => {
                eprintln!("[db] Failed to store memory: {}", e);
                format!("Error storing memory: {}", e)
            }
        }
    })
}

// ── Project memory system ─────────────────────────────────

/// Load project memory for injection into system prompt (max 4K chars).
pub fn load_project_memory(project_id: &str) -> String {
    if project_id.is_empty() { return String::new(); }

    crate::db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT key, value, category FROM memory \
             WHERE (project_id = ?1 OR project_id IS NULL) \
             ORDER BY CASE WHEN project_id = ?1 THEN 0 ELSE 1 END, created_at DESC \
             LIMIT 30"
        ).unwrap();

        let entries: Vec<String> = stmt.query_map(
            rusqlite::params![project_id],
            |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                let cat: String = row.get(2)?;
                Ok(format!("- [{}] {}: {}", cat, key, if value.len() > 300 { &value[..300] } else { &value }))
            },
        ).unwrap().filter_map(|r| r.ok()).collect();

        if entries.is_empty() { return String::new(); }

        let mut result = String::from("\n\n## Project Memory\n");
        let mut chars = 0;
        for entry in &entries {
            if chars + entry.len() > 4000 { break; }
            result.push_str(entry);
            result.push('\n');
            chars += entry.len();
        }
        result
    })
}

/// Scan workspace for instruction files and store them in memory.
pub fn load_project_files(workspace: &str, project_id: &str) {
    const INSTRUCTION_FILES: &[&str] = &[
        "CLAUDE.md", ".github/copilot-instructions.md", "SPECS.md",
        "VISION.md", "README.md", ".cursorrules", "CONVENTIONS.md",
    ];
    const MAX_FILE_CHARS: usize = 3000;
    const MAX_TOTAL_CHARS: usize = 8000;

    let mut total = 0;
    for filename in INSTRUCTION_FILES {
        if total >= MAX_TOTAL_CHARS { break; }
        let path = std::path::Path::new(workspace).join(filename);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let trimmed = if content.len() > MAX_FILE_CHARS {
                &content[..MAX_FILE_CHARS]
            } else {
                &content
            };
            crate::db::with_db(|conn| {
                let existing: Option<i64> = conn.query_row(
                    "SELECT id FROM memory WHERE key = ?1 AND project_id = ?2 LIMIT 1",
                    rusqlite::params![filename, project_id],
                    |row| row.get(0),
                ).ok();
                if let Some(id) = existing {
                    let _ = conn.execute(
                        "UPDATE memory SET value = ?1, created_at = datetime('now') WHERE id = ?2",
                        rusqlite::params![trimmed, id],
                    );
                } else {
                    let _ = conn.execute(
                        "INSERT INTO memory (key, value, category, project_id) VALUES (?1, ?2, 'project_file', ?3)",
                        rusqlite::params![filename, trimmed, project_id],
                    );
                }
            });
            total += trimmed.len();
            eprintln!("[memory] Loaded {} ({} chars) for project {}", filename, trimmed.len(), &project_id[..8.min(project_id.len())]);
        }
    }
}

/// Compact memory: dedup by key, prune old entries, enforce per-project cap.
pub fn compact_memory(project_id: &str) {
    crate::db::with_db(|conn| {
        // 1. Deduplicate: keep only the latest entry per key+project_id
        let deduped = conn.execute(
            "DELETE FROM memory WHERE id NOT IN (
                SELECT MAX(id) FROM memory GROUP BY key, COALESCE(project_id, '')
            )", [],
        ).unwrap_or(0);

        // 2. Prune entries older than 30 days (except project_file and decision categories)
        let pruned = conn.execute(
            "DELETE FROM memory WHERE created_at < datetime('now', '-30 days') \
             AND category NOT IN ('project_file', 'decision', 'convention')",
            [],
        ).unwrap_or(0);

        // 3. Cap per-project at 200 entries (keep most recent)
        if !project_id.is_empty() {
            let _ = conn.execute(
                "DELETE FROM memory WHERE project_id = ?1 AND id NOT IN (
                    SELECT id FROM memory WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 200
                )",
                rusqlite::params![project_id],
            );
        }

        if deduped > 0 || pruned > 0 {
            eprintln!("[memory] Compacted: {} deduped, {} pruned for project {}",
                deduped, pruned, &project_id[..8.min(project_id.len())]);
        }
    });
}
