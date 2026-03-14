// Ref: FT-SSF-019
//! Tool registry: dispatcher, schemas, and role mapping.
//!
//! Submodules contain the actual implementations:
//!   - `code_tools`   — code_write, code_read, code_edit, code_search
//!   - `file_tools`   — safe_resolve, list_files, deep_search
//!   - `shell_tools`  — build, test, lint, git operations
//!   - `memory_tools` — memory_search, memory_store, project memory
//!   - `schemas`      — JSON schema definitions for the LLM interface

mod code_tools;
mod file_tools;
mod shell_tools;
mod memory_tools;
mod schemas;

use serde_json::Value;

pub use memory_tools::{load_project_memory, load_project_files, compact_memory};

// ══════════════════════════════════════════════════════════════
// DISPATCHER
// ══════════════════════════════════════════════════════════════

/// Execute a tool call. Returns the result as a string.
pub async fn execute_tool(name: &str, args: &Value, workspace: &str) -> String {
    let project_id = std::path::Path::new(workspace)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_string();

    match name {
        // Code tools
        "code_write"  => code_tools::tool_code_write(args, workspace),
        "code_read"   => code_tools::tool_code_read(args, workspace),
        "code_edit"   => code_tools::tool_code_edit(args, workspace),
        "code_search" => code_tools::tool_code_search(args, workspace).await,
        "list_files"  => file_tools::tool_list_files(args, workspace),
        "deep_search" => file_tools::tool_deep_search(args, workspace),
        // Build tools
        "build" => shell_tools::tool_build(args, workspace),
        "test"  => shell_tools::tool_test(args, workspace),
        "lint"  => shell_tools::tool_lint(args, workspace),
        // Git tools
        "git_init"          => shell_tools::tool_git_init(workspace),
        "git_commit"        => shell_tools::tool_git_commit(args, workspace),
        "git_status"        => shell_tools::tool_git_status(workspace),
        "git_log"           => shell_tools::tool_git_log(args, workspace),
        "git_diff"          => shell_tools::tool_git_diff(args, workspace),
        "git_push"          => shell_tools::tool_git_push(args, workspace),
        "git_create_branch" => shell_tools::tool_git_create_branch(args, workspace),
        // Memory tools (project-scoped)
        "memory_search" => memory_tools::tool_memory_search(args, &project_id),
        "memory_store"  => memory_tools::tool_memory_store(args, &project_id),
        _ => format!("Unknown tool: {}", name),
    }
}

// ══════════════════════════════════════════════════════════════
// ROLE → TOOL MAP
// ══════════════════════════════════════════════════════════════

/// ROLE_TOOL_MAP — which tools each role gets access to
const ROLE_TOOLS: &[(&str, &[&str])] = &[
    ("rte",            &["code_read", "list_files", "memory_search", "deep_search", "git_status", "git_log", "build"]),
    ("product_owner",  &["code_read", "code_search", "list_files", "memory_search", "memory_store", "deep_search", "build", "test"]),
    ("scrum_master",   &["code_read", "list_files", "memory_search", "memory_store", "git_status", "git_log"]),
    ("architect",      &["code_read", "code_search", "list_files", "deep_search", "memory_search", "memory_store", "git_status", "git_log", "git_diff"]),
    ("lead_dev",       &["code_read", "code_write", "code_edit", "code_search", "list_files", "deep_search", "build", "test", "lint", "git_status", "git_log", "git_diff", "git_commit", "memory_search", "memory_store"]),
    ("lead_frontend",  &["code_read", "code_write", "code_edit", "code_search", "list_files", "deep_search", "build", "test", "lint", "git_status", "git_log", "git_diff", "git_commit", "memory_search", "memory_store"]),
    ("lead_backend",   &["code_read", "code_write", "code_edit", "code_search", "list_files", "deep_search", "build", "test", "lint", "git_status", "git_log", "git_diff", "git_commit", "memory_search", "memory_store"]),
    ("developer",      &["code_read", "code_write", "code_edit", "code_search", "list_files", "deep_search", "build", "test", "lint", "git_init", "git_commit", "git_status", "memory_search", "memory_store"]),
    ("qa",             &["code_read", "code_search", "list_files", "build", "test", "lint", "git_status", "git_log", "git_diff", "deep_search"]),
    ("qa_lead",        &["code_read", "code_search", "list_files", "build", "test", "lint", "git_status", "git_log", "git_diff", "deep_search", "memory_search", "memory_store"]),
    ("devops",         &["code_read", "code_write", "code_edit", "code_search", "list_files", "build", "test", "git_status", "git_log", "git_diff", "git_commit", "git_push", "git_create_branch", "memory_search", "memory_store"]),
    ("security",       &["code_read", "code_search", "list_files", "deep_search", "git_status", "git_log", "git_diff", "memory_search", "memory_store"]),
    ("ux_designer",    &["code_read", "code_search", "list_files", "memory_search", "memory_store"]),
    ("data_engineer",  &["code_read", "code_write", "code_edit", "code_search", "list_files", "build", "test", "git_commit", "git_status", "memory_search", "memory_store"]),
    ("tech_writer",    &["code_read", "code_write", "code_search", "list_files", "git_status", "git_diff", "memory_search", "memory_store"]),
    ("cloud_architect",&["code_read", "code_write", "code_edit", "code_search", "list_files", "build", "git_commit", "git_status", "memory_search", "memory_store"]),
];

