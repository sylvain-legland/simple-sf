use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{OnceLock, RwLock};

static HTTP: OnceLock<Client> = OnceLock::new();

fn client() -> &'static Client {
    HTTP.get_or_init(|| Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build().unwrap())
}

/// Retry config
const MAX_RETRIES: u32 = 5;
const BASE_DELAY_MS: u64 = 2000;
const MAX_DELAY_MS: u64 = 60_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Clone, Debug)]
pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

pub struct LLMConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

/// RwLock instead of OnceLock<Mutex> — allows runtime model/provider changes (#12)
static LLM_CONFIG: OnceLock<RwLock<Option<LLMConfig>>> = OnceLock::new();

fn config_lock() -> &'static RwLock<Option<LLMConfig>> {
    LLM_CONFIG.get_or_init(|| RwLock::new(None))
}

pub fn configure_llm(provider: &str, api_key: &str, base_url: &str, model: &str) {
    let config = LLMConfig {
        provider: provider.to_string(),
        api_key: api_key.to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
    };
    let mut guard = config_lock().write().unwrap();
    *guard = Some(config);
}

/// Update just the model at runtime (for dynamic routing)
pub fn set_model(model: &str) {
    let mut guard = config_lock().write().unwrap();
    if let Some(ref mut c) = *guard {
        c.model = model.to_string();
    }
}

/// Update provider + base_url + api_key at runtime
pub fn set_provider(provider: &str, api_key: &str, base_url: &str) {
    let mut guard = config_lock().write().unwrap();
    if let Some(ref mut c) = *guard {
        c.provider = provider.to_string();
        c.api_key = api_key.to_string();
        c.base_url = base_url.to_string();
    }
}

pub fn get_config() -> Option<LLMConfig> {
    let guard = config_lock().read().ok()?;
    guard.as_ref().map(|c| LLMConfig {
        provider: c.provider.clone(),
        api_key: c.api_key.clone(),
        base_url: c.base_url.clone(),
        model: c.model.clone(),
    })
}

/// Optional callback for streaming chunks — receives each content delta as it arrives
pub type OnChunkFn = Box<dyn Fn(&str) + Send + Sync>;

/// Optional callback for reasoning state changes — true=started, false=ended
pub type OnReasoningFn = Box<dyn Fn(bool) + Send + Sync>;

/// No artificial token limit — omit max_tokens to let the model use its full capacity
pub async fn chat_completion(
    messages: &[LLMMessage],
    system: Option<&str>,
    tools: Option<&[Value]>,
) -> Result<LLMResponse, String> {
    chat_completion_inner(messages, system, tools, None, None).await
}

/// Kept for backward compat — ignores max_tokens, delegates to chat_completion
pub async fn chat_completion_with_tokens(
    messages: &[LLMMessage],
    system: Option<&str>,
    tools: Option<&[Value]>,
    _max_tokens: u32,
) -> Result<LLMResponse, String> {
    chat_completion_inner(messages, system, tools, None, None).await
}

/// Streaming variant — calls on_chunk for each content delta
pub async fn chat_completion_streaming(
    messages: &[LLMMessage],
    system: Option<&str>,
    tools: Option<&[Value]>,
    on_chunk: OnChunkFn,
    on_reasoning: Option<OnReasoningFn>,
) -> Result<LLMResponse, String> {
    chat_completion_inner(messages, system, tools, Some(on_chunk), on_reasoning).await
}

