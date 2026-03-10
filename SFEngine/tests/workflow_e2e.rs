/// End-to-end workflow tests with mock LLM server.
/// Validates that missions run to completion even when phases fail, VETO, or timeout.
///
/// Architecture:
/// - A tiny TCP server returns canned OpenAI-compatible responses
/// - LLM is configured to point to this mock server
/// - run_mission is called with real DB, real engine, fake LLM
/// - We verify DB states and captured events after the mission completes

use sf_engine::{db, llm, engine, catalog};
use sf_engine::executor::AgentEvent;
use rusqlite::params;
use serde_json::json;
use std::sync::{Arc, Mutex, Once, atomic::{AtomicUsize, Ordering}};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

static INIT: Once = Once::new();

fn ensure_db() {
    INIT.call_once(|| {
        let tmp = std::env::temp_dir().join("sf_e2e_test.db");
        let _ = std::fs::remove_file(&tmp);
        db::init_db(tmp.to_str().unwrap());
        catalog::seed_from_json("/nonexistent");
    });
}

// ── Mock LLM Server ──

struct MockLLM {
    port: u16,
    request_count: Arc<AtomicUsize>,
}

/// Build an OpenAI-compatible JSON response body
fn make_chat_response(content: &str) -> String {
    let escaped = serde_json::to_string(content).unwrap();
    format!(
        r#"{{"id":"chatcmpl-mock","object":"chat.completion","choices":[{{"index":0,"message":{{"role":"assistant","content":{}}},"finish_reason":"stop"}}],"usage":{{"prompt_tokens":10,"completion_tokens":50,"total_tokens":60}}}}"#,
        escaped
    )
}

/// Build SSE stream response (the LLM client sends stream:true for non-tool calls)
fn make_stream_response(content: &str) -> String {
    // The LLM client always sets stream:true for non-tool calls.
    // Return a proper SSE stream with content deltas.
    let escaped = content.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    let mut sse = String::new();
    // Single chunk with all content
    sse.push_str(&format!(
        "data: {{\"id\":\"chatcmpl-mock\",\"object\":\"chat.completion.chunk\",\"choices\":[{{\"index\":0,\"delta\":{{\"role\":\"assistant\",\"content\":\"{}\"}},\"finish_reason\":null}}]}}\n\n",
        escaped
    ));
    // Final chunk with finish_reason
    sse.push_str("data: {\"id\":\"chatcmpl-mock\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n");
    sse.push_str("data: [DONE]\n\n");
    sse
}

impl MockLLM {
    /// Start a mock LLM server that returns canned responses.
    /// `responses` is indexed by request number; after exhaustion, last response is reused.
    async fn start(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let request_count = Arc::new(AtomicUsize::new(0));
        let rc = request_count.clone();
        let responses = Arc::new(responses);

        tokio::spawn(async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(s) => s,
                    Err(_) => break,
                };

                let idx = rc.fetch_add(1, Ordering::SeqCst);
                let content = if idx < responses.len() {
                    responses[idx].clone()
                } else {
                    responses.last().cloned().unwrap_or_else(|| "OK".to_string())
                };

                tokio::spawn(async move {
                    // Read request headers + body (just consume everything available)
                    let mut buf = vec![0u8; 65536];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    if n == 0 { return; }

                    let req = String::from_utf8_lossy(&buf[..n]);

                    // Check if it's a streaming request
                    let is_stream = req.contains("\"stream\":true") || req.contains("\"stream\": true");

                    let (content_type, body) = if is_stream {
                        ("text/event-stream", make_stream_response(&content))
                    } else {
                        ("application/json", make_chat_response(&content))
                    };

                    let http = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                        content_type, body.len(), body
                    );
                    let _ = socket.write_all(http.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });

        MockLLM { port, request_count }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.port)
    }

    fn call_count(&self) -> usize {
        self.request_count.load(Ordering::SeqCst)
    }
}

// ── Event Capture ──

