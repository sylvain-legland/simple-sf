// Ref: FT-SSF-025 — OTEL-compatible trace spans (no external deps)
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub enum SpanStatus {
    Ok,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_id: Option<String>,
    pub operation: String,
    pub start_ns: u64,
    pub end_ns: Option<u64>,
    pub attributes: Vec<(String, String)>,
    pub status: SpanStatus,
}

pub struct TraceCollector {
    pub spans: Vec<Span>,
}

fn generate_id(seed: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    nanos.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

impl TraceCollector {
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Start a new span; returns its span_id.
    pub fn start_span(&mut self, operation: &str, parent: Option<&str>) -> String {
        let span_id = generate_id(operation);
        let trace_id = match parent {
            Some(pid) => self
                .spans
                .iter()
                .find(|s| s.span_id == pid)
                .map(|s| s.trace_id.clone())
                .unwrap_or_else(|| generate_id("trace")),
            None => generate_id("trace"),
        };
        self.spans.push(Span {
            trace_id,
            span_id: span_id.clone(),
            parent_id: parent.map(String::from),
            operation: operation.to_string(),
            start_ns: now_ns(),
            end_ns: None,
            attributes: Vec::new(),
            status: SpanStatus::Ok,
        });
        span_id
    }

    pub fn end_span(&mut self, span_id: &str, status: SpanStatus) {
        if let Some(span) = self.spans.iter_mut().find(|s| s.span_id == span_id) {
            span.end_ns = Some(now_ns());
            span.status = status;
        }
    }

    pub fn add_attribute(&mut self, span_id: &str, key: &str, value: &str) {
        if let Some(span) = self.spans.iter_mut().find(|s| s.span_id == span_id) {
            span.attributes.push((key.to_string(), value.to_string()));
        }
    }

    pub fn spans_for_trace(&self, trace_id: &str) -> Vec<&Span> {
        self.spans.iter().filter(|s| s.trace_id == trace_id).collect()
    }

    /// Export spans as OTLP-compatible JSON.
    pub fn export_json(&self) -> String {
        let spans_json: Vec<String> = self
            .spans
            .iter()
            .map(|s| {
                let attrs: Vec<String> = s
                    .attributes
                    .iter()
                    .map(|(k, v)| format!(r#"{{"key":"{}","value":{{"stringValue":"{}"}}}}"#, k, v))
                    .collect();
                let status = match &s.status {
                    SpanStatus::Ok => r#"{"code":"STATUS_CODE_OK"}"#.to_string(),
                    SpanStatus::Error(msg) => {
                        format!(r#"{{"code":"STATUS_CODE_ERROR","message":"{}"}}"#, msg)
                    }
                };
                let end = s.end_ns.unwrap_or(0);
                let parent = s
                    .parent_id
                    .as_deref()
                    .map(|p| format!(r#""parentSpanId":"{}","#, p))
                    .unwrap_or_default();
                format!(
                    r#"{{"traceId":"{}","spanId":"{}",{}"name":"{}","startTimeUnixNano":{},"endTimeUnixNano":{},"attributes":[{}],"status":{}}}"#,
                    s.trace_id, s.span_id, parent, s.operation, s.start_ns, end,
                    attrs.join(","), status,
                )
            })
            .collect();
        format!(
            r#"{{"resourceSpans":[{{"scopeSpans":[{{"spans":[{}]}}]}}]}}"#,
            spans_json.join(",")
        )
    }
}
