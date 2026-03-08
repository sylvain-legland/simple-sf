use crate::agents::{self, Agent};
use crate::executor::{self, AgentEvent, EventCallback};
use crate::guard;
use crate::db;
use crate::llm::{self, LLMMessage};
use crate::protocols;
use rusqlite::params;
use uuid::Uuid;

/// SAFe workflow phases — now with network pattern for vision and review
const SAFE_PHASES: &[(&str, &str, &[&str])] = &[
    ("vision",  "network",     &["rte-marie", "po-lucas"]),
    ("design",  "sequential",  &["lead-thomas"]),
    ("dev",     "parallel",    &["dev-emma", "dev-karim"]),
    ("qa",      "sequential",  &["qa-sophie"]),
    ("review",  "network",     &["lead-thomas", "po-lucas"]),
];

const MAX_NETWORK_ROUNDS: usize = 3;
const CONTEXT_BUDGET: usize = 6000;

// ──────────────────────────────────────────
// Jarvis Intake Discussion (network pattern)
// ──────────────────────────────────────────

/// Run a Jarvis intake discussion with RTE + PO.
/// This is the network discussion pattern where agents discuss before acting.
/// Returns the full discussion as a string with each agent's contributions.
pub async fn run_intake(
    topic: &str,
    project_context: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    let session_id = Uuid::new_v4().to_string();

    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO discussion_sessions (id, topic, context) VALUES (?1, ?2, ?3)",
            params![&session_id, topic, project_context],
        ).ok();
    });

    let agent_ids = &["rte-marie", "po-lucas"];
    let mut agents_data: Vec<Agent> = Vec::new();
    for id in agent_ids {
        if let Some(a) = agents::get_agent(id) {
            agents_data.push(a);
        }
    }
    if agents_data.is_empty() {
        return Err("No agents found for intake".into());
    }

    let team_list: Vec<String> = agents_data.iter()
        .map(|a| format!("@{} ({})", a.name, a.role))
        .collect();
    let team_str = team_list.join(", ");

    let mut all_outputs: Vec<(String, String, String)> = Vec::new(); // (id, name, content)

    // ── Round 1: RTE frames the discussion ──
    on_event("engine", AgentEvent::Response {
        content: "── Équipe de direction en discussion ──".into(),
    });

    let rte = &agents_data[0];
    on_event(&rte.id, AgentEvent::Thinking);

    let rte_prompt = format!(
        "Tu diriges cette session de cadrage. Voici ton équipe : {}.\n\n\
         1. Cadre le sujet en 2-3 phrases\n\
         2. Assigne à CHAQUE membre ce que tu attends de lui\n\
         3. Pose 1-2 questions clés pour orienter la discussion\n\n\
         Contexte projets existants: {}\n\n\
         Demande du client :\n{}",
        team_str, project_context, topic
    );

    let rte_system = format!(
        "{}\n\n{}\n\nRespond in the same language as the user request.",
        rte.persona, protocols::RESEARCH_PROTOCOL
    );

    let rte_result = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: rte_prompt }],
        Some(&rte_system),
        None,
    ).await?;
    let rte_content = rte_result.content.unwrap_or_else(|| "(no response)".into());

    on_event(&rte.id, AgentEvent::Response { content: rte_content.clone() });
    store_discussion_msg(&session_id, &rte.id, &rte.name, &rte.role, 0, &rte_content);
    all_outputs.push((rte.id.clone(), rte.name.clone(), rte_content.clone()));

    // ── Round 2: Each agent responds to the brief ──
    let mut prev_round = rte_content.clone();

    for round in 0..2 {
        for agent in &agents_data {
            on_event(&agent.id, AgentEvent::Thinking);

            let prompt = if round == 0 {
                format!(
                    "Le RTE a briefé l'équipe (ci-dessous). \
                     Réponds en tant que {} à ce qui te concerne. \
                     Donne ton analyse d'expert et tes recommandations.\n\n\
                     Demande client: {}\n\n\
                     [Brief du RTE]:\n{}",
                    agent.role, topic, prev_round
                )
            } else {
                format!(
                    "Poursuis la discussion. Réagis aux points soulevés, \
                     réponds aux questions, affine tes recommandations.\n\n\
                     Demande client: {}\n\n\
                     [Échanges précédents]:\n{}",
                    topic, prev_round
                )
            };

            let system = format!(
                "{}\n\n{}\n\nRespond in the same language as the user request.",
                agent.persona, protocols::RESEARCH_PROTOCOL
            );

            let result = llm::chat_completion(
                &[LLMMessage { role: "user".into(), content: prompt }],
                Some(&system),
                None,
            ).await;

            let content = match result {
                Ok(r) => r.content.unwrap_or_else(|| "(no response)".into()),
                Err(e) => format!("(error: {})", e),
            };

            on_event(&agent.id, AgentEvent::Response { content: content.clone() });
            store_discussion_msg(&session_id, &agent.id, &agent.name, &agent.role, (round + 1) as i32, &content);
            all_outputs.push((agent.id.clone(), agent.name.clone(), content.clone()));
        }

        // Build context for next round (with budget)
        prev_round = all_outputs.iter()
            .map(|(_, name, content)| format!("**{}**: {}", name, truncate_ctx(content, 400)))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        if prev_round.len() > CONTEXT_BUDGET {
            prev_round = prev_round[..CONTEXT_BUDGET].to_string();
        }
    }

    // ── Final: Jarvis synthesizes ──
    on_event("jarvis", AgentEvent::Thinking);

    let synthesis_prompt = format!(
        "Tu es Jarvis, chef de projet. Ton équipe vient de discuter de la demande du client.\n\n\
         Synthétise la discussion et décide des actions:\n\
         - Si c'est un projet à créer: inclus [CREATE_PROJECT name=\"...\" description=\"...\" tech=\"...\"]\n\
         - Si c'est une mission à lancer: inclus [START_MISSION project=\"...\" brief=\"description détaillée\"]\n\
         - Si c'est juste une question: réponds directement\n\n\
         Le brief doit être DÉTAILLÉ: fonctionnalités, stack technique, structure, contraintes.\n\n\
         Discussion de l'équipe:\n{}\n\n\
         Demande originale du client: {}",
        prev_round, topic
    );

    let jarvis_system = "You are Jarvis, an AI project manager. You synthesize your team's \
        discussion and take action. Include action tags [CREATE_PROJECT ...] and [START_MISSION ...] \
        when appropriate. These tags are invisible to the user.\n\
        Respond in the same language as the user request.";

    let synthesis = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: synthesis_prompt }],
        Some(jarvis_system),
        None,
    ).await?;

    let jarvis_content = synthesis.content.unwrap_or_else(|| "(no synthesis)".into());
    on_event("jarvis", AgentEvent::Response { content: jarvis_content.clone() });

    db::with_db(|conn| {
        conn.execute(
            "UPDATE discussion_sessions SET status = 'completed', completed_at = datetime('now') WHERE id = ?1",
            params![&session_id],
        ).ok();
    });

    Ok(jarvis_content)
}

