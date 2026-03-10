/// Eval harness for Skills, Agents, and Pattern/Org quality.
///
/// Inspired by Phil Schmid's skill testing framework (philschmid.de/testing-skills)
/// and the platform's existing 8-layer AC bench.
///
/// Three eval domains:
///   L10 — Skill Quality (deterministic + optional LLM judge)
///   L11 — Agent Configuration (role-tool-skill coherence)
///   L12 — Pattern & Org (coverage, structure, workflow integrity)

use crate::{catalog, db, tools};
use serde::Serialize;

// ── Eval Result ──

#[derive(Debug, Clone, Serialize)]
pub struct EvalResult {
    pub domain: String,
    pub cases: Vec<EvalCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvalCase {
    pub id: String,
    pub passed: bool,
    pub detail: String,
}

impl EvalResult {
    pub fn new(domain: &str) -> Self {
        Self { domain: domain.into(), cases: vec![] }
    }

    pub fn check(&mut self, id: &str, passed: bool, detail: &str) {
        self.cases.push(EvalCase {
            id: id.into(), passed, detail: detail.into(),
        });
    }

    pub fn passed(&self) -> usize { self.cases.iter().filter(|c| c.passed).count() }
    pub fn total(&self) -> usize { self.cases.len() }
    pub fn all_passed(&self) -> bool { self.cases.iter().all(|c| c.passed) }

    pub fn summary(&self) -> String {
        let fails: Vec<&EvalCase> = self.cases.iter().filter(|c| !c.passed).collect();
        let mut s = format!("{}: {}/{}", self.domain, self.passed(), self.total());
        for f in fails {
            s.push_str(&format!("\n  ✗ {}: {}", f.id, f.detail));
        }
        s
    }
}

// ── L10: Skill Quality Eval ──

#[derive(Debug, Clone, Serialize)]
pub struct SkillCheck {
    pub skill_id: String,
    pub has_name: bool,
    pub has_description: bool,
    pub description_actionable: bool,
    pub has_content: bool,
    pub content_substantial: bool,
    pub no_placeholders: bool,
    pub has_tags: bool,
    pub overall: bool,
}

/// Directive keywords that indicate actionable instructions.
const DIRECTIVE_WORDS: &[&str] = &[
    "use ", "always ", "never ", "must ", "should ", "do not ", "ensure ",
    "verify ", "check ", "apply ", "enforce ", "implement ", "follow ",
    "avoid ", "require ", "when ", "activate ", "guide",
];

/// Placeholder patterns that indicate incomplete content.
const PLACEHOLDER_PATTERNS: &[&str] = &[
    "{{", "}}", "TODO", "TBD", "FIXME", "XXX",
    "INSERT_HERE", "PLACEHOLDER", "CHANGE_ME",
];

/// Eval a single skill's quality (deterministic checks — no LLM).
pub fn eval_skill(skill_id: &str) -> Option<SkillCheck> {
    let (name, desc, content, tags) = db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT name, description, content, tags_json FROM skills WHERE id = ?1"
        ).ok()?;
        stmt.query_row([skill_id], |r| {
            Ok((
                r.get::<_, String>(0).unwrap_or_default(),
                r.get::<_, String>(1).unwrap_or_default(),
                r.get::<_, String>(2).unwrap_or_default(),
                r.get::<_, String>(3).unwrap_or_default(),
            ))
        }).ok()
    })?;

    let has_name = !name.trim().is_empty() && name.len() >= 3;
    let has_description = !desc.trim().is_empty() && desc.len() >= 20;

    let desc_lower = desc.to_lowercase();
    let content_lower = content.to_lowercase();
    let description_actionable = DIRECTIVE_WORDS.iter()
        .any(|w| desc_lower.contains(w) || content_lower.contains(w));

    let has_content = !content.trim().is_empty();
    let content_substantial = content.len() >= 200;

    let combined = format!("{}\n{}", desc, content);
    let no_placeholders = !PLACEHOLDER_PATTERNS.iter()
        .any(|p| combined.contains(p));

    let has_tags = !tags.is_empty() && tags != "[]" && tags != "null";

    let overall = has_name && has_description && description_actionable
        && has_content && content_substantial && no_placeholders;

    Some(SkillCheck {
        skill_id: skill_id.to_string(),
        has_name, has_description, description_actionable,
        has_content, content_substantial, no_placeholders, has_tags,
        overall,
    })
}