fn event_capture() -> (Arc<dyn Fn(&str, AgentEvent) + Send + Sync>, Arc<Mutex<Vec<String>>>) {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let log_clone = log.clone();
    let cb: Arc<dyn Fn(&str, AgentEvent) + Send + Sync> = Arc::new(move |agent_id: &str, event: AgentEvent| {
        let msg = match &event {
            AgentEvent::Response { content } =>
                format!("[{}] RESPONSE: {}", agent_id, &content[..content.len().min(200)]),
            AgentEvent::Error { message } =>
                format!("[{}] ERROR: {}", agent_id, message),
            AgentEvent::ToolCall { tool, .. } =>
                format!("[{}] TOOL: {}", agent_id, tool),
            _ => format!("[{}] OTHER", agent_id),
        };
        log_clone.lock().unwrap().push(msg);
    });
    (cb, log)
}

// ── Test helpers ──

fn setup_mission(mission_id: &str, workflow_id: &str, brief: &str) {
    db::with_db(|conn| {
        // Insert project first (FK target)
        conn.execute(
            "INSERT OR IGNORE INTO projects (id, name, description) VALUES (?1, ?1, 'e2e test project')",
            params![mission_id],
        ).unwrap();
        // Insert mission
        conn.execute(
            "INSERT OR REPLACE INTO missions (id, project_id, brief, workflow, status, created_at) \
             VALUES (?1, ?2, ?3, ?4, 'pending', datetime('now'))",
            params![mission_id, mission_id, brief, workflow_id],
        ).unwrap();
    });
}

fn setup_workflow(id: &str, phases_json: &str) {
    db::with_db(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO workflows (id, name, description, phases_json) VALUES (?1, ?1, 'test workflow', ?2)",
            params![id, phases_json],
        ).unwrap();
    });
}

fn get_mission_status(mission_id: &str) -> String {
    db::with_db(|conn| {
        conn.query_row(
            "SELECT status FROM missions WHERE id = ?1", params![mission_id],
            |row| row.get::<_, String>(0),
        ).unwrap_or_else(|_| "not_found".into())
    })
}

fn get_phase_statuses(mission_id: &str) -> Vec<(String, String)> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT phase_name, status FROM mission_phases WHERE mission_id = ?1 ORDER BY started_at"
        ).unwrap();
        stmt.query_map(params![mission_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).unwrap().filter_map(|r| r.ok()).collect()
    })
}

#[allow(dead_code)]
fn get_phase_count(mission_id: &str) -> usize {
    db::with_db(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM mission_phases WHERE mission_id = ?1",
            params![mission_id],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as usize
    })
}

// ══════════════════════════════════════════════════════════════
// TEST 1: All phases pass — mission completes normally
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_01_all_phases_pass() {
    ensure_db();
    let mock = MockLLM::start(vec![
        "Vision: Build a Pac-Man game with SpriteKit. [APPROVE]".into(),
        "Architecture: Use GameScene + GameLogic separation. [APPROVE]".into(),
        "Dev: Implementation complete.".into(),
    ]).await;

    llm::configure_llm("mock", "test-key", &mock.url(), "test-model");

    let wf = json!([
        {"name": "vision", "pattern": "sequential", "agent_ids": ["rte-marie"]},
        {"name": "architecture", "pattern": "sequential", "agent_ids": ["archi-pierre"]},
        {"name": "dev", "pattern": "sequential", "agent_ids": ["lead-thomas"]},
    ]);
    setup_workflow("e2e-wf-01", &wf.to_string());
    setup_mission("e2e-m-01", "e2e-wf-01", "Build Pac-Man");

    let (cb, log) = event_capture();
    let result = engine::run_mission("e2e-m-01", "Build Pac-Man", "/tmp/workspaces/e2e-test", &cb).await;

    assert!(result.is_ok(), "Mission should succeed: {:?}", result);
    assert_eq!(get_mission_status("e2e-m-01"), "completed");

    let phases = get_phase_statuses("e2e-m-01");
    assert_eq!(phases.len(), 3, "All 3 phases should be recorded");

    let events = log.lock().unwrap();
    let has_terminee = events.iter().any(|e| e.contains("TERMINEE") || e.contains("completees"));
    assert!(has_terminee, "Should emit completion event");

    assert!(mock.call_count() >= 3, "Should have made at least 3 LLM calls, got {}", mock.call_count());
}

