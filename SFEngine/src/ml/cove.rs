// Ref: FT-SSF-022
// Chain of Verification (CoVe) — structured claim verification

#[derive(Debug, Clone)]
pub struct CoveResult {
    pub verified: bool,
    pub confidence: f64,
    pub issues: Vec<String>,
}

/// Phase 1: Generate verification questions from the claim.
pub fn plan_questions(claim: &str) -> Vec<String> {
    vec![
        format!("Is the core assertion in '{}' factually supported?", claim),
        format!("Are there logical contradictions in '{}'?", claim),
        format!("Does '{}' contain unsupported quantitative claims?", claim),
    ]
}

/// Phase 2: Check each step against verification questions.
pub fn check_steps(steps: &[String], questions: &[String]) -> Vec<(String, bool)> {
    steps
        .iter()
        .map(|step| {
            let passed = questions.iter().all(|q| {
                // Heuristic: step addresses the question if it shares keywords
                let q_words: Vec<&str> = q.split_whitespace().collect();
                let matches = q_words.iter().filter(|w| step.to_lowercase().contains(&w.to_lowercase())).count();
                matches >= 2
            });
            (step.clone(), passed)
        })
        .collect()
}

/// Phase 3: Aggregate results into a confidence score.
pub fn aggregate(checks: &[(String, bool)]) -> CoveResult {
    if checks.is_empty() {
        return CoveResult { verified: false, confidence: 0.0, issues: vec!["No steps to verify".into()] };
    }

    let passed = checks.iter().filter(|(_, ok)| *ok).count();
    let confidence = passed as f64 / checks.len() as f64;
    let issues: Vec<String> = checks
        .iter()
        .filter(|(_, ok)| !ok)
        .map(|(step, _)| format!("Unverified: {}", step))
        .collect();

    CoveResult {
        verified: confidence >= 0.7,
        confidence,
        issues,
    }
}

/// Full verification pipeline (synchronous version).
pub fn verify_chain(claim: &str, steps: &[String]) -> CoveResult {
    let questions = plan_questions(claim);
    let checks = check_steps(steps, &questions);
    aggregate(&checks)
}
