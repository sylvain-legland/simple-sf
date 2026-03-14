/// Aggregate test runners for skills and agents.
///
/// Ref: FT-SSF-025

use crate::{catalog, db, tools};
use super::eval_metrics::{EvalResult, eval_skill};

// ── L10: Skill Quality Eval (aggregate) ──

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
