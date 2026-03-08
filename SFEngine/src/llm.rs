use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::OnceLock;

static HTTP: OnceLock<Client> = OnceLock::new();

fn client() -> &'static Client {
    HTTP.get_or_init(|| Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build().unwrap())
}

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
        "max_tokens": 4096,
        "temperature": 0.7,
    });

    if let Some(t) = tools {
        if !t.is_empty() {
            body["tools"] = Value::Array(t.to_vec());
        }
    }

    let resp = client()
        .post(format!("{}/chat/completions", config.base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("LLM API {} : {}", status, text));
    }

    let json: Value = resp.json().await.map_err(|e| format!("JSON parse: {}", e))?;

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
