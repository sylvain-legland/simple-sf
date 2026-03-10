use crate::agents::{self, Agent};
use crate::executor::{self, AgentEvent, EventCallback};
use crate::guard;

use crate::db;
use crate::llm::{self, LLMMessage};
use crate::protocols;
use crate::tools;
use rusqlite::params;
use uuid::Uuid;
use std::sync::atomic::{AtomicBool, Ordering};

const PHASE_TIMEOUT_SECS: u64 = 900; // 15 min max per phase

/// YOLO mode: auto-approve all gates (skip human-in-the-loop checkpoints)
pub static YOLO_MODE: AtomicBool = AtomicBool::new(false);

// ──────────────────────────────────────────
// PM-Driven Workflow Plan
// ──────────────────────────────────────────

/// Phase execution semantics
#[derive(Clone, Debug)]
enum PhaseType {
    /// Execute once (ideation, architecture, deploy)
    Once,
    /// Iterative development loop — PM checkpoint after each sprint
    Sprint { max_iterations: usize },
    /// Go/No-Go gate — can loop back to a named phase on veto
    Gate { on_veto: Option<String> },
    /// QA → tickets → dev → re-QA feedback cycle
    FeedbackLoop { max_iterations: usize },
}

/// A single phase in the workflow plan
#[derive(Clone, Debug)]
struct PhaseDef {
    name: String,
    phase_type: PhaseType,
    pattern: String,
    agents: Vec<String>,
}

/// The full workflow plan produced by the PM
#[derive(Clone, Debug)]
struct WorkflowPlan {
    phases: Vec<PhaseDef>,
}

/// Default 14-phase SAFe plan (fallback if PM fails to produce a plan)
fn default_plan() -> WorkflowPlan {
    WorkflowPlan {
        phases: vec![
            PhaseDef { name: "ideation".into(), phase_type: PhaseType::Once, pattern: "network".into(), agents: vec!["rte-marie".into(), "po-lucas".into()] },
            PhaseDef { name: "comite-strategique".into(), phase_type: PhaseType::Gate { on_veto: None }, pattern: "sequential".into(), agents: vec!["rte-marie".into(), "po-lucas".into()] },
            PhaseDef { name: "constitution".into(), phase_type: PhaseType::Once, pattern: "sequential".into(), agents: vec!["rte-marie".into(), "po-lucas".into()] },
            PhaseDef { name: "architecture".into(), phase_type: PhaseType::Once, pattern: "sequential".into(), agents: vec!["archi-pierre".into(), "lead-thomas".into()] },
            PhaseDef { name: "design-system".into(), phase_type: PhaseType::Once, pattern: "sequential".into(), agents: vec!["archi-pierre".into(), "po-lucas".into()] },
            PhaseDef { name: "dev".into(), phase_type: PhaseType::Sprint { max_iterations: 5 }, pattern: "parallel".into(), agents: vec!["dev-emma".into(), "dev-karim".into()] },
            PhaseDef { name: "build".into(), phase_type: PhaseType::Sprint { max_iterations: 3 }, pattern: "sequential".into(), agents: vec!["lead-thomas".into()] },
            PhaseDef { name: "pipeline-ci".into(), phase_type: PhaseType::Once, pattern: "sequential".into(), agents: vec!["lead-thomas".into(), "dev-karim".into()] },
            PhaseDef { name: "revue-ux".into(), phase_type: PhaseType::Gate { on_veto: Some("design-system".into()) }, pattern: "sequential".into(), agents: vec!["po-lucas".into(), "archi-pierre".into()] },
            PhaseDef { name: "qa".into(), phase_type: PhaseType::FeedbackLoop { max_iterations: 3 }, pattern: "sequential".into(), agents: vec!["qa-sophie".into(), "lead-thomas".into()] },
            PhaseDef { name: "tests".into(), phase_type: PhaseType::Sprint { max_iterations: 2 }, pattern: "parallel".into(), agents: vec!["qa-sophie".into(), "dev-emma".into()] },
            PhaseDef { name: "deploy".into(), phase_type: PhaseType::Once, pattern: "sequential".into(), agents: vec!["lead-thomas".into(), "rte-marie".into()] },
            PhaseDef { name: "routage-tma".into(), phase_type: PhaseType::Once, pattern: "sequential".into(), agents: vec!["rte-marie".into()] },
            PhaseDef { name: "correctif-tma".into(), phase_type: PhaseType::FeedbackLoop { max_iterations: 3 }, pattern: "sequential".into(), agents: vec!["lead-thomas".into(), "qa-sophie".into()] },
        ],
    }
}

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

