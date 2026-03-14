// Ref: FT-SSF-020

use super::patterns::{run_loop, run_network, run_parallel, run_sequential, run_solo};
use crate::agents;
use crate::executor::{self, AgentEvent, EventCallback};
use crate::guard;
use crate::protocols;

/// Hierarchical pattern: manager decomposes task into subtasks, workers execute them,
/// manager re-integrates results.
pub(crate) async fn run_hierarchical(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.is_empty() {
        return Err("Hierarchical pattern requires at least one agent".into());
    }
    let manager_id = agent_ids[0];
    let workers = &agent_ids[1..];

    let manager = agents::get_agent(manager_id).ok_or(format!("Agent {} not found", manager_id))?;

    // Step 1: Manager decomposes the task
    let worker_names: Vec<String> = workers.iter()
        .filter_map(|id| agents::get_agent(id).map(|a| format!("{} ({})", a.name, a.role)))
        .collect();

    let decompose_task = format!(
        "You are the team lead. Decompose this task into subtasks for your team.\n\
         Team: {}\n\nTask: {}\n\n\
         Output a numbered list of subtasks, one per worker. Format:\n\
         [SUBTASK 1] <description for worker 1>\n\
         [SUBTASK 2] <description for worker 2>\n\
         ... etc",
        worker_names.join(", "), task
    );

    on_event(&manager.id, AgentEvent::Response {
        content: "📋 Decomposing task into subtasks...".into(),
    });

    let manager_protocol = protocols::protocol_for_role(&manager.role, phase);
    let decomposition = executor::run_agent(
        &manager.id, &manager.name, &manager.persona, &manager.role,
        &decompose_task, workspace, mission_id, phase_id,
        Some(manager_protocol), on_event,
    ).await?;

    // Parse subtasks (lines starting with [SUBTASK N])
    let subtasks: Vec<&str> = decomposition.lines()
        .filter(|l| l.trim().starts_with("[SUBTASK"))
        .collect();

    // Step 2: Workers execute subtasks (or full decomposition if no parse)
    let mut worker_outputs = Vec::new();
    if workers.is_empty() {
        // Solo manager — just return the decomposition as the output
        return Ok(decomposition);
    }

    for (i, worker_id) in workers.iter().enumerate() {
        let worker = agents::get_agent(worker_id).ok_or(format!("Agent {} not found", worker_id))?;
        let subtask = if i < subtasks.len() {
            subtasks[i].to_string()
        } else {
            format!("Part of the team effort. Full plan:\n{}\n\nFocus on your area of expertise ({}).",
                decomposition, worker.role)
        };

        let worker_task = format!(
            "Task assigned by team lead ({}):\n{}\n\nOriginal goal: {}",
            manager.name, subtask, task
        );

        let worker_protocol = protocols::protocol_for_role(&worker.role, phase);
        let result = executor::run_agent(
            &worker.id, &worker.name, &worker.persona, &worker.role,
            &worker_task, workspace, mission_id, phase_id,
            Some(worker_protocol), on_event,
        ).await?;

        let guard_result = guard::check_l0(&result, &worker.role, &[]);
        if !guard_result.passed {
            on_event(&worker.id, AgentEvent::Response {
                content: format!("⚠️ Quality: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
            });
        }

        worker_outputs.push(format!("{} ({}):\n{}", worker.name, worker.role, result));
    }

    // Step 3: Manager re-integrates
    let integrate_task = format!(
        "You are the team lead. Your workers have completed their subtasks.\n\
         Original task: {}\n\n\
         Worker outputs:\n{}\n\n\
         Synthesize these into a coherent final deliverable. Resolve any conflicts.",
        task, worker_outputs.join("\n\n---\n\n")
    );

    on_event(&manager.id, AgentEvent::Response {
        content: "🔗 Re-integrating worker outputs...".into(),
    });

    let final_output = executor::run_agent(
        &manager.id, &manager.name, &manager.persona, &manager.role,
        &integrate_task, workspace, mission_id, phase_id,
        Some(manager_protocol), on_event,
    ).await?;

    Ok(final_output)
}

/// Aggregator pattern: all agents work independently (no dispatcher),
/// then a final aggregator consolidates all outputs.
pub(crate) async fn run_aggregator(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.is_empty() {
        return Err("Aggregator pattern requires at least one agent".into());
    }

    // All agents work independently on the same task
    let mut outputs = Vec::new();
    for agent_id in agent_ids.iter() {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let protocol = protocols::protocol_for_role(&agent.role, phase);
        let role_task = format!(
            "{}\n\nApproach this from your perspective as {}. Provide your independent analysis.",
            task, agent.role
        );

        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            &role_task, workspace, mission_id, phase_id,
            Some(protocol), on_event,
        ).await?;

        let guard_result = guard::check_l0(&result, &agent.role, &[]);
        if !guard_result.passed {
            on_event(&agent.id, AgentEvent::Response {
                content: format!("⚠️ Quality: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
            });
        }

        outputs.push(format!("{} ({}):\n{}", agent.name, agent.role, result));
    }

    // Last agent doubles as aggregator (or first agent if only one)
    let aggregator_id = agent_ids.last().unwrap();
    let aggregator = agents::get_agent(aggregator_id)
        .ok_or(format!("Agent {} not found", aggregator_id))?;

    let agg_task = format!(
        "Consolidate these independent analyses into a single coherent output.\n\
         Remove duplicates, resolve conflicts, and produce a unified deliverable.\n\n\
         Original task: {}\n\n\
         Contributions:\n{}",
        task, outputs.join("\n\n---\n\n")
    );
    let agg_protocol = protocols::protocol_for_role(&aggregator.role, phase);

    on_event(&aggregator.id, AgentEvent::Response {
        content: "📊 Aggregating all contributions...".into(),
    });

    let final_output = executor::run_agent(
        &aggregator.id, &aggregator.name, &aggregator.persona, &aggregator.role,
        &agg_task, workspace, mission_id, phase_id,
        Some(agg_protocol), on_event,
    ).await?;

    Ok(final_output)
}

/// Router pattern: first agent (router) reads the task and picks the best
/// specialist from the remaining agents. Only the chosen specialist runs.
pub(crate) async fn run_router(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.len() < 2 {
        return run_solo(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await;
    }
    let router_id = agent_ids[0];
    let candidates = &agent_ids[1..];

    let router = agents::get_agent(router_id).ok_or(format!("Agent {} not found", router_id))?;

    // Build candidate descriptions
    let mut candidate_desc = Vec::new();
    for (i, cid) in candidates.iter().enumerate() {
        if let Some(a) = agents::get_agent(cid) {
            candidate_desc.push(format!("{}. {} (id: {}, role: {}): {}", i + 1, a.name, a.id, a.role, a.persona));
        }
    }

    let route_task = format!(
        "You are a router. Read the task and choose the BEST specialist to handle it.\n\n\
         Task: {}\n\n\
         Available specialists:\n{}\n\n\
         Respond with ONLY the id of the chosen specialist (e.g. 'dev-emma'). Nothing else.",
        task, candidate_desc.join("\n")
    );

    on_event(&router.id, AgentEvent::Response {
        content: "🔀 Routing to best specialist...".into(),
    });

    let router_protocol = protocols::protocol_for_role(&router.role, phase);
    let chosen = executor::run_agent(
        &router.id, &router.name, &router.persona, &router.role,
        &route_task, workspace, mission_id, phase_id,
        Some(router_protocol), on_event,
    ).await?;

    // Parse chosen agent id from router's response
    let chosen_id = chosen.trim().trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');

    // Find matching candidate
    let specialist_id = candidates.iter()
        .find(|cid| chosen_id.contains(*cid) || cid.contains(chosen_id))
        .copied()
        .unwrap_or(candidates[0]); // fallback to first candidate

    let specialist = agents::get_agent(specialist_id)
        .ok_or(format!("Agent {} not found", specialist_id))?;

    on_event(&specialist.id, AgentEvent::Response {
        content: format!("🎯 Routed to {} ({})", specialist.name, specialist.role),
    });

    let spec_protocol = protocols::protocol_for_role(&specialist.role, phase);
    let result = executor::run_agent(
        &specialist.id, &specialist.name, &specialist.persona, &specialist.role,
        task, workspace, mission_id, phase_id,
        Some(spec_protocol), on_event,
    ).await?;

    let guard_result = guard::check_l0(&result, &specialist.role, &[]);
    if !guard_result.passed {
        on_event(&specialist.id, AgentEvent::Response {
            content: format!("⚠️ Quality: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
        });
    }

    Ok(format!("[Routed to {}]\n\n{}", specialist.name, result))
}

/// Wave pattern: agents are organized into dependency waves.
/// Wave 1 runs in parallel, wave 2 runs after wave 1 completes, etc.
/// Agent grouping: split into waves of ~equal size, each wave gets
/// cumulative context from prior waves.
pub(crate) async fn run_wave(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.is_empty() {
        return Err("Wave pattern requires at least one agent".into());
    }

    // Split agents into waves of 2-3 agents each
    let wave_size = if agent_ids.len() <= 3 { agent_ids.len() } else { 3 };
    let waves: Vec<&[&str]> = agent_ids.chunks(wave_size).collect();
    let total_waves = waves.len();

    let mut all_outputs = Vec::new();
    let mut wave_context = String::new();

    for (wave_idx, wave_agents) in waves.iter().enumerate() {
        on_event("engine", AgentEvent::Response {
            content: format!("🌊 Wave {}/{} ({} agents)", wave_idx + 1, total_waves, wave_agents.len()),
        });

        // All agents in wave run with same context
        let mut wave_outputs = Vec::new();
        for agent_id in wave_agents.iter() {
            let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
            let protocol = protocols::protocol_for_role(&agent.role, phase);

            let wave_task = if wave_context.is_empty() {
                task.to_string()
            } else {
                format!("{}\n\nPrevious wave outputs:\n{}", task, wave_context)
            };

            let result = executor::run_agent(
                &agent.id, &agent.name, &agent.persona, &agent.role,
                &wave_task, workspace, mission_id, phase_id,
                Some(protocol), on_event,
            ).await?;

            let guard_result = guard::check_l0(&result, &agent.role, &[]);
            if !guard_result.passed {
                on_event(&agent.id, AgentEvent::Response {
                    content: format!("⚠️ Quality: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
                });
            }

            wave_outputs.push(format!("{}: {}", agent.name, result));
        }

        let wave_combined = wave_outputs.join("\n\n");
        wave_context = if wave_context.is_empty() {
            wave_combined.clone()
        } else {
            format!("{}\n\n---\n\n{}", wave_context, wave_combined)
        };
        all_outputs.extend(wave_outputs);
    }

    Ok(all_outputs.join("\n\n---\n\n"))
}

// ── Public test wrappers for pattern functions ──

pub async fn run_sequential_test(
    agent_ids: &[&str], task: &str, phase: &str, workspace: &str,
    mission_id: &str, phase_id: &str, on_event: &EventCallback,
) -> Result<String, String> {
    run_sequential(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await
}

pub async fn run_parallel_test(
    agent_ids: &[&str], task: &str, phase: &str, workspace: &str,
    mission_id: &str, phase_id: &str, on_event: &EventCallback,
) -> Result<String, String> {
    run_parallel(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await
}

pub async fn run_network_test(
    agent_ids: &[&str], task: &str, phase: &str, workspace: &str,
    mission_id: &str, phase_id: &str, on_event: &EventCallback,
) -> Result<String, String> {
    run_network(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await
}

pub async fn run_solo_test(
    agent_ids: &[&str], task: &str, phase: &str, workspace: &str,
    mission_id: &str, phase_id: &str, on_event: &EventCallback,
) -> Result<String, String> {
    run_solo(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await
}

pub async fn run_loop_test(
    agent_ids: &[&str], task: &str, phase: &str, workspace: &str,
    mission_id: &str, phase_id: &str, on_event: &EventCallback,
) -> Result<String, String> {
    run_loop(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await
}

pub async fn run_hierarchical_test(
    agent_ids: &[&str], task: &str, phase: &str, workspace: &str,
    mission_id: &str, phase_id: &str, on_event: &EventCallback,
) -> Result<String, String> {
    run_hierarchical(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await
}

pub async fn run_aggregator_test(
    agent_ids: &[&str], task: &str, phase: &str, workspace: &str,
    mission_id: &str, phase_id: &str, on_event: &EventCallback,
) -> Result<String, String> {
    run_aggregator(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await
}

pub async fn run_router_test(
    agent_ids: &[&str], task: &str, phase: &str, workspace: &str,
    mission_id: &str, phase_id: &str, on_event: &EventCallback,
) -> Result<String, String> {
    run_router(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await
}

pub async fn run_wave_test(
    agent_ids: &[&str], task: &str, phase: &str, workspace: &str,
    mission_id: &str, phase_id: &str, on_event: &EventCallback,
) -> Result<String, String> {
    run_wave(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await
}
