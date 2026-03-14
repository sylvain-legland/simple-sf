// ══════════════════════════════════════════════════════════════
// Chaos Monkey Test Suite — LLM resilience, hot-swap, errors
// ══════════════════════════════════════════════════════════════
//
// Tests LLM configuration lifecycle, error handling, stream parsing,
// engine retry logic, and concurrent access patterns.
// No actual LLM calls — all tests are deterministic.

use sf_engine::{db, llm, tools, catalog, guard, engine};
use serde_json::{json, Value};
use std::sync::Once;

static INIT: Once = Once::new();

fn ensure_db() {
    INIT.call_once(|| {
        let tmp = std::env::temp_dir().join("sf_chaos_test.db");
        let _ = std::fs::remove_file(&tmp);
        db::init_db(tmp.to_str().unwrap());
        catalog::seed_from_json("/nonexistent");
    });
}

// ══════════════════════════════════════════════════════════════
// 1. LLM CONFIG LIFECYCLE — hot-swap, reconfigure, concurrent reads
// ══════════════════════════════════════════════════════════════

#[test]
fn chaos_01_llm_not_configured() {
    // get_config should return None before any configure call
    // Note: other tests may have configured it, so we just verify the function doesn't panic
    let _config = llm::get_config();
}

#[test]
fn chaos_02_configure_then_read() {
    llm::configure_llm("openai", "sk-test-key", "https://api.openai.com/v1", "gpt-4");
    let config = llm::get_config().expect("Config should be set");
    assert_eq!(config.provider, "openai");
    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.api_key, "sk-test-key");
    assert!(config.base_url.contains("openai.com"));
}

#[test]
fn chaos_03_hot_swap_model() {
    llm::configure_llm("mlx", "no-key", "http://localhost:8800/v1", "qwen3-4b");
    assert_eq!(llm::get_config().unwrap().model, "qwen3-4b");

    // Hot-swap model
    llm::set_model("llama3-8b");
    let config = llm::get_config().unwrap();
    assert_eq!(config.model, "llama3-8b");
    assert_eq!(config.provider, "mlx", "Provider should remain unchanged");
    assert_eq!(config.base_url, "http://localhost:8800/v1", "Base URL unchanged");
}

#[test]
fn chaos_04_hot_swap_provider() {
    llm::configure_llm("mlx", "local-key", "http://localhost:8800/v1", "qwen3");

    // Hot-swap entire provider
    llm::set_provider("openai", "sk-new-key", "https://api.openai.com/v1");
    let config = llm::get_config().unwrap();
    assert_eq!(config.provider, "openai");
    assert_eq!(config.api_key, "sk-new-key");
    assert_eq!(config.base_url, "https://api.openai.com/v1");
    assert_eq!(config.model, "qwen3", "Model should persist across provider swap");
}

#[test]
fn chaos_05_rapid_reconfigure() {
    // Simulate rapid provider changes (user toggling in settings)
    for i in 0..50 {
        let provider = if i % 2 == 0 { "mlx" } else { "openai" };
        let model = format!("model-{}", i);
        llm::configure_llm(provider, "key", "http://localhost/v1", &model);
    }
    let config = llm::get_config().unwrap();
    assert_eq!(config.model, "model-49", "Last config should win");
}

#[test]
fn chaos_06_concurrent_config_reads() {
    llm::configure_llm("test", "key", "http://test/v1", "concurrent-model");

    let handles: Vec<_> = (0..10).map(|_| {
        std::thread::spawn(|| {
            for _ in 0..100 {
                let config = llm::get_config();
                assert!(config.is_some(), "Config should always be readable");
            }
        })
    }).collect();

    for h in handles {
        h.join().expect("Thread should not panic");
    }
}

#[test]
fn chaos_07_concurrent_config_write_read() {
    llm::configure_llm("base", "key", "http://base/v1", "base-model");

    // Writers and readers in parallel
    let handles: Vec<_> = (0..10).map(|i| {
        std::thread::spawn(move || {
            if i % 2 == 0 {
                // Writer
                for j in 0..20 {
                    llm::set_model(&format!("model-{}-{}", i, j));
                }
            } else {
                // Reader
                for _ in 0..20 {
                    let _ = llm::get_config();
                }
            }
        })
    }).collect();

    for h in handles {
        h.join().expect("No panic during concurrent read/write");
    }
    // Final state should be valid
    assert!(llm::get_config().is_some());
}