/// Ask the PM (LLM) to create a plan, fallback to default 14-phase plan
async fn plan_via_pm(brief: &str, on_event: &EventCallback) -> WorkflowPlan {
    on_event("engine", AgentEvent::Response {
        content: "── PM PLANNING ── Le PM analyse le brief et construit le plan de phases...".into(),
    });
    match pm_create_plan(brief, on_event).await {
        Ok(p) => {
            on_event("engine", AgentEvent::Response {
                content: format!("── Plan PM accepte: {} phases ──", p.phases.len()),
            });
            p
        }
        Err(e) => {
            on_event("engine", AgentEvent::Response {
                content: format!("── Plan PM echoue ({}), utilisation du plan par defaut (14 phases) ──", e),
            });
            default_plan()
        }
    }
}

/// PM planning phase — RTE+PO analyze the brief and produce a WorkflowPlan
async fn pm_create_plan(brief: &str, on_event: &EventCallback) -> Result<WorkflowPlan, String> {
    let rte = agents::get_agent("rte-marie").unwrap_or_else(|| agents::get_agent("rte-marie").unwrap());
    on_event(&rte.id, AgentEvent::Thinking);

    let system = format!(
        "{}\n\n{}\n\nTu es le RTE. Tu dois analyser le brief projet et produire un plan de phases SAFe.\n\
         Réponds UNIQUEMENT avec un JSON valide (pas de texte avant/après).",
        rte.persona, STYLE_RULES
    );

    let prompt = format!(
        "Analyse ce brief projet et produis un plan de phases SAFe adapté.\n\n\
         BRIEF: {}\n\n\
         Produis un JSON avec cette structure exacte:\n\
         {{\n\
           \"phases\": [\n\
             {{\n\
               \"name\": \"ideation\",\n\
               \"type\": \"once\",\n\
               \"pattern\": \"network\",\n\
               \"agents\": [\"rte-marie\", \"po-lucas\"]\n\
             }},\n\
             {{\n\
               \"name\": \"dev\",\n\
               \"type\": \"sprint\",\n\
               \"max_iterations\": 5,\n\
               \"pattern\": \"parallel\",\n\
               \"agents\": [\"dev-emma\", \"dev-karim\"]\n\
             }},\n\
             {{\n\
               \"name\": \"qa\",\n\
               \"type\": \"feedback_loop\",\n\
               \"max_iterations\": 3,\n\
               \"pattern\": \"sequential\",\n\
               \"agents\": [\"qa-sophie\"]\n\
             }},\n\
             {{\n\
               \"name\": \"revue-ux\",\n\
               \"type\": \"gate\",\n\
               \"on_veto\": \"design-system\",\n\
               \"pattern\": \"sequential\",\n\
               \"agents\": [\"po-lucas\"]\n\
             }}\n\
           ]\n\
         }}\n\n\
         Types disponibles:\n\
         - \"once\": exécution unique (idéation, architecture, deploy)\n\
         - \"sprint\": boucle itérative dev, max_iterations sprints, le PM valide après chaque sprint\n\
         - \"gate\": point GO/NOGO, on_veto = nom de la phase de retour si VETO\n\
         - \"feedback_loop\": cycle QA -> tickets -> dev -> re-QA, max_iterations cycles\n\n\
         Agents disponibles: rte-marie, po-lucas, archi-pierre, lead-thomas, dev-emma, dev-karim, qa-sophie\n\n\
         Adapte le nombre de phases et les boucles au brief. Un projet simple peut avoir 6-8 phases, un projet complexe 12-14.",
        brief
    );

    let resp = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: prompt }],
        Some(&system),
        None,
    ).await?;

    let content = resp.content.unwrap_or_default();
    on_event("rte-marie", AgentEvent::Response { content: content.clone() });

    parse_workflow_plan(&content)
}