// ──────────────────────────────────────────
// Mission Execution (SAFe phases)
// ──────────────────────────────────────────

/// Run a full mission through the SAFe workflow with patterns and gates.
pub async fn run_mission(
    mission_id: &str,
    brief: &str,
    workspace: &str,
    on_event: &EventCallback,
) -> Result<(), String> {
    db::with_db(|conn| {
        conn.execute("UPDATE missions SET status = 'running' WHERE id = ?1", params![mission_id]).ok();
    });

    let mut phase_outputs: Vec<String> = Vec::new();
    let mut vetoed = false;

    for (phase_name, pattern, agent_ids) in SAFE_PHASES {
        if vetoed {
            on_event("engine", AgentEvent::Response {
                content: format!("⚠️ Phase {} skipped — previous phase vetoed", phase_name),
            });
            continue;
        }

        let phase_id = Uuid::new_v4().to_string();
        let agent_list = serde_json::to_string(agent_ids).unwrap_or_default();

        on_event("engine", AgentEvent::Response {
            content: format!("── Phase: {} ({}) ──", phase_name.to_uppercase(), pattern),
        });

        db::with_db(|conn| {
            conn.execute(
                "INSERT INTO mission_phases (id, mission_id, phase_name, pattern, status, agent_ids, started_at)
                 VALUES (?1, ?2, ?3, ?4, 'running', ?5, datetime('now'))",
                params![&phase_id, mission_id, phase_name, pattern, &agent_list],
            ).ok();
        });

        let task = build_phase_task(phase_name, brief, &phase_outputs);

        let result = match *pattern {
            "network" => run_network(agent_ids, &task, phase_name, workspace, mission_id, &phase_id, on_event).await,
            "parallel" => run_parallel(agent_ids, &task, phase_name, workspace, mission_id, &phase_id, on_event).await,
            _ => run_sequential(agent_ids, &task, phase_name, workspace, mission_id, &phase_id, on_event).await,
        };

        match result {
            Ok(output) => {
                // Check for gates (veto/approve)
                let gate = check_gate(&output);
                let gate_status = if gate == "vetoed" { "vetoed" } else { "completed" };

                phase_outputs.push(format!("[{}] {}", phase_name, output));
                db::with_db(|conn| {
                    conn.execute(
                        "UPDATE mission_phases SET status = ?1, output = ?2, gate_result = ?3, completed_at = datetime('now') WHERE id = ?4",
                        params![gate_status, &output, &gate, &phase_id],
                    ).ok();
                });

                if gate == "vetoed" {
                    vetoed = true;
                    on_event("engine", AgentEvent::Response {
                        content: format!("🛑 Phase {} — VETO detected. Mission halted.", phase_name),
                    });
                }
            }
            Err(e) => {
                db::with_db(|conn| {
                    conn.execute(
                        "UPDATE mission_phases SET status = 'failed', output = ?1, completed_at = datetime('now') WHERE id = ?2",
                        params![&e, &phase_id],
                    ).ok();
                });
                phase_outputs.push(format!("[{} FAILED] {}", phase_name, e));
                on_event("engine", AgentEvent::Error {
                    message: format!("Phase {} failed: {}", phase_name, e),
                });
            }
        }
    }

    let final_status = if vetoed { "vetoed" } else { "completed" };
    db::with_db(|conn| {
        conn.execute(
            "UPDATE missions SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![final_status, mission_id],
        ).ok();
    });

    Ok(())
}

