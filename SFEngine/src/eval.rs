/// Eval harness for Skills, Agents, and Pattern/Org quality.
///
/// Inspired by Phil Schmid's skill testing framework (philschmid.de/testing-skills)
/// and the platform's existing 8-layer AC bench.
///
/// Three eval domains:
///   L10 — Skill Quality (deterministic + optional LLM judge)
///   L11 — Agent Configuration (role-tool-skill coherence)
///   L12 — Pattern & Org (coverage, structure, workflow integrity)

mod eval_metrics;
mod eval_runner;

// Re-export public API
pub use eval_metrics::{EvalResult, EvalCase, SkillCheck, eval_skill};
pub use eval_runner::{eval_all_skills, eval_all_agents};

use crate::{catalog, db, tools};
use eval_metrics::EvalResult as ER;

// ── L12: Pattern & Org Eval ──
// Ref: FT-SSF-025

/// Eval patterns, workflows, and organizational coverage.
pub fn eval_all_patterns() -> ER {
    let mut r = ER::new("L12-Patterns");

    // Pattern catalog
    let (_, _, pattern_count, workflow_count) = catalog::catalog_stats();
    r.check("patterns-exist", pattern_count >= 10,
        &format!("{} patterns in catalog", pattern_count));

    // Required pattern types
    let pattern_types: Vec<(String, String)> = db::with_db(|conn| {
        let mut stmt = conn.prepare("SELECT id, type FROM patterns").unwrap();
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default()))
        }).unwrap().filter_map(|r| r.ok()).collect()
    });
    let type_set: std::collections::HashSet<&str> = pattern_types.iter()
        .map(|(_, t)| t.as_str())
        .collect();
    let required_types = ["solo", "sequential", "parallel", "hierarchical",
                          "loop", "network", "router", "aggregator", "wave"];
    let covered: Vec<&str> = required_types.iter()
        .filter(|t| type_set.contains(*t))
        .copied()
        .collect();
    r.check("patterns-type-coverage", covered.len() >= 7,
        &format!("{}/{} pattern types: {}", covered.len(), required_types.len(),
            covered.join(", ")));

    // Pattern agent references: all agents in patterns must exist
    let agents_set: std::collections::HashSet<String> = catalog::all_agents().iter()
        .map(|a| a.id.clone())
        .collect();
    let mut orphan_agents = 0;
    let mut total_pattern_agents = 0;
    let pattern_agents: Vec<(String, String)> = db::with_db(|conn| {
        let mut stmt = conn.prepare("SELECT id, agents_json FROM patterns").unwrap();
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default()))
        }).unwrap().filter_map(|r| r.ok()).collect()
    });
    for (_pid, agents_json) in &pattern_agents {
        let parsed: Vec<serde_json::Value> = serde_json::from_str(agents_json).unwrap_or_default();
        let agent_ids: Vec<String> = parsed.iter().filter_map(|v| {
            v.as_str().map(String::from)
                .or_else(|| v.get("agent_id").and_then(|a| a.as_str()).map(String::from))
        }).collect();
        total_pattern_agents += agent_ids.len();
        for aid in &agent_ids {
            if !agents_set.contains(aid) && catalog::get_agent_info(aid).is_some() == false
                && !["worker", "brain", "dispatcher", "aggregator", "reviewer", "judge"].contains(&aid.as_str()) {
                if aid.contains('-') || aid.contains('_') {
                    orphan_agents += 1;
                }
            }
        }
    }
    r.check("patterns-valid-agents", orphan_agents <= total_pattern_agents / 10,
        &format!("{}/{} pattern agent refs valid (max 10% orphan)", total_pattern_agents - orphan_agents, total_pattern_agents));

    // Pattern structure: each pattern should have >= 1 agent
    let empty_patterns: Vec<&str> = pattern_agents.iter()
        .filter(|(_, aj)| {
            let parsed: Vec<serde_json::Value> = serde_json::from_str(aj).unwrap_or_default();
            parsed.is_empty()
        })
        .map(|(id, _)| id.as_str())
        .collect();
    r.check("patterns-have-agents", empty_patterns.len() <= pattern_agents.len() / 5,
        &format!("{}/{} patterns have agents", pattern_agents.len() - empty_patterns.len(), pattern_agents.len()));

    // ── Workflow checks ──
    r.check("workflows-exist", workflow_count >= 10,
        &format!("{} workflows in catalog", workflow_count));

    let workflows = catalog::list_workflows();
    let mut valid_wf = 0;
    let mut invalid_wf = Vec::new();
    for (wf_id, _, _) in &workflows {
        match catalog::get_workflow_phases(wf_id) {
            Some(phases) if !phases.is_empty() => { valid_wf += 1; }
            _ => { invalid_wf.push(wf_id.as_str()); }
        }
    }
    r.check("workflows-parseable", invalid_wf.len() <= workflows.len() / 10,
        &format!("{}/{} workflows parse OK; invalid: {}",
            valid_wf, workflows.len(),
            invalid_wf.iter().take(5).cloned().collect::<Vec<_>>().join(", ")));

    // Workflow phases reference valid pattern types
    let valid_phase_patterns = ["sequential", "parallel", "hierarchical", "loop",
                                "network", "debate", "router", "aggregator", "wave",
                                "solo", "human-in-the-loop", "map-reduce", "fractal",
                                "adversarial-pair", "adversarial-cascade", "blackboard",
                                "supervisor-retry", "swarm", "saga", "consensus", "pub-sub"];
    let mut bad_patterns = Vec::new();
    for (wf_id, _, _) in &workflows {
        if let Some(phases) = catalog::get_workflow_phases(wf_id) {
            for (phase_name, pattern, _) in &phases {
                if !valid_phase_patterns.contains(&pattern.as_str()) {
                    bad_patterns.push(format!("{}:{} has '{}'", wf_id, phase_name, pattern));
                }
            }
        }
    }
    r.check("workflows-valid-patterns", bad_patterns.is_empty(),
        &format!("{} invalid pattern refs: {}", bad_patterns.len(),
            bad_patterns.iter().take(3).cloned().collect::<Vec<_>>().join("; ")));

    // Workflow agent refs
    let mut wf_orphan = 0;
    let mut wf_total_refs = 0;
    for (wf_id, _, _) in &workflows {
        if let Some(phases) = catalog::get_workflow_phases(wf_id) {
            for (_, _, agent_ids) in &phases {
                for aid in agent_ids {
                    wf_total_refs += 1;
                    if !agents_set.contains(aid) {
                        wf_orphan += 1;
                    }
                }
            }
        }
    }
    r.check("workflows-valid-agent-refs", wf_orphan <= wf_total_refs / 20,
        &format!("{}/{} workflow agent refs valid (max 5% orphan)", wf_total_refs - wf_orphan, wf_total_refs));

    // Role diversity in workflows
    let mut diverse_wf = 0;
    for (wf_id, _, _) in &workflows {
        if let Some(phases) = catalog::get_workflow_phases(wf_id) {
            let all_aids: std::collections::HashSet<String> = phases.iter()
                .flat_map(|(_, _, aids)| aids.clone())
                .collect();
            if all_aids.len() >= 3 {
                diverse_wf += 1;
            }
        }
    }
    r.check("workflows-role-diversity", diverse_wf >= workflows.len() / 4,
        &format!("{}/{} workflows have >=3 distinct agents (min 25%)", diverse_wf, workflows.len()));

    // Org check: minimum viable SAFe org
    let agents = catalog::all_agents();
    let role_counts: std::collections::HashMap<&str, usize> = {
        let mut m = std::collections::HashMap::new();
        for a in &agents {
            let nr = tools::normalize_role(&a.role);
            *m.entry(nr).or_insert(0) += 1;
        }
        m
    };
    let safe_roles = ["rte", "product_owner", "scrum_master", "developer",
                      "qa_lead", "cloud_architect", "devops", "security"];
    let safe_covered: Vec<&str> = safe_roles.iter()
        .filter(|r| role_counts.get(*r).copied().unwrap_or(0) > 0)
        .copied()
        .collect();
    r.check("org-safe-roles", safe_covered.len() == safe_roles.len(),
        &format!("{}/{} SAFe roles: {}", safe_covered.len(), safe_roles.len(),
            safe_covered.join(", ")));

    let min_devs = role_counts.get("developer").copied().unwrap_or(0)
        + role_counts.get("lead_dev").copied().unwrap_or(0)
        + role_counts.get("lead_frontend").copied().unwrap_or(0)
        + role_counts.get("lead_backend").copied().unwrap_or(0);
    r.check("org-enough-devs", min_devs >= 5,
        &format!("{} developer-type agents (min 5)", min_devs));

    let min_qa = role_counts.get("qa_lead").copied().unwrap_or(0)
        + role_counts.get("qa").copied().unwrap_or(0);
    r.check("org-enough-qa", min_qa >= 2,
        &format!("{} QA agents (min 2)", min_qa));

    r
}

// ── Full eval: all three domains ──

pub fn run_full_eval() -> Vec<EvalResult> {
    vec![
        eval_all_skills(),
        eval_all_agents(),
        eval_all_patterns(),
    ]
}

pub fn full_eval_report() -> String {
    let results = run_full_eval();
    let mut report = String::from("═══ SF Eval Report ═══\n\n");
    let mut total_pass = 0;
    let mut total_cases = 0;
    for r in &results {
        report.push_str(&r.summary());
        report.push_str("\n\n");
        total_pass += r.passed();
        total_cases += r.total();
    }
    report.push_str(&format!("TOTAL: {}/{} ({:.0}%)\n",
        total_pass, total_cases,
        if total_cases > 0 { total_pass as f64 / total_cases as f64 * 100.0 } else { 0.0 }));
    report
}
