// Ref: FT-SSF-020

use super::resilience::run_phase_with_retry;
use super::types::*;
use super::workflow::check_gate_raw;
use crate::catalog;
use crate::db;
use crate::executor::{self, AgentEvent, EventCallback};
use crate::protocols;
use rusqlite::params;
use uuid::Uuid;

/// Execute a single phase (used by all phase types)
pub(crate) async fn execute_single_phase(
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

/// PM Sprint Checkpoint — real agent with tools, checks workspace + memory
pub(crate) async fn pm_sprint_checkpoint(
    brief: &str,
    phase_name: &str,
    sprint_num: usize,
    sprint_output: &str,
    workspace: &str,
    mission_id: &str,
    on_event: &EventCallback,
) -> bool {
    on_event("po-lucas", AgentEvent::Thinking);

    // Try to load real PM agent from catalog, fallback to defaults
    let (pm_name, pm_persona, pm_role) = catalog::get_agent_info("po-lucas")
        .map(|a| (a.name.clone(), a.persona.clone(), a.role.clone()))
        .unwrap_or_else(|| (
            "PO Lucas".into(),
            "Product Owner pragmatique, orienté livraison".into(),
            "product_owner".into(),
        ));

    let task = format!(
        "## Checkpoint sprint {sprint_num} — {phase_name}\n\n\
         Brief projet: {brief}\n\n\
         Résultat du sprint {sprint_num}:\n{output}\n\n\
         ## TA MISSION (Product Owner):\n\
         1. Utilise `list_files` pour voir les fichiers produits dans le workspace\n\
         2. Utilise `code_read` pour lire les fichiers critiques (Package.swift, main sources)\n\
         3. Utilise `build` pour compiler le projet et vérifier qu'il compile\n\
         4. Utilise `memory_search` pour voir le contexte des phases précédentes\n\
         5. Compare le résultat avec le brief : les acceptance criteria sont-ils couverts ?\n\n\
         ## DECISION FINALE:\n\
         - Réponds **CONTINUE** si le code est incomplet, ne compile pas, ou ne couvre pas le brief\n\
         - Réponds **DONE** si tout est implémenté et compilé avec succès\n\
         Commence par tes observations (tools), puis termine par CONTINUE ou DONE.",
        sprint_num = sprint_num,
        phase_name = phase_name,
        brief = truncate_ctx(brief, 1500),
        output = truncate_ctx(sprint_output, 1500),
    );

    let phase_id = Uuid::new_v4().to_string();
    match executor::run_agent(
        "po-lucas", &pm_name, &pm_persona, &pm_role,
        &task, workspace, mission_id, &phase_id,
        Some(protocols::protocol_for_role("product_owner", phase_name)),
        on_event,
    ).await {
        Ok(content) => {
            on_event("po-lucas", AgentEvent::Response { content: content.clone() });
            let upper = content.to_uppercase();
            // CONTINUE unless explicitly DONE
            if upper.contains("DONE") && !upper.contains("CONTINUE") {
                false
            } else {
                true // default: continue if unclear
            }
        }
        Err(e) => {
            on_event("po-lucas", AgentEvent::Error {
                message: format!("PM checkpoint failed: {}", e),
            });
            true // On error, assume more work needed
        }
    }
}

/// Extract actionable tickets from QA/test output
pub(crate) fn extract_tickets(qa_output: &str) -> Vec<String> {
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

pub(crate) fn build_phase_task(phase: &str, brief: &str, previous: &[String]) -> String {
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
