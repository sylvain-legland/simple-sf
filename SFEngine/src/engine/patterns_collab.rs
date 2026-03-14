// Ref: FT-SSF-020
//
// Collaborative orchestration patterns: red-blue, relay, mob, hitl.

use crate::agents;
use crate::executor::{self, AgentEvent, EventCallback};
use crate::guard;
use crate::protocols;

/// Red-Blue adversarial pattern: blue team produces, red team attacks, blue fixes.
/// 2 rounds max (blue→red→blue→red→final blue). First half = red, second half = blue.
pub(crate) async fn run_red_blue(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.is_empty() {
        return Err("Red-blue pattern requires at least one agent".into());
    }
    eprintln!("[RED_BLUE] Starting with {} agents", agent_ids.len());

    let mid = agent_ids.len() / 2;
    let (red_ids, blue_ids) = if mid == 0 {
        (&agent_ids[..], agent_ids)
    } else {
        (&agent_ids[..mid], &agent_ids[mid..])
    };

    let mut blue_output = String::new();

    for round in 0..2 {
        eprintln!("[RED_BLUE] Round {} — blue phase", round + 1);

        // Blue team produces / fixes
        for blue_id in blue_ids {
            let agent = agents::get_agent(blue_id).ok_or(format!("Agent {} not found", blue_id))?;
            let protocol = protocols::protocol_for_role(&agent.role, phase);
            let blue_task = if round == 0 {
                task.to_string()
            } else {
                format!(
                    "{}\n\nPrevious solution was critiqued. Address these issues:\n{}\n\nProduce an improved version.",
                    task, blue_output
                )
            };

            let result = executor::run_agent(
                &agent.id, &agent.name, &agent.persona, &agent.role,
                &blue_task, workspace, mission_id, phase_id,
                Some(protocol), on_event,
            ).await?;
            blue_output = result;
        }

        eprintln!("[RED_BLUE] Round {} — red phase", round + 1);

        // Red team critiques / attacks
        let mut red_feedback = Vec::new();
        for red_id in red_ids {
            let agent = agents::get_agent(red_id).ok_or(format!("Agent {} not found", red_id))?;
            let protocol = protocols::protocol_for_role(&agent.role, phase);
            let red_task = format!(
                "Find weaknesses, bugs, security issues in:\n\n{}\n\nOriginal task: {}",
                blue_output, task
            );

            on_event(&agent.id, AgentEvent::Response {
                content: format!("[Red team] Round {} — attacking solution", round + 1),
            });

            let result = executor::run_agent(
                &agent.id, &agent.name, &agent.persona, &agent.role,
                &red_task, workspace, mission_id, phase_id,
                Some(protocol), on_event,
            ).await?;
            red_feedback.push(format!("{} (red): {}", agent.name, result));
        }

        blue_output = format!(
            "Solution:\n{}\n\nRed team feedback:\n{}",
            blue_output, red_feedback.join("\n\n")
        );
    }

    // Final blue pass to address last red feedback
    eprintln!("[RED_BLUE] Final blue pass");
    let final_agent_id = blue_ids.last().unwrap_or(&agent_ids[0]);
    let agent = agents::get_agent(final_agent_id).ok_or(format!("Agent {} not found", final_agent_id))?;
    let protocol = protocols::protocol_for_role(&agent.role, phase);
    let final_task = format!(
        "{}\n\nAddress all red team feedback and produce the final deliverable:\n{}",
        task, blue_output
    );

    let final_output = executor::run_agent(
        &agent.id, &agent.name, &agent.persona, &agent.role,
        &final_task, workspace, mission_id, phase_id,
        Some(protocol), on_event,
    ).await?;

    on_event(&agent.id, AgentEvent::Response {
        content: "[Red-Blue] Final deliverable produced".into(),
    });

    Ok(final_output)
}

/// Relay pattern: agents hand off work sequentially, each building on previous output.
pub(crate) async fn run_relay(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.is_empty() {
        return Err("Relay pattern requires at least one agent".into());
    }
    eprintln!("[RELAY] Starting relay with {} agents", agent_ids.len());

    let mut accumulated = String::new();

    for (i, agent_id) in agent_ids.iter().enumerate() {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let protocol = protocols::protocol_for_role(&agent.role, phase);

        let relay_task = if i == 0 {
            task.to_string()
        } else {
            format!(
                "Continue this work. Previous progress:\n{}\n\nOriginal task: {}\n\nBuild upon and improve what was done.",
                accumulated, task
            )
        };

        eprintln!("[RELAY] Leg {} — {} ({})", i + 1, agent.name, agent.role);
        on_event(&agent.id, AgentEvent::Response {
            content: format!("[Relay] Leg {}/{} — {}", i + 1, agent_ids.len(), agent.name),
        });

        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            &relay_task, workspace, mission_id, phase_id,
            Some(protocol), on_event,
        ).await?;

        let guard_result = guard::check_l0(&result, &agent.role, &[]);
        if !guard_result.passed {
            on_event(&agent.id, AgentEvent::Response {
                content: format!("Quality: {} (score: {})", guard_result.issues.join(", "), guard_result.score),
            });
        }

        accumulated = if accumulated.is_empty() {
            result
        } else {
            format!("{}\n\n---\n\n{}", accumulated, result)
        };
    }

    eprintln!("[RELAY] Relay complete — {} legs", agent_ids.len());
    Ok(accumulated)
}

