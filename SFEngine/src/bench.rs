//! AC/LLM Bench — Acceptance criteria testing for the SF engine.
//! No UI. Runs silently, returns JSON results via FFI.
//! Verifies: LLM connectivity, tool calling, guard, network pattern.

use crate::llm::{self, LLMMessage};
use crate::tools;
use crate::guard;
use crate::catalog;
use serde_json::{json, Value};

#[derive(Debug)]
pub struct BenchResult {
    pub case_id: String,
    pub name: String,
    pub passed: bool,
    pub details: String,
}

/// Run all acceptance criteria tests. Returns JSON array of results.
pub async fn run_all() -> String {
    let mut results: Vec<Value> = Vec::new();

    // Quick check: is LLM configured?
    if llm::get_config().is_none() {
        return json!([{
            "case_id": "ac-0",
            "name": "LLM Configuration",
            "passed": false,
            "details": "LLM not configured. Call sf_configure_llm first."
        }]).to_string();
    }

    // AC-1: LLM Connectivity
    results.push(run_case("ac-1", "LLM Connectivity", ac_llm_connectivity().await));

    // AC-2: LLM responds with text
    results.push(run_case("ac-2", "LLM Text Response", ac_llm_text_response().await));

    // AC-3: LLM calls tools when asked
    results.push(run_case("ac-3", "LLM Tool Calling", ac_llm_tool_calling().await));

    // AC-4: Tool execution works (code_write + code_read)
    results.push(run_case("ac-4", "Tool Execution", ac_tool_execution()));

    // AC-5: code_edit works
    results.push(run_case("ac-5", "Code Edit Tool", ac_code_edit()));

    // AC-6: Adversarial guard catches SLOP
    results.push(run_case("ac-6", "Guard Catches SLOP", ac_guard_slop()));

    // AC-7: Guard catches FAKE_BUILD
    results.push(run_case("ac-7", "Guard Catches FAKE_BUILD", ac_guard_fake_build()));

    // AC-8: Agent catalog loaded
    results.push(run_case("ac-8", "Agent Catalog", ac_agent_catalog()));

    // AC-9: Workflow catalog loaded
    results.push(run_case("ac-9", "Workflow Catalog", ac_workflow_catalog()));

    // AC-10: ROLE_TOOL_MAP coverage
    results.push(run_case("ac-10", "Role Tool Map", ac_role_tool_map()));

    serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".into())
}

fn run_case(id: &str, name: &str, result: BenchResult) -> Value {
    json!({
        "case_id": id,
        "name": name,
        "passed": result.passed,
        "details": result.details,
    })
}

// ══════════════════════════════════════════════════════════════
// AC TEST CASES
// ══════════════════════════════════════════════════════════════

/// AC-1: Can we reach the LLM and get any response?
async fn ac_llm_connectivity() -> BenchResult {
    let messages = vec![LLMMessage {
        role: "user".into(),
        content: "Reply with exactly: PING_OK".into(),
    }];

    match llm::chat_completion(&messages, None, None).await {
        Ok(resp) => {
            if resp.content.is_some() || !resp.tool_calls.is_empty() {
                BenchResult {
                    case_id: "ac-1".into(), name: "LLM Connectivity".into(),
                    passed: true, details: "LLM responded successfully".into(),
                }
            } else {
                BenchResult {
                    case_id: "ac-1".into(), name: "LLM Connectivity".into(),
                    passed: false, details: "LLM returned empty response".into(),
                }
            }
        }
        Err(e) => BenchResult {
            case_id: "ac-1".into(), name: "LLM Connectivity".into(),
            passed: false, details: format!("LLM error: {}", e),
        },
    }
}

/// AC-2: Does the LLM return coherent text?
async fn ac_llm_text_response() -> BenchResult {
    let messages = vec![LLMMessage {
        role: "user".into(),
        content: "What is 2 + 2? Reply with just the number.".into(),
    }];

    match llm::chat_completion(&messages, None, None).await {
        Ok(resp) => {
            if let Some(content) = &resp.content {
                if content.contains('4') {
                    BenchResult {
                        case_id: "ac-2".into(), name: "LLM Text Response".into(),
                        passed: true, details: format!("Got correct answer: {}", content.trim()),
                    }
                } else {
                    BenchResult {
                        case_id: "ac-2".into(), name: "LLM Text Response".into(),
                        passed: false, details: format!("Expected '4', got: {}", content.trim()),
                    }
                }
            } else {
                BenchResult {
                    case_id: "ac-2".into(), name: "LLM Text Response".into(),
                    passed: false, details: "No text content in response".into(),
                }
            }
        }
        Err(e) => BenchResult {
            case_id: "ac-2".into(), name: "LLM Text Response".into(),
            passed: false, details: format!("LLM error: {}", e),
        },
    }
}