/// Normalize free-form role strings (from catalog) to ROLE_TOOLS keys.
pub fn normalize_role(role: &str) -> &'static str {
    let lower = role.to_lowercase();

    for (key, _) in ROLE_TOOLS {
        if lower == *key { return key; }
    }

    if lower.contains("scrum master") { return "scrum_master"; }
    if lower.contains("product owner") || lower.contains("product manager") || lower == "po" { return "product_owner"; }
    if lower.contains("rte") || lower.contains("release train") { return "rte"; }
    if lower.contains("qa") || lower.contains("test") || lower.contains("quality") { return "qa_lead"; }
    if lower.contains("lead") && lower.contains("front") { return "lead_frontend"; }
    if lower.contains("lead") && (lower.contains("back") || lower.contains("dév") || lower.contains("dev")) { return "lead_backend"; }
    if lower.contains("lead dev") || lower.contains("lead dév") || lower.contains("tech lead") { return "lead_dev"; }
    if lower.contains("devops") || lower.contains("sre") || lower.contains("pipeline") { return "devops"; }
    if lower.contains("security") || lower.contains("ciso") || lower.contains("pentest") || lower.contains("secops") { return "security"; }
    if lower.contains("ux") || lower.contains("design") { return "ux_designer"; }
    if lower.contains("data engineer") { return "data_engineer"; }
    if lower.contains("cloud") || lower.contains("architect") { return "cloud_architect"; }
    if lower.contains("tech writ") || lower.contains("documentation") { return "tech_writer"; }
    if lower.contains("front") { return "lead_frontend"; }
    if lower.contains("back") { return "lead_backend"; }
    if lower.contains("develop") || lower.contains("dével") || lower.contains("programmer") { return "developer"; }

    "developer"
}

/// Get tool schemas for a given agent role.
pub fn tool_schemas_for_role(role: &str) -> Vec<Value> {
    tool_schemas_for_role_with_extras(role, &[])
}

/// Get tool schemas for a role, plus additional tool names from the agent definition.
pub fn tool_schemas_for_role_with_extras(role: &str, extra_tools: &[&str]) -> Vec<Value> {
    let all = schemas::all_tool_schemas();
    let normalized = normalize_role(role);

    let role_tools = ROLE_TOOLS.iter()
        .find(|(r, _)| *r == normalized)
        .map(|(_, tools)| *tools)
        .unwrap_or(&["code_read", "code_write", "code_edit", "code_search", "list_files", "build", "test"]);

    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for tool_name in role_tools {
        if seen.insert(*tool_name) {
            if let Some(schema) = all.get(tool_name) {
                result.push(schema.clone());
            }
        }
    }

    for tool_name in extra_tools {
        if seen.insert(*tool_name) {
            if let Some(schema) = all.get(tool_name) {
                result.push(schema.clone());
            }
        }
    }

    result
}