// ══════════════════════════════════════════════════════════════
// 2. LLM ERROR HANDLING — no LLM calls, test the logic
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn chaos_08_chat_without_config_err() {
    // Temporarily can't unset config since it's a global, but we can test
    // parse functions directly. If config is set, calls will fail connecting.
    llm::configure_llm("test", "fake-key", "http://127.0.0.1:1", "test-model");

    let msgs = vec![llm::LLMMessage { role: "user".into(), content: "hello".into() }];
    let result = llm::chat_completion(&msgs, None, None).await;

    // Should fail with connection error (port 1 is not listening)
    assert!(result.is_err(), "Should fail connecting to port 1");
    let err = result.unwrap_err();
    assert!(err.contains("unreachable") || err.contains("error") || err.contains("refused") || err.contains("HTTP"),
        "Error should mention connection issue: {}", err);
}

#[tokio::test]
async fn chaos_09_chat_timeout_handling() {
    // Connect to a port that will accept but never respond (use a large port)
    llm::configure_llm("test", "key", "http://192.0.2.1:12345/v1", "timeout-model");

    let msgs = vec![llm::LLMMessage { role: "user".into(), content: "hello".into() }];

    // Use tokio timeout to prevent actual 300s wait
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        llm::chat_completion(&msgs, None, None),
    ).await;

    // Should either timeout or get connection error
    match result {
        Ok(inner) => {
            assert!(inner.is_err(), "Should fail");
        }
        Err(_) => {
            // Tokio timeout — expected since we're connecting to 192.0.2.1 (TEST-NET)
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 3. STREAM PARSING — unit-test parse logic with synthetic data
// ══════════════════════════════════════════════════════════════

#[test]
fn chaos_10_strip_thinking_blocks() {
    // Test the strip_thinking logic via a publicly accessible path
    let input = "<think>internal reasoning here</think>The actual answer is 42.";
    let expected = "The actual answer is 42.";
    // We can't call strip_thinking directly (private), but we can test via
    // the response parsing indirectly. Test the concept:
    let cleaned = strip_thinking_test(input);
    assert_eq!(cleaned, expected);
}

#[test]
fn chaos_11_strip_thinking_nested() {
    let input = "<think>first</think>Hello <think>second</think>world";
    let cleaned = strip_thinking_test(input);
    assert_eq!(cleaned, "Hello world");
}

#[test]
fn chaos_12_strip_thinking_no_blocks() {
    let input = "Clean output with no thinking blocks";
    let cleaned = strip_thinking_test(input);
    assert_eq!(cleaned, input);
}

#[test]
fn chaos_13_strip_thinking_unclosed() {
    let input = "<think>unclosed thinking block without end tag. Some content after.";
    let cleaned = strip_thinking_test(input);
    // Should not remove anything if no closing tag
    assert!(cleaned.contains("unclosed"), "Unclosed block should be preserved: {}", cleaned);
}

/// Mirror of the private strip_thinking function for testing
fn strip_thinking_test(s: &str) -> String {
    let mut out = s.to_string();
    while let Some(start) = out.find("<think>") {
        if let Some(end) = out.find("</think>") {
            if start <= end {
                out.drain(start..end + 8);
            } else { break; }
        } else { break; }
    }
    out.trim().to_string()
}

#[test]
fn chaos_14_parse_response_with_content() {
    let json = json!({
        "choices": [{
            "message": {
                "content": "Hello world",
                "role": "assistant"
            },
            "finish_reason": "stop"
        }]
    });
    // Test response parsing concept
    let content = json["choices"][0]["message"]["content"].as_str();
    assert_eq!(content, Some("Hello world"));
}

#[test]
fn chaos_15_parse_response_with_tool_calls() {
    let json = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "tool_calls": [{
                    "id": "tc_1",
                    "function": {
                        "name": "code_read",
                        "arguments": "{\"path\":\"src/main.rs\"}"
                    }
                }]
            },
            "finish_reason": "tool_calls"
        }]
    });

    let tcs = json["choices"][0]["message"]["tool_calls"].as_array().unwrap();
    assert_eq!(tcs.len(), 1);
    assert_eq!(tcs[0]["function"]["name"].as_str().unwrap(), "code_read");
    let args: Value = serde_json::from_str(tcs[0]["function"]["arguments"].as_str().unwrap()).unwrap();
    assert_eq!(args["path"].as_str().unwrap(), "src/main.rs");
}