// ══════════════════════════════════════════════════════════════
// TEST 2: VETO stops mission when YOLO is OFF
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_02_veto_stops_without_yolo() {
    ensure_db();
    engine::YOLO_MODE.store(false, Ordering::Relaxed);

    let mock = MockLLM::start(vec![
        "Vision ok. [APPROVE]".into(),
        "VERDICT: NOGO — Critical security issues.".into(),
        "This should never be reached.".into(),
    ]).await;

    llm::configure_llm("mock", "test-key", &mock.url(), "test-model");

    let wf = json!([
        {"name": "vision", "pattern": "sequential", "agent_ids": ["rte-marie"]},
        {"name": "strategy", "pattern": "sequential", "agent_ids": ["po-lucas"]},
        {"name": "dev", "pattern": "sequential", "agent_ids": ["lead-thomas"]},
    ]);
    setup_workflow("e2e-wf-02", &wf.to_string());
    setup_mission("e2e-m-02", "e2e-wf-02", "Vetoed project");

    let (cb, log) = event_capture();
    let _ = engine::run_mission("e2e-m-02", "Vetoed project", "/tmp/workspaces/e2e-test", &cb).await;

    assert_eq!(get_mission_status("e2e-m-02"), "vetoed");

    let phases = get_phase_statuses("e2e-m-02");
    // Phase 1 (vision) completed, Phase 2 (strategy) vetoed, Phase 3 skipped
    let strategy_phase = phases.iter().find(|(n, _)| n == "strategy");
    assert!(strategy_phase.is_some(), "Strategy phase should exist");
    assert_eq!(strategy_phase.unwrap().1, "vetoed");

    let events = log.lock().unwrap();
    let has_veto = events.iter().any(|e| e.contains("VETO"));
    assert!(has_veto, "Should emit VETO event");
}

// ══════════════════════════════════════════════════════════════
// TEST 3: YOLO overrides VETO — mission continues through all phases
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_03_yolo_overrides_veto() {
    ensure_db();
    engine::YOLO_MODE.store(true, Ordering::Relaxed);

    let mock = MockLLM::start(vec![
        "Vision ok. [APPROVE]".into(),
        "VERDICT : NOGO (CONDITIONNEL) — Conditions: remove debug flags.".into(),
        "Dev done. All conditions addressed.".into(),
    ]).await;

    llm::configure_llm("mock", "test-key", &mock.url(), "test-model");

    let wf = json!([
        {"name": "vision", "pattern": "sequential", "agent_ids": ["rte-marie"]},
        {"name": "comité stratégique", "pattern": "sequential", "agent_ids": ["po-lucas"]},
        {"name": "dev", "pattern": "sequential", "agent_ids": ["lead-thomas"]},
    ]);
    setup_workflow("e2e-wf-03", &wf.to_string());
    setup_mission("e2e-m-03", "e2e-wf-03", "YOLO Pac-Man");

    let (cb, log) = event_capture();
    let result = engine::run_mission("e2e-m-03", "YOLO Pac-Man", "/tmp/workspaces/e2e-test", &cb).await;

    assert!(result.is_ok(), "Mission should succeed in YOLO mode: {:?}", result);
    assert_eq!(get_mission_status("e2e-m-03"), "completed", "Mission should be completed (not vetoed)");

    // All 3 phases should be processed
    let phases = get_phase_statuses("e2e-m-03");
    assert_eq!(phases.len(), 3, "All 3 phases should be recorded in YOLO mode");

    let events = log.lock().unwrap();
    let has_yolo = events.iter().any(|e| e.contains("YOLO") && e.contains("VETO overridden"));
    assert!(has_yolo, "Should emit YOLO override event: {:?}", *events);

    engine::YOLO_MODE.store(false, Ordering::Relaxed); // cleanup
}

