// Ref: FT-SSF-020

use super::patterns_ext::{run_aggregator, run_hierarchical, run_router, run_wave};
use super::types::*;
use crate::agents::{self, Agent};
use crate::executor::{self, AgentEvent, EventCallback};
use crate::guard;
use crate::llm::{self, LLMMessage};
use crate::protocols;

/// Dispatch to the correct pattern implementation
pub(crate) async fn run_pattern(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    pattern: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    match pattern {
        "network" | "debate" => run_network(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
        "parallel" => run_parallel(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
        "solo" => run_solo(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
        "loop" | "adversarial-pair" => run_loop(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
        "hierarchical" => run_hierarchical(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
        "aggregator" => run_aggregator(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
        "router" => run_router(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
        "wave" => run_wave(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
        _ => run_sequential(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
    }
}

/// Network pattern: agents discuss in rounds (like the Python SF's run_network).
/// Used for vision and review phases.
pub(crate) async fn run_network(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    _workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    let mut agents_data: Vec<Agent> = Vec::new();
    for id in agent_ids {
        if let Some(a) = agents::get_agent(id) {
            agents_data.push(a);
        }
    }
    if agents_data.is_empty() {
        return Err("No agents found".into());
    }

    let team_list: Vec<String> = agents_data.iter()
        .map(|a| format!("@{} ({})", a.name, a.role))
        .collect();

    let leader = &agents_data[0];
    let debaters: Vec<&Agent> = agents_data.iter().skip(1).collect();

    let mut all_outputs: Vec<String> = Vec::new();

    // ── Leader brief ──
    on_event(&leader.id, AgentEvent::Thinking);
    let leader_prompt = format!(
        "Tu diriges cette phase {} en tant que {}. Ton équipe: {}.\n\n\
         1. Cadre la phase en 2-3 phrases\n\
         2. Dis à chaque membre ce que tu attends de lui\n\n\
         Sujet:\n{}",
        phase, leader.role, team_list.join(", "), task
    );

    let leader_system = format!(
        "{}\n\n{}\n\n{}",
        leader.persona, protocols::protocol_for_role(&leader.role, phase), STYLE_RULES
    );

    let leader_result = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: leader_prompt }],
        Some(&leader_system),
        None,
    ).await?;
    let leader_content = strip_emoji(&leader_result.content.unwrap_or_default());
    let other_ids: Vec<&str> = debaters.iter().map(|a| a.id.as_str()).collect();
    emit_rich(on_event, leader, &leader_content, &other_ids, 0);
    store_agent_msg(mission_id, phase_id, &leader.id, &leader.name, "assistant", &leader_content, None);
    all_outputs.push(format!("**{} ({})**: {}", leader.name, leader.role, leader_content));

    // ── Discussion rounds ──
    let mut prev_round = leader_content;

    for round in 0..MAX_NETWORK_ROUNDS {
        // All participants respond
        for agent in agents_data.iter() {
            on_event(&agent.id, AgentEvent::Thinking);

            let prompt = if round == 0 {
                format!(
                    "Le responsable a briefé l'équipe. Réponds en tant que {}. \
                     Donne ton analyse et tes recommandations.\n\n\
                     [Brief]:\n{}\n\nSujet:\n{}",
                    agent.role, prev_round, task
                )
            } else {
                format!(
                    "Poursuis la discussion (round {}). Réagis aux points des collègues, \
                     affine tes recommandations.\n\n\
                     [Échanges précédents]:\n{}\n\nSujet:\n{}",
                    round + 1, prev_round, task
                )
            };

            let system = format!(
                "{}\n\n{}\n\n{}",
                agent.persona, protocols::protocol_for_role(&agent.role, phase), STYLE_RULES
            );

            let result = llm::chat_completion(
                &[LLMMessage { role: "user".into(), content: prompt }],
                Some(&system),
                None,
            ).await;

            let content = match result {
                Ok(r) => strip_emoji(&r.content.unwrap_or_default()),
                Err(e) => return Err(format!("LLM error for {}: {}", agent.name, e)),
            };

            // Recipients: all other agents in the team
            let to: Vec<&str> = agents_data.iter()
                .filter(|a| a.id != agent.id)
                .map(|a| a.id.as_str())
                .collect();
            emit_rich(on_event, agent, &content, &to, round + 1);
            store_agent_msg(mission_id, phase_id, &agent.id, &agent.name, "assistant", &content, None);
            all_outputs.push(format!("**{} ({})**: {}", agent.name, agent.role, content));
        }

        // Build context for next round (with budget)
        prev_round = all_outputs.iter()
            .rev()
            .take(agents_data.len() * 2) // Keep recent outputs
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        if prev_round.len() > CONTEXT_BUDGET {
            prev_round = prev_round[..CONTEXT_BUDGET].to_string();
        }
    }

    // ── Leader synthesis ──
    on_event(&leader.id, AgentEvent::Thinking);
    let synthesis_prompt = format!(
        "Synthétise les contributions de l'équipe pour cette phase {}.\n\
         1. Résume les points clés\n\
         2. Identifie consensus et désaccords\n\
         3. Décision finale: [APPROVE] ou [VETO] avec justification\n\n\
         Contributions:\n{}",
        phase, prev_round
    );

    let synthesis = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: synthesis_prompt }],
        Some(&format!("{}\n\n{}\n\n{}", leader.persona, protocols::protocol_for_role(&leader.role, phase), STYLE_RULES)),
        None,
    ).await?;
    let synthesis_content = strip_emoji(&synthesis.content.unwrap_or_default());
    let all_ids: Vec<&str> = agents_data.iter().map(|a| a.id.as_str()).collect();
    emit_rich(on_event, leader, &synthesis_content, &all_ids, MAX_NETWORK_ROUNDS + 1);
    store_agent_msg(mission_id, phase_id, &leader.id, &leader.name, "assistant", &synthesis_content, None);
    all_outputs.push(synthesis_content.clone());

    Ok(all_outputs.join("\n\n---\n\n"))
}

/// Sequential pattern with protocol injection and adversarial guard.
pub(crate) async fn run_sequential(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    let mut outputs = Vec::new();
    let mut cumulative_task = task.to_string();

    for agent_id in agent_ids {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let protocol = protocols::protocol_for_role(&agent.role, phase);

        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            &cumulative_task, workspace, mission_id, phase_id,
            Some(protocol), on_event,
        ).await?;

        // L0 adversarial guard check
        let guard_result = guard::check_l0(&result, &agent.role, &[]);
        if !guard_result.passed {
            let issues = guard_result.issues.join(", ");
            on_event(&agent.id, AgentEvent::Response {
                content: format!("⚠️ Quality check: {} (score: {})", issues, guard_result.score),
            });
        }

        cumulative_task = format!("{}\n\nPrevious agent ({}) output:\n{}", task, agent.name, result);
        outputs.push(format!("{}: {}", agent.name, result));
    }

    Ok(outputs.join("\n\n---\n\n"))
}

/// Parallel pattern: agents work independently on the same task.
pub(crate) async fn run_parallel(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    let mut outputs = Vec::new();

    for agent_id in agent_ids {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let protocol = protocols::protocol_for_role(&agent.role, phase);
        let role_task = format!(
            "{}\n\nYou are the {} on this team. Focus on your area of expertise.",
            task, agent.role
        );

        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            &role_task, workspace, mission_id, phase_id,
            Some(protocol), on_event,
        ).await?;

        // L0 guard
        let guard_result = guard::check_l0(&result, &agent.role, &[]);
        if !guard_result.passed {
            on_event(&agent.id, AgentEvent::Response {
                content: format!("⚠️ Quality: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
            });
        }

        outputs.push(format!("{}: {}", agent.name, result));
    }

    Ok(outputs.join("\n\n---\n\n"))
}

/// Solo pattern: single agent runs the full task.
pub(crate) async fn run_solo(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    let agent_id = agent_ids.first().ok_or("Solo pattern requires at least one agent")?;
    let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
    let protocol = protocols::protocol_for_role(&agent.role, phase);

    let result = executor::run_agent(
        &agent.id, &agent.name, &agent.persona, &agent.role,
        task, workspace, mission_id, phase_id,
        Some(protocol), on_event,
    ).await?;

    let guard_result = guard::check_l0(&result, &agent.role, &[]);
    if !guard_result.passed {
        on_event(&agent.id, AgentEvent::Response {
            content: format!("⚠️ Quality: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
        });
    }

    Ok(result)
}

/// Loop pattern: writer ↔ reviewer cycle, up to max_iterations.
/// First agent writes, second agent reviews. If reviewer vetoes, writer retries
/// with reviewer's feedback until approval or max iterations.
pub(crate) async fn run_loop(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    const MAX_LOOP_ITERATIONS: usize = 5;

    let (writer_id, reviewer_id) = match agent_ids {
        [w, r, ..] => (*w, *r),
        [w] => (*w, *w),
        [] => return Err("Loop pattern requires at least one agent".into()),
    };

    let writer = agents::get_agent(writer_id).ok_or(format!("Agent {} not found", writer_id))?;
    let reviewer = agents::get_agent(reviewer_id).ok_or(format!("Agent {} not found", reviewer_id))?;

    let mut current_task = task.to_string();
    let mut last_output = String::new();

    for iteration in 1..=MAX_LOOP_ITERATIONS {
        on_event(&writer.id, AgentEvent::Response {
            content: format!("🔄 Loop iteration {}/{}", iteration, MAX_LOOP_ITERATIONS),
        });

        // Writer produces
        let writer_protocol = protocols::protocol_for_role(&writer.role, phase);
        let writer_output = executor::run_agent(
            &writer.id, &writer.name, &writer.persona, &writer.role,
            &current_task, workspace, mission_id, phase_id,
            Some(writer_protocol), on_event,
        ).await?;

        let guard_result = guard::check_l0(&writer_output, &writer.role, &[]);
        if !guard_result.passed && guard_result.score >= 7 {
            on_event(&writer.id, AgentEvent::Response {
                content: format!("⚠️ Quality reject: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
            });
            current_task = format!("{}\n\nPrevious attempt rejected (quality check): {}\nPlease fix and retry.",
                task, guard_result.issues.join(", "));
            continue;
        }

        // Reviewer evaluates
        let review_task = format!(
            "Review this output from {} for phase '{}':\n\n{}\n\nOriginal task: {}\n\nRespond with VERDICT: GO if acceptable, or VERDICT: NOGO with specific feedback for improvement.",
            writer.name, phase, writer_output, task
        );
        let reviewer_protocol = protocols::protocol_for_role(&reviewer.role, phase);
        let review_output = executor::run_agent(
            &reviewer.id, &reviewer.name, &reviewer.persona, &reviewer.role,
            &review_task, workspace, mission_id, phase_id,
            Some(reviewer_protocol), on_event,
        ).await?;

        last_output = writer_output.clone();

        // Check reviewer's verdict
        let gate = super::workflow::check_gate_raw(&review_output);
        if gate != "vetoed" {
            on_event(&reviewer.id, AgentEvent::Response {
                content: format!("✅ Approved at iteration {}", iteration),
            });
            return Ok(format!("{}\n\n---\nReview ({}): {}", writer_output, reviewer.name, review_output));
        }

        // Veto — feed back reviewer's comments for next iteration
        on_event(&reviewer.id, AgentEvent::Response {
            content: format!("🔁 Requesting revision (iteration {})", iteration),
        });
        current_task = format!(
            "{}\n\nYour previous output was reviewed and needs revision.\nReviewer ({}) feedback:\n{}\n\nPlease address the feedback and produce an improved version.",
            task, reviewer.name, review_output
        );
    }

    // Max iterations reached — return last output with warning
    on_event(&writer.id, AgentEvent::Response {
        content: format!("⚠️ Loop exhausted after {} iterations", MAX_LOOP_ITERATIONS),
    });
    Ok(format!("(max iterations reached)\n\n{}", last_output))
}