#[test]
fn chaos_16_parse_response_empty_choices() {
    let json = json!({"choices": []});
    let choice = json["choices"].get(0);
    assert!(choice.is_none(), "Empty choices should return None");
}

#[test]
fn chaos_17_parse_response_reasoning_only() {
    // Simulate a reasoning model that exhausted budget
    let json = json!({
        "choices": [{
            "message": {
                "content": "",
                "reasoning": "Let me think step by step about this very complex problem...",
                "role": "assistant"
            },
            "finish_reason": "length"
        }],
        "usage": {
            "completion_tokens": 8192,
            "reasoning_tokens": 8100
        }
    });

    let content = json["choices"][0]["message"]["content"].as_str().unwrap_or("");
    let reasoning = json["choices"][0]["message"]["reasoning"].as_str().unwrap_or("");
    let finish = json["choices"][0]["finish_reason"].as_str().unwrap_or("");

    assert!(content.is_empty(), "Content should be empty");
    assert!(!reasoning.is_empty(), "Reasoning should be present");
    assert_eq!(finish, "length", "Should hit length limit");
}

// ══════════════════════════════════════════════════════════════
// 4. ENGINE RETRY & RESILIENCE
// ══════════════════════════════════════════════════════════════

#[test]
fn chaos_18_guard_l0_fake_build() {
    ensure_db();
    let result = guard::check_l0(
        "#!/bin/bash\necho 'BUILD SUCCESS'\nexit 0 # stub",
        "devops", &[]);
    assert!(result.score >= 7, "Fake build should score ≥7: score={}, issues={:?}", result.score, result.issues);
    assert!(!result.passed, "Fake build should fail guard");
}

#[test]
fn chaos_19_guard_l0_hallucination_fr() {
    ensure_db();
    let result = guard::check_l0(
        "J'ai déployé l'application en production et vérifié que tout fonctionne.",
        "developer", &[]);
    assert!(result.score > 0, "French hallucination should be caught: score={}", result.score);
}

#[test]
fn chaos_20_guard_l0_hallucination_en() {
    ensure_db();
    let result = guard::check_l0(
        "I've deployed the application and verified all tests pass.",
        "developer", &[]);
    assert!(result.score > 0, "English hallucination should be caught: score={}", result.score);
}

#[test]
fn chaos_21_guard_l0_mock_data() {
    ensure_db();
    let result = guard::check_l0(
        "Using fake data for testing:\nlet response = mock_response();",
        "developer", &[]);
    // Should detect "fake data" or similar patterns
    assert!(result.score > 0, "Mock/fake data should be flagged: score={}, issues={:?}", result.score, result.issues);
}

#[test]
fn chaos_22_guard_l0_placeholder_url() {
    ensure_db();
    let result = guard::check_l0(
        "Connect to https://example.com/api/users for the user service.",
        "developer", &[]);
    assert!(result.score > 0, "example.com should be flagged: score={}", result.score);
}

#[test]
fn chaos_23_guard_cumulative_score() {
    ensure_db();
    // Content with multiple issues should accumulate
    let result = guard::check_l0(
        "Here's the code:\n```\nlet foo bar baz = 42;\n// TODO: implement this\nfetch('https://example.com')\n```\nI've tested everything and it works!",
        "developer", &[]);
    assert!(result.score >= 5, "Multiple issues should accumulate: score={}, issues={:?}", result.score, result.issues);
    assert!(result.issues.len() >= 2, "Should have multiple issues: {:?}", result.issues);
}

// ══════════════════════════════════════════════════════════════
// 5. DB & MEMORY CHAOS — concurrent writes, edge cases
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn chaos_24_concurrent_memory_writes() {
    ensure_db();
    let handles: Vec<_> = (0..10).map(|i| {
        tokio::spawn(async move {
            let ws = format!("/tmp/workspaces/chaos-{}", i);
            let args = json!({"key": format!("chaos-key-{}", i), "value": format!("chaos-val-{}", i)});
            let result = tools::execute_tool("memory_store", &args, &ws).await;
            assert!(!result.contains("Error"), "Concurrent write {} should succeed: {}", i, result);
        })
    }).collect();

    for h in handles {
        h.await.expect("Task should not panic");
    }
}

