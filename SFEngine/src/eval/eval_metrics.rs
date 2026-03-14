/// Eval metric definitions and single-item evaluators.
///
/// Ref: FT-SSF-025

use crate::db;
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

// ── L10: Skill Quality Metrics ──

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
pub(crate) const DIRECTIVE_WORDS: &[&str] = &[
    "use ", "always ", "never ", "must ", "should ", "do not ", "ensure ",
    "verify ", "check ", "apply ", "enforce ", "implement ", "follow ",
    "avoid ", "require ", "when ", "activate ", "guide",
];

/// Placeholder patterns that indicate incomplete content.
pub(crate) const PLACEHOLDER_PATTERNS: &[&str] = &[
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
