// Ref: FT-SSF-022
// Thompson Sampling for agent selection

use std::collections::HashMap;

/// Simple Beta approximation: mean + noise scaled by variance.
fn beta_sample(alpha: f64, beta: f64) -> f64 {
    let mean = alpha / (alpha + beta);
    let variance = (alpha * beta) / ((alpha + beta).powi(2) * (alpha + beta + 1.0));
    let noise = simple_noise(alpha as u64 ^ beta.to_bits()) * variance.sqrt();
    (mean + noise).clamp(0.0, 1.0)
}

/// Deterministic pseudo-noise from a seed, range [-1, 1].
fn simple_noise(seed: u64) -> f64 {
    let h = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (h as i64 as f64) / i64::MAX as f64
}

pub struct ThompsonSampler {
    pub agents: HashMap<String, (f64, f64)>,
}

impl ThompsonSampler {
    pub fn new(agent_ids: &[String]) -> Self {
        let agents = agent_ids.iter().map(|id| (id.clone(), (1.0, 1.0))).collect();
        Self { agents }
    }

    pub fn select(&self) -> String {
        self.agents
            .iter()
            .map(|(id, (a, b))| (id.clone(), beta_sample(*a, *b)))
            .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap())
            .map(|(id, _)| id)
            .unwrap_or_default()
    }

    pub fn update(&mut self, agent_id: &str, success: bool) {
        if let Some(ab) = self.agents.get_mut(agent_id) {
            if success {
                ab.0 += 1.0;
            } else {
                ab.1 += 1.0;
            }
        }
    }

    pub fn rankings(&self) -> Vec<(String, f64)> {
        let mut ranked: Vec<(String, f64)> = self
            .agents
            .iter()
            .map(|(id, (a, b))| (id.clone(), a / (a + b)))
            .collect();
        ranked.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap());
        ranked
    }
}
