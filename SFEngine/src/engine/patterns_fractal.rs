// Ref: FT-SSF-020
//! Fractal family patterns (qa, stories, tests, worktree) + backprop merge.

use crate::agents;
use crate::executor::{self, AgentEvent, EventCallback};
use crate::guard;
use crate::protocols;
// LLMMessage available if direct chat_completion needed in future extensions
#[allow(unused_imports)]
use crate::llm::LLMMessage;

// ── Helpers ────────────────────────────────────────────────────────────

fn get(id: &str) -> Result<agents::Agent, String> {
    agents::get_agent(id).ok_or_else(|| format!("Agent not found: {id}"))
}

/// Parse bracketed items: "[PREFIX N] ..." lines from LLM output.
fn parse_items(text: &str, prefix: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| l.starts_with(&format!("[{prefix}")))
        .map(String::from)
        .collect()
}

/// Run an agent with the protocol matching its role + phase.
async fn run(
    agent: &agents::Agent, task: &str, phase: &str,
    workspace: &str, mission_id: &str, phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    let proto = protocols::protocol_for_role(&agent.role, phase);
    on_event(&agent.id, AgentEvent::Thinking);
    let result = executor::run_agent(
        &agent.id, &agent.name, &agent.persona, &agent.role,
        task, workspace, mission_id, phase_id, Some(proto), on_event,
    ).await?;
    on_event(&agent.id, AgentEvent::Response { content: result.clone() });
    Ok(result)
}

// ── 1. Fractal QA ─────────────────────────────────────────────────────

/// Recursive quality decomposition: QA lead splits checks, agents run them,
/// lead synthesizes a verdict.
pub(crate) async fn run_fractal_qa(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    eprintln!("[FRACTAL_QA] {} agents, phase={phase}", agent_ids.len());
    if agent_ids.len() < 2 {
        return Err("fractal_qa requires at least 2 agents".into());
    }

    let lead = get(agent_ids[0])?;
    let qa_agents: Vec<_> = agent_ids[1..].iter().map(|id| get(id)).collect::<Result<_, _>>()?;

    // Lead decomposes into quality checks
    let checks_raw = run(
        &lead,
        &format!("Decompose into quality checks. Output each as [CHECK N] description.\n\n{task}"),
        phase, workspace, mission_id, phase_id, on_event,
    ).await?;
    let checks = parse_items(&checks_raw, "CHECK");
    if checks.is_empty() {
        return Err("QA lead produced no checks".into());
    }

    // Distribute checks round-robin
    let mut results = Vec::new();
    for (i, check) in checks.iter().enumerate() {
        let agent = &qa_agents[i % qa_agents.len()];
        let r = run(
            agent,
            &format!("Run this quality check. Reply [PASS] or [FAIL: reason].\n\n{check}"),
            phase, workspace, mission_id, phase_id, on_event,
        ).await?;
        results.push(format!("{check}\n→ {r}"));
    }

    // Lead synthesizes verdict
    let verdict = run(
        &lead,
        &format!("Synthesize QA results into an overall verdict:\n\n{}", results.join("\n\n")),
        phase, workspace, mission_id, phase_id, on_event,
    ).await?;

    Ok(format!("{verdict}\n\n---\nDetails:\n{}", results.join("\n")))
}

// ── 2. Fractal Stories ────────────────────────────────────────────────

