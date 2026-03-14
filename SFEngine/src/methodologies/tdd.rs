// Ref: FT-SSF-023

#[derive(Debug, Clone, PartialEq)]
pub enum TDDPhase {
    Red,
    Green,
    Refactor,
}

#[derive(Debug, Clone)]
pub struct TDDCycle {
    pub phase: TDDPhase,
    pub test_file: String,
    pub impl_file: String,
    pub iterations: usize,
}

impl TDDCycle {
    pub fn new(test: &str, impl_: &str) -> Self {
        Self {
            phase: TDDPhase::Red,
            test_file: test.to_string(),
            impl_file: impl_.to_string(),
            iterations: 0,
        }
    }

    pub fn advance(&mut self) -> TDDPhase {
        self.phase = match self.phase {
            TDDPhase::Red => TDDPhase::Green,
            TDDPhase::Green => TDDPhase::Refactor,
            TDDPhase::Refactor => {
                self.iterations += 1;
                TDDPhase::Red
            }
        };
        self.phase.clone()
    }

    pub fn build_prompt(&self, task: &str) -> String {
        match self.phase {
            TDDPhase::Red => format!("Write a failing test for: {task}"),
            TDDPhase::Green => "Write minimal code to make this test pass".to_string(),
            TDDPhase::Refactor => "Refactor this code while keeping tests green".to_string(),
        }
    }

    pub fn is_complete(&self, max_cycles: usize) -> bool {
        self.iterations >= max_cycles
    }
}
