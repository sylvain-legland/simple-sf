// Ref: FT-SSF-024
//! Event Sourcing — append-only domain event store

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub enum DomainEvent {
    MissionCreated { id: String, project: String, task: String },
    PhaseStarted { mission_id: String, phase: String, pattern: String },
    PhaseCompleted { mission_id: String, phase: String, result: String },
    PhaseVetoed { mission_id: String, phase: String, reason: String },
    AgentInvoked { mission_id: String, agent_id: String, phase: String },
    GuardRejected { mission_id: String, agent_id: String, score: i32 },
    MissionCompleted { id: String, outcome: String },
    MissionFailed { id: String, reason: String },
}

impl DomainEvent {
    fn mission_id(&self) -> &str {
        match self {
            Self::MissionCreated { id, .. } | Self::MissionCompleted { id, .. } | Self::MissionFailed { id, .. } => id,
            Self::PhaseStarted { mission_id, .. } | Self::PhaseCompleted { mission_id, .. }
            | Self::PhaseVetoed { mission_id, .. } | Self::AgentInvoked { mission_id, .. }
            | Self::GuardRejected { mission_id, .. } => mission_id,
        }
    }
}

impl fmt::Display for DomainEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissionCreated { id, project, task } => write!(f, "MissionCreated({id}, {project}: {task})"),
            Self::PhaseStarted { mission_id, phase, pattern } => write!(f, "PhaseStarted({mission_id}, {phase}[{pattern}])"),
            Self::PhaseCompleted { mission_id, phase, result } => write!(f, "PhaseCompleted({mission_id}, {phase}: {result})"),
            Self::PhaseVetoed { mission_id, phase, reason } => write!(f, "PhaseVetoed({mission_id}, {phase}: {reason})"),
            Self::AgentInvoked { mission_id, agent_id, phase } => write!(f, "AgentInvoked({mission_id}, {agent_id}@{phase})"),
            Self::GuardRejected { mission_id, agent_id, score } => write!(f, "GuardRejected({mission_id}, {agent_id} score={score})"),
            Self::MissionCompleted { id, outcome } => write!(f, "MissionCompleted({id}: {outcome})"),
            Self::MissionFailed { id, reason } => write!(f, "MissionFailed({id}: {reason})"),
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

pub struct EventStore {
    events: Vec<(u64, DomainEvent, u64)>,
}

impl EventStore {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn append(&mut self, event: DomainEvent) {
        let seq = self.events.len() as u64 + 1;
        self.events.push((seq, event, now_ms()));
    }

    pub fn replay(&self) -> &[(u64, DomainEvent, u64)] {
        &self.events
    }

    pub fn replay_for_mission(&self, mission_id: &str) -> Vec<&DomainEvent> {
        self.events.iter().filter(|(_, e, _)| e.mission_id() == mission_id).map(|(_, e, _)| e).collect()
    }

    pub fn since(&self, seq: u64) -> &[(u64, DomainEvent, u64)] {
        let idx = self.events.partition_point(|(s, _, _)| *s <= seq);
        &self.events[idx..]
    }
}

impl Default for EventStore {
    fn default() -> Self { Self::new() }
}
