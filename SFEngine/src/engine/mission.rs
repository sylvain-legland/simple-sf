// Ref: FT-SSF-020

use super::build::{auto_build_check, finalize_build};
use super::phase::{execute_single_phase, extract_tickets, pm_sprint_checkpoint};
use super::types::*;
use super::workflow::{auto_assign_agents, parse_workflow_plan, plan_via_pm};
use crate::db;
use crate::executor::{AgentEvent, EventCallback};
use crate::tools;
use rusqlite::params;
use std::sync::atomic::Ordering;

/// Run a full mission through the PM-driven SAFe workflow with loops and gates.
pub async fn run_mission(
    mission_id: &str,
    brief: &str,
    workspace: &str,
    on_event: &EventCallback,
) -> Result<(), String> {
    // Look up workflow from mission DB record
    let workflow_id = db::with_db(|conn| {
        if let Err(e) = conn.execute("UPDATE missions SET status = 'running' WHERE id = ?1", params![mission_id]) {
            eprintln!("[db] Failed to update mission status: {}", e);
        }
        conn.query_row(
            "SELECT workflow FROM missions WHERE id = ?1", params![mission_id],
            |row| row.get::<_, String>(0),
        ).unwrap_or_else(|_| "safe-standard".into())
    });

    // ── Memory system: load project files at mission start ──
    let project_id = std::path::Path::new(workspace)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    if !project_id.is_empty() {
        tools::load_project_files(workspace, project_id);
    }

    // ── Try to load pre-defined workflow phases from DB ──
    let predefined_plan = db::with_db(|conn| {
        conn.query_row(
            "SELECT phases_json FROM workflows WHERE id = ?1", params![&workflow_id],
            |row| row.get::<_, String>(0),
        ).ok()
    });

    let plan = if let Some(ref phases_json) = predefined_plan {
        match parse_workflow_plan(phases_json) {
            Ok(p) => {
                on_event("engine", AgentEvent::Response {
                    content: format!("── Workflow pre-defini charge: {} phases ──", p.phases.len()),
                });
                p
            }
            Err(_) => {
                // Pre-defined workflow exists but phases_json is not a valid plan — fall through to PM
                plan_via_pm(brief, on_event).await
            }
        }
    } else {
        plan_via_pm(brief, on_event).await
    };

    // ── Execute the plan with the state machine ──
    execute_workflow_plan(mission_id, brief, workspace, &plan, on_event).await
}

