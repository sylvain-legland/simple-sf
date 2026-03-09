use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Command;
use std::path::Path;
use std::fs;
use std::time::Duration;

// ══════════════════════════════════════════════════════════════
// TOOL REGISTRY — 18 tools ported from the SF platform
// ══════════════════════════════════════════════════════════════

/// Execute a tool call. Returns the result as a string.
pub fn execute_tool(name: &str, args: &Value, workspace: &str) -> String {
    // Extract project_id from workspace path (last segment)
    let project_id = std::path::Path::new(workspace)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_string();

    match name {
        // Code tools
        "code_write"  => tool_code_write(args, workspace),
        "code_read"   => tool_code_read(args, workspace),
        "code_edit"   => tool_code_edit(args, workspace),
        "code_search" => tool_code_search(args, workspace),
        "list_files"  => tool_list_files(args, workspace),
        "deep_search" => tool_deep_search(args, workspace),
        // Build tools
        "build" => tool_build(args, workspace),
        "test"  => tool_test(args, workspace),
        "lint"  => tool_lint(args, workspace),
        // Git tools
        "git_init"          => tool_git_init(workspace),
        "git_commit"        => tool_git_commit(args, workspace),
        "git_status"        => tool_git_status(workspace),
        "git_log"           => tool_git_log(args, workspace),
        "git_diff"          => tool_git_diff(args, workspace),
        "git_push"          => tool_git_push(args, workspace),
        "git_create_branch" => tool_git_create_branch(args, workspace),
        // Memory tools (project-scoped)
        "memory_search" => tool_memory_search(args, &project_id),
        "memory_store"  => tool_memory_store(args, &project_id),
        _ => format!("Unknown tool: {}", name),
    }
}

// ══════════════════════════════════════════════════════════════
// ROLE → TOOL MAP (ported from platform/agents/tool_schemas/_mapping.py)
// ══════════════════════════════════════════════════════════════

