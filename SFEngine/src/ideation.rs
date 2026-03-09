use crate::llm::{self, LLMMessage};
use crate::db;
use crate::executor::{AgentEvent, EventCallback};
use rusqlite::params;

/// Default ideation agents — configurable via run_ideation_with_team (#9)
const DEFAULT_IDEATION_AGENTS: &[(&str, &str, &str)] = &[
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

/// Default: 3 agents, 3 rounds
pub async fn run_ideation(
    session_id: &str,
    idea: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    run_ideation_rounds(session_id, idea, DEFAULT_IDEATION_AGENTS, 3, on_event).await
}

/// Configurable ideation: custom agents + round count (#9)
pub async fn run_ideation_rounds(
    session_id: &str,
    idea: &str,
    agents: &[(&str, &str, &str)],
    rounds: usize,
    on_event: &EventCallback,
) -> Result<String, String> {
    let round_labels = ["Initial Analysis", "Discussion & Debate", "Final Recommendations",
                        "Deep Dive", "Convergence"];
    let mut discussion: Vec<(String, String, String)> = Vec::new();

    for round in 0..rounds {
        let label = round_labels.get(round).unwrap_or(&"Additional Round");
        on_event("engine", AgentEvent::Response {
            content: format!("--- Round {}: {} ---", round + 1, label),
        });

        for (agent_id, agent_name, persona) in agents {
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
                r if r == rounds - 1 => format!(
                    "Based on the full discussion, give your final synthesis and concrete \
                     recommendation. What should we build? Key risks? Go/No-go? (200 words max)\n\n\
                     **Idea:** {}{}",
                    idea, context
                ),
                _ => format!(
                    "The team has shared perspectives on this idea. \
                     React to their analysis. Challenge weak points, reinforce good ideas, \
                     add what was missed. Be specific and constructive (200 words max).\n\n\
                     **Idea:** {}{}",
                    idea, context
                ),
            };

            let messages = vec![LLMMessage {
                role: "user".into(),
                content: prompt,
            }];

            let system = format!(
                "{}\n\nYou are in a team ideation session (round {}/{}). \
                 Respond concisely from your role's perspective. No tool calls needed.",
                persona, round + 1, rounds
            );

            let resp = llm::chat_completion(&messages, Some(&system), None).await;

            let content = match resp {
                Ok(r) => r.content.unwrap_or_else(|| "(no response)".into()),
                Err(e) => format!("(error: {})", e),
            };

            on_event(agent_id, AgentEvent::Response {
                content: content.clone(),
            });

            if let Err(e) = db::with_db(|conn| {
                conn.execute(
                    "INSERT INTO ideation_messages (session_id, agent_id, agent_name, round, content)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![session_id, agent_id, agent_name, round as i32, &content],
                )
            }) {
                eprintln!("[db] Failed to store ideation message: {}", e);
            }

            discussion.push((agent_id.to_string(), agent_name.to_string(), content));
        }
    }

    let synthesis: Vec<String> = discussion.iter()
        .rev()
        .take(agents.len())
        .rev()
        .map(|(_, name, content)| format!("**{}**: {}", name, content))
        .collect();

    Ok(synthesis.join("\n\n"))
}
