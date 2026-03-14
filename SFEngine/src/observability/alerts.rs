// Ref: FT-SSF-025 — Alert rules evaluated against MetricsRegistry

#[derive(Debug, Clone)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub enum AlertCondition {
    CounterAbove(String, u64),
    HistogramP99Above(String, f64),
    RateAbove(String, f64, u64),
}

#[derive(Debug, Clone)]
pub struct AlertRule {
    pub name: String,
    pub condition: AlertCondition,
    pub severity: Severity,
    pub message: String,
}

pub struct AlertManager {
    pub rules: Vec<AlertRule>,
    pub fired: Vec<(String, u64)>,
}

impl AlertManager {
    /// Create with default SF Simple alert rules.
    pub fn new() -> Self {
        let rules = vec![
            AlertRule {
                name: "MissionStuck".to_string(),
                condition: AlertCondition::HistogramP99Above(
                    "sf_mission_duration_seconds".to_string(), 7200.0,
                ),
                severity: Severity::Critical,
                message: "Mission duration p99 exceeds 2 hours".to_string(),
            },
            AlertRule {
                name: "LLMFailing".to_string(),
                condition: AlertCondition::RateAbove(
                    "sf_guard_rejections_total".to_string(), 10.0, 300,
                ),
                severity: Severity::Critical,
                message: "Guard rejections > 10 in 5 minutes".to_string(),
            },
            AlertRule {
                name: "HighRejectRate".to_string(),
                condition: AlertCondition::CounterAbove(
                    "sf_guard_rejections_total".to_string(), 80,
                ),
                severity: Severity::Warning,
                message: "Guard rejection count exceeds 80".to_string(),
            },
            AlertRule {
                name: "AgentStuck".to_string(),
                condition: AlertCondition::CounterAbove(
                    "sf_agent_rounds_total".to_string(), 100,
                ),
                severity: Severity::Warning,
                message: "Agent rounds exceed 100".to_string(),
            },
        ];
        Self { rules, fired: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }

    /// Evaluate all rules against the current registry. Returns fired alerts.
    pub fn check(&mut self, registry: &super::metrics::MetricsRegistry) -> Vec<&AlertRule> {
        let mut triggered: Vec<usize> = Vec::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for (i, rule) in self.rules.iter().enumerate() {
            let fire = match &rule.condition {
                AlertCondition::CounterAbove(name, threshold) => {
                    registry.counters.get(name).map_or(false, |c| c.value > *threshold)
                }
                AlertCondition::HistogramP99Above(name, threshold) => {
                    registry.histograms.get(name).map_or(false, |h| {
                        if h.values.is_empty() {
                            return false;
                        }
                        let mut sorted = h.values.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let idx = ((sorted.len() as f64) * 0.99).ceil() as usize;
                        let p99 = sorted[idx.min(sorted.len() - 1)];
                        p99 > *threshold
                    })
                }
                AlertCondition::RateAbove(name, threshold, _window_secs) => {
                    // Simplified: treat counter value as rate proxy
                    registry.counters.get(name).map_or(false, |c| (c.value as f64) > *threshold)
                }
            };
            if fire {
                self.fired.push((rule.name.clone(), now));
                triggered.push(i);
            }
        }
        triggered.iter().map(|&i| &self.rules[i]).collect()
    }
}
