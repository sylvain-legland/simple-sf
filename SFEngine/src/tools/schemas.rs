// Ref: FT-SSF-019
//! Tool JSON schema definitions for the LLM function-calling interface.

use serde_json::{json, Value};
use std::collections::HashMap;

/// All tool schema definitions — the full catalog.
pub(super) fn all_tool_schemas() -> HashMap<&'static str, Value> {
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
            "description": "Semantic code search: finds relevant functions, classes, and code blocks by meaning (AST-indexed). Also supports regex patterns.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Natural language query or code snippet to search for"},
                    "pattern": {"type": "string", "description": "Regex pattern for exact text matching (optional)"},
                    "limit": {"type": "integer", "description": "Maximum number of results (default: 10)"}
                },
                "required": ["query"]
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
            "description": "Recursively search files and their contents for a query. Returns file paths and matching lines.",
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
            "description": "Run a build command in the workspace.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Build command (e.g. 'npm run build', 'cargo build')"}
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
                    "command": {"type": "string", "description": "Lint command (e.g. 'npx eslint .', 'cargo clippy')"}
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
                    "message": {"type": "string", "description": "Commit message (use conventional commits)"}
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
            "description": "Store a key-value pair in memory. Upserts: same key updates existing entry.",
            "parameters": {
                "type": "object",
                "properties": {
                    "key": {"type": "string", "description": "Memory key (e.g. 'architecture-decision', 'tech-stack')"},
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
