// Ref: FT-SSF-019
//! Shell tools: build, test, lint, and git operations.

use serde_json::Value;

// ── Build / Test / Lint ───────────────────────────────────

pub(super) fn tool_build(args: &Value, workspace: &str) -> String {
    let cmd = args["command"].as_str().unwrap_or("echo 'no command'");
    run_shell_allowlisted(cmd, workspace, 120)
}

pub(super) fn tool_test(args: &Value, workspace: &str) -> String {
    let cmd = args["command"].as_str().unwrap_or("echo 'no test command'");
    run_shell_allowlisted(cmd, workspace, 120)
}

pub(super) fn tool_lint(args: &Value, workspace: &str) -> String {
    let cmd = args["command"].as_str().unwrap_or("echo 'no lint command'");
    run_shell_allowlisted(cmd, workspace, 60)
}

// ── Git Tools ─────────────────────────────────────────────

pub(super) fn tool_git_init(workspace: &str) -> String {
    run_shell("git init && git add -A && git commit -m 'Initial commit'", workspace, 30)
}

pub(super) fn tool_git_commit(args: &Value, workspace: &str) -> String {
    let msg = args["message"].as_str().unwrap_or("update");
    let cmd = format!("git add -A && git commit -m '{}'", msg.replace('\'', "'\\''"));
    run_shell(&cmd, workspace, 30)
}

pub(super) fn tool_git_status(workspace: &str) -> String {
    run_shell("git status --short 2>/dev/null || echo 'Not a git repository'", workspace, 10)
}

pub(super) fn tool_git_log(args: &Value, workspace: &str) -> String {
    let limit = args["limit"].as_u64().unwrap_or(10);
    let cmd = format!("git log --oneline -n {} 2>/dev/null || echo 'No git history'", limit);
    run_shell(&cmd, workspace, 10)
}

pub(super) fn tool_git_diff(args: &Value, workspace: &str) -> String {
    let path = args["path"].as_str().unwrap_or("");
    let cmd = if path.is_empty() {
        "git diff --stat 2>/dev/null && echo '---' && git diff 2>/dev/null || echo 'No changes'".to_string()
    } else {
        format!("git diff -- '{}' 2>/dev/null || echo 'No changes'", path)
    };
    run_shell(&cmd, workspace, 10)
}

pub(super) fn tool_git_push(args: &Value, workspace: &str) -> String {
    let branch = args["branch"].as_str().unwrap_or("");
    let cmd = if branch.is_empty() {
        "git push 2>&1".to_string()
    } else {
        format!("git push -u origin '{}' 2>&1", branch)
    };
    run_shell(&cmd, workspace, 30)
}

pub(super) fn tool_git_create_branch(args: &Value, workspace: &str) -> String {
    let branch = args["branch"].as_str().unwrap_or("feature");
    let cmd = format!("git checkout -b '{}' 2>&1", branch);
    run_shell(&cmd, workspace, 10)
}

// ── Shell Runner — delegated to sandbox module ────────────

fn run_shell(cmd: &str, workspace: &str, timeout_secs: u64) -> String {
    run_shell_sandboxed(cmd, workspace, timeout_secs, false)
}

fn run_shell_allowlisted(cmd: &str, workspace: &str, timeout_secs: u64) -> String {
    run_shell_sandboxed(cmd, workspace, timeout_secs, true)
}

fn run_shell_sandboxed(cmd: &str, workspace: &str, timeout_secs: u64, require_allowlist: bool) -> String {
    match crate::sandbox::sandboxed_exec(cmd, workspace, timeout_secs, require_allowlist) {
        Ok(output) => output,
        Err(e) => e,
    }
}