/// Core implementation — NO max_tokens in the request body
async fn chat_completion_inner(
    messages: &[LLMMessage],
    system: Option<&str>,
    tools: Option<&[Value]>,
    on_chunk: Option<OnChunkFn>,
    on_reasoning: Option<OnReasoningFn>,
) -> Result<LLMResponse, String> {
    let config = get_config().ok_or("LLM not configured")?;

    let mut msgs: Vec<Value> = Vec::new();
    if let Some(sys) = system {
        msgs.push(json!({"role": "system", "content": sys}));
    }
    for m in messages {
        msgs.push(json!({"role": m.role, "content": m.content}));
    }

    let mut body = json!({
        "model": config.model,
        "messages": msgs,
        "temperature": 0.7,
        "max_tokens": 65536,
        "stream": true,
    });

    if let Some(t) = tools {
        if !t.is_empty() {
            body["tools"] = Value::Array(t.to_vec());
            body["stream"] = json!(false);
        }
    }

    let url = format!("{}/chat/completions", config.base_url);
    let mut last_err = String::new();

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            let delay = compute_backoff(attempt, None);
            eprintln!("[llm] Retry {}/{} in {}ms...", attempt, MAX_RETRIES, delay);
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }

        let result = client()
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", config.api_key))
            .json(&body)
            .send()
            .await;

        let resp = match result {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("HTTP error: {}", e);
                if e.is_connect() {
                    // Connection refused — server is down, fail fast after 1 retry
                    if attempt >= 1 {
                        return Err(format!("Server unreachable (connection refused): {}", e));
                    }
                    eprintln!("[llm] Connection refused (attempt {}), retrying once...", attempt + 1);
                    continue;
                }
                if e.is_timeout() {
                    eprintln!("[llm] Timeout (attempt {}): {}", attempt + 1, e);
                    continue;
                }
                return Err(last_err);
            }
        };

        let status = resp.status();

        if status.as_u16() == 429 {
            let retry_after = resp.headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            let text = resp.text().await.unwrap_or_default();
            last_err = format!("Rate limited (429): {}", text);
            eprintln!("[llm] Rate limited (attempt {}). Retry-After: {:?}", attempt + 1, retry_after);
            if attempt < MAX_RETRIES {
                let delay = compute_backoff(attempt + 1, retry_after);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
            continue;
        }

        if status.is_server_error() {
            let text = resp.text().await.unwrap_or_default();
            last_err = format!("Server error {} : {}", status, text);
            eprintln!("[llm] Server error {} (attempt {})", status, attempt + 1);
            continue;
        }

        if status.is_client_error() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("LLM API {} : {}", status, text));
        }

        // Streaming response (#7)
        let is_stream = body["stream"].as_bool().unwrap_or(false);
        if is_stream {
            return parse_stream(resp, &on_chunk, &on_reasoning).await;
        }

        let json: Value = resp.json().await.map_err(|e| format!("JSON parse: {}", e))?;
        return parse_response(&json);
    }

    Err(format!("LLM call failed after {} retries: {}", MAX_RETRIES + 1, last_err))
}

