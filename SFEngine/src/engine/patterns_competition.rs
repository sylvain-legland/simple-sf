// Ref: FT-SSF-020
//
// Competition orchestration patterns: tournament, voting, escalation, speculative.

use crate::agents;
use crate::executor::{self, AgentEvent, EventCallback};
use crate::guard;
use crate::llm::{self, LLMMessage};
use crate::protocols;

/// Tournament pattern: bracket-style competition between agents.
/// Agents are paired, a judge picks the winner of each match, winners advance.
pub(crate) async fn run_tournament(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.len() <= 1 {
        return super::patterns::run_solo(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await;
    }
    eprintln!("[TOURNAMENT] Starting with {} agents", agent_ids.len());

    // Collect initial competitors with their outputs
    let mut competitors: Vec<(String, String, String)> = Vec::new(); // (id, name, output)
    for agent_id in agent_ids {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let protocol = protocols::protocol_for_role(&agent.role, phase);
        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            task, workspace, mission_id, phase_id,
            Some(protocol), on_event,
        ).await?;
        competitors.push((agent.id.clone(), agent.name.clone(), result));
    }

    // Bracket rounds until one champion remains
    let mut round = 0;
    while competitors.len() > 1 {
        round += 1;
        eprintln!("[TOURNAMENT] Round {} — {} competitors", round, competitors.len());
        let mut winners = Vec::new();

        for pair in competitors.chunks(2) {
            if pair.len() == 1 {
                eprintln!("[TOURNAMENT] Bye for {}", pair[0].1);
                winners.push(pair[0].clone());
                continue;
            }

            let judge_prompt = format!(
                "You are an impartial judge. Compare these two solutions to the task.\n\n\
                 Task: {}\n\n\
                 [SOLUTION 1 — {}]:\n{}\n\n\
                 [SOLUTION 2 — {}]:\n{}\n\n\
                 Pick the better solution. Respond with [WINNER: 1] or [WINNER: 2] and a brief justification.",
                task, pair[0].1, pair[0].2, pair[1].1, pair[1].2,
            );

            let verdict = llm::chat_completion(
                &[LLMMessage { role: "user".into(), content: judge_prompt }],
                Some("You are a strict technical judge. Always end with [WINNER: 1] or [WINNER: 2]."),
                None,
            ).await?;
            let verdict_text = verdict.content.unwrap_or_default();

            let winner_idx = if verdict_text.contains("[WINNER: 2]") { 1 } else { 0 };
            eprintln!("[TOURNAMENT] {} vs {} → winner: {}", pair[0].1, pair[1].1, pair[winner_idx].1);

            on_event(&pair[winner_idx].0, AgentEvent::Response {
                content: format!("🏆 Won round {} against {}", round, pair[1 - winner_idx].1),
            });

            winners.push(pair[winner_idx].clone());
        }
        competitors = winners;
    }

    let champion = &competitors[0];
    eprintln!("[TOURNAMENT] Champion: {}", champion.1);
    on_event(&champion.0, AgentEvent::Response {
        content: format!("🥇 Tournament champion: {}", champion.1),
    });

    Ok(champion.2.clone())
}

/// Voting pattern: all agents produce solutions, then cross-vote with scores.
/// Solution with highest total score wins.
pub(crate) async fn run_voting(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.len() <= 1 {
        return super::patterns::run_solo(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await;
    }
    eprintln!("[VOTING] Starting with {} agents", agent_ids.len());

    // Step 1: Each agent produces a solution
    let mut solutions: Vec<(String, String)> = Vec::new(); // (agent_name, output)
    for agent_id in agent_ids {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let protocol = protocols::protocol_for_role(&agent.role, phase);
        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            task, workspace, mission_id, phase_id,
            Some(protocol), on_event,
        ).await?;
        solutions.push((agent.name.clone(), result));
    }

    // Step 2: Each agent votes on all solutions
    let mut scores: Vec<i32> = vec![0; solutions.len()];
    let solutions_text: String = solutions.iter().enumerate()
        .map(|(i, (name, output))| format!("[SOLUTION {} — {}]:\n{}", i + 1, name, output))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    for agent_id in agent_ids {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let vote_prompt = format!(
            "Score each solution from 1 to 10. For each, write [SCORE: N] on its own line.\n\n\
             Task: {}\n\n{}\n\n\
             Provide exactly {} scores, one per solution, in order.",
            task, solutions_text, solutions.len(),
        );

        on_event(&agent.id, AgentEvent::Thinking);
        let vote_result = llm::chat_completion(
            &[LLMMessage { role: "user".into(), content: vote_prompt }],
            Some(&format!("{}\n\nYou are voting on solutions. Be fair and objective.", agent.persona)),
            None,
        ).await?;
        let vote_text = vote_result.content.unwrap_or_default();

        // Parse [SCORE: N] markers
        let parsed: Vec<i32> = vote_text.lines()
            .filter_map(|line| {
                let upper = line.to_uppercase();
                if let Some(pos) = upper.find("[SCORE:") {
                    let after = &upper[pos + 7..];
                    let num_str: String = after.chars().take_while(|c| c.is_ascii_digit() || *c == ' ').collect();
                    num_str.trim().parse::<i32>().ok()
                } else {
                    None
                }
            })
            .collect();

        for (i, &s) in parsed.iter().enumerate() {
            if i < scores.len() {
                scores[i] += s.clamp(1, 10);
            }
        }
        eprintln!("[VOTING] {} voted: {:?}", agent.name, parsed);
    }

    // Pick winner (highest total, first on tie)
    let winner_idx = scores.iter().enumerate()
        .max_by_key(|(_, s)| *s)
        .map(|(i, _)| i)
        .unwrap_or(0);

    eprintln!("[VOTING] Scores: {:?} → winner: {} (idx={})", scores, solutions[winner_idx].0, winner_idx);
    on_event("engine", AgentEvent::Response {
        content: format!("🗳️ Voting result: {} wins (score: {})", solutions[winner_idx].0, scores[winner_idx]),
    });

    Ok(solutions[winner_idx].1.clone())
}

