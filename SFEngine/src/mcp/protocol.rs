// Ref: FT-SSF-025 — JSON-RPC 2.0 message types for MCP

#[derive(Debug, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<String>,
    pub id: u64,
}

#[derive(Debug, Clone)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<String>,
    pub error: Option<JsonRpcError>,
    pub id: u64,
}

#[derive(Debug, Clone)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// Standard JSON-RPC error codes.
pub const PARSE_ERROR: i32 = -32700;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;

/// Parse a raw JSON string into a `JsonRpcRequest`.
pub fn parse_request(json: &str) -> Result<JsonRpcRequest, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("parse error: {}", e))?;

    let jsonrpc = v.get("jsonrpc")
        .and_then(|v| v.as_str())
        .unwrap_or("2.0")
        .to_string();

    let method = v.get("method")
        .and_then(|v| v.as_str())
        .ok_or("missing 'method'")?
        .to_string();

    let params = v.get("params").map(|p| p.to_string());

    let id = v.get("id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    Ok(JsonRpcRequest { jsonrpc, method, params, id })
}

pub fn format_response(id: u64, result: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","result":{},"id":{}}}"#, result, id)
}

pub fn format_error(id: u64, code: i32, msg: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","error":{{"code":{},"message":"{}"}},"id":{}}}"#,
        code, msg, id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_request() {
        let json = r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id, 1);
        assert_eq!(req.jsonrpc, "2.0");
    }

    #[test]
    fn parse_missing_method_fails() {
        let json = r#"{"jsonrpc":"2.0","id":1}"#;
        assert!(parse_request(json).is_err());
    }

    #[test]
    fn format_response_valid_json() {
        let resp = format_response(42, r#""ok""#);
        assert!(resp.contains("\"result\":\"ok\""));
        assert!(resp.contains("\"id\":42"));
    }

    #[test]
    fn format_error_contains_code() {
        let err = format_error(1, PARSE_ERROR, "bad json");
        assert!(err.contains("-32700"));
        assert!(err.contains("bad json"));
    }
}
