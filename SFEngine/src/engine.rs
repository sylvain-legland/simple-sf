use crate::agents::{self, Agent};
use crate::executor::{self, AgentEvent, EventCallback};
use crate::guard;
use crate::catalog;
use crate::db;
use crate::llm::{self, LLMMessage};
use crate::protocols;
use rusqlite::params;
use uuid::Uuid;
use std::sync::atomic::{AtomicBool, Ordering};

/// YOLO mode: auto-approve all gates (skip human-in-the-loop checkpoints)
pub static YOLO_MODE: AtomicBool = AtomicBool::new(false);

/// Fallback SAFe workflow phases (used if workflow not found in catalog)
const SAFE_PHASES: &[(&str, &str, &[&str])] = &[
    ("vision",  "network",     &["rte", "product"]),
    ("design",  "sequential",  &["lead_dev", "architecte"]),
    ("dev",     "parallel",    &["dev", "dev_frontend"]),
    ("qa",      "sequential",  &["qa_lead"]),
    ("review",  "network",     &["lead_dev", "product"]),
];

const MAX_NETWORK_ROUNDS: usize = 10;
const CONTEXT_BUDGET: usize = 12000;

/// Instruction appended to every system prompt — enforce no emoji
const STYLE_RULES: &str = "RÈGLES DE FORMAT : ZÉRO emoji, ZÉRO émoticône, ZÉRO caractère Unicode décoratif. \
Utilise uniquement du texte, des tirets (-), des pipes (|), des étoiles (*) pour la mise en forme. \
Sois structuré avec des titres en **gras** et des listes à tirets.";

/// Strip emoji and decorative Unicode from LLM output
fn strip_emoji(text: &str) -> String {
    text.chars().filter(|c| {
        let cp = *c as u32;
        // Keep ASCII + Latin Extended + common punctuation + CJK
        cp < 0x2600 || // Basic Multilingual Plane below symbols
        (cp >= 0x3000 && cp < 0xFE00) || // CJK
        (cp >= 0xFF00 && cp < 0xFFF0)    // Fullwidth forms
    }).collect::<String>()
    .lines()
    .map(|l| l.trim_end())
    .collect::<Vec<_>>()
    .join("\n")
}

// ──────────────────────────────────────────
// Jarvis Intake Discussion (SAFe network pattern)
// ──────────────────────────────────────────
//
// Real SAFe intake flow:
//   1. RTE frames the discussion, assigns roles
//   2. Archi + Lead Dev give technical analysis
//   3. PO synthesizes and proposes mission (with CREATE_PROJECT/START_MISSION tags)
//
// PO is the decision-maker, NOT Jarvis.

/// Intake team — configurable. Defaults to the standard SAFe direction team.
/// IDs match the SF platform DB: rte, architecte, lead_dev, product
const DEFAULT_INTAKE_TEAM: &[&str] = &["rte", "architecte", "lead_dev", "product"];

/// Run a SAFe intake discussion with the direction team.
/// Flow: RTE frames → Experts discuss (2 rounds) → PO decides and proposes mission.
pub async fn run_intake(
    topic: &str,
    project_context: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    run_intake_with_team(topic, project_context, DEFAULT_INTAKE_TEAM, 2, on_event).await
}

