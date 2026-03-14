// Ref: FT-SSF-027
//! Design pattern registry — documents which GoF patterns are used in SF Simple

pub struct PatternUsage {
    pub pattern: &'static str,
    pub category: &'static str,
    pub location: &'static str,
    pub description: &'static str,
}

pub fn all_patterns() -> Vec<PatternUsage> {
    vec![
        // Creational
        PatternUsage { pattern: "Singleton", category: "Creational",
            location: "db.rs (DB_INSTANCE)",
            description: "Global database instance via OnceLock" },
        PatternUsage { pattern: "Factory", category: "Creational",
            location: "agents.rs, llm.rs",
            description: "Agent creation from JSON, LLM provider factory" },
        PatternUsage { pattern: "Builder", category: "Creational",
            location: "engine/workflow.rs",
            description: "WorkflowPlan construction from YAML/JSON phases" },
        // Structural
        PatternUsage { pattern: "Adapter", category: "Structural",
            location: "ffi.rs, llm.rs",
            description: "C FFI adapter for Swift, 10 LLM provider adapters" },
        PatternUsage { pattern: "Decorator", category: "Structural",
            location: "design_patterns/decorator.rs",
            description: "Composable agent wrappers: logging, timing, caching, guard" },
        PatternUsage { pattern: "Facade", category: "Structural",
            location: "ffi.rs, engine/mod.rs",
            description: "SFBridge facade for Swift UI, engine module facade" },
        PatternUsage { pattern: "Proxy", category: "Structural",
            location: "design_patterns/proxy.rs",
            description: "Lazy-loading, access-controlled agent proxy" },
        // Behavioral
        PatternUsage { pattern: "Strategy", category: "Behavioral",
            location: "llm.rs, sandbox.rs",
            description: "LLM provider strategy, sandbox execution strategy" },
        PatternUsage { pattern: "Observer", category: "Behavioral",
            location: "executor.rs (EventCallback)",
            description: "Agent event callbacks: Thinking, ToolCall, Response" },
        PatternUsage { pattern: "Chain of Responsibility", category: "Behavioral",
            location: "guard.rs",
            description: "25 regex rules chained for validation scoring" },
        PatternUsage { pattern: "State Machine", category: "Behavioral",
            location: "engine/mission.rs",
            description: "Mission phase state machine (TLA+ verified)" },
        PatternUsage { pattern: "Command", category: "Behavioral",
            location: "tools/*.rs",
            description: "Tool execution as command objects" },
        PatternUsage { pattern: "Template Method", category: "Behavioral",
            location: "engine/phase.rs",
            description: "Phase execution lifecycle: prepare->execute->guard->store" },
        PatternUsage { pattern: "Iterator", category: "Behavioral",
            location: "engine/patterns.rs",
            description: "Pattern dispatch iterates over agents" },
        PatternUsage { pattern: "Mediator", category: "Behavioral",
            location: "engine/mission.rs",
            description: "Mission engine mediates agent-to-agent interactions" },
    ]
}

pub fn patterns_by_category(category: &str) -> Vec<&'static PatternUsage> {
    let all: &'static Vec<PatternUsage> = Box::leak(Box::new(all_patterns()));
    all.iter().filter(|p| p.category == category).collect()
}

pub fn pattern_coverage() -> (usize, usize) { (all_patterns().len(), 15) }

pub fn format_pattern_report() -> String {
    let mut out = String::from("| Pattern | Category | Location | Description |\n|---------|----------|----------|-------------|\n");
    for p in all_patterns() {
        out.push_str(&format!("| {} | {} | {} | {} |\n", p.pattern, p.category, p.location, p.description));
    }
    let (n, t) = pattern_coverage();
    out.push_str(&format!("\nCoverage: {n}/{t} patterns implemented\n"));
    out
}