/// AC-3: Does the LLM call tools when given tool schemas?
async fn ac_llm_tool_calling() -> BenchResult {
    let messages = vec![LLMMessage {
        role: "user".into(),
        content: "Create a file called hello.txt containing 'Hello World'. Use the code_write tool.".into(),
    }];
    let tool_schemas = tools::tool_schemas_for_role("developer");

    match llm::chat_completion(&messages, None, Some(&tool_schemas)).await {
        Ok(resp) => {
            if !resp.tool_calls.is_empty() {
                let tool_names: Vec<&str> = resp.tool_calls.iter().map(|tc| tc.name.as_str()).collect();
                let has_write = tool_names.iter().any(|n| *n == "code_write");
                BenchResult {
                    case_id: "ac-3".into(), name: "LLM Tool Calling".into(),
                    passed: has_write,
                    details: format!("Tools called: {:?}. code_write present: {}", tool_names, has_write),
                }
            } else {
                BenchResult {
                    case_id: "ac-3".into(), name: "LLM Tool Calling".into(),
                    passed: false,
                    details: format!("No tool calls. LLM replied with text: {:?}", resp.content.unwrap_or_default().chars().take(200).collect::<String>()),
                }
            }
        }
        Err(e) => BenchResult {
            case_id: "ac-3".into(), name: "LLM Tool Calling".into(),
            passed: false, details: format!("LLM error: {}", e),
        },
    }
}

/// AC-4: Do code_write and code_read tools work correctly?
fn ac_tool_execution() -> BenchResult {
    let workspace = "/tmp/sf_bench_test";
    std::fs::create_dir_all(workspace).ok();

    // Write
    let write_args = json!({"path": "test_file.txt", "content": "Hello from bench!"});
    let write_result = tools::execute_tool("code_write", &write_args, workspace);

    // Read back
    let read_args = json!({"path": "test_file.txt"});
    let read_result = tools::execute_tool("code_read", &read_args, workspace);

    // Cleanup
    std::fs::remove_dir_all(workspace).ok();

    if write_result.contains("Written") && read_result.contains("Hello from bench!") {
        BenchResult {
            case_id: "ac-4".into(), name: "Tool Execution".into(),
            passed: true, details: "code_write + code_read roundtrip OK".into(),
        }
    } else {
        BenchResult {
            case_id: "ac-4".into(), name: "Tool Execution".into(),
            passed: false,
            details: format!("write: {}, read: {}", write_result, read_result),
        }
    }
}

/// AC-5: Does code_edit (find & replace) work?
fn ac_code_edit() -> BenchResult {
    let workspace = "/tmp/sf_bench_edit";
    std::fs::create_dir_all(workspace).ok();

    // Write initial file
    let write_args = json!({"path": "edit_test.py", "content": "def hello():\n    return 'hello'\n"});
    tools::execute_tool("code_write", &write_args, workspace);

    // Edit it
    let edit_args = json!({"path": "edit_test.py", "old_str": "return 'hello'", "new_str": "return 'world'"});
    let edit_result = tools::execute_tool("code_edit", &edit_args, workspace);

    // Read back
    let read_args = json!({"path": "edit_test.py"});
    let read_result = tools::execute_tool("code_read", &read_args, workspace);

    // Cleanup
    std::fs::remove_dir_all(workspace).ok();

    if edit_result.contains("Edited") && read_result.contains("return 'world'") {
        BenchResult {
            case_id: "ac-5".into(), name: "Code Edit Tool".into(),
            passed: true, details: "code_edit find-and-replace OK".into(),
        }
    } else {
        BenchResult {
            case_id: "ac-5".into(), name: "Code Edit Tool".into(),
            passed: false,
            details: format!("edit: {}, content: {}", edit_result, read_result),
        }
    }
}