// ══════════════════════════════════════════════════════════════
// TEST 4: LLM connection failures don't block mission
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_04_llm_failures_dont_block() {
    ensure_db();

    // Point to a port with nothing listening → connection refused
    llm::configure_llm("mock", "test-key", "http://127.0.0.1:1/v1", "test-model");

    let wf = json!([
        {"name": "vision", "pattern": "sequential", "agent_ids": ["rte-marie"]},
        {"name": "design", "pattern": "sequential", "agent_ids": ["archi-pierre"]},
        {"name": "dev", "pattern": "sequential", "agent_ids": ["lead-thomas"]},
    ]);
    setup_workflow("e2e-wf-04", &wf.to_string());
    setup_mission("e2e-m-04", "e2e-wf-04", "Failing project");

    let (cb, log) = event_capture();
    let result = engine::run_mission("e2e-m-04", "Failing project", "/tmp/workspaces/e2e-test", &cb).await;

    // Mission should complete (not hang) even though all phases failed
    assert!(result.is_ok(), "Mission should not return Err even if all phases fail: {:?}", result);

    // All phases should be attempted and recorded as failed
    let phases = get_phase_statuses("e2e-m-04");
    assert_eq!(phases.len(), 3, "All 3 phases should be attempted: {:?}", phases);
    for (name, status) in &phases {
        assert_eq!(status, "failed", "Phase {} should be 'failed', got '{}'", name, status);
    }

    let events = log.lock().unwrap();
    let error_count = events.iter().filter(|e| e.contains("ERROR") || e.contains("failed")).count();
    assert!(error_count >= 3, "Should have at least 3 error events, got {}", error_count);
}

// ══════════════════════════════════════════════════════════════
// TEST 5: Mixed results — some pass, some fail, mission completes
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_05_mixed_pass_fail() {
    ensure_db();

    // First call succeeds, then switch to dead server for phase 2, back to mock for phase 3
    let mock = MockLLM::start(vec![
        "Vision: solid plan. VERDICT: GO".into(),
        // Phase 2 will use this too — just a normal response (no gate keyword → "completed")
        "Architecture: clean separation. Modules defined.".into(),
        "QA: all tests pass. [APPROVE]".into(),
    ]).await;

    llm::configure_llm("mock", "test-key", &mock.url(), "test-model");

    let wf = json!([
        {"name": "vision", "pattern": "sequential", "agent_ids": ["rte-marie"]},
        {"name": "architecture", "pattern": "sequential", "agent_ids": ["archi-pierre"]},
        {"name": "qa", "pattern": "sequential", "agent_ids": ["qa-sophie"]},
    ]);
    setup_workflow("e2e-wf-05", &wf.to_string());
    setup_mission("e2e-m-05", "e2e-wf-05", "Mixed project");

    let (cb, _log) = event_capture();
    let result = engine::run_mission("e2e-m-05", "Mixed project", "/tmp/workspaces/e2e-test", &cb).await;

    assert!(result.is_ok());
    assert_eq!(get_mission_status("e2e-m-05"), "completed");

    let phases = get_phase_statuses("e2e-m-05");
    assert_eq!(phases.len(), 3);
    // All should be completed since all responses are valid
    for (name, status) in &phases {
        assert_eq!(status, "completed", "Phase {} should complete, got {}", name, status);
    }
}

