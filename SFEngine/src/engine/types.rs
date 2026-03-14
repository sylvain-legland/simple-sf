// Ref: FT-SSF-020

use crate::agents::Agent;
use crate::db;
use crate::executor::{AgentEvent, EventCallback};
use rusqlite::params;
use std::sync::atomic::AtomicBool;

pub(crate) const PHASE_TIMEOUT_SECS: u64 = 900; // 15 min max per phase

/// YOLO mode: auto-approve all gates (skip human-in-the-loop checkpoints)
pub static YOLO_MODE: AtomicBool = AtomicBool::new(false);

pub(crate) const MAX_NETWORK_ROUNDS: usize = 3;
pub(crate) const CONTEXT_BUDGET: usize = 12000;
pub(crate) const MAX_PHASE_RETRIES: usize = 3;

/// Instruction appended to every system prompt — enforce no emoji
pub(crate) const STYLE_RULES: &str = "RÈGLES DE FORMAT : ZÉRO emoji, ZÉRO émoticône, ZÉRO caractère Unicode décoratif. \
Utilise uniquement du texte, des tirets (-), des pipes (|), des étoiles (*) pour la mise en forme. \
Sois structuré avec des titres en **gras** et des listes à tirets.";

/// Phase execution semantics
#[derive(Clone, Debug)]
pub enum PhaseType {
    /// Execute once (ideation, architecture, deploy)
    Once,
    /// Iterative development loop — PM checkpoint after each sprint
    Sprint { max_iterations: usize },
    /// Go/No-Go gate — can loop back to a named phase on veto
    Gate { on_veto: Option<String> },
    /// QA → tickets → dev → re-QA feedback cycle
    FeedbackLoop { max_iterations: usize },
}

/// A single phase in the workflow plan
#[derive(Clone, Debug)]
pub struct PhaseDef {
    pub name: String,
    pub phase_type: PhaseType,
    pub pattern: String,
    pub agents: Vec<String>,
}

/// The full workflow plan produced by the PM
#[derive(Clone, Debug)]
pub struct WorkflowPlan {
    pub phases: Vec<PhaseDef>,
}

pub(crate) enum PhaseResult {
    Completed(String),
    Vetoed(String),
    Failed(String),
    #[allow(dead_code)]
    Skipped,
}

/// Strip emoji and decorative Unicode from LLM output
pub(crate) fn strip_emoji(text: &str) -> String {
    text.chars().filter(|c| {
        let cp = *c as u32;
        // Keep ASCII + Latin Extended + common punctuation + CJK
        cp < 0x2600 || // Basic Multilingual Plane below symbols
        (cp >= 0x3000 && cp < 0xFE00) || // CJK
        (cp >= 0xFF00 && cp < 0xFFF0)    // Fullwidth forms
    }).collect::<String>()
    .lines()
    .map(|l| l.trim_end())
    .collect::<Vec<_>>()
    .join("\n")
}

pub(crate) fn truncate_ctx(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max]) }
}

/// Emit a rich JSON event so Swift can display agent name, role, recipients, round.
pub(crate) fn emit_rich(on_event: &EventCallback, agent: &Agent, content: &str, to_agents: &[&str], round: usize) {
    let to_json: Vec<String> = to_agents.iter().map(|s| format!("\"{}\"", s)).collect();
    let json = format!(
        r#"{{"content":{},"agent_name":"{}","role":"{}","message_type":"response","to_agents":[{}],"round":{}}}"#,
        serde_json::to_string(content).unwrap_or_else(|_| format!("\"{}\"", content.replace('"', "\\\""))),
        agent.name.replace('"', "\\\""),
        agent.role.replace('"', "\\\""),
        to_json.join(","),
        round,
    );
    on_event(&agent.id, AgentEvent::Response { content: json });
}

pub(crate) fn store_agent_msg(mission_id: &str, phase_id: &str, agent_id: &str, agent_name: &str, role: &str, content: &str, tool: Option<&str>) {
    if let Err(e) = db::with_db(|conn| {
        conn.execute(
            "INSERT INTO agent_messages (mission_id, phase_id, agent_id, agent_name, role, content, tool_calls)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![mission_id, phase_id, agent_id, agent_name, role, content, tool],
        )
    }) {
        eprintln!("[db] Failed to store agent message: {}", e);
    }
}
