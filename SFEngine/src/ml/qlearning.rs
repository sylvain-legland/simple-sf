// Ref: FT-SSF-022
// Q-Learning for pattern selection

use std::collections::HashMap;

pub struct QLearner {
    pub q_table: HashMap<(String, String), f64>,
    pub alpha: f64,
    pub gamma: f64,
    pub epsilon: f64,
}

impl QLearner {
    pub fn new(alpha: f64, gamma: f64, epsilon: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            alpha,
            gamma,
            epsilon,
        }
    }

    /// ε-greedy action selection.
    pub fn select_action(&self, state: &str, actions: &[String]) -> String {
        if actions.is_empty() {
            return String::new();
        }

        // Deterministic ε check using state hash
        let hash = state.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let probe = (hash % 1000) as f64 / 1000.0;

        if probe < self.epsilon {
            // Explore: pick action based on hash
            actions[hash as usize % actions.len()].clone()
        } else {
            // Exploit: best known action
            self.best_action(state).unwrap_or_else(|| actions[0].clone())
        }
    }

    pub fn update(&mut self, state: &str, action: &str, reward: f64, next_state: &str) {
        let key = (state.to_string(), action.to_string());
        let current = *self.q_table.get(&key).unwrap_or(&0.0);

        let max_next = self
            .q_table
            .iter()
            .filter(|((s, _), _)| s == next_state)
            .map(|(_, v)| *v)
            .fold(0.0_f64, f64::max);

        let updated = current + self.alpha * (reward + self.gamma * max_next - current);
        self.q_table.insert(key, updated);
    }

    pub fn best_action(&self, state: &str) -> Option<String> {
        self.q_table
            .iter()
            .filter(|((s, _), _)| s == state)
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|((_, action), _)| action.clone())
    }
}