/// All tool schema definitions — the full catalog
fn all_tool_schemas() -> HashMap<&'static str, Value> {
    let mut m = HashMap::new();

    m.insert("code_write", json!({
        "type": "function",
        "function": {
            "name": "code_write",
            "description": "Write content to a file. Creates parent directories if needed.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Relative file path from workspace root"},
                    "content": {"type": "string", "description": "Complete file content to write"}
                },
                "required": ["path", "content"]
            }
        }
    }));
    m.insert("code_read", json!({
        "type": "function",
        "function": {
            "name": "code_read",
            "description": "Read a file's complete content.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Relative file path"}
                },
                "required": ["path"]
            }
        }
    }));
    m.insert("code_edit", json!({
        "type": "function",
        "function": {
            "name": "code_edit",
            "description": "Find and replace text in a file. Replaces the first occurrence of old_str with new_str.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Relative file path"},
                    "old_str": {"type": "string", "description": "Exact text to find (must match exactly)"},
                    "new_str": {"type": "string", "description": "Replacement text"}
                },
                "required": ["path", "old_str", "new_str"]
            }
        }
    }));
    m.insert("code_search", json!({
        "type": "function",
        "function": {
            "name": "code_search",
            "description": "Search for a regex pattern in the workspace using grep.",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Search pattern (regex)"},
                    "path": {"type": "string", "description": "Directory to search in (default: workspace root)"}
                },
                "required": ["pattern"]
            }
        }
    }));
    m.insert("list_files", json!({
        "type": "function",
        "function": {
            "name": "list_files",
            "description": "List files and directories. Shows type (file/dir) and size.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory path (default: workspace root)"},
                    "recursive": {"type": "boolean", "description": "List recursively (default: false)"}
                }
            }
        }
    }));
    m.insert("deep_search", json!({
        "type": "function",
        "function": {
            "name": "deep_search",
            "description": "Recursively search files and their contents for a query. Returns file paths and matching lines. Use for understanding project structure and finding relevant code.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "What to search for (file names, content patterns, concepts)"},
                    "path": {"type": "string", "description": "Starting directory (default: workspace root)"}
                },
                "required": ["query"]
            }
        }
    }));
    m.insert("build", json!({
        "type": "function",
        "function": {
            "name": "build",
            "description": "Run a build command in the workspace. Use for compiling, bundling, etc.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Build command (e.g. 'npm run build', 'python -m py_compile app.py', 'cargo build')"}
                },
                "required": ["command"]
            }
        }
    }));
    m.insert("test", json!({
        "type": "function",
        "function": {
            "name": "test",
            "description": "Run a test command. Returns stdout/stderr output.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Test command (e.g. 'npm test', 'pytest', 'cargo test')"}
                },
                "required": ["command"]
            }
        }
    }));
    m.insert("lint", json!({
        "type": "function",
        "function": {
            "name": "lint",
            "description": "Run a linter or code quality checker.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Lint command (e.g. 'npx eslint .', 'flake8', 'cargo clippy')"}
                },
                "required": ["command"]
            }
        }
    }));
    m.insert("git_init", json!({
        "type": "function",
        "function": {
            "name": "git_init",
            "description": "Initialize a git repository, stage all files and create initial commit.",
            "parameters": { "type": "object", "properties": {} }
        }
    }));
    m.insert("git_commit", json!({
        "type": "function",
        "function": {
            "name": "git_commit",
            "description": "Stage all changes and create a commit.",
            "parameters": {
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "Commit message (use conventional commits: feat:, fix:, etc.)"}
                },
                "required": ["message"]
            }
        }
    }));
    m.insert("git_status", json!({
        "type": "function",
        "function": {
            "name": "git_status",
            "description": "Show git status: staged, unstaged, and untracked files.",
            "parameters": { "type": "object", "properties": {} }
        }
    }));
    m.insert("git_log", json!({
        "type": "function",
        "function": {
            "name": "git_log",
            "description": "Show recent git commit history.",
            "parameters": {
                "type": "object",
                "properties": {
                    "limit": {"type": "integer", "description": "Number of commits to show (default: 10)"}
                }
            }
        }
    }));
    m.insert("git_diff", json!({
        "type": "function",
        "function": {
            "name": "git_diff",
            "description": "Show git diff of current changes.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Specific file to diff (default: all changes)"}
                }
            }
        }
    }));
    m.insert("git_push", json!({
        "type": "function",
        "function": {
            "name": "git_push",
            "description": "Push commits to the remote repository.",
            "parameters": {
                "type": "object",
                "properties": {
                    "branch": {"type": "string", "description": "Branch name (default: current branch)"}
                }
            }
        }
    }));
    m.insert("git_create_branch", json!({
        "type": "function",
        "function": {
            "name": "git_create_branch",
            "description": "Create and checkout a new git branch.",
            "parameters": {
                "type": "object",
                "properties": {
                    "branch": {"type": "string", "description": "New branch name"}
                },
                "required": ["branch"]
            }
        }
    }));
    m.insert("memory_search", json!({
        "type": "function",
        "function": {
            "name": "memory_search",
            "description": "Search project memory and context. Returns relevant past decisions, findings, and notes.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "What to search for in memory"},
                    "scope": {"type": "string", "description": "Memory scope: 'project', 'global', or 'all' (default: 'all')"}
                },
                "required": ["query"]
            }
        }
    }));
    m.insert("memory_store", json!({
        "type": "function",
        "function": {
            "name": "memory_store",
            "description": "Store a key-value pair in memory. Use for decisions, findings, conventions. Upserts: same key updates existing entry.",
            "parameters": {
                "type": "object",
                "properties": {
                    "key": {"type": "string", "description": "Memory key (e.g. 'architecture-decision', 'tech-stack', 'api-design')"},
                    "value": {"type": "string", "description": "Content to store"},
                    "category": {"type": "string", "description": "Category: 'decision', 'finding', 'convention', 'context' (default: 'note')"},
                    "scope": {"type": "string", "description": "Scope: 'project' (default) or 'global'"}
                },
                "required": ["key", "value"]
            }
        }
    }));

    m
}