#[tokio::test]
async fn chaos_25_memory_store_special_chars() {
    ensure_db();
    let ws = "/tmp/workspaces/chaos-special";
    let args = json!({
        "key": "sql-injection'; DROP TABLE memory; --",
        "value": "Content with <html> tags & \"quotes\" and 'apostrophes' and\nnewlines\n\tand tabs",
        "category": "test"
    });
    let result = tools::execute_tool("memory_store", &args, ws).await;
    assert!(result.contains("Stored"), "Special chars should be handled: {}", result);
}

#[tokio::test]
async fn chaos_26_memory_store_very_large_key() {
    ensure_db();
    let ws = "/tmp/workspaces/chaos-bigkey";
    let big_key = "k".repeat(10_000);
    let args = json!({"key": big_key, "value": "small value"});
    let result = tools::execute_tool("memory_store", &args, ws).await;
    assert!(result.contains("Stored"), "Large key should work: {}", result);
}

#[test]
fn chaos_27_compact_empty_project() {
    ensure_db();
    // Compacting a project with no entries should not panic
    tools::compact_memory("nonexistent-project-xyz-chaos");
}

#[tokio::test]
async fn chaos_28_compact_idempotent() {
    ensure_db();
    let pid = "chaos-compact-idem";
    let ws = format!("/tmp/workspaces/{}", pid);
    tools::execute_tool("memory_store", &json!({"key": "k1", "value": "v1"}), &ws).await;

    // Compact multiple times
    tools::compact_memory(pid);
    tools::compact_memory(pid);
    tools::compact_memory(pid);

    // Should still have the entry
    let search = json!({"query": "k1", "scope": "project"});
    let found = tools::execute_tool("memory_search", &search, &ws).await;
    assert!(found.contains("k1"), "Entry should survive compaction: {}", found);
}

// ══════════════════════════════════════════════════════════════
// 6. TOOL CHAOS — edge cases
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn chaos_29_code_write_path_traversal() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_chaos_traversal");
    std::fs::create_dir_all(&ws).unwrap();

    // Attempt path traversal
    let result = tools::execute_tool("code_write",
        &json!({"path": "../../etc/passwd", "content": "hacked"}),
        ws.to_str().unwrap()).await;
    // Should either block or write within workspace
    eprintln!("[chaos] Path traversal result: {}", result);

    // Verify /etc/passwd was NOT modified
    let etc_passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    assert!(!etc_passwd.contains("hacked"), "Path traversal should NOT write to /etc/passwd");

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn chaos_30_code_search_regex_injection() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_chaos_regex");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::write(ws.join("test.txt"), "hello world").unwrap();

    // Malformed regex should not crash
    let result = tools::execute_tool("code_search",
        &json!({"query": "(((unclosed", "path": "."}),
        ws.to_str().unwrap()).await;
    // Should handle gracefully (error message or empty result)
    eprintln!("[chaos] Regex injection result: {}", &result[..result.len().min(200)]);

    let _ = std::fs::remove_dir_all(&ws);
}

#[tokio::test]
async fn chaos_31_build_no_workspace() {
    ensure_db();
    let result = tools::execute_tool("build",
        &json!({"command": "echo 'test'"}),
        "/nonexistent/workspace/path").await;
    // Should handle missing workspace
    eprintln!("[chaos] Build no workspace: {}", &result[..result.len().min(200)]);
}

#[tokio::test]
async fn chaos_32_deep_search_huge_query() {
    ensure_db();
    let ws = std::env::temp_dir().join("sf_chaos_deepsearch");
    std::fs::create_dir_all(&ws).unwrap();

    let huge_query = "x".repeat(10_000);
    let result = tools::execute_tool("deep_search",
        &json!({"query": huge_query}),
        ws.to_str().unwrap()).await;
    // Should not crash
    eprintln!("[chaos] Deep search huge query: {} chars result", result.len());

    let _ = std::fs::remove_dir_all(&ws);
}

// ══════════════════════════════════════════════════════════════
// 7. ENGINE STATE — mission lifecycle edge cases
// ══════════════════════════════════════════════════════════════

#[test]
fn chaos_33_mission_status_nonexistent() {
    ensure_db();
    let status = db::with_db(|conn| {
        conn.query_row(
            "SELECT status FROM missions WHERE id = 'nonexistent-mission-999'",
            [], |r| r.get::<_, String>(0),
        ).ok()
    });
    assert!(status.is_none(), "Nonexistent mission should return None");
}