/// Mob programming pattern: one driver executes, navigators provide guidance.
/// Max 2 rotations. Agent[0] starts as driver.
pub(crate) async fn run_mob(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.is_empty() {
        return Err("Mob pattern requires at least one agent".into());
    }
    if agent_ids.len() == 1 {
        return super::patterns::run_solo(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await;
    }
    eprintln!("[MOB] Starting with {} agents", agent_ids.len());

    const MAX_ROTATIONS: usize = 2;
    let mut last_driver_output = String::new();

    for rotation in 0..MAX_ROTATIONS {
        let driver_idx = rotation % agent_ids.len();
        let driver_id = agent_ids[driver_idx];
        let driver = agents::get_agent(driver_id).ok_or(format!("Agent {} not found", driver_id))?;
        let nav_ids: Vec<&str> = agent_ids.iter().enumerate()
            .filter(|(i, _)| *i != driver_idx)
            .map(|(_, id)| *id)
            .collect();

        eprintln!("[MOB] Rotation {} — driver: {} ({}) — {} navigators", rotation + 1, driver.name, driver.role, nav_ids.len());
        on_event(&driver.id, AgentEvent::Response {
            content: format!("[Mob] Rotation {} — driver: {}", rotation + 1, driver.name),
        });

        // Round 1: navigators discuss / provide guidance
        let mut nav_guidance = Vec::new();
        for nav_id in nav_ids.iter() {
            let nav = agents::get_agent(nav_id).ok_or(format!("Agent {} not found", nav_id))?;
            let protocol = protocols::protocol_for_role(&nav.role, phase);
            let nav_task = if rotation == 0 {
                format!(
                    "You are a navigator in a mob session. Provide guidance and recommendations for the driver.\n\nTask: {}",
                    task
                )
            } else {
                format!(
                    "You are a navigator in a mob session. Review the driver's previous output and provide corrections.\n\n\
                     Driver output:\n{}\n\nOriginal task: {}",
                    last_driver_output, task
                )
            };

            let result = executor::run_agent(
                &nav.id, &nav.name, &nav.persona, &nav.role,
                &nav_task, workspace, mission_id, phase_id,
                Some(protocol), on_event,
            ).await?;
            nav_guidance.push(format!("{} ({}): {}", nav.name, nav.role, result));
        }

        // Driver synthesizes navigator guidance into solution
        let driver_protocol = protocols::protocol_for_role(&driver.role, phase);
        let driver_task = if rotation == 0 {
            format!(
                "You are the driver in a mob session. Synthesize this navigator guidance into a concrete solution.\n\n\
                 Navigator guidance:\n{}\n\nTask: {}",
                nav_guidance.join("\n\n---\n\n"), task
            )
        } else {
            format!(
                "You are the driver in a mob session. Apply navigator corrections to improve the solution.\n\n\
                 Navigator corrections:\n{}\n\nPrevious solution:\n{}\n\nOriginal task: {}",
                nav_guidance.join("\n\n---\n\n"), last_driver_output, task
            )
        };

        last_driver_output = executor::run_agent(
            &driver.id, &driver.name, &driver.persona, &driver.role,
            &driver_task, workspace, mission_id, phase_id,
            Some(driver_protocol), on_event,
        ).await?;

        let guard_result = guard::check_l0(&last_driver_output, &driver.role, &[]);
        if guard_result.passed && guard_result.score < 5 {
            eprintln!("[MOB] Passed at rotation {} (score: {})", rotation + 1, guard_result.score);
            break;
        }
    }

    eprintln!("[MOB] Complete");
    Ok(last_driver_output)
}

/// Human-in-the-loop pattern: agent produces, guard checks, HITL pause if needed.
/// Currently auto-approves (placeholder for future UI integration).
pub(crate) async fn run_hitl(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    if agent_ids.is_empty() {
        return Err("HITL pattern requires at least one agent".into());
    }
    eprintln!("[HITL] Starting with {} agents", agent_ids.len());

    let mut best_output = String::new();
    let mut best_score = i32::MAX;

    for (i, agent_id) in agent_ids.iter().enumerate() {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let protocol = protocols::protocol_for_role(&agent.role, phase);

        let agent_task = if i == 0 {
            task.to_string()
        } else {
            format!(
                "{}\n\nPrevious attempt had quality issues: score {}. Please produce an improved version.\n\nPrevious output:\n{}",
                task, best_score, best_output
            )
        };

        eprintln!("[HITL] Agent {} — {} ({})", i + 1, agent.name, agent.role);
        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            &agent_task, workspace, mission_id, phase_id,
            Some(protocol), on_event,
        ).await?;

        let guard_result = guard::check_l0(&result, &agent.role, &[]);
        eprintln!("[HITL] Guard score: {} (passed: {})", guard_result.score, guard_result.passed);

        if guard_result.score >= 5 {
            // Emit HITL pause event for UI integration
            on_event("[HITL]", AgentEvent::Response {
                content: format!(
                    "[HITL_PAUSE] Agent output needs review. Score: {}. Issues: {:?}",
                    guard_result.score, guard_result.issues
                ),
            });

            // Auto-approve for now (HITL placeholder — UI integration later)
            eprintln!("[HITL] Auto-approving (HITL placeholder)");
        }

        if guard_result.passed {
            eprintln!("[HITL] Passed — returning output from {}", agent.name);
            on_event(&agent.id, AgentEvent::Response {
                content: format!("[HITL] Approved (score: {})", guard_result.score),
            });
            return Ok(result);
        }

        if guard_result.score < best_score {
            best_score = guard_result.score;
            best_output = result;
        }
    }

    eprintln!("[HITL] No agent passed guard — returning best output (score: {})", best_score);
    on_event("engine", AgentEvent::Response {
        content: format!("[HITL] Returning best effort (score: {})", best_score),
    });

    Ok(best_output)
}