/// Eval ALL skills, return aggregate result.
pub fn eval_all_skills() -> EvalResult {
    let mut r = EvalResult::new("L10-Skills");

    let skill_ids: Vec<String> = db::with_db(|conn| {
        let mut stmt = conn.prepare("SELECT id FROM skills ORDER BY id").unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    });

    let total = skill_ids.len();
    r.check("skills-exist", total > 0,
        &format!("{} skills in catalog", total));

    let mut pass = 0;
    let mut fail_names = Vec::new();
    let mut fail_desc = Vec::new();
    let mut fail_actionable = Vec::new();
    let mut fail_content = Vec::new();
    let mut fail_substance = Vec::new();
    let mut fail_placeholder = Vec::new();
    let mut no_tags = 0;

    for sid in &skill_ids {
        if let Some(sc) = eval_skill(sid) {
            if sc.overall { pass += 1; }
            if !sc.has_name { fail_names.push(sid.clone()); }
            if !sc.has_description { fail_desc.push(sid.clone()); }
            if !sc.description_actionable { fail_actionable.push(sid.clone()); }
            if !sc.has_content { fail_content.push(sid.clone()); }
            if !sc.content_substantial { fail_substance.push(sid.clone()); }
            if !sc.no_placeholders { fail_placeholder.push(sid.clone()); }
            if !sc.has_tags { no_tags += 1; }
        }
    }

    let rate = if total > 0 { (pass as f64 / total as f64) * 100.0 } else { 0.0 };
    r.check("skills-pass-rate", rate >= 60.0,
        &format!("{}/{} pass ({:.0}%)", pass, total, rate));

    r.check("skills-have-name", fail_names.len() <= total / 20,
        &format!("{} missing name (max 5%)", fail_names.len()));

    r.check("skills-have-description", fail_desc.len() <= total / 20,
        &format!("{} missing description (max 5%)", fail_desc.len()));

    r.check("skills-actionable", fail_actionable.len() <= total / 5,
        &format!("{} not actionable (max 20%): {}",
            fail_actionable.len(),
            fail_actionable.iter().take(5).cloned().collect::<Vec<_>>().join(", ")));

    r.check("skills-have-content", fail_content.len() <= total / 10,
        &format!("{} missing content (max 10%)", fail_content.len()));

    r.check("skills-substantial", fail_substance.len() <= total / 3,
        &format!("{} too short <200 chars (max 33%)", fail_substance.len()));

    r.check("skills-no-placeholders", fail_placeholder.len() <= total / 10,
        &format!("{} have placeholder patterns (max 10%)", fail_placeholder.len()));

    r.check("skills-have-tags", no_tags <= total / 2,
        &format!("{}/{} have tags", total - no_tags, total));

    // SecureByDesign coverage check
    let sbd = eval_skill("securebydesign");
    r.check("skills-securebydesign", sbd.map_or(false, |s| s.overall),
        "SecureByDesign skill present and complete");

    // Domain coverage: at least skills in these domains
    let domains = ["security", "test", "code-review", "architecture", "devops",
                   "ux", "data", "project-management"];
    let covered: Vec<&str> = domains.iter().filter(|d| {
        skill_ids.iter().any(|sid| sid.contains(*d))
    }).copied().collect();
    r.check("skills-domain-coverage", covered.len() >= 6,
        &format!("{}/{} domains covered: {}", covered.len(), domains.len(), covered.join(", ")));

    r
}

// ── L11: Agent Configuration Eval ──

