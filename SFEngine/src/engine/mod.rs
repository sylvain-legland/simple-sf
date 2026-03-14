// Ref: FT-SSF-020
//
// Engine module — split into logical submodules:
//   types        : shared types, constants, utilities
//   discussion   : SAFe intake discussion flow
//   workflow      : plan parsing, PM planning, gate detection, agent assignment
//   mission      : top-level mission + workflow execution
//   phase        : single-phase execution, task building, sprint checkpoints
//   build        : auto-build checks, finalize build, artifact detection
//   resilience   : retry logic, LLM health probes, server restart
//   patterns     : pattern dispatch + basic patterns (network, sequential, parallel, solo, loop)
//   patterns_ext : advanced patterns (hierarchical, aggregator, router, wave) + test wrappers

mod types;
mod discussion;
mod workflow;
mod mission;
mod phase;
mod build;
mod resilience;
mod patterns;
mod patterns_ext;
mod patterns_distributed;
mod patterns_competition;
mod patterns_fractal;
mod patterns_collab;

// ── Public API re-exports (preserves the original engine:: surface) ──

pub use types::{PhaseType, PhaseDef, WorkflowPlan, YOLO_MODE};
pub use discussion::{run_intake, run_intake_with_team};
pub use workflow::{parse_workflow_plan, check_gate_raw};
pub use mission::run_mission;
pub use patterns_ext::{
    run_sequential_test, run_parallel_test, run_network_test,
    run_solo_test, run_loop_test, run_hierarchical_test,
    run_aggregator_test, run_router_test, run_wave_test,
};
