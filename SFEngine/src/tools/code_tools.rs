// Ref: FT-SSF-019
//! Code tools: read, write, edit, search.

use serde_json::Value;
use std::fs;
use std::process::Command;

use super::file_tools::safe_resolve;

pub(super) fn tool_code_write(args: &Value, workspace: &str) -> String {
    let path = args["path"].as_str().unwrap_or("untitled.txt");
    let content = args["content"].as_str().unwrap_or("");

    let full = match safe_resolve(workspace, path) {
        Ok(p) => p,
        Err(e) => return format!("Error: {}", e),
    };

    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).ok();
    }
    match fs::write(&full, content) {
        Ok(_) => format!("Written {} ({} bytes)", path, content.len()),
        Err(e) => format!("Error writing {}: {}", path, e),
    }
}

pub(super) fn tool_code_read(args: &Value, workspace: &str) -> String {
    let path = args["path"].as_str().unwrap_or("");
    let full = match safe_resolve(workspace, path) {
        Ok(p) => p,
        Err(e) => return format!("Error: {}", e),
    };
    match fs::read_to_string(&full) {
        Ok(c) => c,
        Err(e) => format!("Error reading {}: {}", path, e),
    }
}

pub(super) fn tool_code_edit(args: &Value, workspace: &str) -> String {
    let path = args["path"].as_str().unwrap_or("");
    let old_str = args["old_str"].as_str().unwrap_or("");
    let new_str = args["new_str"].as_str().unwrap_or("");

    let full = match safe_resolve(workspace, path) {
        Ok(p) => p,
        Err(e) => return format!("Error: {}", e),
    };
    if old_str.is_empty() {
        return "Error: old_str cannot be empty".to_string();
    }

    match fs::read_to_string(&full) {
        Ok(content) => {
            let count = content.matches(old_str).count();
            if count == 0 {
                return format!("Error: old_str not found in {}", path);
            }
            if count > 1 {
                return format!("Error: old_str found {} times in {} — must be unique", count, path);
            }
            let new_content = content.replacen(old_str, new_str, 1);
            match fs::write(&full, &new_content) {
                Ok(_) => format!("Edited {} — replaced {} bytes with {} bytes", path, old_str.len(), new_str.len()),
                Err(e) => format!("Error writing {}: {}", path, e),
            }
        }
        Err(e) => format!("Error reading {}: {}", path, e),
    }
}

pub(super) async fn tool_code_search(args: &Value, workspace: &str) -> String {
    let query = args["query"].as_str()
        .or_else(|| args["pattern"].as_str())
        .unwrap_or("");
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;

    // If "pattern" is provided (regex), also do grep-style search on indexed chunks
    if let Some(pattern) = args["pattern"].as_str() {
        if args["query"].is_null() {
            let results = crate::indexer::grep_search(pattern, workspace, limit);
            if !results.is_empty() {
                return crate::indexer::format_results(&results);
            }
            return grep_fallback(pattern, workspace);
        }
    }

    // Semantic search (AST + embeddings + FTS5)
    match crate::indexer::search(query, workspace, limit).await {
        Ok(results) if !results.is_empty() => crate::indexer::format_results(&results),
        Ok(_) => grep_fallback(query, workspace),
        Err(e) => {
            eprintln!("[indexer] Search error: {}", e);
            grep_fallback(query, workspace)
        }
    }
}

fn grep_fallback(pattern: &str, workspace: &str) -> String {
    let output = Command::new("grep")
        .args(["-rn", "--include=*.*", pattern])
        .arg(workspace)
        .output();
    match output {
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout);
            if out.is_empty() { "No matches found".to_string() }
            else { out.to_string() }
        }
        Err(e) => format!("Search error: {}", e),
    }
}
