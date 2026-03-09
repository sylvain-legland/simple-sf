use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::OnceLock;

static HTTP: OnceLock<Client> = OnceLock::new();

fn client() -> &'static Client {
    HTTP.get_or_init(|| Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build().unwrap())
}

/// Retry config
const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 2000;   // 2s initial backoff
const MAX_DELAY_MS: u64 = 30_000;  // 30s max backoff

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

static LLM_CONFIG: OnceLock<std::sync::Mutex<Option<LLMConfig>>> = OnceLock::new();

pub fn configure_llm(provider: &str, api_key: &str, base_url: &str, model: &str) {
    let config = LLMConfig {
        provider: provider.to_string(),
        api_key: api_key.to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
    };
    let lock = LLM_CONFIG.get_or_init(|| std::sync::Mutex::new(None));
    *lock.lock().unwrap() = Some(config);
}

pub fn get_config() -> Option<LLMConfig> {
    let lock = LLM_CONFIG.get()?;
    let guard = lock.lock().ok()?;
    guard.as_ref().map(|c| LLMConfig {
        provider: c.provider.clone(),
        api_key: c.api_key.clone(),
        base_url: c.base_url.clone(),
        model: c.model.clone(),
    })
}

pub async fn chat_completion(
    messages: &[LLMMessage],
    system: Option<&str>,
    tools: Option<&[Value]>,
) -> Result<LLMResponse, String> {
    chat_completion_with_tokens(messages, system, tools, 4096).await
}

/// Chat completion with configurable max_tokens + retry with exponential backoff.
pub async fn chat_completion_with_tokens(
    messages: &[LLMMessage],
    system: Option<&str>,
    tools: Option<&[Value]>,
    max_tokens: u32,
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
        "max_tokens": max_tokens,
        "temperature": 0.7,
    });

    if let Some(t) = tools {
        if !t.is_empty() {
            body["tools"] = Value::Array(t.to_vec());
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
                if e.is_timeout() || e.is_connect() {
                    eprintln!("[llm] Network error (attempt {}): {}", attempt + 1, e);
                    continue; // retry on network errors
                }
                return Err(last_err);
            }
        };

        let status = resp.status();

        // Rate limited (429) — retry with Retry-After if provided
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

        // Server errors (500-599) — retry
        if status.is_server_error() {
            let text = resp.text().await.unwrap_or_default();
            last_err = format!("Server error {} : {}", status, text);
            eprintln!("[llm] Server error {} (attempt {})", status, attempt + 1);
            continue;
        }

        // Client errors (400-499 except 429) — don't retry
        if status.is_client_error() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("LLM API {} : {}", status, text));
        }

        // Success — parse response
        let json: Value = resp.json().await.map_err(|e| format!("JSON parse: {}", e))?;
        return parse_response(&json);
    }

    Err(format!("LLM call failed after {} retries: {}", MAX_RETRIES + 1, last_err))
}

/// Exponential backoff: 2s, 4s, 8s... capped at MAX_DELAY_MS.
/// Respects Retry-After header if provided (in seconds).
fn compute_backoff(attempt: u32, retry_after_secs: Option<u64>) -> u64 {
    if let Some(ra) = retry_after_secs {
        return (ra * 1000).min(MAX_DELAY_MS);
    }
    let delay = BASE_DELAY_MS * 2u64.pow(attempt.saturating_sub(1));
    delay.min(MAX_DELAY_MS)
}

fn parse_response(json: &Value) -> Result<LLMResponse, String> {
    let choice = json["choices"].get(0).ok_or("No choices in response")?;
    let message = &choice["message"];

    let content = message["content"].as_str().map(|s| strip_thinking(s));

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

fn strip_thinking(s: &str) -> String {
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
