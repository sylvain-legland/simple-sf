// Ref: FT-SSF-025 — Multi-agent negotiation protocol
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq)]
pub enum NegotiationState {
    Proposed,
    Discussing,
    Agreed,
    Rejected,
    Timeout,
}

#[derive(Debug, Clone)]
pub struct Proposal {
    pub id: String,
    pub proposer: String,
    pub content: String,
    pub votes: HashMap<String, bool>,
    pub state: NegotiationState,
}

pub struct NegotiationProtocol {
    pub proposals: Vec<Proposal>,
    pub quorum: f64,
}

fn gen_proposal_id(content: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut h = DefaultHasher::new();
    content.hash(&mut h);
    nanos.hash(&mut h);
    format!("prop-{:012x}", h.finish() & 0xFFFF_FFFF_FFFF)
}

impl NegotiationProtocol {
    pub fn new(quorum: f64) -> Self {
        Self { proposals: Vec::new(), quorum: quorum.clamp(0.0, 1.0) }
    }

    /// Submit a new proposal; returns its id.
    pub fn propose(&mut self, proposer: &str, content: &str) -> String {
        let id = gen_proposal_id(content);
        self.proposals.push(Proposal {
            id: id.clone(),
            proposer: proposer.to_string(),
            content: content.to_string(),
            votes: HashMap::new(),
            state: NegotiationState::Proposed,
        });
        id
    }

    pub fn vote(&mut self, proposal_id: &str, voter: &str, approve: bool) {
        if let Some(p) = self.proposals.iter_mut().find(|p| p.id == proposal_id) {
            p.votes.insert(voter.to_string(), approve);
            if p.state == NegotiationState::Proposed {
                p.state = NegotiationState::Discussing;
            }
        }
    }

    /// Check whether quorum has been reached for the proposal.
    pub fn check_consensus(&mut self, proposal_id: &str) -> NegotiationState {
        if let Some(p) = self.proposals.iter_mut().find(|p| p.id == proposal_id) {
            if p.votes.is_empty() {
                return p.state.clone();
            }
            let total = p.votes.len() as f64;
            let approvals = p.votes.values().filter(|&&v| v).count() as f64;
            let rejections = total - approvals;

            if approvals / total >= self.quorum {
                p.state = NegotiationState::Agreed;
            } else if rejections / total > (1.0 - self.quorum) {
                p.state = NegotiationState::Rejected;
            }
            p.state.clone()
        } else {
            NegotiationState::Timeout
        }
    }

    pub fn resolve(&mut self, proposal_id: &str) -> &Proposal {
        self.check_consensus(proposal_id);
        self.proposals.iter().find(|p| p.id == proposal_id).expect("proposal not found")
    }
}
