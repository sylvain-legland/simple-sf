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
    Phases,              \* Set of phase names (e.g. {"design", "dev", "gate", "qa"})
    PhaseOrder,          \* Sequence of phase names (execution order)
    PhaseTypes,          \* Function: phase -> {"once", "sprint", "gate", "feedback_loop"}
    MaxSprintIter,       \* Max sprint iterations per phase
    MaxFeedbackIter,     \* Max feedback loop iterations per phase
    GateTargets,         \* Function: gate_phase -> target_phase name (or "none")
    NumAgents,           \* Number of agents available
    MaxRetries           \* MAX_PHASE_RETRIES (3 in code)

VARIABLES
    missionState,        \* "running" | "completed" | "vetoed"
    phaseIdx,            \* Current index in PhaseOrder (1-based)
    phaseIter,           \* Current iteration within current phase
    phaseResult,         \* Result of last phase execution: "completed" | "vetoed" | "failed"
    guardScore,          \* L0 adversarial guard score (0..10)
    retryCount,          \* Current retry attempt for current phase
    yoloMode,            \* Boolean: auto-approve gates
    agentRound,          \* Current agent tool-call round (0..MAX_ROUNDS)
    loopIter,            \* Current loop pattern iteration (writer↔reviewer)
    history              \* Sequence of [phase, result] pairs for traceability

vars == <<missionState, phaseIdx, phaseIter, phaseResult, guardScore,
          retryCount, yoloMode, agentRound, loopIter, history>>

\* -------------------------------------------------------------------
\* Constants derived from code
\* -------------------------------------------------------------------
GUARD_REJECT == 7           \* Score >= 7 → reject
MAX_ROUNDS == 100           \* Agent execution max tool-call rounds
MAX_LOOP_ITER == 5          \* Writer↔Reviewer loop max
MAX_NETWORK_ROUNDS == 3    \* Network discussion rounds

\* -------------------------------------------------------------------
\* Type invariant — every state must satisfy this
\* -------------------------------------------------------------------
TypeInvariant ==
    /\ missionState \in {"running", "completed", "vetoed"}
    /\ phaseIdx \in 1..(Len(PhaseOrder) + 1)
    /\ phaseIter \in 0..20
    /\ phaseResult \in {"none", "completed", "vetoed", "failed"}
    /\ guardScore \in 0..10
    /\ retryCount \in 0..MaxRetries
    /\ yoloMode \in BOOLEAN
    /\ agentRound \in 0..MAX_ROUNDS
    /\ loopIter \in 0..MAX_LOOP_ITER

\* -------------------------------------------------------------------
\* Safety invariants — properties that must ALWAYS hold
\* -------------------------------------------------------------------

\* A mission can only be vetoed if YOLO mode is off
NoVetoInYolo ==
    yoloMode => missionState /= "vetoed"

\* Phase index never goes backwards unless a gate loops back
\* (we track this via history length — it must never decrease)
HistoryMonotonic ==
    Len(history) >= 0  \* trivially true but placeholder for refinement

\* Mission must eventually terminate (checked as liveness, not invariant)
\* See Liveness property below

\* A completed mission processed at least one phase
CompletedMeansWork ==
    missionState = "completed" => Len(history) > 0

\* A vetoed mission has at least one vetoed phase in history
VetoedMeansVeto ==
    missionState = "vetoed" =>
        \E i \in 1..Len(history) : history[i][2] = "vetoed"

\* Retry count never exceeds max
RetryBounded ==
    retryCount <= MaxRetries

\* Agent rounds never exceed max
AgentRoundBounded ==
    agentRound <= MAX_ROUNDS

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
    /\ guardScore = 0
    /\ retryCount = 0
    /\ yoloMode \in BOOLEAN      \* Model check both modes
    /\ agentRound = 0
    /\ loopIter = 0
    /\ history = <<>>