/// AC-6: Does the adversarial guard catch SLOP patterns?
fn ac_guard_slop() -> BenchResult {
    let slop_output = "Lorem ipsum dolor sit amet, this is a placeholder implementation with TODO sections throughout. We'll implement this later.";
    let result = guard::check_l0(slop_output, "developer", &[]);

    if !result.passed {
        BenchResult {
            case_id: "ac-6".into(), name: "Guard Catches SLOP".into(),
            passed: true,
            details: format!("Guard correctly rejected SLOP (score: {}, issues: {:?})", result.score, result.issues),
        }
    } else {
        BenchResult {
            case_id: "ac-6".into(), name: "Guard Catches SLOP".into(),
            passed: false,
            details: format!("Guard should have rejected SLOP but passed (score: {})", result.score),
        }
    }
}

/// AC-7: Does the guard catch fake build scripts?
fn ac_guard_fake_build() -> BenchResult {
    let fake_build = r#"#!/bin/bash
echo "BUILD SUCCESS"
exit 0"#;
    let result = guard::check_l0(fake_build, "developer", &["code_write".to_string()]);

    if !result.passed {
        BenchResult {
            case_id: "ac-7".into(), name: "Guard Catches FAKE_BUILD".into(),
            passed: true,
            details: format!("Guard correctly rejected fake build (score: {}, issues: {:?})", result.score, result.issues),
        }
    } else {
        BenchResult {
            case_id: "ac-7".into(), name: "Guard Catches FAKE_BUILD".into(),
            passed: false,
            details: format!("Guard should have rejected fake build but passed (score: {})", result.score),
        }
    }
}

/// AC-8: Are all agents loaded from JSON?
fn ac_agent_catalog() -> BenchResult {
    let count = catalog::agent_count();
    let has_rte = catalog::get_agent_info("rte-marie").is_some();
    let has_po = catalog::get_agent_info("po-lucas").is_some();
    let has_dev = catalog::get_agent_info("dev-karim").is_some()
        || catalog::get_agent_info("dev-clara").is_some();
    let has_qa = catalog::get_agent_info("qa-sophie").is_some();
    let has_archi = catalog::get_agent_info("archi-pierre").is_some();

    if count >= 7 && has_rte && has_po && has_archi {
        BenchResult {
            case_id: "ac-8".into(), name: "Agent Catalog".into(),
            passed: true,
            details: format!("{} agents loaded, core roles present", count),
        }
    } else {
        BenchResult {
            case_id: "ac-8".into(), name: "Agent Catalog".into(),
            passed: false,
            details: format!("Only {} agents. rte={} po={} dev={} qa={} archi={}", count, has_rte, has_po, has_dev, has_qa, has_archi),
        }
    }
}

/// AC-9: Are workflows loaded?
fn ac_workflow_catalog() -> BenchResult {
    let wfs = catalog::list_workflows();
    let has_any = !wfs.is_empty();

    if wfs.len() >= 3 && has_any {
        BenchResult {
            case_id: "ac-9".into(), name: "Workflow Catalog".into(),
            passed: true,
            details: format!("{} workflows loaded", wfs.len()),
        }
    } else {
        BenchResult {
            case_id: "ac-9".into(), name: "Workflow Catalog".into(),
            passed: false,
            details: format!("Only {} workflows loaded", wfs.len()),
        }
    }
}

/// AC-10: Does every role in the catalog have a tool mapping?
fn ac_role_tool_map() -> BenchResult {
    let mut missing_roles = Vec::new();
    let mut role_tool_counts = Vec::new();

    for agent in catalog::all_agents() {
        let schemas = tools::tool_schemas_for_role(&agent.role);
        if schemas.is_empty() {
            missing_roles.push(agent.role.clone());
        } else {
            role_tool_counts.push(format!("{}:{}", agent.role, schemas.len()));
        }
    }

    // Deduplicate
    role_tool_counts.sort();
    role_tool_counts.dedup();

    if missing_roles.is_empty() {
        BenchResult {
            case_id: "ac-10".into(), name: "Role Tool Map".into(),
            passed: true,
            details: format!("All roles have tools: {}", role_tool_counts.join(", ")),
        }
    } else {
        BenchResult {
            case_id: "ac-10".into(), name: "Role Tool Map".into(),
            passed: false,
            details: format!("Roles missing tools: {:?}", missing_roles),
        }
    }
}
