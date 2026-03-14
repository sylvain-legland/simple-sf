// Ref: FT-SSF-020

use super::types::*;
use crate::agents::{self, Agent};
use crate::db;
use crate::executor::{AgentEvent, EventCallback};
use crate::llm::{self, LLMMessage};
use crate::protocols;
use rusqlite::params;
use uuid::Uuid;

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
         \n{}\n\nRéponds en 1-2 paragraphes structurés.",
        rte.name, team_str, topic, project_context, history_section, STYLE_RULES
    );

    let rte_system = format!("{}\n\n{}", rte.persona, protocols::protocol_for_role(&rte.role, "cadrage"));
    let rte_result = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: rte_prompt }],
        Some(&rte_system),
        None,
    ).await?;
    let rte_content = strip_emoji(&rte_result.content.unwrap_or_default());

    let other_ids: Vec<&str> = agents_data.iter().skip(1).map(|a| a.id.as_str()).collect();
    emit_discuss(rte, &rte_content, "framing", &other_ids, 0);
    store_discussion_msg(&session_id, &rte.id, &rte.name, &rte.role, 0, &rte_content);
    all_outputs.push((rte.id.clone(), rte.name.clone(), rte.role.clone(), rte_content.clone()));

    // ── Phase 2: Discussion rounds ──
    let experts: Vec<&Agent> = agents_data.iter().skip(1).take(agents_data.len().saturating_sub(2)).collect();
    let po = agents_data.last().unwrap();

    for round in 0..rounds {
        on_event("engine", AgentEvent::Response {
            content: format!("── Tour de table {} ──", round + 1),
        });

        let prev_context: String = all_outputs.iter()
            .rev().take(4).rev()
            .map(|(_, name, role, content)| format!("@{} ({}) : {}", name, role, truncate_ctx(content, 800)))
            .collect::<Vec<_>>()
            .join("\n\n");

        for expert in &experts {
            on_event(&expert.id, AgentEvent::Thinking);

            let expert_prompt = format!(
                "Discussion de cadrage — tour {}. Tu es {} ({}).\n\n\
                 Sujet client : \"{}\"\n\nÉchanges précédents :\n{}\n\n\
                 Donne ton analyse selon ton expertise. Adresse-toi aux autres par leur prénom.\n\n{}",
                round + 1, expert.name, expert.role, topic, prev_context, STYLE_RULES
            );

            let expert_system = format!("{}\n\n{}", expert.persona, protocols::protocol_for_role(&expert.role, "cadrage"));
            let result = llm::chat_completion(
                &[LLMMessage { role: "user".into(), content: expert_prompt }],
                Some(&expert_system),
                None,
            ).await;
            let content = match result {
                Ok(r) => strip_emoji(&r.content.unwrap_or_default()),
                Err(e) => format!("[Erreur LLM pour {}] {}", expert.name, e),
            };

            let to: Vec<&str> = agents_data.iter().filter(|a| a.id != expert.id).map(|a| a.id.as_str()).collect();
            emit_discuss(expert, &content, "analysis", &to, round + 1);
            store_discussion_msg(&session_id, &expert.id, &expert.name, &expert.role, (round + 1) as i32, &content);
            all_outputs.push((expert.id.clone(), expert.name.clone(), expert.role.clone(), content));
        }
    }

    // ── Phase 3: PO synthesis and decision ──
    on_event("engine", AgentEvent::Response {
        content: "── Synthèse PO ──".into(),
    });
    on_event(&po.id, AgentEvent::Thinking);

    let full_context: String = all_outputs.iter()
        .map(|(_, name, role, content)| format!("@{} ({}) : {}", name, role, truncate_ctx(content, 600)))
        .collect::<Vec<_>>()
        .join("\n\n");

    let po_prompt = format!(
        "Tu es {} ({}). Tu as écouté toute la discussion.\n\n\
         Sujet client : \"{}\"\n\nContributions de l'équipe :\n{}\n\n\
         En tant que PO, tu dois :\n\
         1. Synthétiser les points clés (consensus et désaccords)\n\
         2. Prendre une décision : faut-il lancer une mission ?\n\
         3. Si OUI, propose un brief de mission en incluant le tag [CREATE_PROJECT: nom-du-projet] \
            et [START_MISSION: description de la mission]\n\
         4. Si NON, explique pourquoi et propose des prochaines étapes\n\n\
         {}\n\nRéponds de manière structurée et décisionnelle.",
        po.name, po.role, topic, full_context, STYLE_RULES
    );

    let po_system = format!("{}\n\n{}", po.persona, protocols::protocol_for_role(&po.role, "cadrage"));
    let po_result = llm::chat_completion(
        &[LLMMessage { role: "user".into(), content: po_prompt }],
        Some(&po_system),
        None,
    ).await?;
    let po_content = strip_emoji(&po_result.content.unwrap_or_default());

    let all_ids: Vec<&str> = agents_data.iter().map(|a| a.id.as_str()).collect();
    emit_discuss(po, &po_content, "decision", &all_ids, rounds + 1);
    store_discussion_msg(&session_id, &po.id, &po.name, &po.role, (rounds + 1) as i32, &po_content);

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

pub(crate) fn store_discussion_msg(session_id: &str, agent_id: &str, agent_name: &str, agent_role: &str, round: i32, content: &str) {
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
pub(crate) fn load_conversation_history(max_sessions: usize, max_chars: usize) -> String {
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
