// Ref: FT-SSF-025 — MCP (Model Context Protocol) server

#[derive(Debug, Clone)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    pub input_schema: String,
}

#[derive(Debug, Clone)]
pub struct MCPResource {
    pub uri: String,
    pub name: String,
    pub mime_type: String,
}

pub struct MCPServer {
    pub tools: Vec<MCPTool>,
    pub resources: Vec<MCPResource>,
}

impl MCPServer {
    pub fn new() -> Self {
        Self {
            tools: Self::default_tools(),
            resources: Vec::new(),
        }
    }

    pub fn register_tool(&mut self, tool: MCPTool) {
        self.tools.push(tool);
    }

    pub fn register_resource(&mut self, resource: MCPResource) {
        self.resources.push(resource);
    }

    /// JSON-RPC method dispatch.
    pub fn handle_request(&self, method: &str, _params: &str) -> String {
        match method {
            "tools/list" => {
                let items: Vec<String> = self
                    .tools
                    .iter()
                    .map(|t| {
                        format!(
                            r#"{{"name":"{}","description":"{}","inputSchema":{}}}"#,
                            t.name, t.description, t.input_schema,
                        )
                    })
                    .collect();
                format!(r#"{{"tools":[{}]}}"#, items.join(","))
            }
            "resources/list" => {
                let items: Vec<String> = self
                    .resources
                    .iter()
                    .map(|r| {
                        format!(
                            r#"{{"uri":"{}","name":"{}","mimeType":"{}"}}"#,
                            r.uri, r.name, r.mime_type,
                        )
                    })
                    .collect();
                format!(r#"{{"resources":[{}]}}"#, items.join(","))
            }
            "tools/call" => {
                // Placeholder — real dispatch would parse params and route to tool executor
                r#"{"content":[{"type":"text","text":"tool call not yet wired"}]}"#.to_string()
            }
            _ => super::protocol::format_error(0, -32601, &format!("Method not found: {}", method)),
        }
    }

    /// Register SF Simple's built-in tools as MCP-compatible tool descriptors.
    pub fn default_tools() -> Vec<MCPTool> {
        let defs: Vec<(&str, &str, &str)> = vec![
            ("code_write", "Write content to a file", r#"{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}"#),
            ("code_read", "Read content from a file", r#"{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}"#),
            ("code_edit", "Edit a file with search/replace", r#"{"type":"object","properties":{"path":{"type":"string"},"old":{"type":"string"},"new":{"type":"string"}},"required":["path","old","new"]}"#),
            ("code_search", "Search code in workspace", r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}"#),
            ("list_files", "List files in directory", r#"{"type":"object","properties":{"path":{"type":"string"}}}"#),
            ("deep_search", "Deep semantic search across files", r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}"#),
            ("build", "Build the project", r#"{"type":"object","properties":{"args":{"type":"string"}}}"#),
            ("test", "Run project tests", r#"{"type":"object","properties":{"args":{"type":"string"}}}"#),
            ("lint", "Lint the project", r#"{"type":"object","properties":{"args":{"type":"string"}}}"#),
            ("git_init", "Initialize a git repository", r#"{"type":"object","properties":{}}"#),
            ("git_commit", "Commit staged changes", r#"{"type":"object","properties":{"message":{"type":"string"}},"required":["message"]}"#),
            ("git_status", "Show git status", r#"{"type":"object","properties":{}}"#),
            ("git_log", "Show git log", r#"{"type":"object","properties":{"n":{"type":"integer"}}}"#),
            ("git_diff", "Show git diff", r#"{"type":"object","properties":{"args":{"type":"string"}}}"#),
            ("git_push", "Push to remote", r#"{"type":"object","properties":{"remote":{"type":"string"},"branch":{"type":"string"}}}"#),
            ("git_create_branch", "Create and checkout a branch", r#"{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}"#),
            ("memory_search", "Search project memory", r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}"#),
            ("memory_store", "Store to project memory", r#"{"type":"object","properties":{"key":{"type":"string"},"value":{"type":"string"}},"required":["key","value"]}"#),
        ];
        defs.into_iter()
            .map(|(name, desc, schema)| MCPTool {
                name: name.to_string(),
                description: desc.to_string(),
                input_schema: schema.to_string(),
            })
            .collect()
    }
}