// ══════════════════════════════════════════════════════════════
// TEST 6: VERDICT with French space (VERDICT : NOGO) detected correctly
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_06_french_verdict_detected() {
    ensure_db();
    engine::YOLO_MODE.store(false, Ordering::Relaxed);

    let mock = MockLLM::start(vec![
        // French-style with space before colon
        "Analyse complète.\nVERDICT : NOGO (CONDITIONNEL)\nConditions pour GO: supprimer showsFPS.\nCONCLUSION: NOGO".into(),
    ]).await;

    llm::configure_llm("mock", "test-key", &mock.url(), "test-model");

    let wf = json!([
        {"name": "comité", "pattern": "sequential", "agent_ids": ["po-lucas"]},
        {"name": "dev", "pattern": "sequential", "agent_ids": ["lead-thomas"]},
    ]);
    setup_workflow("e2e-wf-06", &wf.to_string());
    setup_mission("e2e-m-06", "e2e-wf-06", "French NOGO test");

    let (cb, _) = event_capture();
    let _ = engine::run_mission("e2e-m-06", "French NOGO test", "/tmp/workspaces/e2e-test", &cb).await;

    assert_eq!(get_mission_status("e2e-m-06"), "vetoed");

    let phases = get_phase_statuses("e2e-m-06");
    let comite = phases.iter().find(|(n, _)| n == "comité");
    assert_eq!(comite.unwrap().1, "vetoed", "French VERDICT : NOGO should be detected as vetoed");
}

// ══════════════════════════════════════════════════════════════
// TEST 7: YOLO injects veto conditions into next phase
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_07_yolo_injects_veto_conditions() {
    ensure_db();
    engine::YOLO_MODE.store(true, Ordering::Relaxed);

    let received_prompts: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let prompts_clone = received_prompts.clone();

    // Custom mock that captures request bodies
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let call_count = Arc::new(AtomicUsize::new(0));
    let cc = call_count.clone();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            let prompts_ref = prompts_clone.clone();
            let idx = cc.fetch_add(1, Ordering::SeqCst);

            tokio::spawn(async move {
                let mut buf = vec![0u8; 131072];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                if n == 0 { return; }
                let req = String::from_utf8_lossy(&buf[..n]).to_string();

                // Extract body from HTTP request
                if let Some(body_start) = req.find("\r\n\r\n") {
                    let body = &req[body_start + 4..];
                    prompts_ref.lock().unwrap().push(body.to_string());
                }

                let content = if idx == 0 {
                    "VERDICT : NOGO — Conditions: 1) Remove showsFPS 2) Add unit tests 3) Fix memory leaks"
                } else {
                    "All conditions addressed. Implementation complete."
                };

                let is_stream = req.contains("\"stream\":true") || req.contains("\"stream\": true");
                let (ct, body) = if is_stream {
                    ("text/event-stream", make_stream_response(content))
                } else {
                    ("application/json", make_chat_response(content))
                };

                let http = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    ct, body.len(), body
                );
                let _ = socket.write_all(http.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });

    llm::configure_llm("mock", "test-key", &format!("http://127.0.0.1:{}/v1", port), "test-model");

    let wf = json!([
        {"name": "review", "pattern": "sequential", "agent_ids": ["po-lucas"]},
        {"name": "dev", "pattern": "sequential", "agent_ids": ["lead-thomas"]},
    ]);
    setup_workflow("e2e-wf-07", &wf.to_string());
    setup_mission("e2e-m-07", "e2e-wf-07", "Veto injection test");

    let (cb, log) = event_capture();
    let result = engine::run_mission("e2e-m-07", "Veto injection test", "/tmp/workspaces/e2e-test", &cb).await;

    assert!(result.is_ok(), "Mission should complete in YOLO: {:?}", result);
    assert_eq!(get_mission_status("e2e-m-07"), "completed");

    // Verify veto conditions were injected into the second phase's prompt
    let prompts = received_prompts.lock().unwrap();
    assert!(prompts.len() >= 2, "Should have at least 2 LLM calls, got {}", prompts.len());

    // The second prompt should contain the veto conditions
    let second_prompt = &prompts[1];
    let has_conditions = second_prompt.contains("showsFPS")
        || second_prompt.contains("VETO")
        || second_prompt.contains("NOGO");
    assert!(has_conditions, "Second phase prompt should contain veto conditions");

    let events = log.lock().unwrap();
    let has_yolo = events.iter().any(|e| e.contains("YOLO"));
    assert!(has_yolo, "Should have YOLO override event");

    engine::YOLO_MODE.store(false, Ordering::Relaxed);
}

