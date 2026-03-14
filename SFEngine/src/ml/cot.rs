// Ref: FT-SSF-022
// Chain of Thought — structured reasoning scaffolding

#[derive(Debug, PartialEq)]
pub enum ReasoningQuality {
    Thorough,
    Adequate,
    Shallow,
    Missing,
}

pub fn wrap_with_cot(task: &str) -> String {
    format!(
        "Think step by step:\n\
         1. Analyze the requirements and constraints\n\
         2. Plan the approach and identify edge cases\n\
         3. Execute the solution systematically\n\
         4. Verify correctness and completeness\n\n\
         Task: {task}"
    )
}

pub fn extract_steps(response: &str) -> Vec<String> {
    response
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let starts_numbered = trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                && trimmed.contains('.');
            if starts_numbered {
                let content = trimmed.splitn(2, '.').nth(1).unwrap_or("").trim();
                if !content.is_empty() {
                    return Some(content.to_string());
                }
            }
            None
        })
        .collect()
}

pub fn validate_reasoning(steps: &[String]) -> bool {
    if steps.is_empty() {
        return false;
    }
    steps.iter().all(|s| !s.trim().is_empty() && s.len() > 3)
}

pub fn assess_quality(steps: &[String]) -> ReasoningQuality {
    if steps.is_empty() {
        return ReasoningQuality::Missing;
    }
    let avg_len = steps.iter().map(|s| s.len()).sum::<usize>() / steps.len();
    match (steps.len(), avg_len) {
        (n, l) if n >= 4 && l > 30 => ReasoningQuality::Thorough,
        (n, l) if n >= 2 && l > 15 => ReasoningQuality::Adequate,
        _ => ReasoningQuality::Shallow,
    }
}