/// Eval ALL agents for configuration coherence.
pub fn eval_all_agents() -> EvalResult {
    let mut r = EvalResult::new("L11-Agents");

    let agents = catalog::all_agents();
    let total = agents.len();
    r.check("agents-exist", total >= 100,
        &format!("{} agents in catalog", total));

    // Check zero-tool and zero-skill agents
    let zero_tools: Vec<&str> = agents.iter()
        .filter(|a| a.tools.is_empty())
        .map(|a| a.id.as_str())
        .collect();
    r.check("agents-have-tools", zero_tools.is_empty(),
        &format!("{} agents with 0 tools: {}",
            zero_tools.len(),
            zero_tools.iter().take(5).cloned().collect::<Vec<_>>().join(", ")));

    let zero_skills: Vec<&str> = agents.iter()
        .filter(|a| a.skills.is_empty())
        .map(|a| a.id.as_str())
        .collect();
    r.check("agents-have-skills", zero_skills.is_empty(),
        &format!("{} agents with 0 skills: {}",
            zero_skills.len(),
            zero_skills.iter().take(5).cloned().collect::<Vec<_>>().join(", ")));

    // Role-tool coherence: agent's tools should overlap significantly with role tools
    let mut role_mismatch = 0;
    let mut role_details = Vec::new();
    for a in &agents {
        let norm = tools::normalize_role(&a.role);
        let expected = tools::tool_schemas_for_role(&a.role);
        let expected_names: std::collections::HashSet<String> = expected.iter()
            .filter_map(|s| s.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from))
            .collect();
        // Check overlap: at least 50% of agent's declared tools should be in role tools
        let overlap = a.tools.iter()
            .filter(|t| expected_names.contains(t.as_str()))
            .count();
        let overlap_pct = if a.tools.is_empty() { 100.0 }
            else { overlap as f64 / a.tools.len() as f64 * 100.0 };
        if overlap_pct < 50.0 {
            role_mismatch += 1;
            if role_details.len() < 3 {
                role_details.push(format!("{}({}): {:.0}% overlap with {}", a.id, a.role, overlap_pct, norm));
            }
        }
    }
    r.check("agents-role-tool-coherence", role_mismatch <= total / 10,
        &format!("{} agents with tool/role mismatch (max 10%): {}",
            role_mismatch, role_details.join("; ")));

    // Persona quality: must have name, role, and persona/system_prompt
    let no_persona: Vec<&str> = agents.iter()
        .filter(|a| a.persona.is_empty() && a.system_prompt.is_empty())
        .map(|a| a.id.as_str())
        .collect();
    r.check("agents-have-persona", no_persona.is_empty(),
        &format!("{} agents missing persona/system_prompt", no_persona.len()));

    let no_name: Vec<&str> = agents.iter()
        .filter(|a| a.name.is_empty())
        .map(|a| a.id.as_str())
        .collect();
    r.check("agents-have-name", no_name.is_empty(),
        &format!("{} agents missing name", no_name.len()));

    // Role distribution: critical roles must be present
    let required_roles = ["rte", "product_owner", "developer", "qa_lead",
                          "cloud_architect", "devops", "security", "lead_dev",
                          "scrum_master", "ux_designer"];
    let mut covered_roles = Vec::new();
    for req in &required_roles {
        let found = agents.iter().any(|a| tools::normalize_role(&a.role) == *req);
        if found { covered_roles.push(*req); }
    }
    r.check("agents-role-coverage", covered_roles.len() >= 8,
        &format!("{}/{} critical roles: {}", covered_roles.len(), required_roles.len(),
            covered_roles.join(", ")));

    // Name uniqueness
    let mut names: Vec<&str> = agents.iter().map(|a| a.name.as_str()).collect();
    names.sort();
    let unique = names.windows(2).filter(|w| w[0] == w[1]).count();
    r.check("agents-unique-names", unique == 0,
        &format!("{} duplicate agent names", unique));

    // ID uniqueness
    let mut ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();
    ids.sort();
    let dup_ids = ids.windows(2).filter(|w| w[0] == w[1]).count();
    r.check("agents-unique-ids", dup_ids == 0,
        &format!("{} duplicate agent IDs", dup_ids));

    // Skill references validity: all agent.skills[] should exist in DB
    let all_skill_ids: Vec<String> = db::with_db(|conn| {
        let mut stmt = conn.prepare("SELECT id FROM skills").unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    });
    let skill_set: std::collections::HashSet<&str> = all_skill_ids.iter()
        .map(|s| s.as_str())
        .collect();
    let mut orphan_refs = 0;
    for a in &agents {
        for s in &a.skills {
            if !skill_set.contains(s.as_str()) {
                orphan_refs += 1;
            }
        }
    }
    r.check("agents-valid-skill-refs", orphan_refs == 0,
        &format!("{} orphan skill references (skill not in catalog)", orphan_refs));

    // Security agents must have SecureByDesign skill
    let sec_agents: Vec<&str> = agents.iter()
        .filter(|a| {
            let r = a.role.to_lowercase();
            r.contains("security") || r.contains("pentest") || r.contains("ciso")
                || r.contains("secops") || r.contains("devsecops")
                || a.id.contains("security") || a.id.contains("securite")
        })
        .filter(|a| !a.skills.iter().any(|s| s == "securebydesign"))
        .map(|a| a.id.as_str())
        .collect();
    r.check("agents-security-sbd", sec_agents.is_empty(),
        &format!("{} security agents missing SecureByDesign: {}",
            sec_agents.len(),
            sec_agents.iter().take(5).cloned().collect::<Vec<_>>().join(", ")));

    // Memory tools: devs, leads, QA should have memory_store
    let need_memory = ["developer", "lead_dev", "lead_frontend", "lead_backend", "qa_lead",
                       "devops", "cloud_architect", "scrum_master"];
    let mut missing_mem = 0;
    for a in &agents {
        let norm = tools::normalize_role(&a.role);
        if need_memory.contains(&norm) && !a.tools.contains(&"memory_store".to_string()) {
            missing_mem += 1;
        }
    }
    r.check("agents-have-memory-store", missing_mem == 0,
        &format!("{} agents missing memory_store (needed for learning loop)", missing_mem));

    // Hierarchy: at least some rank diversity
    let ranks: Vec<i64> = agents.iter().map(|a| a.hierarchy_rank).collect();
    let distinct_ranks: std::collections::HashSet<i64> = ranks.iter().copied().collect();
    r.check("agents-hierarchy-diversity", distinct_ranks.len() >= 3,
        &format!("{} distinct ranks", distinct_ranks.len()));

    r
}