\* -------------------------------------------------------------------
\* Agent execution: non-deterministic outcome
\* -------------------------------------------------------------------
AgentExecute ==
    \* Agent produces output, guard scores it
    /\ agentRound' \in 1..5     \* Non-det: agent takes 1-5 rounds
    /\ guardScore' \in 0..10    \* Non-det: guard gives any score

\* -------------------------------------------------------------------
\* Execute a "once" phase
\* -------------------------------------------------------------------
ExecuteOnce ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "once"
    /\ retryCount = 0
    /\ AgentExecute
    /\ phaseIter' = 1
    /\ loopIter' = 0
    \* Determine phase result from guard score
    /\ IF guardScore' >= GUARD_REJECT
       THEN /\ phaseResult' = "failed"
            /\ history' = Append(history, <<CurrentPhase, "failed">>)
            /\ phaseIdx' = phaseIdx + 1
            /\ missionState' = "running"
            /\ retryCount' = 0
            /\ yoloMode' = yoloMode
       ELSE /\ phaseResult' = "completed"
            /\ history' = Append(history, <<CurrentPhase, "completed">>)
            /\ phaseIdx' = phaseIdx + 1
            /\ missionState' = "running"
            /\ retryCount' = 0
            /\ yoloMode' = yoloMode

\* -------------------------------------------------------------------
\* Execute a "sprint" phase (iterative with PM checkpoint)
\* -------------------------------------------------------------------
ExecuteSprint ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "sprint"
    /\ phaseIter < MaxSprintIter
    /\ AgentExecute
    /\ loopIter' = 0
    /\ retryCount' = 0
    /\ yoloMode' = yoloMode
    \* PM checkpoint: non-deterministic CONTINUE or DONE
    /\ \/ \* PM says CONTINUE — iterate again
          /\ phaseIter' = phaseIter + 1
          /\ phaseResult' = "none"
          /\ phaseIdx' = phaseIdx
          /\ missionState' = "running"
          /\ history' = history
       \/ \* PM says DONE — move to next phase
          /\ phaseResult' = "completed"
          /\ history' = Append(history, <<CurrentPhase, "completed">>)
          /\ phaseIdx' = phaseIdx + 1
          /\ phaseIter' = 0
          /\ missionState' = "running"

\* Sprint max iterations reached — force advance
SprintMaxReached ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "sprint"
    /\ phaseIter >= MaxSprintIter
    /\ phaseResult' = "completed"
    /\ history' = Append(history, <<CurrentPhase, "completed">>)
    /\ phaseIdx' = phaseIdx + 1
    /\ phaseIter' = 0
    /\ missionState' = "running"
    /\ retryCount' = 0
    /\ yoloMode' = yoloMode
    /\ guardScore' = guardScore
    /\ agentRound' = agentRound
    /\ loopIter' = 0

\* -------------------------------------------------------------------
\* Execute a "gate" phase (go/no-go decision)
\* -------------------------------------------------------------------
ExecuteGate ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "gate"
    /\ AgentExecute
    /\ phaseIter' = 1
    /\ loopIter' = 0
    /\ retryCount' = 0
    \* Non-deterministic gate result
    /\ \/ \* Gate APPROVED — continue
          /\ phaseResult' = "completed"
          /\ history' = Append(history, <<CurrentPhase, "completed">>)
          /\ phaseIdx' = phaseIdx + 1
          /\ missionState' = "running"
          /\ yoloMode' = yoloMode
       \/ \* Gate VETOED
          /\ IF yoloMode
             THEN \* YOLO: override veto, continue
                  /\ phaseResult' = "completed"
                  /\ history' = Append(history, <<CurrentPhase, "yolo_override">>)
                  /\ phaseIdx' = phaseIdx + 1
                  /\ missionState' = "running"
                  /\ yoloMode' = yoloMode
             ELSE IF GateTargets[CurrentPhase] /= "none"
                  THEN \* Loop back to target phase
                       LET targetIdx == CHOOSE i \in 1..Len(PhaseOrder) :
                                            PhaseOrder[i] = GateTargets[CurrentPhase]
                       IN /\ phaseResult' = "vetoed"
                          /\ history' = Append(history, <<CurrentPhase, "vetoed_loopback">>)
                          /\ phaseIdx' = targetIdx
                          /\ missionState' = "running"
                          /\ yoloMode' = yoloMode
                  ELSE \* No target — mission vetoed
                       /\ phaseResult' = "vetoed"
                       /\ history' = Append(history, <<CurrentPhase, "vetoed">>)
                       /\ phaseIdx' = phaseIdx
                       /\ missionState' = "vetoed"
                       /\ yoloMode' = yoloMode

