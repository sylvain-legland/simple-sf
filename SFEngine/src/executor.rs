use crate::llm::{self, LLMMessage, ToolCall};
use crate::tools;
use crate::db;
use serde_json::Value;
use rusqlite::params;

const MAX_ROUNDS: usize = 10;

/// Event emitted during execution — sent to Swift via callback
#[repr(C)]
pub enum AgentEvent {
    Thinking,
    ToolCall { tool: String, args: String },
    ToolResult { tool: String, result: String },
    Response { content: String },
    Error { message: String },
}

pub type EventCallback = Box<dyn Fn(&str, AgentEvent) + Send + Sync>;

/// Run one agent through the tool-calling loop.
/// Now accepts an optional protocol that's injected into the system prompt.
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
    // Get tool schemas: role-based + any extras from agent catalog
    let extra: Vec<&str> = crate::catalog::get_agent_def(agent_id)
        .map(|a| a.tools.to_vec())
        .unwrap_or_default();
    let tool_schemas = tools::tool_schemas_for_role_with_extras(agent_role, &extra);

    // Build system prompt: persona + protocol + task context
    let protocol_section = protocol.map(|p| format!("\n\n{}", p)).unwrap_or_default();
    let system = format!(
        "{}{}\n\nYour task:\n{}\n\nWorkspace: {}. Use tools to complete the task. Write real, production-quality code.",
        agent_persona, protocol_section, task, workspace
    );

    let mut messages: Vec<LLMMessage> = vec![
        LLMMessage { role: "user".into(), content: task.to_string() },
    ];

    let mut tool_calls_log: Vec<String> = Vec::new();

    for round in 0..MAX_ROUNDS {
        on_event(agent_id, AgentEvent::Thinking);

        let resp = llm::chat_completion(
            &messages,
            Some(&system),
            if tool_schemas.is_empty() { None } else { Some(&tool_schemas) },
        ).await?;

        // If has tool calls, execute them
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

                // Store tool message in DB
                db::with_db(|conn| {
                    conn.execute(
                        "INSERT INTO agent_messages (mission_id, phase_id, agent_id, agent_name, role, content, tool_calls)
                         VALUES (?1, ?2, ?3, ?4, 'tool', ?5, ?6)",
                        params![mission_id, phase_id, agent_id, agent_name, &result, &tc.name],
                    ).ok();
                });

                messages.push(LLMMessage {
                    role: "user".into(),
                    content: format!("Tool {} result:\n{}", tc.name, result),
                });
            }
            continue;
        }

        // Text response — done
        if let Some(content) = resp.content {
            on_event(agent_id, AgentEvent::Response { content: content.clone() });

            db::with_db(|conn| {
                conn.execute(
                    "INSERT INTO agent_messages (mission_id, phase_id, agent_id, agent_name, role, content)
                     VALUES (?1, ?2, ?3, ?4, 'assistant', ?5)",
                    params![mission_id, phase_id, agent_id, agent_name, &content],
                ).ok();
            });

            return Ok(content);
        }

        break;
    }

    Err(format!("Agent {} exceeded max rounds", agent_name))
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}
