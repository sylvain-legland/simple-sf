// Ref: FT-SSF-026

#[derive(Debug, Clone, PartialEq)]
pub enum GateLevel {
    Hard,
    Soft,
}

pub struct GateContext {
    pub code: String,
    pub test_output: String,
    pub guard_score: i32,
    pub veto_count: usize,
    pub complexity: f64,
    pub loc: usize,
}

#[derive(Debug, Clone)]
pub struct GateResult {
    pub passed: bool,
    pub message: String,
}

pub struct Gate {
    pub id: u8,
    pub name: String,
    pub level: GateLevel,
    pub check: fn(&GateContext) -> GateResult,
}

fn pass(msg: &str) -> GateResult {
    GateResult { passed: true, message: msg.into() }
}

fn fail(msg: &str) -> GateResult {
    GateResult { passed: false, message: msg.into() }
}

// Forbidden patterns in code output
const FORBIDDEN: &[&str] = &["TODO: implement", "HACK:", "FIXME:", "XXX:", "panic!(\"not implemented"];

fn check_guardrails(ctx: &GateContext) -> GateResult {
    for pat in FORBIDDEN {
        if ctx.code.contains(pat) {
            return fail(&format!("Forbidden pattern found: {}", pat));
        }
    }
    pass("No forbidden patterns")
}

fn check_veto(ctx: &GateContext) -> GateResult {
    if ctx.veto_count > 0 { fail(&format!("{} unresolved veto(s)", ctx.veto_count)) }
    else { pass("No unresolved vetoes") }
}

fn check_prompt_injection(ctx: &GateContext) -> GateResult {
    if ctx.guard_score >= 7 { fail(&format!("Guard score {} >= 7", ctx.guard_score)) }
    else { pass("Guard score acceptable") }
}

fn check_tool_acl(_ctx: &GateContext) -> GateResult {
    // Structural gate — actual ACL check done at runtime
    pass("Tool ACL delegated to executor")
}

fn check_adversarial_l0(ctx: &GateContext) -> GateResult {
    if ctx.guard_score >= 7 { fail("L0 detectors triggered") }
    else { pass("L0 detectors passed") }
}

fn check_adversarial_l1(ctx: &GateContext) -> GateResult {
    if ctx.guard_score >= 5 { fail(&format!("Semantic score {} >= 5", ctx.guard_score)) }
    else { pass("Semantic check passed") }
}

fn check_ac_reward(ctx: &GateContext) -> GateResult {
    // Quality threshold at 60%
    if ctx.complexity > 0.0 && ctx.complexity < 0.6 {
        fail(&format!("Quality {:.0}% < 60%", ctx.complexity * 100.0))
    } else {
        pass("AC reward acceptable")
    }
}

fn check_convergence(_ctx: &GateContext) -> GateResult {
    pass("Convergence check (advisory)")
}

fn check_rbac(_ctx: &GateContext) -> GateResult {
    pass("RBAC delegated to auth layer")
}

fn check_lint(ctx: &GateContext) -> GateResult {
    if ctx.test_output.contains("error[") || ctx.test_output.contains("error:") {
        fail("Compile errors detected")
    } else {
        pass("No compile errors")
    }
}

fn check_tests(ctx: &GateContext) -> GateResult {
    if ctx.test_output.contains("FAILED") || ctx.test_output.contains("failures:") {
        fail("Test failures detected")
    } else {
        pass("All tests pass")
    }
}

fn check_complexity(ctx: &GateContext) -> GateResult {
    if ctx.loc > 500 {
        fail(&format!("LOC {} > 500", ctx.loc))
    } else if ctx.complexity > 10.0 {
        fail(&format!("Complexity {:.1} > 10", ctx.complexity))
    } else {
        pass("Complexity within limits")
    }
}

fn check_sonar(_ctx: &GateContext) -> GateResult {
    pass("Sonar metrics (advisory)")
}

fn check_deploy_canary(_ctx: &GateContext) -> GateResult {
    pass("Canary health delegated to deploy phase")
}

fn check_output_validator(ctx: &GateContext) -> GateResult {
    let halluc_markers = ["I cannot", "As an AI", "I don't have access"];
    for m in halluc_markers {
        if ctx.code.contains(m) {
            return fail(&format!("Hallucination marker: {}", m));
        }
    }
    pass("No hallucination detected")
}

fn check_stale_prune(_ctx: &GateContext) -> GateResult {
    pass("Stale artifact check (advisory)")
}

fn check_coverage(_ctx: &GateContext) -> GateResult {
    pass("Coverage check (advisory, requires external data)")
}

pub fn all_gates() -> Vec<Gate> {
    vec![
        Gate { id: 1,  name: "Guardrails".into(),        level: GateLevel::Hard, check: check_guardrails },
        Gate { id: 2,  name: "Veto check".into(),         level: GateLevel::Hard, check: check_veto },
        Gate { id: 3,  name: "Prompt injection".into(),   level: GateLevel::Hard, check: check_prompt_injection },
        Gate { id: 4,  name: "Tool ACL".into(),            level: GateLevel::Hard, check: check_tool_acl },
        Gate { id: 5,  name: "Adversarial L0".into(),     level: GateLevel::Hard, check: check_adversarial_l0 },
        Gate { id: 6,  name: "Adversarial L1".into(),     level: GateLevel::Soft, check: check_adversarial_l1 },
        Gate { id: 7,  name: "AC Reward".into(),           level: GateLevel::Hard, check: check_ac_reward },
        Gate { id: 8,  name: "Convergence".into(),         level: GateLevel::Soft, check: check_convergence },
        Gate { id: 9,  name: "RBAC".into(),                level: GateLevel::Hard, check: check_rbac },
        Gate { id: 10, name: "Ruff/Clippy".into(),        level: GateLevel::Hard, check: check_lint },
        Gate { id: 11, name: "Tests pass".into(),          level: GateLevel::Hard, check: check_tests },
        Gate { id: 12, name: "Complexity".into(),          level: GateLevel::Soft, check: check_complexity },
        Gate { id: 13, name: "Sonar".into(),               level: GateLevel::Soft, check: check_sonar },
        Gate { id: 14, name: "Deploy canary".into(),      level: GateLevel::Hard, check: check_deploy_canary },
        Gate { id: 15, name: "Output validator".into(),   level: GateLevel::Soft, check: check_output_validator },
        Gate { id: 16, name: "Stale prune".into(),        level: GateLevel::Soft, check: check_stale_prune },
        Gate { id: 17, name: "Coverage".into(),            level: GateLevel::Soft, check: check_coverage },
    ]
}

pub fn run_all_gates(ctx: &GateContext) -> Vec<(u8, GateResult)> {
    all_gates().iter().map(|g| (g.id, (g.check)(ctx))).collect()
}

pub fn hard_gates_pass(results: &[(u8, GateResult)]) -> bool {
    let gates = all_gates();
    for (id, result) in results {
        if let Some(gate) = gates.iter().find(|g| g.id == *id) {
            if gate.level == GateLevel::Hard && !result.passed {
                return false;
            }
        }
    }
    true
}
