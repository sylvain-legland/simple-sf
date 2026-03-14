// Ref: FT-SSF-020
//
// Distributed orchestration patterns: blackboard, map-reduce, composite.

use crate::agents;
use crate::executor::{self, AgentEvent, EventCallback};
use crate::guard;
use crate::protocols;

/// Blackboard pattern: agents iteratively contribute to a shared workspace.
pub(crate) async fn run_blackboard(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    eprintln!("[BLACKBOARD] Starting with {} agents, max 3 rounds", agent_ids.len());

    let agents_data: Vec<_> = agent_ids.iter()
        .map(|id| agents::get_agent(id).ok_or(format!("Agent {} not found", id)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut blackboard = String::new();
    const MAX_ROUNDS: usize = 3;

    for round in 0..MAX_ROUNDS {
        eprintln!("[BLACKBOARD] Round {}/{}", round + 1, MAX_ROUNDS);

        for agent in &agents_data {
            let protocol = protocols::protocol_for_role(&agent.role, phase);
            let prompt = format!(
                "Task: {}\n\nCurrent blackboard state:\n{}\n\n\
                 Read the blackboard and contribute your additions or modifications. \
                 Focus on your expertise as {}. Round {}/{}.",
                task,
                if blackboard.is_empty() { "(empty)" } else { &blackboard },
                agent.role, round + 1, MAX_ROUNDS,
            );

            let result = executor::run_agent(
                &agent.id, &agent.name, &agent.persona, &agent.role,
                &prompt, workspace, mission_id, phase_id,
                Some(protocol), on_event,
            ).await?;

            let guard_result = guard::check_l0(&result, &agent.role, &[]);
            if !guard_result.passed {
                on_event(&agent.id, AgentEvent::Response {
                    content: format!("⚠️ Quality: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
                });
            }

            blackboard.push_str(&format!(
                "\n\n[CONTRIBUTION from {} (round {})]:\n{}",
                agent.name, round + 1, result
            ));
        }
    }

    // Synthesize: first agent produces final coherent output
    let synthesizer = &agents_data[0];
    let synth_protocol = protocols::protocol_for_role(&synthesizer.role, phase);
    let synth_prompt = format!(
        "Synthesize this blackboard into a single coherent output:\n\n{}\n\nOriginal task: {}",
        blackboard, task,
    );

    let synthesis = executor::run_agent(
        &synthesizer.id, &synthesizer.name, &synthesizer.persona, &synthesizer.role,
        &synth_prompt, workspace, mission_id, phase_id,
        Some(synth_protocol), on_event,
    ).await?;

    eprintln!("[BLACKBOARD] Synthesis complete");
    Ok(synthesis)
}

/// Map-Reduce pattern: split task into subtasks, execute in parallel, aggregate.
pub(crate) async fn run_map_reduce(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.len() < 2 {
        return Err("Map-reduce requires at least 2 agents".into());
    }
    eprintln!("[MAP_REDUCE] Starting with {} agents", agent_ids.len());

    let mapper_id = agent_ids[0];
    let workers = &agent_ids[1..];

    // Map phase: first agent decomposes the task
    let mapper = agents::get_agent(mapper_id).ok_or(format!("Agent {} not found", mapper_id))?;
    let mapper_protocol = protocols::protocol_for_role(&mapper.role, phase);
    let split_prompt = format!(
        "Split this task into {} independent subtasks. Mark each with [SUBTASK N] header.\n\nTask: {}",
        workers.len(), task,
    );

    let decomposition = executor::run_agent(
        &mapper.id, &mapper.name, &mapper.persona, &mapper.role,
        &split_prompt, workspace, mission_id, phase_id,
        Some(mapper_protocol), on_event,
    ).await?;

    // Parse subtasks from [SUBTASK N] markers
    let mut subtasks: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in decomposition.lines() {
        if line.trim().starts_with("[SUBTASK") {
            if !current.trim().is_empty() {
                subtasks.push(current.trim().to_string());
            }
            current = String::new();
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }
    if !current.trim().is_empty() {
        subtasks.push(current.trim().to_string());
    }

    // Fallback: round-robin split if no markers found
    if subtasks.is_empty() {
        eprintln!("[MAP_REDUCE] No subtask markers, splitting by agent count");
        let lines: Vec<&str> = task.lines().collect();
        let chunk_size = (lines.len() / workers.len()).max(1);
        for chunk in lines.chunks(chunk_size) {
            subtasks.push(chunk.join("\n"));
        }
    }

    eprintln!("[MAP_REDUCE] {} subtasks for {} workers", subtasks.len(), workers.len());

    // Execute phase: each worker gets a subtask
    let mut worker_outputs = Vec::new();
    for (i, worker_id) in workers.iter().enumerate() {
        let worker = agents::get_agent(worker_id).ok_or(format!("Agent {} not found", worker_id))?;
        let worker_protocol = protocols::protocol_for_role(&worker.role, phase);
        let subtask = subtasks.get(i).cloned().unwrap_or_else(|| task.to_string());

        let result = executor::run_agent(
            &worker.id, &worker.name, &worker.persona, &worker.role,
            &subtask, workspace, mission_id, phase_id,
            Some(worker_protocol), on_event,
        ).await?;

        let guard_result = guard::check_l0(&result, &worker.role, &[]);
        if !guard_result.passed {
            on_event(&worker.id, AgentEvent::Response {
                content: format!("⚠️ Quality: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
            });
        }

        worker_outputs.push(format!("[Worker {} — {}]: {}", i + 1, worker.name, result));
    }

    // Reduce phase: first agent aggregates
    let reduce_prompt = format!(
        "Synthesize these results into a coherent output:\n{}",
        worker_outputs.join("\n\n---\n\n"),
    );

    let reduced = executor::run_agent(
        &mapper.id, &mapper.name, &mapper.persona, &mapper.role,
        &reduce_prompt, workspace, mission_id, phase_id,
        Some(mapper_protocol), on_event,
    ).await?;

    eprintln!("[MAP_REDUCE] Reduce complete");
    Ok(reduced)
}

/// Composite pattern: nested sub-patterns (plan → parallel execute → synthesize).
pub(crate) async fn run_composite(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.len() < 3 {
        return Err("Composite pattern requires at least 3 agents".into());
    }
    eprintln!("[COMPOSITE] Starting with {} agents (plan → parallel → synthesize)", agent_ids.len());

    let planner = &agent_ids[..1];
    let executors = &agent_ids[1..agent_ids.len() - 1];
    let synthesizer = &agent_ids[agent_ids.len() - 1..];

    // Step 1: Solo planning
    eprintln!("[COMPOSITE] Step 1/3 — Solo planning");
    let plan_task = format!(
        "Create a detailed execution plan for this task. \
         Identify key work items for {} parallel workers.\n\nTask: {}",
        executors.len(), task,
    );

    let plan = super::patterns::run_solo(
        planner, &plan_task, phase, workspace, mission_id, phase_id, on_event,
    ).await?;

    // Step 2: Parallel execution by middle agents
    eprintln!("[COMPOSITE] Step 2/3 — Parallel execution ({} agents)", executors.len());
    let exec_task = format!(
        "Execute your part of this plan. Focus on your expertise.\n\nPlan:\n{}\n\nOriginal task: {}",
        plan, task,
    );

    let exec_output = super::patterns::run_parallel(
        executors, &exec_task, phase, workspace, mission_id, phase_id, on_event,
    ).await?;

    // Step 3: Solo synthesis
    eprintln!("[COMPOSITE] Step 3/3 — Solo synthesis");
    let synth_task = format!(
        "Synthesize the plan and all execution outputs into a final deliverable.\n\n\
         Plan:\n{}\n\nExecution results:\n{}\n\nOriginal task: {}",
        plan, exec_output, task,
    );

    let final_output = super::patterns::run_solo(
        synthesizer, &synth_task, phase, workspace, mission_id, phase_id, on_event,
    ).await?;

    eprintln!("[COMPOSITE] Complete");
    Ok(final_output)
}
