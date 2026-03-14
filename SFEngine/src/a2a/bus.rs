// Ref: FT-SSF-025 — Agent-to-Agent message bus
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum MessageType {
    Request,
    Response,
    Broadcast,
    Veto,
    Acknowledge,
}

#[derive(Debug, Clone)]
pub struct A2AMessage {
    pub from: String,
    pub to: String,
    pub content: String,
    pub msg_type: MessageType,
    pub timestamp: u64,
}

pub struct A2ABus {
    pub inbox: HashMap<String, Vec<A2AMessage>>,
    pub history: Vec<A2AMessage>,
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl A2ABus {
    pub fn new() -> Self {
        Self { inbox: HashMap::new(), history: Vec::new() }
    }

    pub fn send(&mut self, msg: A2AMessage) {
        self.inbox.entry(msg.to.clone()).or_default().push(msg.clone());
        self.history.push(msg);
    }

    /// Drain and return all pending messages for `agent_id`.
    pub fn receive(&mut self, agent_id: &str) -> Vec<A2AMessage> {
        self.inbox.remove(agent_id).unwrap_or_default()
    }

    /// Broadcast a message to every agent that has an inbox entry.
    pub fn broadcast(&mut self, from: &str, content: &str) {
        let targets: Vec<String> = self.inbox.keys().cloned().collect();
        for to in targets {
            if to != from {
                self.send(A2AMessage {
                    from: from.to_string(),
                    to,
                    content: content.to_string(),
                    msg_type: MessageType::Broadcast,
                    timestamp: now_ts(),
                });
            }
        }
    }

    pub fn veto(&mut self, from: &str, to: &str, reason: &str) {
        self.send(A2AMessage {
            from: from.to_string(),
            to: to.to_string(),
            content: reason.to_string(),
            msg_type: MessageType::Veto,
            timestamp: now_ts(),
        });
    }

    pub fn history_for(&self, agent_id: &str) -> Vec<&A2AMessage> {
        self.history
            .iter()
            .filter(|m| m.from == agent_id || m.to == agent_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_and_receive() {
        let mut bus = A2ABus::new();
        bus.send(A2AMessage {
            from: "a".into(), to: "b".into(), content: "hello".into(),
            msg_type: MessageType::Request, timestamp: 0,
        });
        let msgs = bus.receive("b");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "hello");
        assert!(bus.receive("b").is_empty());
    }

    #[test]
    fn broadcast_sends_to_all() {
        let mut bus = A2ABus::new();
        bus.inbox.insert("agent1".into(), vec![]);
        bus.inbox.insert("agent2".into(), vec![]);
        bus.broadcast("sender", "alert");
        assert_eq!(bus.receive("agent1").len(), 1);
        assert_eq!(bus.receive("agent2").len(), 1);
    }

    #[test]
    fn veto_creates_veto_message() {
        let mut bus = A2ABus::new();
        bus.veto("lead", "dev", "quality too low");
        let msgs = bus.receive("dev");
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0].msg_type, MessageType::Veto));
        assert_eq!(msgs[0].content, "quality too low");
    }
}
