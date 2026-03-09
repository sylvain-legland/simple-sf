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

const MAX_NETWORK_ROUNDS: usize = 3;
const CONTEXT_BUDGET: usize = 6000;

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

/// Intake team — the agents who participate in the intake discussion.
/// IDs match the SF platform DB: rte, architecte, lead_dev, product
const INTAKE_TEAM: &[&str] = &["rte", "architecte", "lead_dev", "product"];

/// Run a SAFe intake discussion with the direction team.
/// Flow: RTE frames → Experts discuss (2 rounds) → PO decides and proposes mission.
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

    let mut agents_data: Vec<Agent> = Vec::new();
    for id in INTAKE_TEAM {
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

    let rte_prompt = format!(
        "Tu es {} et tu diriges cette session de cadrage avec ton équipe : {}.\n\n\
         Le client demande : \"{}\"\n\n\
         Contexte projets existants : {}\n\n\
         En tant que RTE :\n\
         1. Cadre le sujet : de quoi s'agit-il, quel type de projet ?\n\
         2. Adresse-toi à chaque membre par son prénom : dis à @Pierre (Architecte) ce que tu attends \
            de lui sur la stack technique, à @Thomas (Lead Dev) sur la faisabilité et la décomposition, \
            et à @Lucas (PO) sur le scope produit et les priorités.\n\
         3. Pose 1-2 questions clés pour orienter la discussion.\n\
         4. Estime une durée et un niveau de complexité.\n\n\
         Sois directe et structurée. Pas de code, pas de longs paragraphes.",
        rte.name, team_str, topic, project_context
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

    for round in 0..2 {
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
         Discussion de l'équipe :\n{}\n\n\
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
        po.name, topic, prev_context
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

    db::with_db(|conn| {
        conn.execute(
            "UPDATE discussion_sessions SET status = 'completed', completed_at = datetime('now') WHERE id = ?1",
            params![&session_id],
        ).ok();
    });

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
        conn.execute("UPDATE missions SET status = 'running' WHERE id = ?1", params![mission_id]).ok();
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

    for (phase_name, pattern, agent_ids) in &owned_phases {
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

        let agent_ids_slice: Vec<&str> = agent_ids.iter().map(|s| s.as_str()).collect();
        let result = match pattern.as_str() {
            "network" => run_network(&agent_ids_slice, &task, phase_name, workspace, mission_id, &phase_id, on_event).await,
            "parallel" => run_parallel(&agent_ids_slice, &task, phase_name, workspace, mission_id, &phase_id, on_event).await,
            _ => run_sequential(&agent_ids_slice, &task, phase_name, workspace, mission_id, &phase_id, on_event).await,
        };

        match result {
            Ok(output) => {
                // Check for gates (veto/approve)
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
                db::with_db(|conn| {
                    conn.execute(
                        "UPDATE mission_phases SET status = ?1, output = ?2, gate_result = ?3, completed_at = datetime('now') WHERE id = ?4",
                        params![gate_status, &output, &gate, &phase_id],
                    ).ok();
                });

                if gate == "vetoed" {
                    vetoed = true;
                    on_event("engine", AgentEvent::Response {
                        content: format!("Phase {} -- VETO detected. Mission halted.", phase_name),
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
        "{}\n\n{}\n\n{}",
        leader.persona, protocols::protocol_for_role(&leader.role, phase), STYLE_RULES
    );

    let leader_result = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: leader_prompt }],
        Some(&leader_system),
        None,
    ).await?;
    let leader_content = strip_emoji(&leader_result.content.unwrap_or_default());
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
        Some(&format!("{}\n\n{}\n\n{}", leader.persona, protocols::protocol_for_role(&leader.role, phase), STYLE_RULES)),
        None,
    ).await?;
    let synthesis_content = strip_emoji(&synthesis.content.unwrap_or_default());
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

/// Detect raw veto/approve signals in phase output (no YOLO override).
fn check_gate_raw(output: &str) -> String {
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
        "completed".into()
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