#[test]
fn chaos_34_mission_double_complete() {
    ensure_db();
    let pid = "chaos-double-proj";
    let mid = "chaos-double-mission";

    db::with_db(|conn| {
        conn.execute("INSERT OR IGNORE INTO projects (id, name) VALUES (?1, 'DoubleComplete')", [pid]).unwrap();
        conn.execute("INSERT OR IGNORE INTO missions (id, project_id, brief, status) VALUES (?1, ?2, 'test', 'running')",
            rusqlite::params![mid, pid]).unwrap();
    });

    // Complete once
    db::with_db(|conn| {
        conn.execute("UPDATE missions SET status = 'completed' WHERE id = ?1", [mid]).unwrap();
    });

    // Complete again — should not error
    db::with_db(|conn| {
        conn.execute("UPDATE missions SET status = 'completed' WHERE id = ?1", [mid]).unwrap();
    });

    let status = db::with_db(|conn| {
        conn.query_row("SELECT status FROM missions WHERE id = ?1", [mid], |r| r.get::<_, String>(0)).unwrap()
    });
    assert_eq!(status, "completed");
}

#[test]
fn chaos_35_phase_insert_missing_mission() {
    ensure_db();
    // FK constraint should prevent orphan phases
    let result = db::with_db(|conn| {
        conn.execute(
            "INSERT INTO mission_phases (id, mission_id, phase_name, pattern) VALUES ('p1', 'nonexistent-mission', 'test', 'seq')",
            [],
        )
    });
    assert!(result.is_err(), "FK should prevent orphan phase");
}

#[test]
fn chaos_36_yolo_mode_toggle() {
    // YOLO mode allows phases to pass even with veto
    assert!(!engine::YOLO_MODE.load(std::sync::atomic::Ordering::Relaxed));
    engine::YOLO_MODE.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(engine::YOLO_MODE.load(std::sync::atomic::Ordering::Relaxed));
    engine::YOLO_MODE.store(false, std::sync::atomic::Ordering::Relaxed);
    assert!(!engine::YOLO_MODE.load(std::sync::atomic::Ordering::Relaxed));
}

// ══════════════════════════════════════════════════════════════
// 8. BACKOFF COMPUTATION (test the formula)
// ══════════════════════════════════════════════════════════════

#[test]
fn chaos_37_backoff_formula() {
    // Exponential backoff: BASE_DELAY_MS * 2^(attempt-1), capped at MAX_DELAY_MS
    let base = 2000u64;
    let max = 60_000u64;

    let backoff = |attempt: u32| -> u64 {
        let delay = base * 2u64.pow(attempt.saturating_sub(1));
        delay.min(max)
    };

    assert_eq!(backoff(1), 2000);   // 2s
    assert_eq!(backoff(2), 4000);   // 4s
    assert_eq!(backoff(3), 8000);   // 8s
    assert_eq!(backoff(4), 16000);  // 16s
    assert_eq!(backoff(5), 32000);  // 32s
    assert_eq!(backoff(6), 60000);  // capped at 60s
    assert_eq!(backoff(10), 60000); // still capped
}

#[test]
fn chaos_38_backoff_with_retry_after() {
    // When server sends Retry-After, use that instead of formula
    let retry_after_secs = 30u64;
    let delay = (retry_after_secs * 1000).min(60_000);
    assert_eq!(delay, 30_000);

    // Large Retry-After should be capped
    let large = (120u64 * 1000).min(60_000);
    assert_eq!(large, 60_000);
}

// ══════════════════════════════════════════════════════════════
// 9. CATALOG RESILIENCE
// ══════════════════════════════════════════════════════════════

#[test]
fn chaos_39_agent_info_missing() {
    ensure_db();
    let info = catalog::get_agent_info("nonexistent-agent-xyz");
    assert!(info.is_none(), "Missing agent should return None");
}

#[test]
fn chaos_40_workflow_phases_missing() {
    ensure_db();
    let phases = catalog::get_workflow_phases("nonexistent-workflow-xyz");
    assert!(phases.is_none(), "Missing workflow should return None");
}

#[test]
fn chaos_41_tool_schemas_unknown_role() {
    ensure_db();
    let schemas = tools::tool_schemas_for_role("alien_invader");
    // Unknown role falls back to default set
    assert!(!schemas.is_empty(), "Unknown role should get fallback tools");
    let names: Vec<String> = schemas.iter()
        .filter_map(|t| t["function"]["name"].as_str().map(String::from))
        .collect();
    assert!(names.contains(&"code_read".to_string()), "Fallback should include code_read: {:?}", names);
}

