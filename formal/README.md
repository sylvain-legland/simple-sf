# TLA+ Formal Verification — Mission Engine

## What This Proves

This TLA+ spec models the **complete mission orchestration state machine** and
mathematically verifies that:

### Safety (invariants — must ALWAYS hold)
| Property | What it checks |
|----------|---------------|
| `TypeInvariant` | All variables stay in valid ranges (no corrupt state) |
| `NoVetoInYolo` | YOLO mode can never produce a vetoed mission |
| `CompletedMeansWork` | A completed mission processed at least one phase |
| `RetryBounded` | Retries never exceed MAX_PHASE_RETRIES (3) |
| `AgentRoundBounded` | Agent tool-call rounds never exceed 100 |

### Liveness (temporal — must EVENTUALLY hold)
| Property | What it checks |
|----------|---------------|
| `MissionTerminates` | Every mission eventually reaches completed or vetoed |
| `NoPhaseLivelock` | No phase runs forever (bounded iterations) |

## What's Modeled

```
Mission (running → completed/vetoed)
  └── Phases (ordered sequence)
       ├── Once: execute once, advance
       ├── Sprint: iterate up to N times, PM checkpoint each
       ├── Gate: go/no-go, can loop back to target phase on veto
       └── FeedbackLoop: QA→tickets→dev→QA, up to N cycles
  └── Adversarial Guard (score 0-10, reject ≥ 7)
  └── Retry (3 attempts with exponential backoff)
  └── YOLO mode (auto-approve gates)
```

## Running

```sh
# Install TLA+ toolbox
brew install tlaplus

# Run model checker (exhaustive state exploration)
cd formal/
tlc MissionEngine.tla -config MissionEngine.cfg

# Or use the TLA+ Toolbox GUI
# Download: https://github.com/tlaplus/tlaplus/releases
```

## Expected Output

```
Model checking completed. No error has been found.
  Diameter of the complete state graph: XX
  XXXX distinct states found.
```

If TLC finds a bug, it prints a **counterexample trace** — the exact sequence
of states that leads to the violation. This is the "instant feedback" for agents.

## Extending the Spec

To model a real workflow (e.g., product-lifecycle with 15 phases):

1. Edit `MissionEngine.cfg`
2. Add phases to `Phases`, `PhaseOrder`, `PhaseTypes`
3. Set gate targets in `GateTargets`
4. Run `tlc` — it explores all possible interleavings

## Integration with Agentic Loop

The counterexample trace from TLC can be fed back to an LLM agent:

```
"TLA+ found a deadlock: phase 'gate' vetoes to 'dev', which sprints back
to 'gate', creating an infinite loop when MaxSprintIter=1 and the gate
always vetoes. Fix: increase MaxSprintIter or add a retry limit on gate
loop-backs."
```

The agent then modifies the workflow definition and re-runs TLC until
all properties pass — **formal verification in the agentic loop**.