/// Parse a JSON workflow plan — accepts both `{"phases": [...]}` and bare `[...]` arrays.
/// Agent lists can use `"agents"` or `"agent_ids"` keys.
fn parse_workflow_plan(text: &str) -> Result<WorkflowPlan, String> {
    let trimmed = text.trim();

    // Try parsing the text directly first (for DB-stored JSON)
    let parsed: serde_json::Value = if trimmed.starts_with('[') || trimmed.starts_with('{') {
        serde_json::from_str(trimmed)
            .map_err(|e| format!("Invalid JSON: {}", e))?
    } else {
        // Extract JSON from free-text LLM response (might be wrapped in ```json ... ```)
        let json_str = if let Some(start) = text.find('{') {
            let depth_start = start;
            let mut depth = 0i32;
            let mut end = start;
            for (i, c) in text[depth_start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => { depth -= 1; if depth == 0 { end = depth_start + i + 1; break; } }
                    _ => {}
                }
            }
            &text[depth_start..end]
        } else if let Some(start) = text.find('[') {
            let depth_start = start;
            let mut depth = 0i32;
            let mut end = start;
            for (i, c) in text[depth_start..].char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => { depth -= 1; if depth == 0 { end = depth_start + i + 1; break; } }
                    _ => {}
                }
            }
            &text[depth_start..end]
        } else {
            return Err("No JSON found in PM response".into());
        };
        serde_json::from_str(json_str)
            .map_err(|e| format!("Invalid JSON from PM: {}", e))?
    };

    // Accept both {"phases": [...]} and bare [...]
    let phases_arr = if let Some(arr) = parsed.as_array() {
        arr.clone()
    } else if let Some(arr) = parsed["phases"].as_array() {
        arr.clone()
    } else {
        return Err("Missing 'phases' array in PM plan".into());
    };

    let mut phases = Vec::new();
    for p in &phases_arr {
        let name = p["name"].as_str().unwrap_or("unknown").to_string();
        let pattern = p["pattern"].as_str().unwrap_or("sequential").to_string();
        let phase_type_str = p["type"].as_str().unwrap_or("once");

        // Accept both "agents" and "agent_ids"
        let agents: Vec<String> = p["agents"].as_array()
            .or_else(|| p["agent_ids"].as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| auto_assign_agents(&name));

        let phase_type = match phase_type_str {
            "sprint" => PhaseType::Sprint {
                max_iterations: p["max_iterations"].as_u64().unwrap_or(5) as usize,
            },
            "gate" => PhaseType::Gate {
                on_veto: p["on_veto"].as_str().map(String::from),
            },
            "feedback_loop" => PhaseType::FeedbackLoop {
                max_iterations: p["max_iterations"].as_u64().unwrap_or(3) as usize,
            },
            _ => PhaseType::Once,
        };

        phases.push(PhaseDef { name, phase_type, pattern, agents });
    }

    if phases.is_empty() {
        return Err("PM plan has no phases".into());
    }

    Ok(WorkflowPlan { phases })
}

