// Ref: FT-SSF-022
// Instinct Learning — extract and reinforce patterns from session history

pub struct Instinct {
    pub patterns: Vec<LearnedPattern>,
}

pub struct LearnedPattern {
    pub trigger: String,
    pub action: String,
    pub confidence: f64,
    pub seen_count: usize,
}

impl Instinct {
    pub fn new() -> Self {
        Self { patterns: Vec::new() }
    }

    pub fn extract_patterns(session_log: &str) -> Vec<LearnedPattern> {
        let mut patterns = Vec::new();
        let lines: Vec<&str> = session_log.lines().collect();
        let mut i = 0;
        while i + 1 < lines.len() {
            let trigger = lines[i].trim();
            let action = lines[i + 1].trim();
            if trigger.starts_with("SUCCESS:") || trigger.starts_with("RESOLVED:") {
                let existing = patterns.iter_mut().find(|p: &&mut LearnedPattern| {
                    p.trigger == trigger && p.action == action
                });
                if let Some(p) = existing {
                    p.seen_count += 1;
                    p.confidence = (p.confidence + 0.1).min(1.0);
                } else {
                    patterns.push(LearnedPattern {
                        trigger: trigger.to_string(),
                        action: action.to_string(),
                        confidence: 0.5,
                        seen_count: 1,
                    });
                }
            }
            i += 1;
        }
        patterns
    }

    pub fn suggest(&self, context: &str) -> Option<&LearnedPattern> {
        let ctx_lower = context.to_lowercase();
        self.patterns.iter()
            .filter(|p| {
                p.confidence > 0.3
                    && p.trigger.to_lowercase().split_whitespace()
                        .any(|w| ctx_lower.contains(w))
            })
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn reinforce(&mut self, pattern_idx: usize, success: bool) {
        if let Some(p) = self.patterns.get_mut(pattern_idx) {
            if success {
                p.confidence = (p.confidence + 0.1).min(1.0);
                p.seen_count += 1;
            } else {
                p.confidence = (p.confidence - 0.15).max(0.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reinforce_adjusts_confidence() {
        let mut inst = Instinct::new();
        inst.patterns.push(LearnedPattern {
            trigger: "test".into(),
            action: "fix".into(),
            confidence: 0.5,
            seen_count: 1,
        });
        inst.reinforce(0, true);
        assert!((inst.patterns[0].confidence - 0.6).abs() < f64::EPSILON);
        inst.reinforce(0, false);
        assert!((inst.patterns[0].confidence - 0.45).abs() < f64::EPSILON);
    }

    #[test]
    fn extract_patterns_from_log() {
        let log = "SUCCESS: build passed\nrun tests\nSUCCESS: build passed\nrun tests\n";
        let patterns = Instinct::extract_patterns(log);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn suggest_matches_context() {
        let mut inst = Instinct::new();
        inst.patterns.push(LearnedPattern {
            trigger: "build error".into(),
            action: "recompile".into(),
            confidence: 0.8,
            seen_count: 3,
        });
        let suggestion = inst.suggest("there was a build failure");
        assert!(suggestion.is_some());
    }
}