/// ROLE_TOOL_MAP — which tools each role gets access to
const ROLE_TOOLS: &[(&str, &[&str])] = &[
    ("rte",            &["code_read", "list_files", "memory_search", "deep_search", "git_status", "git_log"]),
    ("product_owner",  &["code_read", "code_search", "list_files", "memory_search", "memory_store", "deep_search"]),
    ("scrum_master",   &["code_read", "list_files", "memory_search", "git_status", "git_log"]),
    ("architect",      &["code_read", "code_search", "list_files", "deep_search", "memory_search", "memory_store", "git_status", "git_log", "git_diff"]),
    ("lead_dev",       &["code_read", "code_write", "code_edit", "code_search", "list_files", "deep_search", "build", "test", "lint", "git_status", "git_log", "git_diff", "git_commit", "memory_search", "memory_store"]),
    ("lead_frontend",  &["code_read", "code_write", "code_edit", "code_search", "list_files", "deep_search", "build", "test", "lint", "git_status", "git_log", "git_diff", "git_commit", "memory_search"]),
    ("lead_backend",   &["code_read", "code_write", "code_edit", "code_search", "list_files", "deep_search", "build", "test", "lint", "git_status", "git_log", "git_diff", "git_commit", "memory_search"]),
    ("developer",      &["code_read", "code_write", "code_edit", "code_search", "list_files", "deep_search", "build", "test", "lint", "git_init", "git_commit", "git_status"]),
    ("qa",             &["code_read", "code_search", "list_files", "build", "test", "lint", "git_status", "git_log", "git_diff", "deep_search"]),
    ("qa_lead",        &["code_read", "code_search", "list_files", "build", "test", "lint", "git_status", "git_log", "git_diff", "deep_search", "memory_search"]),
    ("devops",         &["code_read", "code_write", "code_edit", "code_search", "list_files", "build", "test", "git_status", "git_log", "git_diff", "git_commit", "git_push", "git_create_branch"]),
    ("security",       &["code_read", "code_search", "list_files", "deep_search", "git_status", "git_log", "git_diff"]),
    ("ux_designer",    &["code_read", "code_search", "list_files", "memory_search"]),
    ("data_engineer",  &["code_read", "code_write", "code_edit", "code_search", "list_files", "build", "test", "git_commit", "git_status"]),
    ("tech_writer",    &["code_read", "code_write", "code_search", "list_files", "git_status", "git_diff"]),
    ("cloud_architect",&["code_read", "code_write", "code_edit", "code_search", "list_files", "build", "git_commit", "git_status"]),
];

/// Get tool schemas for a given agent role + any extra tools from the agent definition.
pub fn tool_schemas_for_role(role: &str) -> Vec<Value> {
    tool_schemas_for_role_with_extras(role, &[])
}

/// Get tool schemas for a role, plus additional tool names from the agent definition.
pub fn tool_schemas_for_role_with_extras(role: &str, extra_tools: &[&str]) -> Vec<Value> {
    let all = all_tool_schemas();

    // Find the role's tool list (fallback to developer set)
    let role_tools = ROLE_TOOLS.iter()
        .find(|(r, _)| *r == role)
        .map(|(_, tools)| *tools)
        .unwrap_or(&["code_read", "code_search", "list_files"]);

    let mut seen = std::collections::HashSet::new();
    let mut schemas = Vec::new();

    // Add role tools
    for tool_name in role_tools {
        if seen.insert(*tool_name) {
            if let Some(schema) = all.get(tool_name) {
                schemas.push(schema.clone());
            }
        }
    }

    // Add extra tools from agent definition
    for tool_name in extra_tools {
        if seen.insert(*tool_name) {
            if let Some(schema) = all.get(tool_name) {
                schemas.push(schema.clone());
            }
        }
    }

    schemas
}

// ══════════════════════════════════════════════════════════════
// TOOL IMPLEMENTATIONS
// ══════════════════════════════════════════════════════════════

// ── Code Tools ─────────────────────────────────────────────

fn tool_code_write(args: &Value, workspace: &str) -> String {
    let path = args["path"].as_str().unwrap_or("untitled.txt");
    let content = args["content"].as_str().unwrap_or("");

    // Security: block writes outside workspace
    if path.contains("..") {
        return "Error: path traversal not allowed".to_string();
    }

    let full = Path::new(workspace).join(path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).ok();
    }
    match fs::write(&full, content) {
        Ok(_) => format!("Written {} ({} bytes)", path, content.len()),
        Err(e) => format!("Error writing {}: {}", path, e),
    }
}

