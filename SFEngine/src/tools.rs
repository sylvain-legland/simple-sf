use serde_json::{json, Value};
use std::process::Command;
use std::path::Path;
use std::fs;

/// Execute a tool call. Returns the result as a string.
pub fn execute_tool(name: &str, args: &Value, workspace: &str) -> String {
    match name {
        "code_write" => tool_code_write(args, workspace),
        "code_read" => tool_code_read(args, workspace),
        "code_search" => tool_code_search(args, workspace),
        "list_files" => tool_list_files(args, workspace),
        "build" => tool_build(args, workspace),
        "test" => tool_test(args, workspace),
        "git_init" => tool_git_init(workspace),
        "git_commit" => tool_git_commit(args, workspace),
        _ => format!("Unknown tool: {}", name),
    }
}

/// Tool schemas for LLM function calling
pub fn tool_schemas_for_role(role: &str) -> Vec<Value> {
    let mut tools = vec![
        json!({
            "type": "function",
            "function": {
                "name": "code_write",
                "description": "Write content to a file (creates dirs if needed)",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Relative file path"},
                        "content": {"type": "string", "description": "File content"}
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "code_read",
                "description": "Read a file's content",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Relative file path"}
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "code_search",
                "description": "Search for a pattern in the workspace (grep)",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "Search pattern (regex)"},
                        "path": {"type": "string", "description": "Directory to search in (default: workspace root)"}
                    },
                    "required": ["pattern"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files in a directory",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Directory path (default: workspace root)"}
                    }
                }
            }
        }),
    ];

    match role {
        "developer" | "lead_dev" => {
            tools.push(json!({
                "type": "function",
                "function": {
                    "name": "build",
                    "description": "Run a build command in the workspace",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": {"type": "string", "description": "Build command to run (e.g. 'npm run build', 'python -m py_compile app.py')"}
                        },
                        "required": ["command"]
                    }
                }
            }));
            tools.push(json!({
                "type": "function",
                "function": {
                    "name": "git_init",
                    "description": "Initialize a git repo in the workspace",
                    "parameters": { "type": "object", "properties": {} }
                }
            }));
            tools.push(json!({
                "type": "function",
                "function": {
                    "name": "git_commit",
                    "description": "Stage all and commit",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "message": {"type": "string", "description": "Commit message"}
                        },
                        "required": ["message"]
                    }
                }
            }));
        }
        "qa" => {
            tools.push(json!({
                "type": "function",
                "function": {
                    "name": "test",
                    "description": "Run a test command",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": {"type": "string", "description": "Test command (e.g. 'npm test', 'pytest')"}
                        },
                        "required": ["command"]
                    }
                }
            }));
            tools.push(json!({
                "type": "function",
                "function": {
                    "name": "build",
                    "description": "Run a build command",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": {"type": "string", "description": "Build command"}
                        },
                        "required": ["command"]
                    }
                }
            }));
        }
        _ => {}
    }

    tools
}

fn tool_code_write(args: &Value, workspace: &str) -> String {
    let path = args["path"].as_str().unwrap_or("untitled.txt");
    let content = args["content"].as_str().unwrap_or("");
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
    let full = Path::new(workspace).join(path);
    match fs::read_to_string(&full) {
        Ok(c) => {
            if c.len() > 8000 {
                format!("{}\n... (truncated, {} bytes total)", &c[..8000], c.len())
            } else { c }
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
            else if out.len() > 4000 { format!("{}\n... (truncated)", &out[..4000]) }
            else { out.to_string() }
        }
        Err(e) => format!("Search error: {}", e),
    }
}

fn tool_list_files(args: &Value, workspace: &str) -> String {
    let dir = args["path"].as_str().unwrap_or(".");
    let full = Path::new(workspace).join(dir);
    match fs::read_dir(&full) {
        Ok(entries) => {
            let mut files: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        format!("{}/", name)
                    } else { name }
                })
                .collect();
            files.sort();
            files.join("\n")
        }
        Err(e) => format!("Error listing {}: {}", dir, e),
    }
}

fn tool_build(args: &Value, workspace: &str) -> String {
    let cmd = args["command"].as_str().unwrap_or("echo 'no command'");
    run_shell(cmd, workspace, 120)
}

fn tool_test(args: &Value, workspace: &str) -> String {
    let cmd = args["command"].as_str().unwrap_or("echo 'no test command'");
    run_shell(cmd, workspace, 120)
}

fn tool_git_init(workspace: &str) -> String {
    run_shell("git init && git add -A && git commit -m 'Initial commit'", workspace, 30)
}

fn tool_git_commit(args: &Value, workspace: &str) -> String {
    let msg = args["message"].as_str().unwrap_or("update");
    let cmd = format!("git add -A && git commit -m '{}'", msg.replace('\'', "'\\''"));
    run_shell(&cmd, workspace, 30)
}

fn run_shell(cmd: &str, workspace: &str, timeout_secs: u64) -> String {
    let output = Command::new("sh")
        .args(["-c", cmd])
        .current_dir(workspace)
        .output();
    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            let mut result = String::new();
            if !stdout.is_empty() { result.push_str(&stdout); }
            if !stderr.is_empty() {
                if !result.is_empty() { result.push('\n'); }
                result.push_str(&stderr);
            }
            if result.len() > 4000 {
                format!("{}\n... (truncated)", &result[..4000])
            } else if result.is_empty() {
                format!("Command completed (exit {})", o.status.code().unwrap_or(-1))
            } else { result }
        }
        Err(e) => format!("Failed to run command: {}", e),
    }
}