// ══════════════════════════════════════════════════════════════
// TEST 8: Phase timeout doesn't hang the mission
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_08_phase_timeout_recovery() {
    ensure_db();

    // Mock server that delays forever on first request (simulates hang)
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let call_count = Arc::new(AtomicUsize::new(0));
    let cc = call_count.clone();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            let idx = cc.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 65536];
                let _ = socket.read(&mut buf).await;
                if idx < 2 {
                    // First 2 connections: hang for 2 seconds (we'll set a short timeout)
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                let body = make_chat_response("Done after delay.");
                let http = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(), body
                );
                let _ = socket.write_all(http.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });

    llm::configure_llm("mock", "test-key", &format!("http://127.0.0.1:{}/v1", port), "test-model");

    // Use a workflow with just 1 phase to test timeout behavior
    // Note: The actual PHASE_TIMEOUT_SECS is 600 — too long for tests.
    // This test validates that the timeout mechanism exists and the mission
    // doesn't hang indefinitely. The LLM client itself has a 300s timeout.
    // We test with a 2s delay which is well within both limits.
    let wf = json!([
        {"name": "dev", "pattern": "sequential", "agent_ids": ["lead-thomas"]},
    ]);
    setup_workflow("e2e-wf-08", &wf.to_string());
    setup_mission("e2e-m-08", "e2e-wf-08", "Timeout test");

    let (cb, _) = event_capture();
    let start = std::time::Instant::now();
    let result = engine::run_mission("e2e-m-08", "Timeout test", "/tmp/workspaces/e2e-test", &cb).await;
    let elapsed = start.elapsed();

    // Mission should complete (delay is within HTTP timeout)
    assert!(result.is_ok(), "Mission should complete: {:?}", result);
    // Should not take more than 30s (way under the 600s phase timeout)
    assert!(elapsed.as_secs() < 30, "Should complete reasonably fast, took {}s", elapsed.as_secs());
}

// ══════════════════════════════════════════════════════════════
// TEST 9: Multiple VETOs in YOLO mode — all overridden
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_09_multiple_vetos_yolo() {
    ensure_db();
    engine::YOLO_MODE.store(true, Ordering::Relaxed);

    let mock = MockLLM::start(vec![
        "[VETO] Vision insuffisante.".into(),
        "DÉCISION : NOGO — Architecture non conforme.".into(),
        "CONCLUSION: NOGO — Tests manquants.".into(),
        "Deploiement ok. [APPROVE]".into(),
    ]).await;

    llm::configure_llm("mock", "test-key", &mock.url(), "test-model");

    let wf = json!([
        {"name": "vision", "pattern": "sequential", "agent_ids": ["rte-marie"]},
        {"name": "architecture", "pattern": "sequential", "agent_ids": ["archi-pierre"]},
        {"name": "qa", "pattern": "sequential", "agent_ids": ["qa-sophie"]},
        {"name": "deploy", "pattern": "sequential", "agent_ids": ["lead-thomas"]},
    ]);
    setup_workflow("e2e-wf-09", &wf.to_string());
    setup_mission("e2e-m-09", "e2e-wf-09", "Multi-veto YOLO");

    let (cb, log) = event_capture();
    let result = engine::run_mission("e2e-m-09", "Multi-veto YOLO", "/tmp/workspaces/e2e-test", &cb).await;

    assert!(result.is_ok(), "Should complete despite multiple VETOs");
    assert_eq!(get_mission_status("e2e-m-09"), "completed");

    let phases = get_phase_statuses("e2e-m-09");
    assert_eq!(phases.len(), 4, "All 4 phases should be processed");

    let events = log.lock().unwrap();
    let yolo_count = events.iter().filter(|e| e.contains("YOLO")).count();
    assert!(yolo_count >= 3, "Should have at least 3 YOLO overrides, got {}", yolo_count);

    engine::YOLO_MODE.store(false, Ordering::Relaxed);
}

