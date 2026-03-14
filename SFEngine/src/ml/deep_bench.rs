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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composite_score_calculation() {
        let r = BenchResult {
            agent_id: "a".into(),
            speed_ms: 0,
            quality: 1.0,
            cost_tokens: 0,
            specialization: 1.0,
        };
        let score = DeepBench::composite_score(&r);
        // quality*0.4 + speed(1.0)*0.2 + cost(1.0)*0.2 + spec*0.2 = 1.0
        assert!((score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn leaderboard_sorted_descending() {
        let mut bench = DeepBench::new();
        bench.add_result(BenchResult {
            agent_id: "slow".into(), speed_ms: 5000, quality: 0.5,
            cost_tokens: 50000, specialization: 0.3,
        });
        bench.add_result(BenchResult {
            agent_id: "fast".into(), speed_ms: 100, quality: 0.9,
            cost_tokens: 1000, specialization: 0.9,
        });
        let lb = bench.leaderboard();
        assert_eq!(lb[0].0, "fast");
        assert!(lb[0].1 > lb[1].1);
    }
}
