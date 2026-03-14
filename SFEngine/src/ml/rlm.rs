// Ref: FT-SSF-022
// Recursive Language Model loops (arXiv:2512.24601) — iterative self-refinement

pub struct RLMConfig {
    pub max_depth: usize,
    pub min_confidence: f64,
    pub refinement_prompt: String,
}

pub struct RLMResult {
    pub output: String,
    pub depth: usize,
    pub confidence: f64,
    pub iterations: Vec<String>,
}

pub fn default_config() -> RLMConfig {
    RLMConfig {
        max_depth: 3,
        min_confidence: 0.8,
        refinement_prompt: "Review and improve the following output. \
            Fix any errors, fill gaps, and ensure consistency.".to_string(),
    }
}

pub fn should_recurse(current_output: &str, depth: usize, config: &RLMConfig) -> bool {
    if depth >= config.max_depth {
        return false;
    }
    if current_output.trim().is_empty() {
        return true;
    }
    let confidence = estimate_confidence(current_output);
    confidence < config.min_confidence
}

pub fn build_refinement_prompt(original_task: &str, current_output: &str, depth: usize) -> String {
    format!(
        "## Refinement Pass {depth}\n\n\
         ### Original Task\n{original_task}\n\n\
         ### Current Output (iteration {prev})\n{current_output}\n\n\
         ### Instructions\n\
         Review the output above for:\n\
         1. Correctness — fix factual or logical errors\n\
         2. Completeness — fill any gaps\n\
         3. Consistency — ensure self-consistency\n\
         4. Quality — improve clarity\n\n\
         Provide the improved version:",
        depth = depth + 1,
        prev = depth,
        original_task = original_task,
        current_output = current_output,
    )
}

fn estimate_confidence(output: &str) -> f64 {
    let len = output.len() as f64;
    let has_structure = output.contains('\n') && output.lines().count() > 2;
    let no_hedging = !output.contains("maybe") && !output.contains("not sure")
        && !output.contains("I think");
    let mut score: f64 = 0.5;
    if len > 100.0 { score += 0.15; }
    if has_structure { score += 0.15; }
    if no_hedging { score += 0.15; }
    score.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = default_config();
        assert_eq!(cfg.max_depth, 3);
        assert!((cfg.min_confidence - 0.8).abs() < f64::EPSILON);
        assert!(!cfg.refinement_prompt.is_empty());
    }

    #[test]
    fn should_recurse_false_at_max_depth() {
        let cfg = default_config();
        assert!(!should_recurse("some output", cfg.max_depth, &cfg));
    }

    #[test]
    fn should_recurse_true_for_empty() {
        let cfg = default_config();
        assert!(should_recurse("", 0, &cfg));
    }
}
