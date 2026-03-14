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