fn tool_code_read(args: &Value, workspace: &str) -> String {
    let path = args["path"].as_str().unwrap_or("");
    if path.contains("..") {
        return "Error: path traversal not allowed".to_string();
    }
    let full = Path::new(workspace).join(path);
    match fs::read_to_string(&full) {
        Ok(c) => c,
        Err(e) => format!("Error reading {}: {}", path, e),
    }
}

fn tool_code_edit(args: &Value, workspace: &str) -> String {
    let path = args["path"].as_str().unwrap_or("");
    let old_str = args["old_str"].as_str().unwrap_or("");
    let new_str = args["new_str"].as_str().unwrap_or("");

    if path.contains("..") {
        return "Error: path traversal not allowed".to_string();
    }
    if old_str.is_empty() {
        return "Error: old_str cannot be empty".to_string();
    }

    let full = Path::new(workspace).join(path);
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

fn tool_code_search(args: &Value, workspace: &str) -> String {
    let pattern = args["pattern"].as_str().unwrap_or("");
    let dir = args["path"].as_str().unwrap_or(".");
    let full_dir = Path::new(workspace).join(dir);
    let output = Command::new("grep")
        .args(["-rn", "--include=*.*", pattern])
        .arg(&full_dir)
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

fn tool_list_files(args: &Value, workspace: &str) -> String {
    let dir = args["path"].as_str().unwrap_or(".");
    let recursive = args["recursive"].as_bool().unwrap_or(false);
    let full = Path::new(workspace).join(dir);

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

fn tool_deep_search(args: &Value, workspace: &str) -> String {
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

// ── Build Tools ────────────────────────────────────────────

fn tool_build(args: &Value, workspace: &str) -> String {
    let cmd = args["command"].as_str().unwrap_or("echo 'no command'");
    run_shell(cmd, workspace, 120)
}

fn tool_test(args: &Value, workspace: &str) -> String {
    let cmd = args["command"].as_str().unwrap_or("echo 'no test command'");
    run_shell(cmd, workspace, 120)
}

fn tool_lint(args: &Value, workspace: &str) -> String {
    let cmd = args["command"].as_str().unwrap_or("echo 'no lint command'");
    run_shell(cmd, workspace, 60)
}

// ── Git Tools ──────────────────────────────────────────────

fn tool_git_init(workspace: &str) -> String {
    run_shell("git init && git add -A && git commit -m 'Initial commit'", workspace, 30)
}

fn tool_git_commit(args: &Value, workspace: &str) -> String {
    let msg = args["message"].as_str().unwrap_or("update");
    let cmd = format!("git add -A && git commit -m '{}'", msg.replace('\'', "'\\''"));
    run_shell(&cmd, workspace, 30)
}

fn tool_git_status(workspace: &str) -> String {
    run_shell("git status --short 2>/dev/null || echo 'Not a git repository'", workspace, 10)
}

fn tool_git_log(args: &Value, workspace: &str) -> String {
    let limit = args["limit"].as_u64().unwrap_or(10);
    let cmd = format!("git log --oneline -n {} 2>/dev/null || echo 'No git history'", limit);
    run_shell(&cmd, workspace, 10)
}

fn tool_git_diff(args: &Value, workspace: &str) -> String {
    let path = args["path"].as_str().unwrap_or("");
    let cmd = if path.is_empty() {
        "git diff --stat 2>/dev/null && echo '---' && git diff 2>/dev/null || echo 'No changes'".to_string()
    } else {
        format!("git diff -- '{}' 2>/dev/null || echo 'No changes'", path)
    };
    run_shell(&cmd, workspace, 10)
}

fn tool_git_push(args: &Value, workspace: &str) -> String {
    let branch = args["branch"].as_str().unwrap_or("");
    let cmd = if branch.is_empty() {
        "git push 2>&1".to_string()
    } else {
        format!("git push -u origin '{}' 2>&1", branch)
    };
    run_shell(&cmd, workspace, 30)
}

fn tool_git_create_branch(args: &Value, workspace: &str) -> String {
    let branch = args["branch"].as_str().unwrap_or("feature");
    let cmd = format!("git checkout -b '{}' 2>&1", branch);
    run_shell(&cmd, workspace, 10)
}

// ── Memory Tools — persisted to SQLite (#4) ───────────────

fn tool_memory_search(args: &Value, project_id: &str) -> String {
    let query = args["query"].as_str().unwrap_or("").to_lowercase();
    let scope = args["scope"].as_str().unwrap_or("project");

    crate::db::with_db(|conn| {
        let pattern = format!("%{}%", query);
        let sql = match scope {
            "global" => "SELECT key, value, category, project_id FROM memory WHERE \
                         (project_id IS NULL OR project_id = '') AND \
                         (LOWER(key) LIKE ?1 OR LOWER(value) LIKE ?1 OR LOWER(category) LIKE ?1) \
                         ORDER BY created_at DESC LIMIT 20".to_string(),
            "all" => "SELECT key, value, category, project_id FROM memory WHERE \
                      (LOWER(key) LIKE ?1 OR LOWER(value) LIKE ?1 OR LOWER(category) LIKE ?1) \
                      ORDER BY created_at DESC LIMIT 20".to_string(),
            _ => format!("SELECT key, value, category, project_id FROM memory WHERE \
                          (project_id = '{}' OR project_id IS NULL) AND \
                          (LOWER(key) LIKE ?1 OR LOWER(value) LIKE ?1 OR LOWER(category) LIKE ?1) \
                          ORDER BY created_at DESC LIMIT 20", project_id),
        };

        let mut stmt = conn.prepare(&sql).unwrap();
        let results: Vec<String> = stmt.query_map(
            rusqlite::params![pattern],
            |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                let category: String = row.get(2)?;
                let pid: Option<String> = row.get(3)?;
                let scope_tag = if pid.as_ref().map(|s| s.is_empty()).unwrap_or(true) { "global" } else { "project" };
                Ok(format!("[{}/{}] {}: {}", scope_tag, category, key, value))
            },
        ).unwrap().filter_map(|r| r.ok()).collect();

        if results.is_empty() {
            format!("No memory found for '{}'", query)
        } else {
            results.join("\n\n")
        }
    })
}

fn tool_memory_store(args: &Value, project_id: &str) -> String {
    let key = args["key"].as_str().unwrap_or("").to_string();
    let value = args["value"].as_str().unwrap_or("").to_string();
    let category = args["category"].as_str().unwrap_or("note").to_string();
    let scope = args["scope"].as_str().unwrap_or("project");

    if key.is_empty() || value.is_empty() {
        return "Error: key and value are required".to_string();
    }

    let pid = if scope == "global" { None } else { Some(project_id.to_string()) };

    crate::db::with_db(|conn| {
        // Upsert: if same key+project_id exists, update value
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

// ══════════════════════════════════════════════════════════════
// MEMORY SYSTEM — project-scoped, auto-injected, compacted
// ══════════════════════════════════════════════════════════════

/// Load project memory for injection into system prompt (max 4K chars).
/// Returns formatted memory context or empty string if none.
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
/// Files checked (in priority order): CLAUDE.md, SPECS.md, README.md, CONVENTIONS.md
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
                // Upsert project file
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

// ══════════════════════════════════════════════════════════════
// SHELL RUNNER
// ══════════════════════════════════════════════════════════════

/// Blocked commands (security)
const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf /", "rm -rf /*", "mkfs", "dd if=", ":(){", "fork bomb",
    "curl | sh", "wget | sh", "chmod 777",
];

fn run_shell(cmd: &str, workspace: &str, timeout_secs: u64) -> String {
    // Security check
    let cmd_lower = cmd.to_lowercase();
    for blocked in BLOCKED_PATTERNS {
        if cmd_lower.contains(blocked) {
            return format!("BLOCKED: dangerous command pattern detected ({})", blocked);
        }
    }

    let mut child = match Command::new("sh")
        .args(["-c", cmd])
        .current_dir(workspace)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn() {
        Ok(c) => c,
        Err(e) => return format!("Failed to run command: {}", e),
    };

    // Real timeout enforcement (#11)
    let timeout = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return format!("Command timed out after {}s: {}", timeout_secs, cmd);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return format!("Error waiting for command: {}", e),
        }
    }

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return format!("Failed to read output: {}", e),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut result = String::new();
    if !stdout.is_empty() { result.push_str(&stdout); }
    if !stderr.is_empty() {
        if !result.is_empty() { result.push('\n'); }
        result.push_str(&stderr);
    }
    if result.is_empty() {
        format!("Command completed (exit {})", output.status.code().unwrap_or(-1))
    } else { result }
}
