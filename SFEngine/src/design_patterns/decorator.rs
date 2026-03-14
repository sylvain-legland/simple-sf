// Ref: FT-SSF-027
//! Composable agent wrappers (decorator pattern)

use std::collections::HashMap;

pub trait AgentWrapper {
    fn wrap(&self, agent_id: &str, input: &str) -> String;
    fn unwrap(&self, output: &str) -> String;
    fn name(&self) -> &str;
}

pub struct LoggingDecorator;
impl AgentWrapper for LoggingDecorator {
    fn wrap(&self, agent_id: &str, input: &str) -> String {
        let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        format!("[LOG {ts}] Running agent {agent_id}\n{input}")
    }
    fn unwrap(&self, output: &str) -> String {
        let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        format!("{output}\n[LOG {ts}] Agent completed")
    }
    fn name(&self) -> &str { "logging" }
}

pub struct TimingDecorator;
impl AgentWrapper for TimingDecorator {
    fn wrap(&self, _agent_id: &str, input: &str) -> String {
        let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
        format!("[TIMING start={ts}]\n{input}")
    }
    fn unwrap(&self, output: &str) -> String {
        let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
        format!("{output}\n[TIMING end={ts}]")
    }
    fn name(&self) -> &str { "timing" }
}

pub struct CachingDecorator { pub cache: HashMap<String, String> }
impl CachingDecorator { pub fn new() -> Self { Self { cache: HashMap::new() } } }
impl AgentWrapper for CachingDecorator {
    fn wrap(&self, _agent_id: &str, input: &str) -> String {
        let key = format!("{:x}", md5_hash(input));
        if let Some(cached) = self.cache.get(&key) {
            return format!("[CACHE HIT {key}] {cached}");
        }
        format!("[CACHE MISS {key}]\n{input}")
    }
    fn unwrap(&self, output: &str) -> String { output.to_string() }
    fn name(&self) -> &str { "caching" }
}

fn md5_hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325_u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

pub struct GuardDecorator;

const INJECTION_PATTERNS: &[&str] = &[
    "ignore previous", "system prompt", "you are now", "disregard",
];

impl AgentWrapper for GuardDecorator {
    fn wrap(&self, _agent_id: &str, input: &str) -> String {
        let lower = input.to_lowercase();
        for pat in INJECTION_PATTERNS {
            if lower.contains(pat) {
                return format!("[GUARD BLOCKED] Injection pattern detected: {pat}");
            }
        }
        input.to_string()
    }
    fn unwrap(&self, output: &str) -> String {
        let lower = output.to_lowercase();
        for pat in INJECTION_PATTERNS {
            if lower.contains(pat) {
                return "[GUARD BLOCKED] Output contains suspicious patterns".to_string();
            }
        }
        output.to_string()
    }
    fn name(&self) -> &str { "guard" }
}

pub struct DecoratorChain {
    decorators: Vec<Box<dyn AgentWrapper>>,
}

impl DecoratorChain {
    pub fn new() -> Self { Self { decorators: Vec::new() } }

    pub fn add(&mut self, decorator: Box<dyn AgentWrapper>) {
        self.decorators.push(decorator);
    }

    pub fn apply_pre(&self, agent_id: &str, input: &str) -> String {
        self.decorators.iter().fold(input.to_string(), |acc, d| d.wrap(agent_id, &acc))
    }

    pub fn apply_post(&self, output: &str) -> String {
        self.decorators.iter().rev().fold(output.to_string(), |acc, d| d.unwrap(&acc))
    }
}
