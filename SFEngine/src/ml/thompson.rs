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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_sampler_with_uniform_priors() {
        let ids = vec!["a".into(), "b".into()];
        let ts = ThompsonSampler::new(&ids);
        assert_eq!(ts.agents.len(), 2);
        assert_eq!(ts.agents["a"], (1.0, 1.0));
    }

    #[test]
    fn update_changes_alpha_beta() {
        let mut ts = ThompsonSampler::new(&vec!["x".into()]);
        ts.update("x", true);
        assert_eq!(ts.agents["x"], (2.0, 1.0));
        ts.update("x", false);
        assert_eq!(ts.agents["x"], (2.0, 2.0));
    }

    #[test]
    fn select_returns_valid_agent_and_rankings_sorted() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        let mut ts = ThompsonSampler::new(&ids);
        ts.update("b", true);
        ts.update("b", true);
        let selected = ts.select();
        assert!(ids.contains(&selected));
        let ranked = ts.rankings();
        assert!(ranked[0].1 >= ranked[1].1);
    }
}
