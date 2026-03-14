// Ref: FT-SSF-023

#[derive(Debug, Clone)]
pub struct XPPractices {
    pub pair_programming: bool,
    pub ci: bool,
    pub collective_ownership: bool,
    pub simple_design: bool,
    pub refactoring: bool,
}

impl Default for XPPractices {
    fn default() -> Self {
        Self {
            pair_programming: true,
            ci: true,
            collective_ownership: true,
            simple_design: true,
            refactoring: true,
        }
    }
}

impl XPPractices {
    pub fn check_compliance(code: &str) -> Vec<(String, bool)> {
        let line_count = code.lines().count();
        let todo_count = code.matches("TODO").count() + code.matches("FIXME").count();

        vec![
            ("simple_design: file under 500 lines".to_string(), line_count <= 500),
            ("refactoring: TODO/FIXME count <= 5".to_string(), todo_count <= 5),
        ]
    }

    pub fn pair_prompt(driver_task: &str, navigator_role: &str) -> String {
        format!(
            "Pair programming session:\n\
             Driver task: {}\n\
             Navigator role: {}\n\
             Navigator: review each change, suggest improvements, catch bugs.\n\
             Driver: implement the task, explain your reasoning.",
            driver_task, navigator_role
        )
    }
}
