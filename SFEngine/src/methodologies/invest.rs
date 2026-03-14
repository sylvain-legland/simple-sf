// Ref: FT-SSF-023

#[derive(Debug, Clone)]
pub struct INVESTScore {
    pub independent: bool,
    pub negotiable: bool,
    pub valuable: bool,
    pub estimable: bool,
    pub small: bool,
    pub testable: bool,
}

pub fn check_story(title: &str, description: &str, acceptance_criteria: &[String]) -> INVESTScore {
    let desc_lower = description.to_lowercase();
    let title_lower = title.to_lowercase();
    let combined = format!("{} {}", title_lower, desc_lower);

    let dependency_markers = ["depends on", "after ", "requires "];
    let independent = !dependency_markers.iter().any(|m| combined.contains(m));

    let negotiable = description.len() > 20;

    let valuable = desc_lower.contains("so that") || desc_lower.contains("in order to");

    let estimable = !description.is_empty() && description.len() < 500;

    let small = acceptance_criteria.len() <= 5;

    let testable = !acceptance_criteria.is_empty();

    INVESTScore {
        independent,
        negotiable,
        valuable,
        estimable,
        small,
        testable,
    }
}

pub fn score(s: &INVESTScore) -> f64 {
    let count = [
        s.independent,
        s.negotiable,
        s.valuable,
        s.estimable,
        s.small,
        s.testable,
    ]
    .iter()
    .filter(|&&v| v)
    .count();
    count as f64 / 6.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_story_scores_high() {
        let ac = vec!["AC1".into(), "AC2".into()];
        let s = check_story(
            "User login",
            "As a user I want to login so that I can access the dashboard",
            &ac,
        );
        assert!(s.valuable);
        assert!(s.estimable);
        assert!(s.small);
        assert!(s.testable);
        assert!(score(&s) >= 0.8);
    }

    #[test]
    fn bad_story_scores_low() {
        let ac: Vec<String> = vec![];
        let s = check_story("x", "", &ac);
        assert!(!s.valuable);
        assert!(!s.testable);
        assert!(score(&s) <= 0.5);
    }
}
