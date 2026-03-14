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