/// Parse SSE stream into a complete LLMResponse, emitting chunks via on_chunk.
/// Handles reasoning models (Qwen3.5, etc.) that emit `delta.reasoning` separately from `delta.content`.
async fn parse_stream(
    resp: reqwest::Response,
    on_chunk: &Option<OnChunkFn>,
    on_reasoning: &Option<OnReasoningFn>,
) -> Result<LLMResponse, String> {
    use tokio::io::AsyncBufReadExt;
    use tokio_util::io::StreamReader;
    use futures_util::StreamExt;

    let byte_stream = resp.bytes_stream().map(|r| r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));
    let reader = StreamReader::new(byte_stream);
    let mut lines = reader.lines();

    let mut full_content = String::new();
    let mut reasoning_chars: usize = 0;
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut is_reasoning = false;
    let mut had_reasoning = false;
    let mut finish_reason = String::new();
    let mut usage_completion: Option<u64> = None;
    let mut usage_reasoning: Option<u64> = None;

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim_start().to_string();
        if !line.starts_with("data: ") { continue; }
        let data = &line[6..];
        if data == "[DONE]" { break; }

        if let Ok(chunk) = serde_json::from_str::<Value>(data) {
            // Capture usage from final chunk (MLX sends it on the last SSE event)
            if let Some(usage) = chunk.get("usage") {
                usage_completion = usage["completion_tokens"].as_u64();
                usage_reasoning = usage.get("reasoning_tokens")
                    .or_else(|| usage.get("completion_tokens_details").and_then(|d| d.get("reasoning_tokens")))
                    .and_then(|v| v.as_u64());
            }

            if let Some(choice) = chunk["choices"].get(0) {
                if let Some(fr) = choice["finish_reason"].as_str() {
                    finish_reason = fr.to_string();
                }

                if let Some(delta) = choice.get("delta") {
                    // Handle reasoning tokens (Qwen3.5, DeepSeek, etc.)
                    if let Some(reasoning) = delta["reasoning"].as_str() {
                        if !reasoning.is_empty() {
                            reasoning_chars += reasoning.len();
                            if !is_reasoning {
                                is_reasoning = true;
                                had_reasoning = true;
                                if let Some(cb) = on_reasoning {
                                    cb(true);
                                }
                            }
                        }
                    }

                    if let Some(content) = delta["content"].as_str() {
                        if !content.is_empty() {
                            if is_reasoning {
                                is_reasoning = false;
                                if let Some(cb) = on_reasoning {
                                    cb(false);
                                }
                            }
                            full_content.push_str(content);
                            if let Some(cb) = on_chunk {
                                cb(content);
                            }
                        }
                    }

                    // Accumulate tool call deltas
                    if let Some(tcs) = delta["tool_calls"].as_array() {
                        for tc in tcs {
                            let idx = tc["index"].as_u64().unwrap_or(0) as usize;
                            while tool_calls.len() <= idx {
                                tool_calls.push(ToolCall { id: String::new(), name: String::new(), arguments: String::new() });
                            }
                            if let Some(id) = tc["id"].as_str() { tool_calls[idx].id = id.to_string(); }
                            if let Some(f) = tc.get("function") {
                                if let Some(n) = f["name"].as_str() { tool_calls[idx].name.push_str(n); }
                                if let Some(a) = f["arguments"].as_str() { tool_calls[idx].arguments.push_str(a); }
                            }
                        }
                    }
                }
            }
        }
    }

    // If model spent all tokens on reasoning with no visible content
    if full_content.is_empty() && had_reasoning {
        let usage_info = match (usage_completion, usage_reasoning) {
            (Some(comp), Some(reas)) => format!(" [{}t completion, {}t reasoning, finish={}]", comp, reas, finish_reason),
            (Some(comp), None) => format!(" [{}t completion, ~{}c reasoning chars, finish={}]", comp, reasoning_chars, finish_reason),
            _ => format!(" [~{}c reasoning chars, finish={}]", reasoning_chars, finish_reason),
        };
        eprintln!("[llm] Warning: model exhausted token budget on reasoning — no visible content{}", usage_info);
        full_content = format!(
            "(Reasoning model exhausted its token budget without producing visible output.{})",
            usage_info,
        );
        if is_reasoning {
            if let Some(cb) = on_reasoning {
                cb(false);
            }
        }
        if let Some(cb) = on_chunk {
            cb(&full_content);
        }
    } else if had_reasoning {
        let usage_info = match (usage_completion, usage_reasoning) {
            (Some(comp), Some(reas)) => format!("{}t total, {}t reasoning", comp, reas),
            (Some(comp), None) => format!("{}t total, ~{}c reasoning", comp, reasoning_chars),
            _ => format!("~{}c reasoning", reasoning_chars),
        };
        eprintln!("[llm] Reasoning model OK: {} + {}c content, finish={}", usage_info, full_content.len(), finish_reason);
    }

    let content = if full_content.is_empty() { None } else { Some(strip_thinking(&full_content)) };
    Ok(LLMResponse { content, tool_calls })
}

