use crate::llm::{self, LLMMessage};
use crate::db;
use crate::executor::{AgentEvent, EventCallback};
use rusqlite::params;

/// Ideation agents — network discussion pattern
const IDEATION_AGENTS: &[(&str, &str, &str)] = &[
    (
        "ideation-pm",
        "Product Manager",
        "You are an experienced Product Manager. Analyze ideas from a product perspective: \
         user value, market fit, MVP scope, risks, competitive landscape. \
         Build on or challenge other agents' views when they share their perspectives."
    ),
    (
        "ideation-tech",
        "Tech Lead",
        "You are a senior Tech Lead and architect. Analyze ideas technically: \
         recommended stack, architecture, key challenges, estimated complexity, scalability. \
         Build on or challenge other agents' views when they share their perspectives."
    ),
    (
        "ideation-ux",
        "UX Designer",
        "You are a senior UX/Product Designer. Analyze ideas from a user experience perspective: \
         core user flows, key screens, accessibility, UX pitfalls, design system choices. \
         Build on or challenge other agents' views when they share their perspectives."
    ),
];

/// Run a network-pattern ideation session — 3 rounds of multi-agent discussion
///
/// Round 1: Each agent gives initial perspective on the idea
/// Round 2: Each agent reacts to others, debates, challenges
/// Round 3: Each agent gives final synthesis and recommendation
pub async fn run_ideation(
    session_id: &str,
    idea: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    let round_labels = ["Initial Analysis", "Discussion & Debate", "Final Recommendations"];
    let mut discussion: Vec<(String, String, String)> = Vec::new(); // (agent_id, agent_name, content)

    for (round, label) in round_labels.iter().enumerate() {
        on_event("engine", AgentEvent::Response {
            content: format!("--- Round {}: {} ---", round + 1, label),
        });

        for (agent_id, agent_name, persona) in IDEATION_AGENTS {
            on_event(agent_id, AgentEvent::Thinking);

            let context = if discussion.is_empty() {
                String::new()
            } else {
                let parts: Vec<String> = discussion.iter()
                    .map(|(_, name, content)| format!("**{}**:\n{}", name, content))
                    .collect();
                format!("\n\n## Previous discussion:\n\n{}", parts.join("\n\n---\n\n"))
            };

            let prompt = match round {
                0 => format!(
                    "Analyze this idea from your expertise. Be concise (200 words max).\n\n\
                     **Idea:** {}",
                    idea
                ),
                1 => format!(
                    "The team has shared initial perspectives on this idea. \
                     React to their analysis. Challenge weak points, reinforce good ideas, \
                     add what was missed. Be specific and constructive (200 words max).\n\n\
                     **Idea:** {}{}", 
                    idea, context
                ),
                2 => format!(
                    "Based on the full discussion, give your final synthesis and concrete \
                     recommendation. What should we build? Key risks? Go/No-go? (200 words max)\n\n\
                     **Idea:** {}{}",
                    idea, context
                ),
                _ => unreachable!(),
            };

            let messages = vec![LLMMessage {
                role: "user".into(),
                content: prompt,
            }];

            let system = format!(
                "{}\n\nYou are in a team ideation session (round {}/3: {}). \
                 Respond concisely from your role's perspective. No tool calls needed.",
                persona, round + 1, label
            );

            let resp = llm::chat_completion(&messages, Some(&system), None).await;

            let content = match resp {
                Ok(r) => r.content.unwrap_or_else(|| "(no response)".into()),
                Err(e) => format!("(error: {})", e),
            };

            // Emit event to Swift
            on_event(agent_id, AgentEvent::Response {
                content: content.clone(),
            });

            // Store in DB
            db::with_db(|conn| {
                conn.execute(
                    "INSERT INTO ideation_messages (session_id, agent_id, agent_name, round, content)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![session_id, agent_id, agent_name, round as i32, &content],
                ).ok();
            });

            discussion.push((agent_id.to_string(), agent_name.to_string(), content));
        }
    }

    // Final synthesis: last 3 messages (round 3)
    let synthesis: Vec<String> = discussion.iter()
        .rev()
        .take(IDEATION_AGENTS.len())
        .rev()
        .map(|(_, name, content)| format!("**{}**: {}", name, content))
        .collect();

    Ok(synthesis.join("\n\n"))
}
