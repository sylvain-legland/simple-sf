// Ref: FT-SSF-024
//! CQRS — Command/Query Responsibility Segregation

use std::time::Instant;

pub struct CommandResult {
    pub success: bool,
    pub id: Option<String>,
    pub message: String,
}

pub struct QueryResult {
    pub data: String,
    pub count: usize,
}

pub trait Command {
    fn execute(&self) -> Result<CommandResult, String>;
    fn name(&self) -> &str;
}

pub trait Query {
    fn execute(&self) -> Result<QueryResult, String>;
    fn name(&self) -> &str;
}

pub struct CQRSBus {
    command_log: Vec<(String, Instant)>,
    query_log: Vec<(String, Instant)>,
}

impl CQRSBus {
    pub fn new() -> Self {
        Self {
            command_log: Vec::new(),
            query_log: Vec::new(),
        }
    }

    pub fn dispatch_command(&mut self, cmd: &dyn Command) -> Result<CommandResult, String> {
        self.command_log.push((cmd.name().to_string(), Instant::now()));
        cmd.execute()
    }

    pub fn dispatch_query(&mut self, qry: &dyn Query) -> Result<QueryResult, String> {
        self.query_log.push((qry.name().to_string(), Instant::now()));
        qry.execute()
    }

    pub fn command_count(&self) -> usize {
        self.command_log.len()
    }

    pub fn query_count(&self) -> usize {
        self.query_log.len()
    }
}

impl Default for CQRSBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCmd;
    impl Command for TestCmd {
        fn execute(&self) -> Result<CommandResult, String> {
            Ok(CommandResult { success: true, id: Some("c1".into()), message: "ok".into() })
        }
        fn name(&self) -> &str { "test_cmd" }
    }

    #[test]
    fn dispatch_command_logs_and_returns() {
        let mut bus = CQRSBus::new();
        let result = bus.dispatch_command(&TestCmd).unwrap();
        assert!(result.success);
        assert_eq!(result.message, "ok");
    }

    #[test]
    fn command_count_increments() {
        let mut bus = CQRSBus::new();
        assert_eq!(bus.command_count(), 0);
        bus.dispatch_command(&TestCmd).unwrap();
        bus.dispatch_command(&TestCmd).unwrap();
        assert_eq!(bus.command_count(), 2);
    }
}
