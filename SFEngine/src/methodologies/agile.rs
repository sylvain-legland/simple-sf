// Ref: FT-SSF-023

#[derive(Debug, Clone, Default)]
pub struct AgileMetrics {
    pub velocity: Vec<f64>,
    pub cycle_times: Vec<f64>,
    pub throughput: Vec<usize>,
}

impl AgileMetrics {
    pub fn add_sprint(&mut self, completed_points: f64, cycle_time_days: f64, items_done: usize) {
        self.velocity.push(completed_points);
        self.cycle_times.push(cycle_time_days);
        self.throughput.push(items_done);
    }

    pub fn avg_velocity(&self) -> f64 {
        if self.velocity.is_empty() {
            return 0.0;
        }
        self.velocity.iter().sum::<f64>() / self.velocity.len() as f64
    }

    pub fn trend(&self) -> &str {
        let len = self.velocity.len();
        if len < 3 {
            return "stable";
        }
        let recent = &self.velocity[len - 3..];
        if recent[2] > recent[1] && recent[1] > recent[0] {
            "improving"
        } else if recent[2] < recent[1] && recent[1] < recent[0] {
            "declining"
        } else {
            "stable"
        }
    }

    pub fn format_dashboard(&self) -> String {
        let mut out = String::from("=== Agile Dashboard ===\n");
        out.push_str(&format!("Avg velocity: {:.1}\n", self.avg_velocity()));
        out.push_str(&format!("Trend: {}\n", self.trend()));
        if !self.cycle_times.is_empty() {
            let avg_ct = self.cycle_times.iter().sum::<f64>() / self.cycle_times.len() as f64;
            out.push_str(&format!("Avg cycle time: {:.1} days\n", avg_ct));
        }
        if !self.throughput.is_empty() {
            let avg_tp = self.throughput.iter().sum::<usize>() as f64 / self.throughput.len() as f64;
            out.push_str(&format!("Avg throughput: {:.1} items/sprint\n", avg_tp));
        }
        out.push_str(&format!("Sprints tracked: {}\n", self.velocity.len()));
        out
    }
}

pub fn burndown(total: f64, completed: f64, sprints_remaining: usize) -> Vec<f64> {
    if sprints_remaining == 0 {
        return vec![total - completed];
    }
    let remaining = total - completed;
    let step = remaining / sprints_remaining as f64;
    (0..=sprints_remaining)
        .map(|i| remaining - step * i as f64)
        .collect()
}