// ──────────────────────────────────────────
// Pattern Implementations
// ──────────────────────────────────────────

/// Network pattern: agents discuss in rounds (like the Python SF's run_network).
/// Used for vision and review phases.
async fn run_network(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
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

    // Use first agent as the leader/judge
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
        "{}\n\n{}",
        leader.persona, protocols::protocol_for_role(&leader.role, phase)
    );

    let leader_result = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: leader_prompt }],
        Some(&leader_system),
        None,
    ).await?;
    let leader_content = leader_result.content.unwrap_or_default();
    on_event(&leader.id, AgentEvent::Response { content: leader_content.clone() });
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
                "{}\n\n{}",
                agent.persona, protocols::protocol_for_role(&agent.role, phase)
            );

            let result = llm::chat_completion(
                &[LLMMessage { role: "user".into(), content: prompt }],
                Some(&system),
                None,
            ).await;

            let content = match result {
                Ok(r) => r.content.unwrap_or_default(),
                Err(e) => format!("(error: {})", e),
            };

            on_event(&agent.id, AgentEvent::Response { content: content.clone() });
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
        Some(&format!("{}\n\n{}", leader.persona, protocols::protocol_for_role(&leader.role, phase))),
        None,
    ).await?;
    let synthesis_content = synthesis.content.unwrap_or_default();
    on_event(&leader.id, AgentEvent::Response { content: synthesis_content.clone() });
    store_agent_msg(mission_id, phase_id, &leader.id, &leader.name, "assistant", &synthesis_content, None);
    all_outputs.push(synthesis_content.clone());

    Ok(all_outputs.join("\n\n---\n\n"))
}