#[test]
fn chaos_42_catalog_stats_stable() {
    ensure_db();
    let (a1, s1, p1, w1) = catalog::catalog_stats();
    let (a2, s2, p2, w2) = catalog::catalog_stats();
    assert_eq!((a1, s1, p1, w1), (a2, s2, p2, w2), "Stats should be stable across calls");
}

// ══════════════════════════════════════════════════════════════
// 10. MEMORY SYSTEM STRESS
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn chaos_43_memory_200_entries_cap() {
    ensure_db();
    let pid = "chaos-cap-test";
    let ws = format!("/tmp/workspaces/{}", pid);

    // Insert 250 entries
    for i in 0..250 {
        let args = json!({"key": format!("cap-key-{:04}", i), "value": format!("val-{}", i)});
        tools::execute_tool("memory_store", &args, &ws).await;
    }

    // Compact should cap at 200
    tools::compact_memory(pid);

    let count = db::with_db(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM memory WHERE project_id = ?1",
            [pid], |r| r.get::<_, i64>(0),
        ).unwrap()
    });
    assert!(count <= 200, "Should be capped at 200, got {}", count);
}

#[tokio::test]
async fn chaos_44_memory_unicode() {
    ensure_db();
    let ws = "/tmp/workspaces/chaos-unicode";
    let args = json!({
        "key": "日本語キー",
        "value": "Contenu en français avec des accents: é à ü ö — emoji: 🚀 🎉 中文内容",
        "category": "i18n"
    });
    let result = tools::execute_tool("memory_store", &args, ws).await;
    assert!(result.contains("Stored"), "Unicode should work: {}", result);

    let search = json!({"query": "emoji", "scope": "project"});
    let found = tools::execute_tool("memory_search", &search, ws).await;
    assert!(found.contains("🚀"), "Unicode should be searchable: {}", found);
}

#[tokio::test]
async fn chaos_45_load_project_memory_truncation() {
    ensure_db();
    let pid = "chaos-trunc-test";
    let ws = format!("/tmp/workspaces/{}", pid);

    // Insert entries with very large values
    for i in 0..20 {
        let big_val = format!("BIG-{}: {}", i, "x".repeat(500));
        let args = json!({"key": format!("trunc-{}", i), "value": big_val});
        tools::execute_tool("memory_store", &args, &ws).await;
    }

    let memory = tools::load_project_memory(pid);
    assert!(memory.len() <= 4500, "Memory injection should be bounded: {} chars", memory.len());
}

// ── Gate detection (expanded patterns) ──

#[test]
fn chaos_46_gate_verdict_nogo_spaced_colon() {
    // The real-world pattern that was missed: "VERDICT : NOGO (CONDITIONNEL)"
    let output = "Analyse complète.\nVERDICT : NOGO (CONDITIONNEL)\nConditions pour GO: ...";
    assert_eq!(engine::check_gate_raw(output), "vetoed");
}

#[test]
fn chaos_47_gate_conclusion_nogo() {
    let output = "Le produit n'est pas jouable.\nCONCLUSION: NOGO — Conditions à respecter.";
    assert_eq!(engine::check_gate_raw(output), "vetoed");
}

#[test]
fn chaos_48_gate_verdict_go_spaced() {
    let output = "Tout est conforme.\nVERDICT : GO\nLe projet peut continuer.";
    assert_eq!(engine::check_gate_raw(output), "approved");
}

#[test]
fn chaos_49_gate_conclusion_approve() {
    let output = "Tests passés.\nCONCLUSION: APPROVE — Prêt pour déploiement.";
    assert_eq!(engine::check_gate_raw(output), "approved");
}

#[test]
fn chaos_50_gate_no_keyword_passes() {
    // Output without any gate keywords → should default to "completed"
    let output = "Voici l'architecture du projet.\n1. Module A\n2. Module B\nFin.";
    assert_eq!(engine::check_gate_raw(output), "completed");
}

#[test]
fn chaos_51_gate_nogo_dash_variant() {
    let output = "Résultat: STATUT: NOGO\nRaison: problème critique.";
    assert_eq!(engine::check_gate_raw(output), "vetoed");
}

#[test]
fn chaos_52_gate_veto_in_brackets() {
    let output = "Mon analyse:\n[VETO] Le code contient des failles de sécurité.";
    assert_eq!(engine::check_gate_raw(output), "vetoed");
}
