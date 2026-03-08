use axum::{
    extract::{State, Path, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;

#[derive(Serialize, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub display_name: String,
    pub base_url: String,
    pub models: Vec<String>,
    pub env_var: String,
    pub enabled: bool,
    pub has_key: bool,
}

#[derive(Deserialize)]
pub struct TestRequest {
    pub api_key: String,
}

pub fn all_providers() -> Vec<ProviderInfo> {
    let defs: Vec<(&str, &str, &str, Vec<&str>, &str)> = vec![
        ("openai",     "OpenAI",        "https://api.openai.com/v1",              vec!["gpt-4o-mini", "gpt-4o", "o3-mini"],                       "OPENAI_API_KEY"),
        ("anthropic",  "Anthropic",     "https://api.anthropic.com/v1",           vec!["claude-3-5-haiku-20241022", "claude-sonnet-4-5"],          "ANTHROPIC_API_KEY"),
        ("gemini",     "Google Gemini", "https://generativelanguage.googleapis.com/v1beta", vec!["gemini-2.0-flash", "gemini-1.5-pro"],           "GEMINI_API_KEY"),
        ("minimax",    "MiniMax",       "https://api.minimax.io/v1",              vec!["MiniMax-M2.5", "MiniMax-M1-80k"],                          "MINIMAX_API_KEY"),
        ("kimi",       "Kimi (Moonshot)", "https://api.moonshot.cn/v1",           vec!["moonshot-v1-8k", "moonshot-v1-32k"],                       "KIMI_API_KEY"),
        ("openrouter", "OpenRouter",    "https://openrouter.ai/api/v1",           vec!["openai/gpt-4o-mini", "anthropic/claude-3-haiku"],          "OPENROUTER_API_KEY"),
        ("alibaba",    "Alibaba Qwen",  "https://dashscope.aliyuncs.com/compatible-mode/v1", vec!["qwen-turbo", "qwen-plus", "qwen-max"],         "ALIBABA_API_KEY"),
        ("glm",        "Zhipu GLM",     "https://open.bigmodel.cn/api/paas/v4",   vec!["glm-4-flash", "glm-4-air", "glm-4"],                      "GLM_API_KEY"),
    ];

    defs.into_iter().map(|(name, display, base_url, models, env_var)| {
        let key = std::env::var(env_var).unwrap_or_default();
        ProviderInfo {
            name: name.to_string(),
            display_name: display.to_string(),
            base_url: base_url.to_string(),
            models: models.into_iter().map(String::from).collect(),
            env_var: env_var.to_string(),
            enabled: !key.is_empty(),
            has_key: !key.is_empty(),
        }
    }).collect()
}

pub async fn list_providers(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({ "providers": all_providers() }))
}

pub async fn test_provider(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<TestRequest>,
) -> impl IntoResponse {
    let providers = all_providers();
    let Some(provider) = providers.iter().find(|p| p.name == name) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"ok": false, "error": "Unknown provider"}))).into_response();
    };

    let model = provider.models.first().cloned().unwrap_or_default();
    let url = format!("{}/chat/completions", provider.base_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap();

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Say 'ok' in one word."}],
        "max_tokens": 5
    });

    let mut req_builder = client.post(&url)
        .header("Content-Type", "application/json")
        .bearer_auth(&req.api_key);

    // Anthropic uses a different auth header
    if name == "anthropic" {
        req_builder = client.post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &req.api_key)
            .header("anthropic-version", "2023-06-01");
    }
    // Gemini uses query param
    let gemini_url;
    if name == "gemini" {
        gemini_url = format!("{}?key={}", url, req.api_key);
        req_builder = client.post(&gemini_url)
            .header("Content-Type", "application/json");
    }

    match req_builder.json(&body).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let ok = status < 300;
            Json(serde_json::json!({"ok": ok, "status": status})).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "error": e.to_string()}))).into_response(),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
}

pub fn active_provider() -> Option<(String, String, String)> {
    let providers = all_providers();
    providers.into_iter()
        .find(|p| p.has_key)
        .map(|p| {
            let key = std::env::var(&p.env_var).unwrap_or_default();
            let model = p.models.first().cloned().unwrap_or_default();
            (p.base_url, model, key)
        })
}

pub async fn chat_completion(
    messages: Vec<LLMMessage>,
    system: Option<&str>,
) -> anyhow::Result<String> {
    let (base_url, model, key) = active_provider()
        .ok_or_else(|| anyhow::anyhow!("No LLM provider configured. Please add an API key in Settings."))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let mut all_messages = vec![];
    if let Some(sys) = system {
        all_messages.push(serde_json::json!({"role": "system", "content": sys}));
    }
    for m in &messages {
        all_messages.push(serde_json::json!({"role": m.role, "content": m.content}));
    }

    let body = serde_json::json!({
        "model": model,
        "messages": all_messages,
        "max_tokens": 1024,
        "temperature": 0.7
    });

    let url = format!("{}/chat/completions", base_url);
    let resp = client.post(&url)
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        return Err(anyhow::anyhow!("LLM error: {}", text));
    }

    let json: serde_json::Value = resp.json().await?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(content)
}
