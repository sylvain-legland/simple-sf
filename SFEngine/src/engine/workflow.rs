// Ref: FT-SSF-020

use super::types::*;
use crate::agents;
use crate::executor::{AgentEvent, EventCallback};
use crate::llm::{self, LLMMessage};

/// Default 14-phase SAFe plan (fallback if PM fails to produce a plan)
pub(crate) fn default_plan() -> WorkflowPlan {
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

/// Ask the PM (LLM) to create a plan, fallback to default 14-phase plan
pub(crate) async fn plan_via_pm(brief: &str, on_event: &EventCallback) -> WorkflowPlan {
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
pub fn parse_workflow_plan(text: &str) -> Result<WorkflowPlan, String> {
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
        let name = p["name"].as_str()
            .or_else(|| p["id"].as_str())
            .unwrap_or("unknown").to_string();
        // Accept both "pattern" (PM format) and "pattern_id" (DB format)
        let pattern = p["pattern"].as_str()
            .or_else(|| p["pattern_id"].as_str())
            .unwrap_or("sequential").to_string();
        let phase_type_str = p["type"].as_str()
            .or_else(|| p["phase_type"].as_str())
            .unwrap_or("once");

        // Accept "agents", "agent_ids", or "config.agents" (DB format)
        let agents: Vec<String> = p["agents"].as_array()
            .or_else(|| p["agent_ids"].as_array())
            .or_else(|| p["config"]["agents"].as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| auto_assign_agents(&name));

        // DB format has "gate" field (always/no_veto/all_approved) — map to PhaseType
        let gate_str = p["gate"].as_str().unwrap_or("");
        let max_iter = p["max_iterations"].as_u64()
            .or_else(|| p["config"]["max_iterations"].as_u64());
        let has_iterations = max_iter.is_some() && max_iter.unwrap_or(0) > 1;

        let phase_type = match phase_type_str {
            "sprint" => PhaseType::Sprint {
                max_iterations: max_iter.unwrap_or(5) as usize,
            },
            "gate" => PhaseType::Gate {
                on_veto: p["on_veto"].as_str().map(String::from),
            },
            "feedback_loop" => PhaseType::FeedbackLoop {
                max_iterations: max_iter.unwrap_or(3) as usize,
            },
            // DB format: infer from pattern + gate + max_iterations
            _ => {
                // loop pattern with iterations → FeedbackLoop (QA cycle with veto→fix→re-QA)
                if pattern == "loop" && has_iterations {
                    PhaseType::FeedbackLoop {
                        max_iterations: max_iter.unwrap_or(3) as usize,
                    }
                }
                // hierarchical/parallel/sequential with iterations → Sprint (multi-sprint dev)
                else if has_iterations {
                    PhaseType::Sprint {
                        max_iterations: max_iter.unwrap_or(5) as usize,
                    }
                }
                // gate field → Gate (go/nogo checkpoint)
                else if matches!(gate_str, "no_veto" | "all_approved") {
                    PhaseType::Gate { on_veto: p["on_veto"].as_str().map(String::from) }
                }
                // default → Once
                else {
                    PhaseType::Once
                }
            }
        };

        // Use description from DB if available
        let _description = p["description"].as_str().unwrap_or("");
        // Use leader from config if available
        let _leader = p["config"]["leader"].as_str().unwrap_or("");

        phases.push(PhaseDef { name, phase_type, pattern, agents });
    }

    if phases.is_empty() {
        return Err("PM plan has no phases".into());
    }

    Ok(WorkflowPlan { phases })
}

/// Auto-assign agents based on phase name when workflow provides none.
pub(crate) fn auto_assign_agents(phase_name: &str) -> Vec<String> {
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
