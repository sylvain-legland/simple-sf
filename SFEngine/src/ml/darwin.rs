// Ref: FT-SSF-022
// Darwin Selection — ELO-based agent fitness

use std::collections::HashMap;

const K_FACTOR: f64 = 32.0;
const INITIAL_RATING: f64 = 1000.0;

pub struct DarwinSelector {
    pub ratings: HashMap<String, f64>,
}

impl DarwinSelector {
    pub fn new() -> Self {
        Self { ratings: HashMap::new() }
    }

    fn ensure_agent(&mut self, id: &str) {
        self.ratings.entry(id.to_string()).or_insert(INITIAL_RATING);
    }

    pub fn record_match(&mut self, winner: &str, loser: &str) {
        self.ensure_agent(winner);
        self.ensure_agent(loser);

        let r_w = self.ratings[winner];
        let r_l = self.ratings[loser];

        let expected_w = 1.0 / (1.0 + 10.0_f64.powf((r_l - r_w) / 400.0));
        let expected_l = 1.0 - expected_w;

        self.ratings.insert(winner.to_string(), r_w + K_FACTOR * (1.0 - expected_w));
        self.ratings.insert(loser.to_string(), r_l + K_FACTOR * (0.0 - expected_l));
    }

    pub fn select_top(&self, n: usize) -> Vec<String> {
        self.ranking().into_iter().take(n).map(|(id, _)| id).collect()
    }

    pub fn ranking(&self) -> Vec<(String, f64)> {
        let mut ranked: Vec<(String, f64)> = self
            .ratings
            .iter()
            .map(|(id, r)| (id.clone(), *r))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        ranked
    }
}

impl Default for DarwinSelector {
    fn default() -> Self {
        Self::new()
    }
}