/// Escalation pattern: agents try in order of seniority, escalating on guard failure.
/// Each subsequent agent receives feedback about why the previous attempt failed.
pub(crate) async fn run_escalation(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.is_empty() {
        return Err("Escalation pattern requires at least one agent".into());
    }
    eprintln!("[ESCALATION] Starting chain with {} agents", agent_ids.len());

    let mut current_task = task.to_string();

    for (level, agent_id) in agent_ids.iter().enumerate() {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let is_last = level == agent_ids.len() - 1;

        eprintln!("[ESCALATION] Level {} — {} ({})", level + 1, agent.name, agent.role);
        on_event(&agent.id, AgentEvent::Response {
            content: format!("📈 Escalation level {}/{}", level + 1, agent_ids.len()),
        });

        let protocol = protocols::protocol_for_role(&agent.role, phase);
        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            &current_task, workspace, mission_id, phase_id,
            Some(protocol), on_event,
        ).await?;

        // Last agent is final escalation — always return
        if is_last {
            eprintln!("[ESCALATION] Final level reached — returning output from {}", agent.name);
            return Ok(result);
        }

        // Guard check
        let guard_result = guard::check_l0(&result, &agent.role, &[]);
        if guard_result.passed && result.len() > 100 {
            eprintln!("[ESCALATION] Passed at level {} (score: {})", level + 1, guard_result.score);
            on_event(&agent.id, AgentEvent::Response {
                content: format!("✅ Accepted at level {} (guard score: {})", level + 1, guard_result.score),
            });
            return Ok(result);
        }

        // Failed — build escalation context for next agent
        let reason = if !guard_result.passed {
            format!("Guard failed (score: {}): {}", guard_result.score, guard_result.issues.join(", "))
        } else {
            "Output too short (< 100 chars)".into()
        };

        eprintln!("[ESCALATION] Level {} failed: {}", level + 1, reason);
        on_event(&agent.id, AgentEvent::Response {
            content: format!("⚠️ Escalating: {}", reason),
        });

        current_task = format!(
            "{}\n\nPrevious attempt by {} (level {}) was insufficient.\n\
             Reason: {}\n\nPrevious output:\n{}\n\n\
             Please provide an improved, more thorough response.",
            task, agent.name, level + 1, reason, result,
        );
    }

    Err("Escalation chain exhausted without result".into())
}

/// Speculative pattern: all agents execute in parallel, first passing guard wins.
/// Falls back to lowest-scoring output if none pass.
pub(crate) async fn run_speculative(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.is_empty() {
        return Err("Speculative pattern requires at least one agent".into());
    }
    eprintln!("[SPECULATIVE] Launching {} agents in parallel", agent_ids.len());

    // Execute all agents (sequential simulation — Rust async, no tokio::spawn needed)
    let mut results: Vec<(String, String, i32)> = Vec::new(); // (agent_name, output, guard_score)

    for agent_id in agent_ids {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let protocol = protocols::protocol_for_role(&agent.role, phase);

        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            task, workspace, mission_id, phase_id,
            Some(protocol), on_event,
        ).await?;

        let guard_result = guard::check_l0(&result, &agent.role, &[]);
        eprintln!("[SPECULATIVE] {} → guard score: {} (passed: {})", agent.name, guard_result.score, guard_result.passed);

        if !guard_result.passed {
            on_event(&agent.id, AgentEvent::Response {
                content: format!("⚠️ Quality: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
            });
        }

        // Early return: first output that passes with score < 5
        if guard_result.passed && guard_result.score < 5 {
            eprintln!("[SPECULATIVE] Early winner: {} (score: {})", agent.name, guard_result.score);
            on_event(&agent.id, AgentEvent::Response {
                content: format!("⚡ Speculative winner (score: {})", guard_result.score),
            });
            return Ok(result);
        }

        results.push((agent.name.clone(), result, guard_result.score));
    }

    // No early winner — pick output with lowest guard score
    let best = results.iter()
        .min_by_key(|(_, _, score)| *score)
        .ok_or("No results collected")?;

    eprintln!("[SPECULATIVE] No clean pass — best: {} (score: {})", best.0, best.2);
    on_event("engine", AgentEvent::Response {
        content: format!("📊 Speculative fallback: {} (lowest score: {})", best.0, best.2),
    });

    Ok(best.1.clone())
}