// ── L12: Pattern & Org Eval ──

/// Eval patterns, workflows, and organizational coverage.
pub fn eval_all_patterns() -> EvalResult {
    let mut r = EvalResult::new("L12-Patterns");

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
        // agents_json can be: ["id1","id2"] or [{"agent_id":"id1",...},...]
        let parsed: Vec<serde_json::Value> = serde_json::from_str(agents_json).unwrap_or_default();
        let agent_ids: Vec<String> = parsed.iter().filter_map(|v| {
            // If it's a string, use directly; if object, extract agent_id
            v.as_str().map(String::from)
                .or_else(|| v.get("agent_id").and_then(|a| a.as_str()).map(String::from))
        }).collect();
        total_pattern_agents += agent_ids.len();
        for aid in &agent_ids {
            // Pattern agents can be template refs like "worker", "brain"
            // or role placeholders — only count real catalog IDs as orphans
            if !agents_set.contains(aid) && catalog::get_agent_info(aid).is_some() == false
                && !["worker", "brain", "dispatcher", "aggregator", "reviewer", "judge"].contains(&aid.as_str()) {
                // Only flag if it looks like a real agent ID (not a template)
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

    // All workflows parseable with valid phases
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

    // Workflow agent refs: all agent_ids in workflow phases should exist
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

    // Role diversity in workflows: count workflows that reference >=3 distinct agents
    let mut diverse_wf = 0;
    for (wf_id, _, _) in &workflows {
        if let Some(phases) = catalog::get_workflow_phases(wf_id) {
            let all_aids: std::collections::HashSet<String> = phases.iter()
                .flat_map(|(_, _, aids)| aids.clone())
                .collect();
            // Count as diverse if it references >=3 distinct agents
            // (even if some are workflow-specific and not in catalog)
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

    // Min agent count per critical role
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
