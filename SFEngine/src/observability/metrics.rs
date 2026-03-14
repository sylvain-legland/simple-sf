// Ref: FT-SSF-025 — Prometheus-compatible metrics (no external deps)
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Counter {
    pub name: String,
    pub help: String,
    pub value: u64,
}

#[derive(Debug, Clone)]
pub struct Histogram {
    pub name: String,
    pub help: String,
    pub buckets: Vec<f64>,
    pub values: Vec<f64>,
}

pub struct MetricsRegistry {
    pub counters: HashMap<String, Counter>,
    pub histograms: HashMap<String, Histogram>,
}

impl MetricsRegistry {
    /// Create a registry pre-loaded with SF Simple standard metrics.
    pub fn new() -> Self {
        let mut reg = Self {
            counters: HashMap::new(),
            histograms: HashMap::new(),
        };
        let default_hist_buckets = vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0];

        for (name, help) in [
            ("sf_agent_rounds_total", "Total agent execution rounds"),
            ("sf_llm_calls_total", "Total LLM API calls"),
            ("sf_tool_calls_total", "Total tool invocations"),
            ("sf_guard_rejections_total", "Total adversarial guard rejections"),
        ] {
            reg.counters.insert(
                name.to_string(),
                Counter { name: name.to_string(), help: help.to_string(), value: 0 },
            );
        }

        for (name, help) in [
            ("sf_llm_latency_seconds", "LLM call latency in seconds"),
            ("sf_mission_duration_seconds", "Mission total duration in seconds"),
            ("sf_phase_duration_seconds", "Phase duration in seconds"),
        ] {
            reg.histograms.insert(
                name.to_string(),
                Histogram {
                    name: name.to_string(),
                    help: help.to_string(),
                    buckets: default_hist_buckets.clone(),
                    values: Vec::new(),
                },
            );
        }
        reg
    }

    pub fn inc_counter(&mut self, name: &str) {
        if let Some(c) = self.counters.get_mut(name) {
            c.value += 1;
        }
    }

    pub fn inc_counter_by(&mut self, name: &str, n: u64) {
        if let Some(c) = self.counters.get_mut(name) {
            c.value += n;
        }
    }

    pub fn observe_histogram(&mut self, name: &str, value: f64) {
        if let Some(h) = self.histograms.get_mut(name) {
            h.values.push(value);
        }
    }

    /// Render all metrics in Prometheus text exposition format.
    pub fn export_prometheus(&self) -> String {
        let mut out = String::new();

        let mut counter_names: Vec<&String> = self.counters.keys().collect();
        counter_names.sort();
        for name in counter_names {
            let c = &self.counters[name];
            out.push_str(&format!("# HELP {} {}\n", c.name, c.help));
            out.push_str(&format!("# TYPE {} counter\n", c.name));
            out.push_str(&format!("{} {}\n", c.name, c.value));
        }

        let mut hist_names: Vec<&String> = self.histograms.keys().collect();
        hist_names.sort();
        for name in hist_names {
            let h = &self.histograms[name];
            out.push_str(&format!("# HELP {} {}\n", h.name, h.help));
            out.push_str(&format!("# TYPE {} histogram\n", h.name));
            let mut cumulative = 0u64;
            for bucket in &h.buckets {
                cumulative += h.values.iter().filter(|&&v| v <= *bucket).count() as u64;
                out.push_str(&format!("{}_bucket{{le=\"{}\"}} {}\n", h.name, bucket, cumulative));
            }
            let total: f64 = h.values.iter().sum();
            out.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", h.name, h.values.len()));
            out.push_str(&format!("{}_sum {}\n", h.name, total));
            out.push_str(&format!("{}_count {}\n", h.name, h.values.len()));
        }
        out
    }
}