/// Execute a workflow plan with proper loop/gate/feedback semantics
async fn execute_workflow_plan(
    mission_id: &str,
    brief: &str,
    workspace: &str,
    plan: &WorkflowPlan,
    on_event: &EventCallback,
) -> Result<(), String> {
    let project_id = std::path::Path::new(workspace)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");

    let mut phase_outputs: Vec<String> = Vec::new();
    let mut veto_conditions: Option<String> = None;
    let mut mission_vetoed = false;
    let mut gate_loopback_count: usize = 0;
    const MAX_GATE_LOOPBACKS: usize = 3;

    let mut phase_idx: usize = 0;

    while phase_idx < plan.phases.len() {
        let phase_def = &plan.phases[phase_idx];
        let agent_ids: Vec<String> = if phase_def.agents.is_empty() {
            auto_assign_agents(&phase_def.name)
        } else {
            phase_def.agents.clone()
        };

        let phase_type_label = match &phase_def.phase_type {
            PhaseType::Once => "once".to_string(),
            PhaseType::Sprint { max_iterations } => format!("sprint(max={})", max_iterations),
            PhaseType::Gate { on_veto } => format!("gate(on_veto={:?})", on_veto),
            PhaseType::FeedbackLoop { max_iterations } => format!("feedback_loop(max={})", max_iterations),
        };

        on_event("engine", AgentEvent::Response {
            content: format!("── Phase {}/{}: {} ({} | {}) ──",
                phase_idx + 1, plan.phases.len(),
                phase_def.name.to_uppercase(), phase_def.pattern, phase_type_label),
        });

        match &phase_def.phase_type {
            PhaseType::Once => {
                let result = execute_single_phase(
                    mission_id, brief, workspace, phase_def, &agent_ids,
                    &phase_outputs, &mut veto_conditions, 1, 1, on_event,
                ).await;
                match result {
                    PhaseResult::Completed(output) => phase_outputs.push(format!("[{}] {}", phase_def.name, output)),
                    PhaseResult::Vetoed(output) => {
                        let yolo = YOLO_MODE.load(Ordering::Relaxed);
                        if yolo {
                            on_event("engine", AgentEvent::Response {
                                content: "  YOLO — VETO overridé, conditions injectees".into(),
                            });
                            veto_conditions = Some(truncate_ctx(&output, 1500).to_string());
                            phase_outputs.push(format!("[{} YOLO-VETO] {}", phase_def.name, output));
                        } else {
                            on_event("engine", AgentEvent::Response {
                                content: format!("  Phase {} — VETO detecte, mission arretee", phase_def.name),
                            });
                            phase_outputs.push(format!("[{} VETO] {}", phase_def.name, output));
                            mission_vetoed = true;
                        }
                    }
                    PhaseResult::Failed(e) => phase_outputs.push(format!("[{} FAILED] {}", phase_def.name, e)),
                    _ => {}
                }
            }

            PhaseType::Sprint { max_iterations } => {
                let max = *max_iterations;
                let mut sprint_feedback: Option<String> = None;

                for sprint_num in 1..=max {
                    on_event("engine", AgentEvent::Response {
                        content: format!("  Sprint {}/{} — {}", sprint_num, max, phase_def.name),
                    });

                    // Inject previous sprint feedback into the task
                    if let Some(ref fb) = sprint_feedback {
                        veto_conditions = Some(format!(
                            "FEEDBACK DU SPRINT PRECEDENT (sprint {}):\n{}", sprint_num - 1, fb
                        ));
                    }

                    let result = execute_single_phase(
                        mission_id, brief, workspace, phase_def, &agent_ids,
                        &phase_outputs, &mut veto_conditions, sprint_num, max, on_event,
                    ).await;

                    match result {
                        PhaseResult::Completed(output) => {
                            phase_outputs.push(format!("[{} sprint-{}] {}", phase_def.name, sprint_num, output));

                            // PM checkpoint — ask if another sprint is needed
                            if sprint_num < max {
                                let continue_decision = pm_sprint_checkpoint(
                                    brief, &phase_def.name, sprint_num, &output,
                                    workspace, mission_id, on_event
                                ).await;
                                if !continue_decision {
                                    on_event("engine", AgentEvent::Response {
                                        content: format!("  PM: sprint suffisant apres sprint {}", sprint_num),
                                    });
                                    break;
                                } else {
                                    sprint_feedback = Some(truncate_ctx(&output, 1500).to_string());
                                    on_event("engine", AgentEvent::Response {
                                        content: format!("  PM: sprint supplementaire requis"),
                                    });
                                }
                            }
                        }
                        PhaseResult::Failed(e) => {
                            phase_outputs.push(format!("[{} sprint-{} FAILED] {}", phase_def.name, sprint_num, e));
                            // Continue to next sprint on failure (resilience)
                            sprint_feedback = Some(format!("ECHEC: {}", e));
                        }
                        _ => break,
                    }
                }
            }

            PhaseType::Gate { on_veto } => {
                let result = execute_single_phase(
                    mission_id, brief, workspace, phase_def, &agent_ids,
                    &phase_outputs, &mut veto_conditions, 1, 1, on_event,
                ).await;

                match result {
                    PhaseResult::Vetoed(output) => {
                        let yolo = YOLO_MODE.load(Ordering::Relaxed);

                        if !yolo {
                            // Loop back to the on_veto phase (with limit)
                            if let Some(target) = on_veto {
                                if gate_loopback_count < MAX_GATE_LOOPBACKS {
                                    if let Some(target_idx) = plan.phases.iter().position(|p| p.name == *target) {
                                        gate_loopback_count += 1;
                                        on_event("engine", AgentEvent::Response {
                                            content: format!("  GATE VETO — retour a la phase '{}' (tentative {}/{})", target, gate_loopback_count, MAX_GATE_LOOPBACKS),
                                        });
                                        veto_conditions = Some(truncate_ctx(&output, 1500).to_string());
                                        phase_outputs.push(format!("[{} VETO->{}] {}", phase_def.name, target, output));
                                        phase_idx = target_idx;
                                        continue; // skip phase_idx increment
                                    }
                                } else {
                                    on_event("engine", AgentEvent::Response {
                                        content: format!("  GATE VETO — limite de {} loopbacks atteinte, mission arretee", MAX_GATE_LOOPBACKS),
                                    });
                                }
                            }
                            // No loop-back target — halt
                            on_event("engine", AgentEvent::Response {
                                content: format!("  GATE VETO — mission arretee (pas de phase de retour)"),
                            });
                            phase_outputs.push(format!("[{} VETO] {}", phase_def.name, output));
                            mission_vetoed = true;
                        } else {
                            on_event("engine", AgentEvent::Response {
                                content: format!("  YOLO — VETO overridé, conditions injectees dans la phase suivante"),
                            });
                            veto_conditions = Some(truncate_ctx(&output, 1500).to_string());
                            phase_outputs.push(format!("[{} YOLO-VETO] {}", phase_def.name, output));
                        }
                    }
                    PhaseResult::Completed(output) => {
                        phase_outputs.push(format!("[{} APPROVED] {}", phase_def.name, output));
                    }
                    PhaseResult::Failed(e) => {
                        phase_outputs.push(format!("[{} FAILED] {}", phase_def.name, e));
                    }
                    _ => {}
                }
            }

            PhaseType::FeedbackLoop { max_iterations } => {
                let max = *max_iterations;
                let mut tickets: Vec<String> = Vec::new();

                for cycle in 1..=max {
                    on_event("engine", AgentEvent::Response {
                        content: format!("  Feedback cycle {}/{} — {}", cycle, max, phase_def.name),
                    });

                    // ── Auto-build before QA: give reviewers real build status ──
                    let build_report = auto_build_check(workspace).await;
                    let build_context = if !build_report.is_empty() {
                        format!("BUILD STATUS (auto-check):\n{}\n\nIf the build FAILED, you MUST [VETO].", build_report)
                    } else {
                        String::new()
                    };

                    // Inject tickets from previous QA cycle + build report
                    let mut injection = Vec::new();
                    if !tickets.is_empty() {
                        injection.push(format!(
                            "TICKETS DU CYCLE PRECEDENT (cycle {}):\n{}",
                            cycle - 1, tickets.join("\n")
                        ));
                    }
                    if !build_context.is_empty() {
                        injection.push(build_context);
                    }
                    if !injection.is_empty() {
                        veto_conditions = Some(injection.join("\n\n"));
                    }

                    let result = execute_single_phase(
                        mission_id, brief, workspace, phase_def, &agent_ids,
                        &phase_outputs, &mut veto_conditions, cycle, max, on_event,
                    ).await;

                    let (is_vetoed, output) = match result {
                        PhaseResult::Vetoed(o) => (true, o),
                        PhaseResult::Completed(o) => (false, o),
                        PhaseResult::Failed(e) => {
                            phase_outputs.push(format!("[{} cycle-{} FAILED] {}", phase_def.name, cycle, e));
                            if cycle == max { break; }
                            continue;
                        }
                        _ => break,
                    };

                    if is_vetoed {
                        // Extract tickets from the QA output
                        let new_tickets = extract_tickets(&output);
                        if new_tickets.is_empty() {
                            // Vetoed but no extractable tickets — done
                            on_event("engine", AgentEvent::Response {
                                content: format!("  QA VETO — feedback loop terminee au cycle {}", cycle),
                            });
                            phase_outputs.push(format!("[{} cycle-{} VETO] {}", phase_def.name, cycle, output));
                            break;
                        }

                        // QA failed with tickets — run dev fix sprint
                        on_event("engine", AgentEvent::Response {
                            content: format!("  QA: {} tickets trouves — lancement sprint correctif", new_tickets.len()),
                        });
                        tickets = new_tickets;

                        // Run a dev fix sprint with the tickets
                        if cycle < max {
                            let dev_agents = vec!["dev-emma".to_string(), "dev-karim".to_string()];
                            let dev_phase = PhaseDef {
                                name: format!("{}-fix", phase_def.name),
                                phase_type: PhaseType::Once,
                                pattern: "parallel".into(),
                                agents: dev_agents.clone(),
                            };
                            veto_conditions = Some(format!(
                                "SPRINT CORRECTIF — Corrige ces tickets QA:\n{}", tickets.join("\n")
                            ));
                            let dev_ids: Vec<String> = dev_agents;
                            let _ = execute_single_phase(
                                mission_id, brief, workspace, &dev_phase, &dev_ids,
                                &phase_outputs, &mut veto_conditions, cycle, max, on_event,
                            ).await;
                            phase_outputs.push(format!("[{}-fix cycle-{}] corrections appliquees", phase_def.name, cycle));
                        }
                    } else {
                        // Approved
                        on_event("engine", AgentEvent::Response {
                            content: format!("  QA OK — feedback loop terminee au cycle {}", cycle),
                        });
                        phase_outputs.push(format!("[{} cycle-{} APPROVED] {}", phase_def.name, cycle, output));
                        break;
                    }
                }
            }
        }

        if mission_vetoed {
            break;
        }

        phase_idx += 1;
    }

    // ── Mission complete ──
    let final_status = if mission_vetoed { "vetoed" } else { "completed" };
    let completed_count = phase_outputs.len();
    let total_count = plan.phases.len();
    on_event("engine", AgentEvent::Response {
        content: format!("── Mission TERMINEE ({}) ── {}/{} phases completees ──", final_status, completed_count, total_count),
    });
    if let Err(e) = db::with_db(|conn| {
        conn.execute(
            "UPDATE missions SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![final_status, mission_id],
        )
    }) {
        eprintln!("[db] Failed to update mission final status: {}", e);
    }

    // ── Memory: compact on mission complete ──
    if !project_id.is_empty() {
        tools::compact_memory(project_id);
    }

    // ── Final build: try to compile the project ──
    if !mission_vetoed {
        finalize_build(workspace, mission_id, on_event).await;
    }

    Ok(())
}
