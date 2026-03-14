// Ref: FT-SSF-026

#[derive(Debug, Clone)]
pub enum ChaosScenario {
    LLMFailure,
    DBCorruption,
    NetworkPartition,
    Timeout,
    DiskFull,
    HighLoad,
}

#[derive(Debug, Clone)]
pub struct ChaosResult {
    pub scenario: ChaosScenario,
    pub recovered: bool,
    pub duration_ms: u64,
    pub message: String,
}

pub fn describe(scenario: &ChaosScenario) -> &str {
    match scenario {
        ChaosScenario::LLMFailure => "LLM provider returns errors or timeouts on all requests",
        ChaosScenario::DBCorruption => "Database file is corrupted or locked by another process",
        ChaosScenario::NetworkPartition => "Network connectivity lost between engine and external services",
        ChaosScenario::Timeout => "Operations exceed maximum allowed duration",
        ChaosScenario::DiskFull => "Workspace disk reaches capacity, writes fail",
        ChaosScenario::HighLoad => "CPU/memory saturated, concurrent missions compete for resources",
    }
}

pub fn injection_point(scenario: &ChaosScenario) -> &str {
    match scenario {
        ChaosScenario::LLMFailure => "llm",
        ChaosScenario::DBCorruption => "db",
        ChaosScenario::NetworkPartition => "network",
        ChaosScenario::Timeout => "executor",
        ChaosScenario::DiskFull => "filesystem",
        ChaosScenario::HighLoad => "scheduler",
    }
}

pub fn expected_behavior(scenario: &ChaosScenario) -> &str {
    match scenario {
        ChaosScenario::LLMFailure => "Fallback to next provider; retry with exponential backoff; mission paused after max retries",
        ChaosScenario::DBCorruption => "Detect corruption on read; attempt WAL recovery; fail-safe to read-only mode",
        ChaosScenario::NetworkPartition => "Queue outbound requests; continue with cached data; resume on reconnection",
        ChaosScenario::Timeout => "Cancel running operation; emit timeout event; mark phase as failed with retry",
        ChaosScenario::DiskFull => "Detect write failure; alert operator; pause artifact generation; continue in-memory",
        ChaosScenario::HighLoad => "Mission semaphore limits concurrency; queue excess missions; shed load gracefully",
    }
}

pub fn format_chaos_report(results: &[ChaosResult]) -> String {
    if results.is_empty() {
        return "Chaos: No scenarios tested.\n".into();
    }
    let recovered = results.iter().filter(|r| r.recovered).count();
    let mut out = format!(
        "Chaos Report: {}/{} scenarios recovered\n",
        recovered,
        results.len()
    );
    for r in results {
        let status = if r.recovered { "RECOVERED" } else { "FAILED" };
        out.push_str(&format!(
            "  [{}] {:?} — {}ms — {}\n",
            status, r.scenario, r.duration_ms, r.message
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_returns_non_empty() {
        let scenarios = [
            ChaosScenario::LLMFailure, ChaosScenario::DBCorruption,
            ChaosScenario::NetworkPartition, ChaosScenario::Timeout,
            ChaosScenario::DiskFull, ChaosScenario::HighLoad,
        ];
        for s in &scenarios {
            assert!(!describe(s).is_empty());
        }
    }

    #[test]
    fn all_six_scenarios_covered() {
        let scenarios = [
            ChaosScenario::LLMFailure, ChaosScenario::DBCorruption,
            ChaosScenario::NetworkPartition, ChaosScenario::Timeout,
            ChaosScenario::DiskFull, ChaosScenario::HighLoad,
        ];
        assert_eq!(scenarios.len(), 6);
        for s in &scenarios {
            assert!(!injection_point(s).is_empty());
            assert!(!expected_behavior(s).is_empty());
        }
    }
}