pub fn compute_backoff(attempt: u32, retry_after_secs: Option<u64>) -> u64 {
    if let Some(ra) = retry_after_secs {
        return (ra * 1000).min(MAX_DELAY_MS);
    }
    let delay = BASE_DELAY_MS * 2u64.pow(attempt.saturating_sub(1));
    delay.min(MAX_DELAY_MS)
}

fn parse_response(json: &Value) -> Result<LLMResponse, String> {
    let choice = json["choices"].get(0).ok_or("No choices in response")?;
    let message = &choice["message"];

    // Get content, stripping any <think> blocks
    let mut content = message["content"].as_str().map(|s| strip_thinking(s));

    // Handle reasoning models (Qwen3.5, etc.) — if content is empty but reasoning exists,
    // the model exhausted its budget on thinking
    if content.as_ref().map_or(true, |c| c.is_empty()) {
        if let Some(reasoning) = message["reasoning"].as_str() {
            if !reasoning.is_empty() {
                let usage = json.get("usage");
                let comp = usage.and_then(|u| u["completion_tokens"].as_u64());
                let reas = usage.and_then(|u| u.get("reasoning_tokens").or_else(|| u.get("completion_tokens_details").and_then(|d| d.get("reasoning_tokens")))).and_then(|v| v.as_u64());
                let fr = choice["finish_reason"].as_str().unwrap_or("unknown");
                let usage_info = match (comp, reas) {
                    (Some(c), Some(r)) => format!(" [{}t completion, {}t reasoning, finish={}]", c, r, fr),
                    (Some(c), None) => format!(" [{}t completion, ~{}c reasoning chars, finish={}]", c, reasoning.len(), fr),
                    _ => format!(" [~{}c reasoning chars, finish={}]", reasoning.len(), fr),
                };
                eprintln!("[llm] Warning: non-streaming response has reasoning but no content{}", usage_info);
                content = Some(format!("(Reasoning model exhausted its token budget without producing visible output.{})", usage_info));
            }
        }
    }

    let mut tool_calls = Vec::new();
    if let Some(tcs) = message["tool_calls"].as_array() {
        for tc in tcs {
            tool_calls.push(ToolCall {
                id: tc["id"].as_str().unwrap_or("").to_string(),
                name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                arguments: tc["function"]["arguments"].as_str().unwrap_or("{}").to_string(),
            });
        }
    }

    Ok(LLMResponse { content, tool_calls })
}

pub fn strip_thinking(s: &str) -> String {
    let mut out = s.to_string();
    while let Some(start) = out.find("<think>") {
        if let Some(end) = out.find("</think>") {
            if start <= end {
                out.drain(start..end + 8);
            } else { break; }
        } else { break; }
    }
    out.trim().to_string()
}

// ─── Embeddings API ──────────────────────────────────────────

/// Compute embeddings for a batch of texts using the configured LLM provider's
/// /embeddings endpoint (OpenAI-compatible).
pub async fn embed(texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
    // Extract config before await (don't hold RwLockReadGuard across await)
    let (api_key, base_url, provider) = {
        let guard = config_lock().read().map_err(|e| e.to_string())?;
        let config = guard.as_ref().ok_or("LLM not configured")?;
        (config.api_key.clone(), config.base_url.clone(), config.provider.clone())
    };

    let emb_model = match provider.as_str() {
        "openai" | "azure" | "azure-openai" => "text-embedding-3-small",
        "ollama" => "nomic-embed-text",
        _ => "text-embedding-3-small",
    };

    let url = format!("{}/embeddings", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": emb_model,
        "input": texts,
    });

    let resp = client()
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Embedding request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Embedding API error {}: {}", status, body));
    }

    let data: serde_json::Value = resp.json().await
        .map_err(|e| format!("Failed to parse embedding response: {}", e))?;

    let embeddings = data["data"].as_array()
        .ok_or("Invalid embedding response: missing 'data' array")?
        .iter()
        .map(|item| {
            item["embedding"].as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect()
        })
        .collect();

    Ok(embeddings)
}
