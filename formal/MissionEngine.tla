--------------------------- MODULE MissionEngine ---------------------------
(*
 * TLA+ formal specification of the Simple SF mission orchestration engine.
 * Models the full lifecycle: mission → phases → patterns → guard → retry.
 *
 * Ref: FT-SSF-004 (Mission Execution)
 *
 * To run:  tlc MissionEngine.tla -config MissionEngine.cfg
 * Install: brew install tlaplus  (or https://github.com/tlaplus/tlaplus/releases)
 *)
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    NumAgents,           \* Number of agents available
    MaxRetries           \* MAX_PHASE_RETRIES (3 in code)

\* -------------------------------------------------------------------
\* Model constants — 4-phase workflow: design → dev → gate → qa
\* -------------------------------------------------------------------
Phases == {"design", "dev", "gate", "qa"}
PhaseOrder == <<"design", "dev", "gate", "qa">>
PhaseTypes == [design |-> "once", dev |-> "sprint", gate |-> "gate", qa |-> "feedback_loop"]
MaxSprintIter == 3
MaxFeedbackIter == 3
GateTargets == [design |-> "none", dev |-> "none", gate |-> "dev", qa |-> "none"]

VARIABLES
    missionState,        \* "running" | "completed" | "vetoed"
    phaseIdx,            \* Current index in PhaseOrder (1-based)
    phaseIter,           \* Current iteration within current phase
    phaseResult,         \* Result of last phase execution: "completed" | "vetoed" | "failed"
    guardPassed,         \* Boolean: did L0 guard pass? (abstracts score < 7)
    retryCount,          \* Current retry attempt for current phase
    yoloMode,            \* Boolean: auto-approve gates
    vetoCount,           \* How many phases were vetoed (replaces unbounded history)
    completedCount       \* How many phases completed (replaces unbounded history)

vars == <<missionState, phaseIdx, phaseIter, phaseResult, guardPassed,
          retryCount, yoloMode, vetoCount, completedCount>>

\* -------------------------------------------------------------------
\* Constants derived from code
\* -------------------------------------------------------------------
GUARD_REJECT == 7           \* Score >= 7 → reject (modeled as boolean guardPassed)
MAX_GATE_LOOPBACKS == 3    \* Max times a gate can loop back before forcing veto

\* -------------------------------------------------------------------
\* Type invariant — every state must satisfy this
\* -------------------------------------------------------------------
TypeInvariant ==
    /\ missionState \in {"running", "completed", "vetoed"}
    /\ phaseIdx \in 1..(Len(PhaseOrder) + 1)
    /\ phaseIter \in 0..20
    /\ phaseResult \in {"none", "completed", "vetoed", "failed"}
    /\ guardPassed \in BOOLEAN
    /\ retryCount \in 0..MaxRetries
    /\ yoloMode \in BOOLEAN
    /\ vetoCount \in 0..10
    /\ completedCount \in 0..50

\* -------------------------------------------------------------------
\* Safety invariants — properties that must ALWAYS hold
\* -------------------------------------------------------------------

\* A mission can only be vetoed if YOLO mode is off
NoVetoInYolo ==
    yoloMode => missionState /= "vetoed"

\* A completed mission processed at least one phase
CompletedMeansWork ==
    missionState = "completed" => completedCount > 0

\* A vetoed mission has at least one vetoed phase
VetoedMeansVeto ==
    missionState = "vetoed" => vetoCount > 0

\* Retry count never exceeds max
RetryBounded ==
    retryCount <= MaxRetries

\* Phase index is always valid
PhaseIdxValid ==
    phaseIdx >= 1

\* -------------------------------------------------------------------
\* Helper: current phase name and type
\* -------------------------------------------------------------------
CurrentPhase == IF phaseIdx <= Len(PhaseOrder)
                THEN PhaseOrder[phaseIdx]
                ELSE "none"

CurrentPhaseType == IF CurrentPhase /= "none"
                    THEN PhaseTypes[CurrentPhase]
                    ELSE "none"

\* -------------------------------------------------------------------
\* Initial state
\* -------------------------------------------------------------------
Init ==
    /\ missionState = "running"
    /\ phaseIdx = 1
    /\ phaseIter = 0
    /\ phaseResult = "none"
    /\ guardPassed \in BOOLEAN       \* Model check both outcomes
    /\ retryCount = 0
    /\ yoloMode \in BOOLEAN          \* Model check both modes
    /\ vetoCount = 0
    /\ completedCount = 0

\* -------------------------------------------------------------------
\* Execute a "once" phase
\* -------------------------------------------------------------------
ExecuteOnce ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "once"
    /\ retryCount = 0
    /\ guardPassed' \in BOOLEAN
    /\ phaseIter' = 1
    /\ IF guardPassed'
       THEN /\ phaseResult' = "completed"
            /\ completedCount' = completedCount + 1
            /\ phaseIdx' = phaseIdx + 1
            /\ missionState' = "running"
            /\ retryCount' = 0
            /\ vetoCount' = vetoCount
       ELSE /\ phaseResult' = "failed"
            /\ phaseIdx' = phaseIdx + 1
            /\ missionState' = "running"
            /\ retryCount' = 0
            /\ vetoCount' = vetoCount
            /\ completedCount' = completedCount
    /\ yoloMode' = yoloMode

\* -------------------------------------------------------------------
\* Execute a "sprint" phase (iterative with PM checkpoint)
\* -------------------------------------------------------------------
ExecuteSprint ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "sprint"
    /\ phaseIter < MaxSprintIter
    /\ guardPassed' \in BOOLEAN
    /\ retryCount' = 0
    /\ yoloMode' = yoloMode
    /\ vetoCount' = vetoCount
    \* PM checkpoint: non-deterministic CONTINUE or DONE
    /\ \/ \* PM says CONTINUE
          /\ phaseIter' = phaseIter + 1
          /\ phaseResult' = "none"
          /\ phaseIdx' = phaseIdx
          /\ missionState' = "running"
          /\ completedCount' = completedCount
       \/ \* PM says DONE
          /\ phaseResult' = "completed"
          /\ phaseIdx' = phaseIdx + 1
          /\ phaseIter' = 0
          /\ missionState' = "running"
          /\ completedCount' = completedCount + 1

\* Sprint max iterations reached
SprintMaxReached ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "sprint"
    /\ phaseIter >= MaxSprintIter
    /\ phaseResult' = "completed"
    /\ phaseIdx' = phaseIdx + 1
    /\ phaseIter' = 0
    /\ missionState' = "running"
    /\ retryCount' = 0
    /\ completedCount' = completedCount + 1
    /\ UNCHANGED <<yoloMode, guardPassed, vetoCount>>

\* -------------------------------------------------------------------
\* Execute a "gate" phase (go/no-go decision)
\* -------------------------------------------------------------------
ExecuteGate ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "gate"
    /\ phaseIter' = 1
    /\ retryCount' = 0
    /\ guardPassed' \in BOOLEAN
    \* Non-deterministic gate result
    /\ \/ \* Gate APPROVED
          /\ phaseResult' = "completed"
          /\ phaseIdx' = phaseIdx + 1
          /\ missionState' = "running"
          /\ yoloMode' = yoloMode
          /\ completedCount' = completedCount + 1
          /\ vetoCount' = vetoCount
       \/ \* Gate VETOED
          /\ IF yoloMode
             THEN \* YOLO: override veto, continue
                  /\ phaseResult' = "completed"
                  /\ phaseIdx' = phaseIdx + 1
                  /\ missionState' = "running"
                  /\ yoloMode' = yoloMode
                  /\ completedCount' = completedCount + 1
                  /\ vetoCount' = vetoCount
             ELSE IF GateTargets[CurrentPhase] /= "none" /\ vetoCount < MAX_GATE_LOOPBACKS
                  THEN \* Loop back to target phase (if under limit)
                       LET targetIdx == CHOOSE i \in 1..Len(PhaseOrder) :
                                            PhaseOrder[i] = GateTargets[CurrentPhase]
                       IN /\ phaseResult' = "vetoed"
                          /\ phaseIdx' = targetIdx
                          /\ missionState' = "running"
                          /\ yoloMode' = yoloMode
                          /\ vetoCount' = vetoCount + 1
                          /\ completedCount' = completedCount
                  ELSE \* No target OR loopback limit exceeded — mission vetoed
                       /\ phaseResult' = "vetoed"
                       /\ phaseIdx' = phaseIdx
                       /\ missionState' = "vetoed"
                       /\ yoloMode' = yoloMode
                       /\ vetoCount' = vetoCount + 1
                       /\ completedCount' = completedCount

\* -------------------------------------------------------------------
\* Execute a "feedback_loop" phase (QA → tickets → dev → QA)
\* -------------------------------------------------------------------
ExecuteFeedbackLoop ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "feedback_loop"
    /\ phaseIter < MaxFeedbackIter
    /\ guardPassed' \in BOOLEAN
    /\ retryCount' = 0
    /\ yoloMode' = yoloMode
    /\ vetoCount' = vetoCount
    \* QA result: non-deterministic
    /\ \/ \* QA APPROVED
          /\ phaseResult' = "completed"
          /\ phaseIdx' = phaseIdx + 1
          /\ phaseIter' = 0
          /\ missionState' = "running"
          /\ completedCount' = completedCount + 1
       \/ \* QA found tickets — iterate
          /\ phaseIter' = phaseIter + 1
          /\ phaseResult' = "none"
          /\ phaseIdx' = phaseIdx
          /\ missionState' = "running"
          /\ completedCount' = completedCount
       \/ \* QA vetoed no tickets — exit
          /\ phaseResult' = "completed"
          /\ phaseIdx' = phaseIdx + 1
          /\ phaseIter' = 0
          /\ missionState' = "running"
          /\ completedCount' = completedCount + 1

\* Feedback max reached
FeedbackMaxReached ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "feedback_loop"
    /\ phaseIter >= MaxFeedbackIter
    /\ phaseResult' = "completed"
    /\ phaseIdx' = phaseIdx + 1
    /\ phaseIter' = 0
    /\ missionState' = "running"
    /\ retryCount' = 0
    /\ completedCount' = completedCount + 1
    /\ UNCHANGED <<yoloMode, guardPassed, vetoCount>>

\* -------------------------------------------------------------------
\* Phase retry on failure (resilience.rs)
\* -------------------------------------------------------------------
PhaseRetry ==
    /\ missionState = "running"
    /\ phaseResult = "failed"
    /\ retryCount < MaxRetries
    /\ retryCount' = retryCount + 1
    /\ phaseResult' = "none"
    /\ phaseIter' = 0
    /\ UNCHANGED <<missionState, phaseIdx, yoloMode, guardPassed,
                   vetoCount, completedCount>>

\* Phase failure after max retries — advance
PhaseFailMaxRetry ==
    /\ missionState = "running"
    /\ phaseResult = "failed"
    /\ retryCount >= MaxRetries
    /\ phaseIdx' = phaseIdx + 1
    /\ phaseIter' = 0
    /\ phaseResult' = "none"
    /\ retryCount' = 0
    /\ missionState' = "running"
    /\ UNCHANGED <<yoloMode, guardPassed, vetoCount, completedCount>>

\* -------------------------------------------------------------------
\* Mission completion — all phases done
\* -------------------------------------------------------------------
MissionComplete ==
    /\ missionState = "running"
    /\ phaseIdx > Len(PhaseOrder)
    /\ missionState' = "completed"
    /\ UNCHANGED <<phaseIdx, phaseIter, phaseResult, guardPassed,
                   retryCount, yoloMode, vetoCount, completedCount>>

\* -------------------------------------------------------------------
\* Stutter step (terminal states)
\* -------------------------------------------------------------------
Stutter ==
    /\ missionState \in {"completed", "vetoed"}
    /\ UNCHANGED vars

\* -------------------------------------------------------------------
\* Next-state relation
\* -------------------------------------------------------------------
Next ==
    \/ ExecuteOnce
    \/ ExecuteSprint
    \/ SprintMaxReached
    \/ ExecuteGate
    \/ ExecuteFeedbackLoop
    \/ FeedbackMaxReached
    \/ PhaseRetry
    \/ PhaseFailMaxRetry
    \/ MissionComplete
    \/ Stutter

\* -------------------------------------------------------------------
\* Fairness — every enabled action eventually executes
\* -------------------------------------------------------------------
Fairness ==
    /\ WF_vars(ExecuteOnce)
    /\ WF_vars(ExecuteSprint)
    /\ WF_vars(SprintMaxReached)
    /\ WF_vars(ExecuteGate)
    /\ WF_vars(ExecuteFeedbackLoop)
    /\ WF_vars(FeedbackMaxReached)
    /\ WF_vars(PhaseRetry)
    /\ WF_vars(PhaseFailMaxRetry)
    /\ WF_vars(MissionComplete)

\* -------------------------------------------------------------------
\* Specification
\* -------------------------------------------------------------------
Spec == Init /\ [][Next]_vars /\ Fairness

\* -------------------------------------------------------------------
\* Liveness properties
\* -------------------------------------------------------------------

\* Every mission eventually terminates
MissionTerminates ==
    <>(missionState \in {"completed", "vetoed"})

\* No phase runs forever (bounded iterations)
NoPhaseLivelock ==
    [](missionState = "running" =>
        <>(phaseIdx > Len(PhaseOrder) \/ missionState /= "running"))

\* -------------------------------------------------------------------
\* Deadlock freedom — there's always a next step
\* -------------------------------------------------------------------
DeadlockFree ==
    [](ENABLED(Next))

============================================================================