/// Recursive story breakdown: epic → user stories → sub-tasks → validation.
pub(crate) async fn run_fractal_stories(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    eprintln!("[FRACTAL_STORIES] {} agents, phase={phase}", agent_ids.len());
    if agent_ids.is_empty() {
        return Err("fractal_stories requires at least 1 agent".into());
    }

    // First agent breaks epic into stories
    let storyteller = get(agent_ids[0])?;
    let stories_raw = run(
        &storyteller,
        &format!(
            "Break this epic/feature into user stories.\n\
             Format each as: [STORY N] As a <role> I want <goal> So that <benefit>\n\n{task}"
        ),
        phase, workspace, mission_id, phase_id, on_event,
    ).await?;
    let stories = parse_items(&stories_raw, "STORY");
    if stories.is_empty() {
        return Ok(stories_raw);
    }

    // Middle agents break stories into tasks (round-robin)
    let workers: Vec<_> = if agent_ids.len() > 2 {
        agent_ids[1..agent_ids.len() - 1].iter().map(|id| get(id)).collect::<Result<_, _>>()?
    } else {
        vec![get(agent_ids[agent_ids.len().min(1).max(1) - 1])?]
    };

    let mut hierarchy = Vec::new();
    for (i, story) in stories.iter().enumerate() {
        let worker = &workers[i % workers.len()];
        let tasks_raw = run(
            worker,
            &format!("Break this user story into atomic sub-tasks.\nFormat: [TASK N] description\n\n{story}"),
            phase, workspace, mission_id, phase_id, on_event,
        ).await?;
        let tasks = parse_items(&tasks_raw, "TASK");
        hierarchy.push(format!("{story}\n{}", tasks.join("\n")));
    }

    // Last agent validates completeness
    let validator = get(agent_ids[agent_ids.len() - 1])?;
    let validation = run(
        &validator,
        &format!(
            "Validate this story→task hierarchy. Check: all stories have tasks, all tasks are atomic.\n\n{}",
            hierarchy.join("\n\n")
        ),
        phase, workspace, mission_id, phase_id, on_event,
    ).await?;

    Ok(format!("{validation}\n\n---\nHierarchy:\n{}", hierarchy.join("\n\n")))
}

// ── 3. Fractal Tests ──────────────────────────────────────────────────

/// Recursive test generation: categorize → generate per category → review gaps.
pub(crate) async fn run_fractal_tests(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    eprintln!("[FRACTAL_TESTS] {} agents, phase={phase}", agent_ids.len());
    if agent_ids.is_empty() {
        return Err("fractal_tests requires at least 1 agent".into());
    }

    // First agent identifies test categories
    let analyst = get(agent_ids[0])?;
    let cats_raw = run(
        &analyst,
        &format!(
            "Analyze this and identify test categories.\n\
             Format: [CATEGORY N] <type> tests for <scope>\n\n{task}"
        ),
        phase, workspace, mission_id, phase_id, on_event,
    ).await?;
    let categories = parse_items(&cats_raw, "CATEGORY");
    if categories.is_empty() {
        return Ok(cats_raw);
    }

    // Each agent generates tests for one category (round-robin)
    let generators: Vec<_> = if agent_ids.len() > 2 {
        agent_ids[1..agent_ids.len() - 1].iter().map(|id| get(id)).collect::<Result<_, _>>()?
    } else {
        vec![get(agent_ids[0])?]
    };

    let mut test_suite = Vec::new();
    for (i, cat) in categories.iter().enumerate() {
        let generator = &generators[i % generators.len()];
        let tests = run(
            generator,
            &format!("Generate tests for this category. Write concrete test code.\n\n{cat}\n\nContext: {task}"),
            phase, workspace, mission_id, phase_id, on_event,
        ).await?;
        test_suite.push(format!("## {cat}\n{tests}"));
    }

    // Last agent reviews for coverage gaps
    let reviewer = get(agent_ids[agent_ids.len() - 1])?;
    let review = run(
        &reviewer,
        &format!(
            "Review this test suite for coverage gaps. List missing tests.\n\n{}",
            test_suite.join("\n\n")
        ),
        phase, workspace, mission_id, phase_id, on_event,
    ).await?;

    Ok(format!("{review}\n\n---\nTest Suite:\n{}", test_suite.join("\n\n")))
}

// ── 4. Fractal Worktree ───────────────────────────────────────────────