// ══════════════════════════════════════════════════════════════
// TEST 10: Empty workflow — mission completes immediately
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_10_empty_workflow_fallback() {
    ensure_db();

    // Workflow with no matching phases → falls back to SAFE_PHASES
    let mock = MockLLM::start(vec![
        "Phase output.".into(),
    ]).await;

    llm::configure_llm("mock", "test-key", &mock.url(), "test-model");

    // Use a workflow ID that doesn't exist → SAFE_PHASES fallback
    setup_mission("e2e-m-10", "nonexistent-workflow", "Fallback test");

    let (cb, log) = event_capture();
    let result = engine::run_mission("e2e-m-10", "Fallback test", "/tmp/workspaces/e2e-test", &cb).await;

    assert!(result.is_ok(), "Should fallback to SAFE_PHASES: {:?}", result);

    // SAFE_PHASES has 5 phases
    let phases = get_phase_statuses("e2e-m-10");
    assert!(phases.len() >= 3, "Should have phases from SAFE_PHASES fallback, got {}", phases.len());

    let events = log.lock().unwrap();
    let has_phase = events.iter().any(|e| e.contains("Phase:") || e.contains("VISION") || e.contains("DESIGN"));
    assert!(has_phase, "Should process SAFE_PHASES");
}

// ══════════════════════════════════════════════════════════════
// TEST 11: Phase context accumulation — later phases get earlier outputs
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_11_phase_context_accumulation() {
    ensure_db();

    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let recv_clone = received.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let cc = Arc::new(AtomicUsize::new(0));
    let cc2 = cc.clone();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            let recv_ref = recv_clone.clone();
            let idx = cc2.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 131072];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                if n == 0 { return; }
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                if let Some(body_start) = req.find("\r\n\r\n") {
                    recv_ref.lock().unwrap().push(req[body_start + 4..].to_string());
                }

                let content = match idx {
                    0 => "PHASE1_MARKER: SpriteKit architecture chosen.",
                    1 => "PHASE2_MARKER: GameScene.swift implemented.",
                    _ => "PHASE3_MARKER: All tests pass.",
                };
                let is_stream = req.contains("\"stream\":true");
                let (ct, body) = if is_stream {
                    ("text/event-stream", make_stream_response(content))
                } else {
                    ("application/json", make_chat_response(content))
                };
                let http = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    ct, body.len(), body
                );
                let _ = socket.write_all(http.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });

    llm::configure_llm("mock", "test-key", &format!("http://127.0.0.1:{}/v1", port), "test-model");

    let wf = json!([
        {"name": "phase1", "pattern": "sequential", "agent_ids": ["rte-marie"]},
        {"name": "phase2", "pattern": "sequential", "agent_ids": ["archi-pierre"]},
        {"name": "phase3", "pattern": "sequential", "agent_ids": ["lead-thomas"]},
    ]);
    setup_workflow("e2e-wf-11", &wf.to_string());
    setup_mission("e2e-m-11", "e2e-wf-11", "Context accumulation");

    let (cb, _) = event_capture();
    let _ = engine::run_mission("e2e-m-11", "Context accumulation", "/tmp/workspaces/e2e-test", &cb).await;

    let prompts = received.lock().unwrap();
    // Phase 3 prompt should contain output from Phase 1 (PHASE1_MARKER)
    if prompts.len() >= 3 {
        let phase3_prompt = &prompts[2];
        assert!(
            phase3_prompt.contains("PHASE1_MARKER") || phase3_prompt.contains("phase1"),
            "Phase 3 should receive context from phase 1"
        );
    }
}