/// Configurable intake: custom team + round count (#8)
pub async fn run_intake_with_team(
    topic: &str,
    project_context: &str,
    team_ids: &[&str],
    rounds: usize,
    on_event: &EventCallback,
) -> Result<String, String> {
    let session_id = Uuid::new_v4().to_string();

    if let Err(e) = db::with_db(|conn| {
        conn.execute(
            "INSERT INTO discussion_sessions (id, topic, context) VALUES (?1, ?2, ?3)",
            params![&session_id, topic, project_context],
        )
    }) {
        eprintln!("[db] Failed to insert discussion session: {}", e);
    }

    let mut agents_data: Vec<Agent> = Vec::new();
    for id in team_ids {
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

    let mut all_outputs: Vec<(String, String, String, String)> = Vec::new(); // (id, name, role, content)

    // Helper: emit a rich discussion event as JSON so Swift can display full metadata
    let emit_discuss = |agent: &Agent, content: &str, msg_type: &str, to_agents: &[&str], round: usize| {
        let to_json: Vec<String> = to_agents.iter().map(|s| format!("\"{}\"", s)).collect();
        let json = format!(
            r#"{{"content":{},"agent_name":"{}","role":"{}","message_type":"{}","to_agents":[{}],"round":{}}}"#,
            serde_json::to_string(content).unwrap_or_else(|_| format!("\"{}\"", content.replace('"', "\\\""))),
            agent.name.replace('"', "\\\""),
            agent.role.replace('"', "\\\""),
            msg_type,
            to_json.join(","),
            round,
        );
        on_event(&agent.id, AgentEvent::Response { content: json });
    };

    // ── Phase 1: RTE cadre la discussion ──
    on_event("engine", AgentEvent::Response {
        content: "── Réunion de cadrage ──".into(),
    });

    let rte = &agents_data[0]; // rte (Marc Delacroix)
    on_event(&rte.id, AgentEvent::Thinking);

    // Load previous conversation history for continuity
    let prior_history = load_conversation_history(3, 4000);
    let history_section = if prior_history.is_empty() {
        String::new()
    } else {
        format!("\n\n[Historique des échanges précédents] :\n{}\n\n\
                 Tiens compte de cet historique — ne répète pas ce qui a déjà été dit/décidé.", prior_history)
    };

    let rte_prompt = format!(
        "Tu es {} et tu diriges cette session de cadrage avec ton équipe : {}.\n\n\
         Le client demande : \"{}\"\n\n\
         Contexte projets existants : {}{}\n\n\
         En tant que RTE :\n\
         1. Cadre le sujet : de quoi s'agit-il, quel type de projet ?\n\
         2. Adresse-toi à chaque membre par son prénom : dis à @Pierre (Architecte) ce que tu attends \
            de lui sur la stack technique, à @Thomas (Lead Dev) sur la faisabilité et la décomposition, \
            et à @Lucas (PO) sur le scope produit et les priorités.\n\
         3. Pose 1-2 questions clés pour orienter la discussion.\n\
         4. Estime une durée et un niveau de complexité.\n\n\
         Sois directe et structurée. Pas de code, pas de longs paragraphes.",
        rte.name, team_str, topic, project_context, history_section
    );

    let rte_system = format!(
        "{}\n\n{}\n\nTu t'adresses à tes collègues par leur prénom avec @. \
         Réponds dans la même langue que la demande client.\n\n{}",
        rte.persona, protocols::RESEARCH_PROTOCOL, STYLE_RULES
    );

    let rte_result = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: rte_prompt }],
        Some(&rte_system),
        None,
    ).await?;
    let rte_content = strip_emoji(&rte_result.content.unwrap_or_default());

    // RTE addresses all other agents
    let rte_recipients: Vec<&str> = agents_data[1..].iter().map(|a| a.id.as_str()).collect();
    emit_discuss(rte, &rte_content, "instruction", &rte_recipients, 0);
    store_discussion_msg(&session_id, &rte.id, &rte.name, &rte.role, 0, &rte_content);
    all_outputs.push((rte.id.clone(), rte.name.clone(), rte.role.clone(), rte_content.clone()));

    // ── Phase 2: Experts respond (2 rounds of discussion) ──
    // Round 1: each expert responds to RTE's brief
    // Round 2: they react to each other and refine

    let experts = &agents_data[1..]; // archi, lead, po
    let mut prev_context = format!("**{} ({})** :\n{}", rte.name, rte.role, rte_content);

    for round in 0..rounds {
        for agent in experts {
            on_event(&agent.id, AgentEvent::Thinking);

            let colleagues: Vec<String> = agents_data.iter()
                .filter(|a| a.id != agent.id)
                .map(|a| format!("@{} ({})", a.name, a.role))
                .collect();

            let prompt = if round == 0 {
                format!(
                    "La RTE @{} a cadré la discussion (voir ci-dessous).\n\n\
                     Tu es {} ({}). Tes collègues : {}.\n\n\
                     Demande client : \"{}\"\n\n\
                     [Brief de la RTE] :\n{}\n\n\
                     Réponds en tant qu'expert dans ton domaine :\n\
                     - Donne ton analyse technique/produit\n\
                     - Réponds aux questions de @{}\n\
                     - Adresse-toi aux autres par @prénom si tu as des questions pour eux\n\
                     - Propose des recommandations concrètes",
                    rte.name, agent.name, agent.role, colleagues.join(", "),
                    topic, prev_context, rte.name
                )
            } else {
                format!(
                    "La discussion continue (round 2). Tu es {} ({}).\n\n\
                     Demande client : \"{}\"\n\n\
                     [Échanges précédents] :\n{}\n\n\
                     Réagis aux points des collègues, affine tes recommandations, \
                     réponds aux questions qui t'ont été posées via @{}.\n\
                     Sois concis — on converge vers une décision.",
                    agent.name, agent.role, topic, prev_context, agent.name
                )
            };

            let system = format!(
                "{}\n\nTu t'adresses à tes collègues par @prénom. \
                 Réponds dans la même langue que la demande client.\n\n{}",
                agent.persona, STYLE_RULES
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

            // Determine recipients — each expert addresses the whole team
            let recipients: Vec<&str> = agents_data.iter()
                .filter(|a| a.id != agent.id)
                .map(|a| a.id.as_str())
                .collect();
            let msg_type = if round == 0 { "response" } else { "response" };
            emit_discuss(agent, &content, msg_type, &recipients, round + 1);
            store_discussion_msg(&session_id, &agent.id, &agent.name, &agent.role, (round + 1) as i32, &content);
            all_outputs.push((agent.id.clone(), agent.name.clone(), agent.role.clone(), content.clone()));
        }

        // Build context for next round (keep recent, within budget)
        prev_context = all_outputs.iter()
            .map(|(_, name, role, content)| format!("**{} ({})** :\n{}", name, role, truncate_ctx(content, 500)))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        if prev_context.len() > CONTEXT_BUDGET {
            prev_context = prev_context[..CONTEXT_BUDGET].to_string();
        }
    }

    // ── Phase 3: PO synthesizes and proposes mission ──
    // The PO is the decision-maker — they decide whether to create a project and start a mission.
    let po = agents_data.iter().find(|a| a.id == "product")
        .unwrap_or(agents_data.last().unwrap());

    on_event(&po.id, AgentEvent::Thinking);

    let po_synthesis_prompt = format!(
        "Tu es {} (Product Owner). L'équipe vient de discuter la demande du client.\n\n\
         Demande originale : \"{}\"\n\n\
         Discussion de l'équipe :\n{}\n{}\n\
         En tant que PO, tu as l'autorité pour décider. Fais ta synthèse :\n\
         1. Résume les points clés de la discussion (2-3 lignes)\n\
         2. Décide du scope MVP et de la stack technique retenue\n\
         3. Si un nouveau projet doit être créé, inclus exactement ce tag (le système le parsera) :\n\
            [CREATE_PROJECT name=\"NomDuProjet\" description=\"description courte\" tech=\"technologies\"]\n\
         4. Si une mission de développement doit être lancée, inclus ce tag :\n\
            [START_MISSION project=\"NomDuProjet\" brief=\"description détaillée du brief de dev\"]\n\
         5. Si c'est juste une question ou un conseil, réponds directement sans tags.\n\n\
         Le brief dans START_MISSION doit être DÉTAILLÉ : features, structure de fichiers, contraintes, \
         critères d'acceptation.\n\n\
         Adresse-toi au client directement (\"Je vous propose...\", \"Nous allons...\").",
        po.name, topic, prev_context,
        if prior_history.is_empty() { String::new() } else {
            format!("\n[Historique précédent] :\n{}\n", &prior_history[..prior_history.len().min(2000)])
        }
    );

    let po_system = format!(
        "{}\n\nTu es le décideur produit. Tu synthétises la discussion et tu décides.\n\
         Les tags [CREATE_PROJECT ...] et [START_MISSION ...] sont invisibles pour le client \
         — ils déclenchent des actions automatiques.\n\
         Réponds dans la même langue que la demande client.\n\n{}",
        po.persona, STYLE_RULES
    );

    let synthesis = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: po_synthesis_prompt }],
        Some(&po_system),
        None,
    ).await?;

    let po_content = strip_emoji(&synthesis.content.unwrap_or_default());
    emit_discuss(po, &po_content, "synthesis", &["all"], 99);
    store_discussion_msg(&session_id, &po.id, &po.name, &po.role, 99, &po_content);

    if let Err(e) = db::with_db(|conn| {
        conn.execute(
            "UPDATE discussion_sessions SET status = 'completed', completed_at = datetime('now') WHERE id = ?1",
            params![&session_id],
        )
    }) {
        eprintln!("[db] Failed to update discussion session: {}", e);
    }

    // Return the PO's synthesis — Swift will parse the action tags
    Ok(po_content)
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
    // Look up workflow from mission DB record, fall back to "safe-standard"
    let workflow_id = db::with_db(|conn| {
        if let Err(e) = conn.execute("UPDATE missions SET status = 'running' WHERE id = ?1", params![mission_id]) {
            eprintln!("[db] Failed to update mission status: {}", e);
        }
        conn.query_row(
            "SELECT workflow FROM missions WHERE id = ?1", params![mission_id],
            |row| row.get::<_, String>(0),
        ).unwrap_or_else(|_| "safe-standard".into())
    });

    // Get phases from catalog workflow, or fallback to hardcoded SAFE_PHASES
    let owned_phases: Vec<(String, String, Vec<String>)> = if let Some(wf) = catalog::get_workflow_phases(&workflow_id) {
        wf
    } else {
        SAFE_PHASES.iter().map(|(n, p, a)| (n.to_string(), p.to_string(), a.iter().map(|s| s.to_string()).collect())).collect()
    };

    let mut phase_outputs: Vec<String> = Vec::new();
    let mut vetoed = false;

    for (phase_name, pattern, raw_agent_ids) in &owned_phases {
        // Auto-assign agents when the workflow phase has none
        let agent_ids: Vec<String> = if raw_agent_ids.is_empty() {
            auto_assign_agents(phase_name)
        } else {
            raw_agent_ids.clone()
        };
        if vetoed {
            on_event("engine", AgentEvent::Response {
                content: format!("Phase {} skipped -- previous phase vetoed", phase_name),
            });
            continue;
        }

        // Skip phases with no agents (shouldn't happen after auto-assign, but safety)
        if agent_ids.is_empty() {
            on_event("engine", AgentEvent::Response {
                content: format!("Phase {} skipped -- no agents assigned", phase_name),
            });
            continue;
        }

        let phase_id = Uuid::new_v4().to_string();
        let agent_list = serde_json::to_string(&agent_ids).unwrap_or_default();

        on_event("engine", AgentEvent::Response {
            content: format!("── Phase: {} ({}) ──", phase_name.to_uppercase(), pattern),
        });

        if let Err(e) = db::with_db(|conn| {
            conn.execute(
                "INSERT INTO mission_phases (id, mission_id, phase_name, pattern, status, agent_ids, started_at)
                 VALUES (?1, ?2, ?3, ?4, 'running', ?5, datetime('now'))",
                params![&phase_id, mission_id, phase_name, pattern, &agent_list],
            )
        }) {
            eprintln!("[db] Failed to insert mission phase: {}", e);
        }

        let task = build_phase_task(phase_name, brief, &phase_outputs);

        let agent_ids_slice: Vec<&str> = agent_ids.iter().map(|s| s.as_str()).collect();
        // Run pattern with retry on failure (#14)
        let result = run_phase_with_retry(&agent_ids_slice, &task, phase_name, pattern, workspace, mission_id, &phase_id, on_event).await;

        match result {
            Ok(output) => {
                let raw_gate = check_gate_raw(&output);
                let yolo = YOLO_MODE.load(Ordering::Relaxed);
                let gate = if raw_gate == "vetoed" && yolo { "approved".to_string() } else { raw_gate.clone() };
                let gate_status = if gate == "vetoed" { "vetoed" } else { "completed" };

                if raw_gate == "vetoed" && yolo {
                    on_event("engine", AgentEvent::Response {
                        content: format!("YOLO -- Phase {} -- VETO overridden, continuing.", phase_name),
                    });
                }

                phase_outputs.push(format!("[{}] {}", phase_name, output));
                if let Err(e) = db::with_db(|conn| {
                    conn.execute(
                        "UPDATE mission_phases SET status = ?1, output = ?2, gate_result = ?3, completed_at = datetime('now') WHERE id = ?4",
                        params![gate_status, &output, &gate, &phase_id],
                    )
                }) {
                    eprintln!("[db] Failed to update mission phase: {}", e);
                }

                if gate == "vetoed" {
                    vetoed = true;
                    on_event("engine", AgentEvent::Response {
                        content: format!("Phase {} -- VETO detected. Mission halted.", phase_name),
                    });
                }
            }
            Err(e) => {
                if let Err(db_err) = db::with_db(|conn| {
                    conn.execute(
                        "UPDATE mission_phases SET status = 'failed', output = ?1, completed_at = datetime('now') WHERE id = ?2",
                        params![&e, &phase_id],
                    )
                }) {
                    eprintln!("[db] Failed to update failed phase: {}", db_err);
                }
                phase_outputs.push(format!("[{} FAILED] {}", phase_name, e));
                on_event("engine", AgentEvent::Error {
                    message: format!("Phase {} failed: {}", phase_name, e),
                });
            }
        }
    }

    let final_status = if vetoed { "vetoed" } else { "completed" };
    let completed_count = phase_outputs.len();
    let total_count = owned_phases.len();
    on_event("engine", AgentEvent::Response {
        content: format!(
            "── Mission {} ── {}/{} phases completees ──",
            if vetoed { "VETOED" } else { "TERMINEE" },
            completed_count, total_count
        ),
    });
    if let Err(e) = db::with_db(|conn| {
        conn.execute(
            "UPDATE missions SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![final_status, mission_id],
        )
    }) {
        eprintln!("[db] Failed to update mission final status: {}", e);
    }

    Ok(())
}

