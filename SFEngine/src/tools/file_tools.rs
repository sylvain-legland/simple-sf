// Ref: FT-SSF-019
//! File system tools: path validation, listing, deep search.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Validate that a path is safe (no traversal, stays within workspace).
pub(crate) fn safe_resolve(workspace: &str, path: &str) -> Result<std::path::PathBuf, String> {
    if path.is_empty() {
        return Err("Empty path".to_string());
    }
    // Block absolute paths
    if Path::new(path).is_absolute() {
        return Err("Absolute paths not allowed".to_string());
    }
    // Block traversal
    if path.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }
    let full = Path::new(workspace).join(path);
    // Canonicalize and verify it's under workspace
    let ws_canon = fs::canonicalize(workspace).unwrap_or_else(|_| Path::new(workspace).to_path_buf());
    if let Ok(full_canon) = fs::canonicalize(&full) {
        if !full_canon.starts_with(&ws_canon) {
            return Err("Path escapes workspace".to_string());
        }
    }
    Ok(full)
}

pub(super) fn tool_list_files(args: &Value, workspace: &str) -> String {
    let dir = args["path"].as_str().unwrap_or(".");
    let recursive = args["recursive"].as_bool().unwrap_or(false);
    let full = match safe_resolve(workspace, dir) {
        Ok(p) => p,
        Err(e) => return format!("Error: {}", e),
    };

    if recursive {
        let output = Command::new("find")
            .arg(&full)
            .args(["-type", "f", "-not", "-path", "*/.git/*"])
            .output();
        match output {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout);
                out.to_string()
            }
            Err(e) => format!("Error: {}", e),
        }
    } else {
        match fs::read_dir(&full) {
            Ok(entries) => {
                let mut files: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        let meta = e.metadata().ok();
                        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            format!("{}/", name)
                        } else {
                            let size = meta.map(|m| m.len()).unwrap_or(0);
                            format!("{} ({} bytes)", name, size)
                        }
                    })
                    .collect();
                files.sort();
                files.join("\n")
            }
            Err(e) => format!("Error listing {}: {}", dir, e),
        }
    }
}

pub(super) fn tool_deep_search(args: &Value, workspace: &str) -> String {
    let query = args["query"].as_str().unwrap_or("");
    let dir = args["path"].as_str().unwrap_or(".");
    let full_dir = Path::new(workspace).join(dir);

    let mut results = String::new();

    // 1. File name matches
    let find_out = Command::new("find")
        .arg(&full_dir)
        .args(["-type", "f", "-not", "-path", "*/.git/*", "-iname", &format!("*{}*", query)])
        .output();
    if let Ok(o) = find_out {
        let files = String::from_utf8_lossy(&o.stdout);
        if !files.trim().is_empty() {
            results.push_str("=== Files matching name ===\n");
            results.push_str(&files);
            results.push('\n');
        }
    }

    // 2. Content matches (grep)
    let grep_out = Command::new("grep")
        .args(["-rn", "-l", "--include=*.*", query])
        .arg(&full_dir)
        .output();
    if let Ok(o) = grep_out {
        let files = String::from_utf8_lossy(&o.stdout);
        if !files.trim().is_empty() {
            results.push_str("=== Files containing pattern ===\n");
            results.push_str(&files);
            results.push('\n');
        }
    }

    // 3. Content snippets (first 5 matches)
    let grep_ctx = Command::new("grep")
        .args(["-rn", "-m", "5", "--include=*.*", query])
        .arg(&full_dir)
        .output();
    if let Ok(o) = grep_ctx {
        let matches = String::from_utf8_lossy(&o.stdout);
        if !matches.trim().is_empty() {
            results.push_str("=== Matching lines (first 5) ===\n");
            results.push_str(&matches);
        }
    }

    if results.is_empty() {
        format!("No results found for '{}'", query)
    } else {
        results
    }
}
