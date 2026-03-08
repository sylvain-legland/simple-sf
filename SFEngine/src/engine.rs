use crate::agents::{self, Agent};
use crate::executor::{self, AgentEvent, EventCallback};
use crate::db;
use rusqlite::params;
use uuid::Uuid;

/// SAFe workflow phases
const SAFE_PHASES: &[(&str, &str, &[&str])] = &[
    ("vision",     "sequential", &["rte-marie", "po-lucas"]),
    ("design",     "sequential", &["lead-thomas"]),
    ("dev",        "parallel",   &["dev-emma", "dev-karim"]),
    ("qa",         "sequential", &["qa-sophie"]),
    ("review",     "sequential", &["lead-thomas", "po-lucas"]),
];

/// Run a full mission through the SAFe workflow
pub async fn run_mission(
    mission_id: &str,
    brief: &str,
    workspace: &str,
    on_event: &EventCallback,
) -> Result<(), String> {
    // Update mission status
    db::with_db(|conn| {
        conn.execute("UPDATE missions SET status = 'running' WHERE id = ?1", params![mission_id]).ok();
    });

    let mut phase_outputs: Vec<String> = Vec::new();

    for (phase_name, pattern, agent_ids) in SAFE_PHASES {
        let phase_id = Uuid::new_v4().to_string();

        // Create phase record
        let agent_list = serde_json::to_string(agent_ids).unwrap_or_default();
        db::with_db(|conn| {
            conn.execute(
                "INSERT INTO mission_phases (id, mission_id, phase_name, pattern, status, agent_ids, started_at)
                 VALUES (?1, ?2, ?3, ?4, 'running', ?5, datetime('now'))",
                params![&phase_id, mission_id, phase_name, pattern, &agent_list],
            ).ok();
        });

        // Build task for this phase with accumulated context
        let task = build_phase_task(phase_name, brief, &phase_outputs);

        // Run agents based on pattern
        let result = match *pattern {
            "sequential" => run_sequential(agent_ids, &task, workspace, mission_id, &phase_id, on_event).await,
            "parallel" => run_parallel(agent_ids, &task, workspace, mission_id, &phase_id, on_event).await,
            _ => run_sequential(agent_ids, &task, workspace, mission_id, &phase_id, on_event).await,
        };

        match result {
            Ok(output) => {
                phase_outputs.push(format!("[{}] {}", phase_name, output));
                db::with_db(|conn| {
                    conn.execute(
                        "UPDATE mission_phases SET status = 'completed', output = ?1, completed_at = datetime('now') WHERE id = ?2",
                        params![&output, &phase_id],
                    ).ok();
                });
            }
            Err(e) => {
                db::with_db(|conn| {
                    conn.execute(
                        "UPDATE mission_phases SET status = 'failed', output = ?1, completed_at = datetime('now') WHERE id = ?2",
                        params![&e, &phase_id],
                    ).ok();
                });
                // Continue to next phase even on failure
                phase_outputs.push(format!("[{} FAILED] {}", phase_name, e));
            }
        }
    }

    // Mission complete
    db::with_db(|conn| {
        conn.execute("UPDATE missions SET status = 'completed', updated_at = datetime('now') WHERE id = ?1", params![mission_id]).ok();
    });

    Ok(())
}

async fn run_sequential(
    agent_ids: &[&str],
    task: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    let mut outputs = Vec::new();
    let mut cumulative_task = task.to_string();

    for agent_id in agent_ids {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            &cumulative_task, workspace, mission_id, phase_id, on_event,
        ).await?;
        cumulative_task = format!("{}\n\nPrevious agent ({}) output:\n{}", task, agent.name, result);
        outputs.push(format!("{}: {}", agent.name, result));
    }

    Ok(outputs.join("\n\n---\n\n"))
}

async fn run_parallel(
    agent_ids: &[&str],
    task: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    // For parallel, we run agents concurrently but with the same shared workspace
    // In practice, each gets a sub-task derived from their role
    let mut outputs = Vec::new();

    // For true parallelism we'd use tokio::spawn, but shared workspace
    // means we run sequentially to avoid file conflicts (same as real SF)
    for agent_id in agent_ids {
        let agent = agents::get_agent(agent_id).ok_or(format!("Agent {} not found", agent_id))?;
        let role_task = format!("{}\n\nYou are the {} on this team. Focus on your area of expertise.", task, agent.role);
        let result = executor::run_agent(
            &agent.id, &agent.name, &agent.persona, &agent.role,
            &role_task, workspace, mission_id, phase_id, on_event,
        ).await?;
        outputs.push(format!("{}: {}", agent.name, result));
    }

    Ok(outputs.join("\n\n---\n\n"))
}

fn build_phase_task(phase: &str, brief: &str, previous: &[String]) -> String {
    let context = if previous.is_empty() {
        String::new()
    } else {
        format!("\n\nPrevious phases output:\n{}", previous.join("\n\n"))
    };

    match phase {
        "vision" => format!(
            "BRIEF: {}\n\nDefine the product vision, user stories, and acceptance criteria for this project. Be specific and actionable.{}",
            brief, context
        ),
        "design" => format!(
            "BRIEF: {}\n\nDesign the technical architecture. Decompose into concrete development tasks. Choose the tech stack, file structure, and key patterns. Output a clear task list for developers.{}",
            brief, context
        ),
        "dev" => format!(
            "BRIEF: {}\n\nIMPLEMENT the project. Write ALL the code files needed. Use code_write to create each file. Write real, complete, production-quality code — no placeholders, no TODOs.{}",
            brief, context
        ),
        "qa" => format!(
            "BRIEF: {}\n\nReview all code written. Read each file with code_read. Check for bugs, missing error handling, security issues. Write tests if appropriate. Run build/test commands to verify the code works.{}",
            brief, context
        ),
        "review" => format!(
            "BRIEF: {}\n\nFinal review. Read the code and previous phases. Validate that the implementation matches the vision and acceptance criteria. List any remaining issues or approve the delivery.{}",
            brief, context
        ),
        _ => format!("{}\n{}", brief, context),
    }
}