// ──────────────────────────────────────────
// Phase retry (#14) — retry once on failure with error context
// ──────────────────────────────────────────

async fn run_phase_with_retry(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    pattern: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    let first = run_pattern(agent_ids, task, phase, pattern, workspace, mission_id, phase_id, on_event).await;
    match first {
        Ok(output) => Ok(output),
        Err(e) => {
            eprintln!("[engine] Phase {} failed (attempt 1): {}", phase, e);
            on_event("engine", AgentEvent::Response {
                content: format!("Phase {} failed, retrying with error context...", phase),
            });
            // Retry with error feedback injected into task
            let retry_task = format!(
                "{}\n\n## PREVIOUS ATTEMPT FAILED:\n{}\n\nFix the issues and try again.",
                task, e
            );
            run_pattern(agent_ids, &retry_task, phase, pattern, workspace, mission_id, phase_id, on_event).await
        }
    }
}

/// Dispatch to the correct pattern implementation
async fn run_pattern(
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
        "network" => run_network(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
        "parallel" => run_parallel(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
        _ => run_sequential(agent_ids, task, phase, workspace, mission_id, phase_id, on_event).await,
    }
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

/// Auto-assign agents based on phase name when workflow provides none.
fn auto_assign_agents(phase_name: &str) -> Vec<String> {
    let lower = phase_name.to_lowercase();
    let ids: &[&str] = if lower.contains("idéation") || lower.contains("ideation") || lower.contains("vision") {
        &["rte", "product"]
    } else if lower.contains("stratégi") || lower.contains("strategi") || lower.contains("comité") || lower.contains("committee") {
        &["rte", "product"]
    } else if lower.contains("constitution") || lower.contains("setup") {
        &["rte", "product"]
    } else if lower.contains("architect") || lower.contains("design") && !lower.contains("system") {
        &["system-architect-art", "rte"]
    } else if lower.contains("design sys") || lower.contains("token") || lower.contains("ui") {
        &["system-architect-art", "product"]
    } else if lower.contains("sprint") || lower.contains("dev") || lower.contains("développement") {
        &["worker", "code-critic"]
    } else if lower.contains("build") || lower.contains("verify") || lower.contains("ci") || lower.contains("pipeline") {
        &["devops", "worker"]
    } else if lower.contains("revue") || lower.contains("review") || lower.contains("conformité") {
        &["code-critic", "product"]
    } else if lower.contains("test") || lower.contains("qa") || lower.contains("campagne") {
        &["tester", "code-critic"]
    } else if lower.contains("deploy") || lower.contains("production") || lower.contains("release") {
        &["devops", "rte"]
    } else if lower.contains("incident") || lower.contains("tma") || lower.contains("maintenance") || lower.contains("correctif") {
        &["devops", "tester"]
    } else {
        &["rte", "product"]
    };
    ids.iter().map(|s| s.to_string()).collect()
}

/// Robust gate detection (#13) — case-insensitive, flexible patterns
fn check_gate_raw(output: &str) -> String {
    let upper = output.to_uppercase();
    let lower = output.to_lowercase();

    let is_veto = upper.contains("[VETO]") || upper.contains("[NOGO]")
        || upper.contains("STATUT: NOGO") || upper.contains("DÉCISION: NOGO")
        || upper.contains("DECISION: NOGO")
        || lower.contains("[veto]") || lower.contains("[no-go]") || lower.contains("[no go]")
        || upper.contains("VERDICT: NOGO") || upper.contains("VERDICT: VETO")
        || upper.contains("STATUT : NOGO") || upper.contains("DÉCISION : NOGO");

    let is_approve = upper.contains("[APPROVE]") || upper.contains("[APPROVED]")
        || upper.contains("STATUT: GO") || upper.contains("DÉCISION: GO")
        || upper.contains("DECISION: GO")
        || upper.contains("[GO]") || upper.contains("[LGTM]")
        || upper.contains("VERDICT: GO") || upper.contains("VERDICT: APPROVE")
        || upper.contains("STATUT : GO") || upper.contains("DÉCISION : GO");

    if is_veto {
        "vetoed".into()
    } else if is_approve {
        "approved".into()
    } else {
        "completed".into()
    }
}

// ──────────────────────────────────────────
// Phase Task Builder
// ──────────────────────────────────────────

fn build_phase_task(phase: &str, brief: &str, previous: &[String]) -> String {
    // Limit context to last 3 phases, 600 chars each — avoid LLM context overflow
    let context = if previous.is_empty() {
        String::new()
    } else {
        let recent: Vec<_> = previous.iter().rev().take(3).rev().collect();
        let ctx: String = recent.iter()
            .map(|p| truncate_ctx(p, 600))
            .collect::<Vec<_>>()
            .join("\n\n");
        format!("\n\n## Contexte des phases precedentes:\n{}", ctx)
    };

    let lower = phase.to_lowercase();

    // Match known phase names (exact or keyword-based)
    if lower.contains("vision") || lower.contains("idéation") || lower.contains("ideation") {
        format!(
            "BRIEF: {}\n\n\
             Define the product vision:\n\
             1. User stories with GIVEN/WHEN/THEN acceptance criteria\n\
             2. MVP scope -- what's in v1, what's deferred\n\
             3. Key risks and mitigations\n\
             4. Success metrics\n\
             Be specific and actionable.{}",
            brief, context
        )
    } else if lower.contains("stratégi") || lower.contains("strategi") || lower.contains("comité") {
        format!(
            "BRIEF: {}\n\n\
             Strategic committee review:\n\
             1. Evaluate project alignment with business goals\n\
             2. Assess resource requirements and timeline\n\
             3. Risk/reward analysis\n\
             4. Issue [APPROVE] or [VETO] with justification\n\
             This is a GO/NOGO gate.{}",
            brief, context
        )
    } else if lower.contains("constitution") || lower.contains("setup") {
        format!(
            "BRIEF: {}\n\n\
             Project constitution:\n\
             1. Define team composition and roles\n\
             2. Establish coding standards and conventions\n\
             3. Set up repository structure\n\
             4. Define sprint cadence and ceremonies{}",
            brief, context
        )
    } else if lower.contains("architect") {
        format!(
            "BRIEF: {}\n\n\
             Design the technical architecture:\n\
             1. Choose tech stack (language, framework, libraries)\n\
             2. Define file structure and key modules\n\
             3. Decompose into subtasks for developers\n\
             4. Identify dependencies and build steps\n\
             Output concrete file paths and task assignments.{}",
            brief, context
        )
    } else if lower.contains("design sys") || lower.contains("token") {
        format!(
            "BRIEF: {}\n\n\
             Design system and UI tokens:\n\
             1. Define color palette, typography, spacing\n\
             2. Component inventory (buttons, cards, forms)\n\
             3. Responsive breakpoints\n\
             4. Accessibility requirements{}",
            brief, context
        )
    } else if lower.contains("sprint") || lower.contains("dev") || lower.contains("développement") {
        format!(
            "BRIEF: {}\n\n\
             IMPLEMENT the project:\n\
             1. Read the architecture/subtasks from previous phases\n\
             2. Use code_write to create EVERY file (real code, no stubs)\n\
             3. Create dependency manifests\n\
             4. Run build to verify compilation\n\
             NO placeholders, NO TODOs. Real production code only.{}",
            brief, context
        )
    } else if lower.contains("build") || lower.contains("verify") {
        format!(
            "BRIEF: {}\n\n\
             Build and verify:\n\
             1. Run build commands to compile the project\n\
             2. Fix any compilation errors\n\
             3. Verify all dependencies are resolved\n\
             4. Confirm the executable/artifact is produced{}",
            brief, context
        )
    } else if lower.contains("pipeline") || lower.contains("ci") {
        format!(
            "BRIEF: {}\n\n\
             CI/CD pipeline setup:\n\
             1. Define build pipeline (compile, test, lint)\n\
             2. Set up automated testing\n\
             3. Configure deployment targets\n\
             4. Document the pipeline steps{}",
            brief, context
        )
    } else if lower.contains("revue") || lower.contains("review") || lower.contains("conformité") {
        format!(
            "BRIEF: {}\n\n\
             Final review:\n\
             1. Does the implementation match the vision and acceptance criteria?\n\
             2. Is the code quality acceptable?\n\
             3. Are there critical bugs or missing features?\n\
             4. Issue [APPROVE] or [VETO] with reasons\n\
             This is a GO/NOGO gate.{}",
            brief, context
        )
    } else if lower.contains("test") || lower.contains("qa") || lower.contains("campagne") {
        format!(
            "BRIEF: {}\n\n\
             Testing and QA:\n\
             1. Review all code written in previous phases\n\
             2. Run build/test commands\n\
             3. Check for bugs, missing error handling, security issues\n\
             4. Validate against acceptance criteria\n\
             5. Issue [APPROVE] or [VETO] with evidence{}",
            brief, context
        )
    } else if lower.contains("deploy") || lower.contains("production") || lower.contains("release") {
        format!(
            "BRIEF: {}\n\n\
             Deployment to production:\n\
             1. Prepare release artifacts\n\
             2. Define deployment steps\n\
             3. Document rollback procedure\n\
             4. Verify deployment checklist{}",
            brief, context
        )
    } else if lower.contains("incident") || lower.contains("tma") || lower.contains("maintenance") || lower.contains("correctif") {
        format!(
            "BRIEF: {}\n\n\
             Maintenance and incident handling:\n\
             1. Review known issues from previous phases\n\
             2. Prioritize fixes\n\
             3. Apply corrections\n\
             4. Validate fixes{}",
            brief, context
        )
    } else {
        format!(
            "BRIEF: {}\n\n\
             Phase: {}\n\
             Execute this phase of the project lifecycle.\n\
             Review previous phases output and produce actionable results.{}",
            brief, phase, context
        )
    }
}

// ──────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────

fn truncate_ctx(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max]) }
}

