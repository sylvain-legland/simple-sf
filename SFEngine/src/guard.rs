/// L0 Adversarial Guard — Deterministic quality checks.
/// Detects slop, mock code, fake builds, and hallucinations.
/// Ported from Python SF platform (agents/adversarial.py).

use regex::Regex;
use std::sync::OnceLock;

// Ref: FT-SSF-011
pub struct GuardResult {
    pub passed: bool,
    pub score: i32,
    pub issues: Vec<String>,
}

impl GuardResult {
    pub fn approve() -> Self {
        Self { passed: true, score: 0, issues: vec![] }
    }
}

struct Pattern {
    regex: Regex,
    label: &'static str,
    score: i32,
}

fn patterns() -> &'static Vec<Pattern> {
    static PATTERNS: OnceLock<Vec<Pattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        let mut p = Vec::new();
        let add = |p: &mut Vec<Pattern>, pat: &str, label: &'static str, score: i32| {
            if let Ok(r) = Regex::new(&format!("(?i){}", pat)) {
                p.push(Pattern { regex: r, label, score });
            }
        };

        // SLOP — generic filler
        add(&mut p, r"\blorem ipsum\b", "Lorem ipsum placeholder", 3);
        add(&mut p, r"\bfoo\s*bar\s*baz\b", "Placeholder foo/bar/baz", 3);
        add(&mut p, r"example\.com", "example.com placeholder URL", 3);
        add(&mut p, r"\bTBD\b", "TBD marker — incomplete", 3);
        add(&mut p, r"\bXXX\b", "XXX marker — needs attention", 3);

        // MOCK — fake implementations
        add(&mut p, r"#\s*TODO\s*:?\s*implement", "TODO implement marker", 4);
        add(&mut p, r"//\s*TODO\s*:?\s*implement", "TODO implement marker", 4);
        add(&mut p, r"raise\s+NotImplementedError", "NotImplementedError stub", 4);
        add(&mut p, r"pass\s*#\s*(?:todo|fixme|implement)", "pass with TODO", 4);
        add(&mut p, r"(?:fake|mock|dummy|hardcoded)\s+(?:data|response|result)", "Fake/mock data", 4);

        // FAKE BUILD — scripts that do nothing
        add(&mut p, r#"echo\s+['"].*placeholder"#, "Fake build — placeholder", 7);
        add(&mut p, r#"echo\s+['"]BUILD\s+SUCCESS"#, "Fake build — hardcoded SUCCESS", 7);
        add(&mut p, r#"echo\s+['"]Tests?\s+passed"#, "Fake build — hardcoded test pass", 7);
        add(&mut p, r"exit\s+0\s*#?\s*(?:stub|fake|placeholder)", "Fake script — exit 0", 7);

        // HALLUCINATION — claiming actions without evidence
        add(&mut p, r"j'ai\s+(?:deploye|déployé|lancé|exécuté|testé|vérifié|créé le fichier|commit)", "Claims action without tool evidence", 5);
        add(&mut p, r"i(?:'ve| have)\s+(?:deployed|tested|created|committed|executed|verified)", "Claims action without tool evidence", 5);
        add(&mut p, r"le\s+(?:build|test|deploy)\s+(?:a|est)\s+(?:réussi|passé|ok)", "Claims success without evidence", 5);

        p
    })
}

/// Run L0 deterministic guard checks on agent output.
/// Returns GuardResult with pass/fail, score, and issues found.
pub fn check_l0(content: &str, _role: &str, tool_calls: &[String]) -> GuardResult {
    let mut score = 0i32;
    let mut issues = Vec::new();

    // Check content against all patterns
    for pat in patterns() {
        if pat.regex.is_match(content) {
            score += pat.score;
            issues.push(pat.label.to_string());
        }
    }

    // Hallucination check: if agent claims tool actions but no tool_calls recorded
    if tool_calls.is_empty() {
        let claims_action = content.contains("j'ai créé") || content.contains("I've created")
            || content.contains("j'ai écrit") || content.contains("I wrote")
            || content.contains("fichier créé") || content.contains("file created");
        if claims_action {
            score += 5;
            issues.push("Claims file creation without tool evidence".into());
        }
    }

    // Too short check for dev roles
    if content.len() < 50 && !content.contains("[APPROVE]") && !content.contains("[VETO]") {
        score += 2;
        issues.push("Response too short".into());
    }

    // Threshold: score < 5 = pass, 5-6 = soft pass with warning, 7+ = reject
    let passed = score < 7;

    GuardResult { passed, score, issues }
}
