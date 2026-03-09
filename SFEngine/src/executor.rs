use crate::llm::{self, LLMMessage};
use crate::tools;
use crate::db;
use serde_json::Value;
use rusqlite::params;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// No artificial round limit — agents run until they produce a text response (#2)
/// Safety: if an agent truly loops, the LLM will eventually stop producing tool_calls.
const MAX_ROUNDS: usize = 100;

/// Event emitted during execution — sent to Swift via callback
#[repr(C)]
pub enum AgentEvent {
    Thinking,
    ToolCall { tool: String, args: String },
    ToolResult { tool: String, result: String },
    Response { content: String },
    ResponseChunk { content: String },
    Error { message: String },
}

pub type EventCallback = Arc<dyn Fn(&str, AgentEvent) + Send + Sync>;

/// Run one agent through the tool-calling loop.
/// Accepts optional protocol injected into system prompt.
/// Uses agent's max_tokens from catalog if available (#15).
pub async fn run_agent(
    agent_id: &str,
    agent_name: &str,
    agent_persona: &str,
    agent_role: &str,
    task: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    protocol: Option<&str>,
    on_event: &EventCallback,
) -> Result<String, String> {
    // Get agent info for max_tokens and extra tools (#15)
    let agent_info = crate::catalog::get_agent_info(agent_id);
    let extra: Vec<String> = agent_info.as_ref()
        .map(|a| a.tools.clone())
        .unwrap_or_default();
    let extra_refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
    let tool_schemas = tools::tool_schemas_for_role_with_extras(agent_role, &extra_refs);

    // Use agent's max_tokens from catalog, fallback to unlimited (#15)
    let max_tokens = agent_info.as_ref()
        .and_then(|a| if a.max_tokens > 0 { Some(a.max_tokens as u32) } else { None })
        .unwrap_or(128_000);

    let protocol_section = protocol.map(|p| format!("\n\n{}", p)).unwrap_or_default();
    let system = format!(
        "{}{}\n\nYour task:\n{}\n\nWorkspace: {}. Use tools to complete the task. Write real, production-quality code.",
        agent_persona, protocol_section, task, workspace
    );

    let mut messages: Vec<LLMMessage> = vec![
        LLMMessage { role: "user".into(), content: task.to_string() },
    ];

    let mut tool_calls_log: Vec<String> = Vec::new();

    for _round in 0..MAX_ROUNDS {
        on_event(agent_id, AgentEvent::Thinking);

        // Build a streaming chunk callback that emits ResponseChunk events
        let agent_id_owned = agent_id.to_string();
        let on_event_clone = on_event.clone();
        let chunked = Arc::new(AtomicBool::new(false));
        let chunked_clone = chunked.clone();
        let chunk_cb: llm::OnChunkFn = Box::new(move |chunk: &str| {
            chunked_clone.store(true, Ordering::Relaxed);
            on_event_clone(&agent_id_owned, AgentEvent::ResponseChunk { content: chunk.to_string() });
        });

        let resp = llm::chat_completion_streaming(
            &messages,
            Some(&system),
            if tool_schemas.is_empty() { None } else { Some(&tool_schemas) },
            chunk_cb,
        ).await?;

        if !resp.tool_calls.is_empty() {
            let tc_summary: Vec<String> = resp.tool_calls.iter()
                .map(|tc| format!("{}({})", tc.name, truncate(&tc.arguments, 100)))
                .collect();

            messages.push(LLMMessage {
                role: "assistant".into(),
                content: format!("[Calling tools: {}]", tc_summary.join(", ")),
            });

            for tc in &resp.tool_calls {
                on_event(agent_id, AgentEvent::ToolCall {
                    tool: tc.name.clone(),
                    args: truncate(&tc.arguments, 200).to_string(),
                });

                let args: Value = serde_json::from_str(&tc.arguments).unwrap_or(Value::Null);
                let result = tools::execute_tool(&tc.name, &args, workspace);

                on_event(agent_id, AgentEvent::ToolResult {
                    tool: tc.name.clone(),
                    result: truncate(&result, 200).to_string(),
                });

                tool_calls_log.push(tc.name.clone());

                if let Err(e) = db::with_db(|conn| {
                    conn.execute(
                        "INSERT INTO agent_messages (mission_id, phase_id, agent_id, agent_name, role, content, tool_calls)
                         VALUES (?1, ?2, ?3, ?4, 'tool', ?5, ?6)",
                        params![mission_id, phase_id, agent_id, agent_name, &result, &tc.name],
                    )
                }) {
                    eprintln!("[db] Failed to store tool message: {}", e);
                }

                messages.push(LLMMessage {
                    role: "user".into(),
                    content: format!("Tool {} result:\n{}", tc.name, result),
                });
            }
            continue;
        }

        if let Some(content) = resp.content {
            // Only emit Response if streaming didn't already deliver the content
            if !chunked.load(Ordering::Relaxed) {
                on_event(agent_id, AgentEvent::Response { content: content.clone() });
            }

            if let Err(e) = db::with_db(|conn| {
                conn.execute(
                    "INSERT INTO agent_messages (mission_id, phase_id, agent_id, agent_name, role, content)
                     VALUES (?1, ?2, ?3, ?4, 'assistant', ?5)",
                    params![mission_id, phase_id, agent_id, agent_name, &content],
                )
            }) {
                eprintln!("[db] Failed to store agent message: {}", e);
            }

            return Ok(content);
        }

        break;
    }

    Err(format!("Agent {} exceeded max rounds ({})", agent_name, MAX_ROUNDS))
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}