/// Emit a rich JSON event so Swift can display agent name, role, recipients, round.
fn emit_rich(on_event: &EventCallback, agent: &Agent, content: &str, to_agents: &[&str], round: usize) {
    let to_json: Vec<String> = to_agents.iter().map(|s| format!("\"{}\"", s)).collect();
    let json = format!(
        r#"{{"content":{},"agent_name":"{}","role":"{}","message_type":"response","to_agents":[{}],"round":{}}}"#,
        serde_json::to_string(content).unwrap_or_else(|_| format!("\"{}\"", content.replace('"', "\\\""))),
        agent.name.replace('"', "\\\""),
        agent.role.replace('"', "\\\""),
        to_json.join(","),
        round,
    );
    on_event(&agent.id, AgentEvent::Response { content: json });
}

fn store_agent_msg(mission_id: &str, phase_id: &str, agent_id: &str, agent_name: &str, role: &str, content: &str, tool: Option<&str>) {
    if let Err(e) = db::with_db(|conn| {
        conn.execute(
            "INSERT INTO agent_messages (mission_id, phase_id, agent_id, agent_name, role, content, tool_calls)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![mission_id, phase_id, agent_id, agent_name, role, content, tool],
        )
    }) {
        eprintln!("[db] Failed to store agent message: {}", e);
    }
}

