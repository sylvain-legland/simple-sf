// Ref: FT-SSF-022
// Multi-dimensional agent benchmarking

#[derive(Clone, Debug)]
pub struct BenchResult {
    pub agent_id: String,
    pub speed_ms: u64,
    pub quality: f64,
    pub cost_tokens: usize,
    pub specialization: f64,
}

pub struct DeepBench {
    pub results: Vec<BenchResult>,
}

impl DeepBench {
    pub fn new() -> Self {
        Self { results: Vec::new() }
    }

    pub fn add_result(&mut self, result: BenchResult) {
        self.results.push(result);
    }

    /// Weighted composite: quality*0.4 + speed*0.2 + cost*0.2 + spec*0.2
    /// Speed and cost are inverted (lower is better) and normalized.
    pub fn composite_score(result: &BenchResult) -> f64 {
        let speed_score = 1.0 / (1.0 + result.speed_ms as f64 / 1000.0);
        let cost_score = 1.0 / (1.0 + result.cost_tokens as f64 / 10000.0);
        result.quality * 0.4
            + speed_score * 0.2
            + cost_score * 0.2
            + result.specialization * 0.2
    }

    pub fn leaderboard(&self) -> Vec<(String, f64)> {
        let mut board: Vec<(String, f64)> = self
            .results
            .iter()
            .map(|r| (r.agent_id.clone(), Self::composite_score(r)))
            .collect();
        board.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        board
    }
}

impl Default for DeepBench {
    fn default() -> Self {
        Self::new()
    }
}
