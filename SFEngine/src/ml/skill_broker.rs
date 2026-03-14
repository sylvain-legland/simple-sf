// Ref: FT-SSF-022
// Skill Broker — task-to-agent matching via keyword overlap

pub struct SkillBroker;

impl SkillBroker {
    /// Score each agent's skills against task keywords, return sorted desc.
    /// `agents` is a list of (agent_id, skills).
    pub fn match_task(
        task: &str,
        agents: &[(String, Vec<String>)],
    ) -> Vec<(String, f64)> {
        let task_words: Vec<String> = task
            .to_lowercase()
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| !w.is_empty())
            .collect();

        if task_words.is_empty() {
            return Vec::new();
        }

        let mut scores: Vec<(String, f64)> = agents
            .iter()
            .map(|(id, skills)| {
                let mut score = 0.0;
                let skills_lower: Vec<String> =
                    skills.iter().map(|s| s.to_lowercase()).collect();

                for word in &task_words {
                    for skill in &skills_lower {
                        if skill == word {
                            score += 2.0; // exact match bonus
                        } else if skill.contains(word.as_str()) {
                            score += 1.0;
                        }
                    }
                }

                // Normalize by task word count
                let normalized = score / task_words.len() as f64;
                (id.clone(), normalized)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_task_ranks_matching_skills_higher() {
        let agents = vec![
            ("alice".into(), vec!["rust".into(), "testing".into()]),
            ("bob".into(), vec!["python".into(), "web".into()]),
        ];
        let results = SkillBroker::match_task("rust testing", &agents);
        assert_eq!(results[0].0, "alice");
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn match_task_empty_returns_empty() {
        let results = SkillBroker::match_task("", &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn match_task_partial_match() {
        let agents = vec![
            ("dev".into(), vec!["javascript".into(), "react".into()]),
        ];
        let results = SkillBroker::match_task("react component", &agents);
        assert!(results[0].1 > 0.0);
    }
}