fn store_discussion_msg(session_id: &str, agent_id: &str, agent_name: &str, agent_role: &str, round: i32, content: &str) {
    if let Err(e) = db::with_db(|conn| {
        conn.execute(
            "INSERT INTO discussion_messages (session_id, agent_id, agent_name, agent_role, round, content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, agent_id, agent_name, agent_role, round, content],
        )
    }) {
        eprintln!("[db] Failed to store discussion message: {}", e);
    }
}

/// Load conversation history from previous discussion sessions.
/// Returns a formatted string summarizing past exchanges, most recent first.
fn load_conversation_history(max_sessions: usize, max_chars: usize) -> String {
    let mut history = String::new();
    let sessions: Vec<(String, String, String)> = db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, topic, created_at FROM discussion_sessions \
             ORDER BY created_at DESC LIMIT ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![max_sessions as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }).map_err(|e| e.to_string())?;
        Ok::<Vec<_>, String>(rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
    }).unwrap_or_default();

    for (session_id, topic, created_at) in &sessions {
        let msgs: Vec<(String, String, String)> = db::with_db(|conn| {
            let mut stmt = conn.prepare(
                "SELECT agent_name, agent_role, content FROM discussion_messages \
                 WHERE session_id = ?1 ORDER BY round ASC, id ASC"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![session_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            }).map_err(|e| e.to_string())?;
            Ok::<Vec<_>, String>(rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        }).unwrap_or_default();

        if msgs.is_empty() { continue; }

        history.push_str(&format!("\n── Session du {} — «{}» ──\n", created_at, topic));
        for (name, role, content) in &msgs {
            let truncated = if content.len() > 400 { &content[..400] } else { content.as_str() };
            history.push_str(&format!("@{} ({}) : {}\n\n", name, role, truncated));
        }

        if history.len() >= max_chars { break; }
    }

    if history.len() > max_chars {
        history.truncate(max_chars);
    }
    history
}