/// Recursive workspace split: analyze → branch work → merge.
pub(crate) async fn run_fractal_worktree(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    eprintln!("[FRACTAL_WORKTREE] {} agents, phase={phase}", agent_ids.len());
    if agent_ids.len() < 2 {
        return Err("fractal_worktree requires at least 2 agents".into());
    }

    // First agent splits work into branches
    let planner = get(agent_ids[0])?;
    let split_raw = run(
        &planner,
        &format!(
            "Analyze the workspace and split work into branches.\n\
             Format: [BRANCH N] files: <list> — task: <description>\n\n{task}"
        ),
        phase, workspace, mission_id, phase_id, on_event,
    ).await?;
    let branches = parse_items(&split_raw, "BRANCH");
    if branches.is_empty() {
        return Ok(split_raw);
    }

    // Middle agents each work on a branch (round-robin)
    let workers: Vec<_> = if agent_ids.len() > 2 {
        agent_ids[1..agent_ids.len() - 1].iter().map(|id| get(id)).collect::<Result<_, _>>()?
    } else {
        vec![get(agent_ids[0])?]
    };

    let mut branch_outputs = Vec::new();
    for (i, branch) in branches.iter().enumerate() {
        let worker = &workers[i % workers.len()];
        let output = run(
            worker,
            &format!(
                "Work on this branch. Only modify the listed files.\n\n{branch}\n\nFull context: {task}"
            ),
            phase, workspace, mission_id, phase_id, on_event,
        ).await?;
        branch_outputs.push(format!("{branch}\n---\n{output}"));
    }

    // Last agent merges branch outputs
    let merger = get(agent_ids[agent_ids.len() - 1])?;
    let merged = run(
        &merger,
        &format!(
            "Merge these branch outputs. Resolve any conflicts between branches.\n\n{}",
            branch_outputs.join("\n\n===\n\n")
        ),
        phase, workspace, mission_id, phase_id, on_event,
    ).await?;

    Ok(merged)
}

// ── 5. Backprop Merge ─────────────────────────────────────────────────

/// Error signals propagate backward: sequential run, guard check,
/// re-run the culprit agent (+ downstream) with error feedback. Max 2 iterations.
pub(crate) async fn run_backprop(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    eprintln!("[BACKPROP] {} agents, phase={phase}", agent_ids.len());
    if agent_ids.is_empty() {
        return Err("backprop requires at least 1 agent".into());
    }

    let agents: Vec<_> = agent_ids.iter().map(|id| get(id)).collect::<Result<_, _>>()?;
    let mut outputs: Vec<String> = Vec::with_capacity(agents.len());

    // Initial sequential pass
    let mut context = task.to_string();
    for agent in &agents {
        let result = run(
            agent, &context, phase, workspace, mission_id, phase_id, on_event,
        ).await?;
        context = format!("{context}\n\nPrevious agent ({}) output:\n{result}", agent.name);
        outputs.push(result);
    }

    // Backprop loop: guard → identify culprit → re-run from culprit onward
    const MAX_ITERATIONS: usize = 2;
    for iteration in 0..MAX_ITERATIONS {
        let final_output = outputs.last().cloned().unwrap_or_default();
        let guard_result = guard::check_l0(&final_output, &agents.last().unwrap().role, &[]);

        if guard_result.passed {
            eprintln!("[BACKPROP] passed guard on iteration {iteration}");
            break;
        }

        eprintln!("[BACKPROP] guard failed (score={}), backprop iteration {iteration}", guard_result.score);
        let issues = guard_result.issues.join(", ");

        // Find the culprit: last agent whose output first triggered issues
        let mut culprit_idx = outputs.len() - 1;
        for (i, output) in outputs.iter().enumerate() {
            let check = guard::check_l0(output, &agents[i].role, &[]);
            if !check.passed {
                culprit_idx = i;
                break;
            }
        }

        // Re-run from culprit onward with error feedback
        let mut ctx = if culprit_idx > 0 {
            format!("{task}\n\nPrevious output:\n{}", outputs[culprit_idx - 1])
        } else {
            task.to_string()
        };

        for idx in culprit_idx..agents.len() {
            let agent = &agents[idx];
            let feedback_task = format!(
                "{ctx}\n\n⚠️ Your previous output caused quality issues: {issues}. Fix them."
            );
            let result = run(
                agent, &feedback_task, phase, workspace, mission_id, phase_id, on_event,
            ).await?;
            ctx = format!("{ctx}\n\nAgent {} output:\n{result}", agent.name);
            outputs[idx] = result;
        }
    }

    Ok(outputs.join("\n\n---\n\n"))
}
