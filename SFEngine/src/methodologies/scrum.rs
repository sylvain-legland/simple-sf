// Ref: FT-SSF-023

#[derive(Debug, Clone)]
pub enum Ceremony {
    Planning,
    DailyStandup,
    Review,
    Retrospective,
}

#[derive(Debug, Clone)]
pub struct Sprint {
    pub number: usize,
    pub goal: String,
    pub backlog: Vec<String>,
    pub velocity: f64,
}

pub fn ceremony_prompt(ceremony: Ceremony, sprint: &Sprint) -> String {
    match ceremony {
        Ceremony::Planning => {
            let items = sprint.backlog.join(", ");
            format!(
                "Sprint {} planning. Goal: {}. Backlog: [{}]. Velocity: {:.1}",
                sprint.number, sprint.goal, items, sprint.velocity
            )
        }
        Ceremony::DailyStandup => {
            "What did you do? What will you do? Any blockers?".to_string()
        }
        Ceremony::Review => {
            let items = sprint.backlog.join(", ");
            format!(
                "Demo sprint {} results. Goal: {}. Items: [{}]",
                sprint.number, sprint.goal, items
            )
        }
        Ceremony::Retrospective => {
            "What went well? What to improve? Actions?".to_string()
        }
    }
}

pub fn calculate_velocity(completed: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    completed as f64 / total as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planning_prompt_contains_keywords() {
        let sprint = Sprint { number: 1, goal: "MVP".into(), backlog: vec!["US-1".into()], velocity: 10.0 };
        let prompt = ceremony_prompt(Ceremony::Planning, &sprint);
        assert!(prompt.contains("planning") || prompt.contains("Planning") || prompt.contains("Sprint 1"));
        assert!(prompt.contains("MVP"));
    }

    #[test]
    fn retro_prompt_contains_improve() {
        let sprint = Sprint { number: 2, goal: "G".into(), backlog: vec![], velocity: 5.0 };
        let prompt = ceremony_prompt(Ceremony::Retrospective, &sprint);
        assert!(prompt.contains("improve"));
    }

    #[test]
    fn calculate_velocity_correct() {
        assert!((calculate_velocity(8, 10) - 0.8).abs() < f64::EPSILON);
        assert_eq!(calculate_velocity(0, 0), 0.0);
    }
}