\* -------------------------------------------------------------------
\* Execute a "feedback_loop" phase (QA → tickets → dev → QA)
\* -------------------------------------------------------------------
ExecuteFeedbackLoop ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "feedback_loop"
    /\ phaseIter < MaxFeedbackIter
    /\ AgentExecute
    /\ loopIter' = 0
    /\ retryCount' = 0
    /\ yoloMode' = yoloMode
    \* QA result: non-deterministic
    /\ \/ \* QA APPROVED — exit loop
          /\ phaseResult' = "completed"
          /\ history' = Append(history, <<CurrentPhase, "completed">>)
          /\ phaseIdx' = phaseIdx + 1
          /\ phaseIter' = 0
          /\ missionState' = "running"
       \/ \* QA found tickets — iterate
          /\ phaseIter' = phaseIter + 1
          /\ phaseResult' = "none"
          /\ phaseIdx' = phaseIdx
          /\ missionState' = "running"
          /\ history' = history
       \/ \* QA vetoed but no tickets — exit loop
          /\ phaseResult' = "completed"
          /\ history' = Append(history, <<CurrentPhase, "completed_no_tickets">>)
          /\ phaseIdx' = phaseIdx + 1
          /\ phaseIter' = 0
          /\ missionState' = "running"

\* Feedback loop max iterations reached
FeedbackMaxReached ==
    /\ missionState = "running"
    /\ CurrentPhaseType = "feedback_loop"
    /\ phaseIter >= MaxFeedbackIter
    /\ phaseResult' = "completed"
    /\ history' = Append(history, <<CurrentPhase, "completed_max_iter">>)
    /\ phaseIdx' = phaseIdx + 1
    /\ phaseIter' = 0
    /\ missionState' = "running"
    /\ retryCount' = 0
    /\ yoloMode' = yoloMode
    /\ guardScore' = guardScore
    /\ agentRound' = agentRound
    /\ loopIter' = 0

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
    \* Keep everything else the same — retry same phase
    /\ UNCHANGED <<missionState, phaseIdx, yoloMode, guardScore,
                   agentRound, loopIter, history>>

\* Phase failure after max retries — advance anyway
PhaseFailMaxRetry ==
    /\ missionState = "running"
    /\ phaseResult = "failed"
    /\ retryCount >= MaxRetries
    /\ history' = Append(history, <<CurrentPhase, "failed_max_retry">>)
    /\ phaseIdx' = phaseIdx + 1
    /\ phaseIter' = 0
    /\ phaseResult' = "none"
    /\ retryCount' = 0
    /\ missionState' = "running"
    /\ UNCHANGED <<yoloMode, guardScore, agentRound, loopIter>>

\* -------------------------------------------------------------------
\* Mission completion — all phases done
\* -------------------------------------------------------------------
MissionComplete ==
    /\ missionState = "running"
    /\ phaseIdx > Len(PhaseOrder)
    /\ missionState' = "completed"
    /\ UNCHANGED <<phaseIdx, phaseIter, phaseResult, guardScore,
                   retryCount, yoloMode, agentRound, loopIter, history>>

\* -------------------------------------------------------------------
\* Stutter step (system does nothing — needed for liveness)
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

\* If all gates pass, mission eventually completes
GatesPassImpliesComplete ==
    (\A i \in 1..Len(PhaseOrder) :
        PhaseTypes[PhaseOrder[i]] = "gate" => yoloMode)
    ~> (missionState = "completed")

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