// ──────────────────────────────────────────
// Workflow State Machine Executor
// ──────────────────────────────────────────

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
                    mission_id, brief, workspace, &phase_def, &agent_ids,
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
                        mission_id, brief, workspace, &phase_def, &agent_ids,
                        &phase_outputs, &mut veto_conditions, sprint_num, max, on_event,
                    ).await;

                    match result {
                        PhaseResult::Completed(output) => {
                            phase_outputs.push(format!("[{} sprint-{}] {}", phase_def.name, sprint_num, output));

                            // PM checkpoint — ask if another sprint is needed
                            if sprint_num < max {
                                let continue_decision = pm_sprint_checkpoint(
                                    brief, &phase_def.name, sprint_num, &output, on_event
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
                    mission_id, brief, workspace, &phase_def, &agent_ids,
                    &phase_outputs, &mut veto_conditions, 1, 1, on_event,
                ).await;

                match result {
                    PhaseResult::Vetoed(output) => {
                        let yolo = YOLO_MODE.load(Ordering::Relaxed);

                        if !yolo {
                            // Loop back to the on_veto phase
                            if let Some(target) = on_veto {
                                if let Some(target_idx) = plan.phases.iter().position(|p| p.name == *target) {
                                    on_event("engine", AgentEvent::Response {
                                        content: format!("  GATE VETO — retour a la phase '{}'", target),
                                    });
                                    veto_conditions = Some(truncate_ctx(&output, 1500).to_string());
                                    phase_outputs.push(format!("[{} VETO->{}] {}", phase_def.name, target, output));
                                    phase_idx = target_idx;
                                    continue; // skip phase_idx increment
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

                    // Inject tickets from previous QA cycle
                    if !tickets.is_empty() {
                        veto_conditions = Some(format!(
                            "TICKETS DU CYCLE PRECEDENT (cycle {}):\n{}",
                            cycle - 1, tickets.join("\n")
                        ));
                    }

                    let result = execute_single_phase(
                        mission_id, brief, workspace, &phase_def, &agent_ids,
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

    Ok(())
}

// ──────────────────────────────────────────
// Phase execution result
// ──────────────────────────────────────────

enum PhaseResult {
    Completed(String),
    Vetoed(String),
    Failed(String),
    #[allow(dead_code)]
    Skipped,
}

/// Execute a single phase (used by all phase types)
async fn execute_single_phase(
    mission_id: &str,
    brief: &str,
    workspace: &str,
    phase_def: &PhaseDef,
    agent_ids: &[String],
    phase_outputs: &[String],
    veto_conditions: &mut Option<String>,
    iteration: usize,
    max_iterations: usize,
    on_event: &EventCallback,
) -> PhaseResult {
    let project_id = std::path::Path::new(workspace)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");

    let phase_id = Uuid::new_v4().to_string();
    let agent_list = serde_json::to_string(&agent_ids).unwrap_or_default();
    let phase_type_str = match &phase_def.phase_type {
        PhaseType::Once => "once",
        PhaseType::Sprint { .. } => "sprint",
        PhaseType::Gate { .. } => "gate",
        PhaseType::FeedbackLoop { .. } => "feedback_loop",
    };

    if let Err(e) = db::with_db(|conn| {
        conn.execute(
            "INSERT INTO mission_phases (id, mission_id, phase_name, pattern, phase_type, status, agent_ids, iteration, max_iterations, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6, ?7, ?8, datetime('now'))",
            params![&phase_id, mission_id, &phase_def.name, &phase_def.pattern, phase_type_str, &agent_list, iteration as i64, max_iterations as i64],
        )
    }) {
        eprintln!("[db] Failed to insert mission phase: {}", e);
    }

    let mut task = build_phase_task(&phase_def.name, brief, phase_outputs);

    // Inject sprint/iteration context
    if iteration > 1 {
        task = format!(
            "{}\n\n## Iteration {}/{}\nCeci est l'iteration {} sur {}. \
             Améliore les résultats des itérations précédentes.",
            task, iteration, max_iterations, iteration, max_iterations
        );
    }

    // Inject veto/feedback conditions
    if let Some(conditions) = veto_conditions.as_ref() {
        task = format!(
            "{}\n\n## CONDITIONS A ADRESSER:\n{}",
            task, conditions
        );
        *veto_conditions = None;
    }

    let agent_ids_slice: Vec<&str> = agent_ids.iter().map(|s| s.as_str()).collect();
    let phase_future = run_phase_with_retry(
        &agent_ids_slice, &task, &phase_def.name, &phase_def.pattern,
        workspace, mission_id, &phase_id, on_event,
    );

    let result = match tokio::time::timeout(
        std::time::Duration::from_secs(PHASE_TIMEOUT_SECS),
        phase_future,
    ).await {
        Ok(inner) => inner,
        Err(_) => Err(format!("Phase {} timed out after {}s", phase_def.name, PHASE_TIMEOUT_SECS)),
    };

    match result {
        Ok(output) => {
            // Store phase output in memory
            if !project_id.is_empty() {
                let mem_key = format!("phase-output-{}-{}", phase_def.name, iteration);
                let mem_val = if output.len() > 2000 { &output[..2000] } else { &output };
                let _ = db::with_db(|conn| {
                    conn.execute(
                        "INSERT OR REPLACE INTO memory (key, value, category, project_id, created_at) \
                         VALUES (?1, ?2, 'phase_output', ?3, datetime('now'))",
                        params![&mem_key, mem_val, project_id],
                    )
                });
            }

            let gate = check_gate_raw(&output);
            let status = if gate == "vetoed" { "vetoed" } else { "completed" };
            if let Err(e) = db::with_db(|conn| {
                conn.execute(
                    "UPDATE mission_phases SET status = ?1, output = ?2, gate_result = ?3, completed_at = datetime('now') WHERE id = ?4",
                    params![status, &output, &gate, &phase_id],
                )
            }) {
                eprintln!("[db] Failed to update mission phase: {}", e);
            }

            if gate == "vetoed" {
                PhaseResult::Vetoed(output)
            } else {
                PhaseResult::Completed(output)
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
            on_event("engine", AgentEvent::Error {
                message: format!("Phase {} failed: {}", phase_def.name, e),
            });
            PhaseResult::Failed(e)
        }
    }
}

// ──────────────────────────────────────────
// PM Sprint Checkpoint — asks PM if another sprint is needed
// ──────────────────────────────────────────

async fn pm_sprint_checkpoint(
    brief: &str,
    phase_name: &str,
    sprint_num: usize,
    sprint_output: &str,
    on_event: &EventCallback,
) -> bool {
    on_event("po-lucas", AgentEvent::Thinking);

    let system = format!(
        "Tu es le Product Owner. Tu decides si un sprint supplementaire est necessaire.\n\
         Reponds UNIQUEMENT par 'CONTINUE' ou 'DONE' suivi d'une justification courte.\n\n{}",
        STYLE_RULES
    );

    let prompt = format!(
        "Brief projet: {}\n\nPhase: {}\nSprint {} termine. Voici le résultat:\n{}\n\n\
         Le code est-il complet et fonctionnel ? Faut-il un sprint supplementaire ?\n\
         Reponds 'CONTINUE' si un sprint est encore necessaire, 'DONE' si c'est suffisant.",
        brief, phase_name, sprint_num, truncate_ctx(sprint_output, 2000)
    );

    match llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: prompt }],
        Some(&system),
        None,
    ).await {
        Ok(resp) => {
            let content = resp.content.unwrap_or_default();
            on_event("po-lucas", AgentEvent::Response { content: content.clone() });
            content.to_uppercase().contains("CONTINUE")
        }
        Err(_) => false, // On error, stop sprinting
    }
}

// ──────────────────────────────────────────
// Ticket extraction from QA output
// ──────────────────────────────────────────

/// Extract actionable tickets from QA/test output
fn extract_tickets(qa_output: &str) -> Vec<String> {
    let mut tickets = Vec::new();
    let upper = qa_output.to_uppercase();

    // If output contains APPROVE, no tickets
    if upper.contains("[APPROVE") || upper.contains("VERDICT: GO") {
        return tickets;
    }

    // Look for structured issues: "- BUG:", "- ISSUE:", "- ERREUR:", numbered items after VETO
    for line in qa_output.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if (lower.starts_with("- bug") || lower.starts_with("- issue") ||
            lower.starts_with("- erreur") || lower.starts_with("- error") ||
            lower.starts_with("- fix") || lower.starts_with("- missing") ||
            lower.starts_with("- manque") || lower.starts_with("- probleme") ||
            lower.starts_with("- problem")) && trimmed.len() > 5
        {
            tickets.push(trimmed.to_string());
        }
        // Also capture numbered items: "1.", "2.", etc. after keywords
        if (trimmed.starts_with("1.") || trimmed.starts_with("2.") ||
            trimmed.starts_with("3.") || trimmed.starts_with("4.") ||
            trimmed.starts_with("5.")) && lower.contains("fix")
        {
            tickets.push(trimmed.to_string());
        }
    }

    // If no structured tickets found but output has issues, create a generic one
    if tickets.is_empty() && (upper.contains("[VETO") || upper.contains("NOGO") || upper.contains("FAIL")) {
        tickets.push(format!("QA generic: {}", truncate_ctx(qa_output, 500)));
    }

    tickets
}

// ──────────────────────────────────────────
// Phase retry — up to 3 retries with exponential backoff + LLM health probe
// ──────────────────────────────────────────

const MAX_PHASE_RETRIES: usize = 3;

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
    let mut last_error = String::new();
    let mut current_task = task.to_string();

    for attempt in 0..=MAX_PHASE_RETRIES {
        if attempt > 0 {
            let backoff_secs = 2u64.pow(attempt as u32); // 2s, 4s, 8s
            eprintln!("[engine] Phase {} attempt {} — backoff {}s", phase, attempt + 1, backoff_secs);
            on_event("engine", AgentEvent::Response {
                content: format!("Phase {} failed (attempt {}), retrying in {}s...", phase, attempt, backoff_secs),
            });
            tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;

            // LLM health probe — quick check before burning a retry
            if let Err(probe_err) = llm_health_probe().await {
                eprintln!("[engine] LLM health probe failed: {} — attempting auto-restart", probe_err);
                on_event("engine", AgentEvent::Response {
                    content: format!("LLM down ({}), attempting auto-restart...", probe_err),
                });

                // Try to restart the LLM server
                if let Err(restart_err) = restart_llm_server().await {
                    eprintln!("[engine] LLM restart failed: {}", restart_err);
                    on_event("engine", AgentEvent::Response {
                        content: format!("LLM restart failed: {} — waiting 15s...", restart_err),
                    });
                    tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                } else {
                    on_event("engine", AgentEvent::Response {
                        content: "LLM server restarted successfully".to_string(),
                    });
                }
            }

            // Inject previous error feedback
            current_task = format!(
                "{}\n\n## PREVIOUS ATTEMPT {} FAILED:\n{}\n\nFix the issues and try again.",
                task, attempt, last_error
            );
        }

        match run_pattern(agent_ids, &current_task, phase, pattern, workspace, mission_id, phase_id, on_event).await {
            Ok(output) => return Ok(output),
            Err(e) => {
                eprintln!("[engine] Phase {} failed (attempt {}): {}", phase, attempt + 1, e);
                last_error = e;
            }
        }
    }

    Err(format!("Phase {} failed after {} retries: {}", phase, MAX_PHASE_RETRIES, last_error))
}

/// Quick LLM health check — send a trivial prompt to verify connectivity
async fn llm_health_probe() -> Result<(), String> {
    let config = crate::llm::get_config().ok_or("LLM not configured")?;
    let base = config.base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1") {
        format!("{}/chat/completions", base)
    } else {
        format!("{}/v1/chat/completions", base)
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": config.model,
        "messages": [{"role": "user", "content": "ping"}],
        "max_tokens": 5
    });

    let resp = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("unreachable: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}

/// Restart the LLM server (MLX or compatible) when it crashes
/// Kills any existing process on the port, relaunches, waits for readiness
async fn restart_llm_server() -> Result<(), String> {
    let config = crate::llm::get_config().ok_or("LLM not configured")?;

    // Parse port from base_url
    let port = config.base_url
        .split(':').last()
        .and_then(|p| p.trim_matches('/').split('/').next())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8800);

    eprintln!("[engine] Restarting LLM server on port {}...", port);

    // Kill existing process on port
    let kill_output = tokio::process::Command::new("sh")
        .args(["-c", &format!("lsof -ti:{} | xargs kill -9 2>/dev/null; true", port)])
        .output()
        .await
        .map_err(|e| format!("kill failed: {}", e))?;
    eprintln!("[engine] Kill result: {}", String::from_utf8_lossy(&kill_output.stderr));

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Relaunch MLX server
    let model = &config.model;
    let cmd = format!(
        "nohup mlx_lm.server --model {} --port {} > /tmp/mlx-server.log 2>&1 &",
        model, port
    );
    eprintln!("[engine] Launching: {}", cmd);

    let _ = tokio::process::Command::new("sh")
        .args(["-c", &cmd])
        .output()
        .await
        .map_err(|e| format!("launch failed: {}", e))?;

    // Wait for server to become ready (up to 60s)
    let base = config.base_url.trim_end_matches('/');
    let models_url = if base.ends_with("/v1") {
        format!("{}/models", base)
    } else {
        format!("{}/v1/models", base)
    };

    for i in 0..12 {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;

        match client.get(&models_url).send().await {
            Ok(r) if r.status().is_success() => {
                eprintln!("[engine] LLM server ready after {}s", (i + 1) * 5);
                return Ok(());
            }
            _ => {
                eprintln!("[engine] LLM not ready yet (attempt {}/12)...", i + 1);
            }
        }
    }

    Err("LLM server did not become ready within 60s".to_string())
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
        &["rte-marie", "po-lucas"]
    } else if lower.contains("stratégi") || lower.contains("strategi") || lower.contains("comité") || lower.contains("committee") {
        &["rte-marie", "po-lucas"]
    } else if lower.contains("constitution") || lower.contains("setup") {
        &["rte-marie", "po-lucas"]
    } else if lower.contains("architect") || lower.contains("design") && !lower.contains("system") {
        &["archi-pierre", "lead-thomas"]
    } else if lower.contains("design sys") || lower.contains("design-sys") || lower.contains("token") || lower.contains("ui") {
        &["archi-pierre", "po-lucas"]
    } else if lower.contains("sprint") || lower.contains("dev") || lower.contains("développement") {
        &["dev-emma", "dev-karim"]
    } else if lower.contains("build") || lower.contains("verify") || lower.contains("ci") || lower.contains("pipeline") {
        &["lead-thomas", "dev-karim"]
    } else if lower.contains("revue") || lower.contains("review") || lower.contains("conformité") {
        &["lead-thomas", "po-lucas"]
    } else if lower.contains("test") || lower.contains("qa") || lower.contains("campagne") {
        &["qa-sophie", "lead-thomas"]
    } else if lower.contains("deploy") || lower.contains("production") || lower.contains("release") {
        &["lead-thomas", "rte-marie"]
    } else if lower.contains("incident") || lower.contains("tma") || lower.contains("maintenance") || lower.contains("correctif") {
        &["lead-thomas", "qa-sophie"]
    } else {
        &["rte-marie", "po-lucas"]
    };
    ids.iter().map(|s| s.to_string()).collect()
}

/// Robust gate detection (#13) — case-insensitive, flexible patterns
/// Normalizes spacing around colons for reliable matching
pub fn check_gate_raw(output: &str) -> String {
    let upper = output.to_uppercase();
    // Normalize "WORD : VALUE" → "WORD: VALUE" for uniform matching
    let norm = upper.replace(" : ", ": ").replace(" :", ":");

    let is_veto = norm.contains("[VETO]") || norm.contains("[NOGO]")
        || norm.contains("STATUT: NOGO") || norm.contains("DÉCISION: NOGO")
        || norm.contains("DECISION: NOGO")
        || norm.contains("[NO-GO]") || norm.contains("[NO GO]")
        || norm.contains("VERDICT: NOGO") || norm.contains("VERDICT: VETO")
        || norm.contains("CONCLUSION: NOGO") || norm.contains("CONCLUSION: VETO")
        // Bare "NOGO" with word boundary (preceded by space/newline/colon)
        || norm.contains("VERDICT: NOGO (")
        || norm.contains(": NOGO\n") || norm.contains(": NOGO —")
        || norm.contains(": NOGO -");

    let is_approve = norm.contains("[APPROVE]") || norm.contains("[APPROVED]")
        || norm.contains("STATUT: GO") || norm.contains("DÉCISION: GO")
        || norm.contains("DECISION: GO")
        || norm.contains("[GO]") || norm.contains("[LGTM]")
        || norm.contains("VERDICT: GO") || norm.contains("VERDICT: APPROVE")
        || norm.contains("CONCLUSION: GO") || norm.contains("CONCLUSION: APPROVE");

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
    let lower = phase.to_lowercase();

    // Context from previous phases — more generous for DEV/BUILD that need architecture details
    let context = if previous.is_empty() {
        String::new()
    } else {
        let is_exec_phase = lower.contains("dev") || lower.contains("sprint")
            || lower.contains("build") || lower.contains("deploy");
        let (max_phases, max_chars) = if is_exec_phase { (4, 2000) } else { (3, 600) };
        let recent: Vec<_> = previous.iter().rev().take(max_phases).rev().collect();
        let ctx: String = recent.iter()
            .map(|p| truncate_ctx(p, max_chars))
            .collect::<Vec<_>>()
            .join("\n\n");
        format!("\n\n## Contexte des phases precedentes:\n{}", ctx)
    };

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
    } else if lower.contains("design sys") || lower.contains("design-sys") || lower.contains("token") {
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
             IMPLEMENT the COMPLETE project. Follow this EXACT sequence:\n\n\
             STEP 1 — PLAN: Use list_files and memory_search to see what exists.\n\
             Read the architecture from previous phases. Make a COMPLETE file list.\n\n\
             STEP 2 — DEPENDENCY MANIFEST: Use code_write to create the build manifest FIRST:\n\
             - Swift: Package.swift (NO test targets if no tests exist)\n\
             - Rust: Cargo.toml\n\
             - JS/TS: package.json\n\n\
             STEP 3 — WRITE EVERY FILE: Use code_write for EACH source file.\n\
             Each file must be COMPLETE — no stubs, no TODOs, no placeholders.\n\
             If a type is referenced, its file MUST be created.\n\
             If a protocol/trait/interface is used, its file MUST exist.\n\n\
             STEP 4 — VERIFY: Use list_files to confirm ALL files exist.\n\
             Use code_read to spot-check key files.\n\n\
             STEP 5 — BUILD: Use build tool to compile (e.g. 'swift build').\n\
             If build fails, read errors, use code_edit to fix, rebuild.\n\n\
             CRITICAL RULES:\n\
             - Every type/class/struct you reference MUST have its own source file\n\
             - Do NOT create test targets for nonexistent test files\n\
             - Save architecture decisions with memory_store\n\
             - Write REAL code, not summaries or descriptions{}",
            brief, context
        )
    } else if lower.contains("build") || lower.contains("verify") {
        format!(
            "BRIEF: {}\n\n\
             BUILD AND COMPILE — you MUST achieve a clean build. Follow this sequence:\n\n\
             STEP 1 — INVENTORY: Use list_files to see ALL source files.\n\n\
             STEP 2 — COMPLETENESS CHECK: Use code_read on EVERY source file.\n\
             Check for: stubs, TODOs, missing implementations, placeholder code.\n\
             If ANY file is incomplete, use code_write to replace it with COMPLETE code.\n\n\
             STEP 3 — MISSING FILES: If any type/class/struct is referenced but has no file,\n\
             create the missing file with code_write.\n\n\
             STEP 4 — BUILD: Run the build tool.\n\
             - Swift project: build(command='swift build')\n\
             - Rust project: build(command='cargo build')\n\
             - Node project: build(command='npm run build')\n\n\
             STEP 5 — FIX AND REBUILD: If build fails:\n\
             a) Read the compiler errors carefully\n\
             b) Use code_edit to fix each error\n\
             c) Run build again\n\
             d) Repeat until build succeeds\n\n\
             YOU ARE NOT DONE until the build command returns SUCCESS with zero errors.\n\
             Do NOT just read files and report — you must actually BUILD.{}",
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
             DEPLOY AND LAUNCH:\n\
             1. Use list_files to verify all build artifacts exist\n\
             2. Use the build tool to do a final release build\n\
             3. Use the test tool to run the executable and verify it launches\n\
             4. If ANY issue, fix it with code_edit and rebuild\n\
             5. Confirm the application is ready to ship\n\
             The application MUST compile and be launchable. Output the exact command to run it.{}",
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