/// Sequential pattern with protocol injection and adversarial guard.
async fn run_sequential(
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
async fn run_parallel(
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

// ──────────────────────────────────────────
// Phase Gates (Go/No-Go)
// ──────────────────────────────────────────

/// Check for veto/approve signals in phase output.
fn check_gate(output: &str) -> String {
    let upper = output.to_uppercase();
    let is_veto = upper.contains("[VETO]") || upper.contains("[NOGO]")
        || upper.contains("STATUT: NOGO") || upper.contains("DÉCISION: NOGO");
    let is_approve = upper.contains("[APPROVE]") || upper.contains("STATUT: GO")
        || upper.contains("[GO]");

    if is_veto {
        "vetoed".into()
    } else if is_approve {
        "approved".into()
    } else {
        "completed".into() // No explicit gate signal = pass
    }
}

// ──────────────────────────────────────────
// Phase Task Builder
// ──────────────────────────────────────────

fn build_phase_task(phase: &str, brief: &str, previous: &[String]) -> String {
    let context = if previous.is_empty() {
        String::new()
    } else {
        let ctx: String = previous.iter()
            .map(|p| truncate_ctx(p, 800))
            .collect::<Vec<_>>()
            .join("\n\n");
        format!("\n\n## Previous phases:\n{}", ctx)
    };

    match phase {
        "vision" => format!(
            "BRIEF: {}\n\n\
             Define the product vision for this project:\n\
             1. User stories with GIVEN/WHEN/THEN acceptance criteria\n\
             2. MVP scope — what's in v1, what's deferred\n\
             3. Key risks and mitigations\n\
             4. Success metrics\n\
             \n\
             Be specific and actionable. This vision drives all downstream phases.{}",
            brief, context
        ),
        "design" => format!(
            "BRIEF: {}\n\n\
             Design the technical architecture based on the vision phase:\n\
             1. Choose tech stack (language, framework, libraries)\n\
             2. Define file structure and key modules\n\
             3. Decompose into [SUBTASK N] lines for developers\n\
             4. Identify dependencies and build steps\n\
             \n\
             Output concrete file paths and task assignments.{}",
            brief, context
        ),
        "dev" => format!(
            "BRIEF: {}\n\n\
             IMPLEMENT the project based on design phase:\n\
             1. Read the architecture/subtasks from previous phases\n\
             2. Use code_write to create EVERY file (30+ lines each, real code)\n\
             3. Create dependency manifests (package.json, requirements.txt, etc.)\n\
             4. Run build to verify compilation\n\
             5. git_commit when done\n\
             \n\
             NO placeholders, NO TODOs, NO stubs. Real production code only.{}",
            brief, context
        ),
        "qa" => format!(
            "BRIEF: {}\n\n\
             Review and test ALL code written in dev phase:\n\
             1. code_read every file created\n\
             2. Run build/test commands to verify the code compiles\n\
             3. Check for bugs, missing error handling, security issues\n\
             4. Validate against acceptance criteria from vision phase\n\
             5. Issue [APPROVE] or [VETO] with evidence\n\
             \n\
             You MUST run build/test tools. Reading code alone is NOT testing.{}",
            brief, context
        ),
        "review" => format!(
            "BRIEF: {}\n\n\
             Final review of the entire delivery:\n\
             1. Does the implementation match the vision and acceptance criteria?\n\
             2. Is the code quality acceptable?\n\
             3. Are there critical bugs or missing features?\n\
             4. Issue final verdict: [APPROVE] to ship, or [VETO] with reasons\n\
             \n\
             This is a GO/NOGO gate.{}",
            brief, context
        ),
        _ => format!("{}\n{}", brief, context),
    }
}

// ──────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────

fn truncate_ctx(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max]) }
}

fn store_agent_msg(mission_id: &str, phase_id: &str, agent_id: &str, agent_name: &str, role: &str, content: &str, tool: Option<&str>) {
    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO agent_messages (mission_id, phase_id, agent_id, agent_name, role, content, tool_calls)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![mission_id, phase_id, agent_id, agent_name, role, content, tool],
        ).ok();
    });
}

fn store_discussion_msg(session_id: &str, agent_id: &str, agent_name: &str, agent_role: &str, round: i32, content: &str) {
    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO discussion_messages (session_id, agent_id, agent_name, agent_role, round, content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, agent_id, agent_name, agent_role, round, content],
        ).ok();
    });
}