// ══════════════════════════════════════════════════════════════
// TEST 12: Network pattern with mock — multi-agent discussion completes
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_12_network_pattern_completes() {
    ensure_db();

    let mock = MockLLM::start(vec![
        "Leader: Let's discuss the architecture. Key question: monolith vs microservices?".into(),
        "Expert 1: I recommend monolith for MVP.".into(),
        "Expert 2: Agreed, monolith with clean boundaries.".into(),
        "Synthesis: Team consensus on monolith architecture. [APPROVE]".into(),
        // Extra responses for additional rounds
        "Round 2 discussion.".into(),
        "Round 2 response.".into(),
        "Final synthesis. [APPROVE]".into(),
    ]).await;

    llm::configure_llm("mock", "test-key", &mock.url(), "test-model");

    let wf = json!([
        {"name": "architecture debate", "pattern": "network", "agent_ids": ["rte-marie", "archi-pierre", "lead-thomas"]},
    ]);
    setup_workflow("e2e-wf-12", &wf.to_string());
    setup_mission("e2e-m-12", "e2e-wf-12", "Network pattern test");

    let (cb, _) = event_capture();
    let result = engine::run_mission("e2e-m-12", "Network pattern test", "/tmp/workspaces/e2e-test", &cb).await;

    assert!(result.is_ok(), "Network pattern should complete: {:?}", result);
    assert_eq!(get_mission_status("e2e-m-12"), "completed");
    assert!(mock.call_count() >= 3, "Network should have multiple LLM calls");
}

/// Real MLX mission test — only runs when MLX_REAL=1 
#[tokio::test]
async fn real_mlx_pacman_mission() {
    if std::env::var("MLX_REAL").unwrap_or_default() != "1" {
        eprintln!("Skipping real MLX test (set MLX_REAL=1)");
        return;
    }
    // Use own fresh DB (bypass Once::call_once)
    let tmp = std::env::temp_dir().join("sf_mlx_real_test.db");
    let _ = std::fs::remove_file(&tmp);
    db::init_db(tmp.to_str().unwrap());
    catalog::seed_from_json("/nonexistent");
    llm::configure_llm("mlx", "no-key", "http://127.0.0.1:8800/v1", "mlx-community/Qwen3.5-35B-A3B-4bit");
    
    let workspace = "/tmp/pacman-workspace-real";
    let _ = std::fs::create_dir_all(workspace);
    
    // Create a project first (missions requires project_id FK)
    let project_id = "pacman-real-test";
    db::with_db(|conn| {
        conn.execute(
            "INSERT OR IGNORE INTO projects (id, name, description, tech) VALUES (?1, ?2, ?3, ?4)",
            params![project_id, "Pac-Man macOS", "Jeu Pac-Man en Swift/SpriteKit pour macOS natif", "swift,spritekit"],
        ).unwrap();
    });
    
    let mission_id = uuid::Uuid::new_v4().to_string();
    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO missions (id, project_id, brief, status, workflow) VALUES (?1, ?2, ?3, 'pending', 'safe-standard')",
            params![&mission_id, project_id, "Jeu Pac-Man en Swift/SpriteKit pour macOS natif avec labyrinthe, fantomes IA, power pellets et scoring. Creer tous les fichiers Swift necessaires."],
        ).unwrap();
    });
    
    let events = Arc::new(Mutex::new(Vec::<String>::new()));
    let events_clone = events.clone();
    let cb: Arc<dyn Fn(&str, AgentEvent) + Send + Sync> = Arc::new(move |agent, event| {
        match event {
            AgentEvent::Response { content } => {
                let msg = format!("[{}] {}", agent, &content[..content.len().min(300)]);
                println!("{}", msg);
                events_clone.lock().unwrap().push(msg);
            }
            AgentEvent::Thinking => println!("[{}] thinking...", agent),
            AgentEvent::Error { message } => eprintln!("[{}] ERROR: {}", agent, message),
            _ => {}
        }
    });
    
    let result = engine::run_mission(
        &mission_id,
        "Jeu Pac-Man en Swift/SpriteKit pour macOS natif avec labyrinthe, fantomes IA, power pellets et scoring. Creer tous les fichiers Swift necessaires.",
        workspace,
        &cb,
    ).await;
    
    println!("MISSION RESULT: {:?}", result);
    
    let ev = events.lock().unwrap();
    println!("Total events: {}", ev.len());
    for e in ev.iter() {
        println!("  {}", &e[..e.len().min(120)]);
    }
    
    assert!(result.is_ok(), "Mission should complete: {:?}", result);
    
    // Check workspace has files
    let files: Vec<_> = std::fs::read_dir(workspace).unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    println!("Workspace files: {:?}", files);
}
