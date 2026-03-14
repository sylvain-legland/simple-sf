// Ref: FT-SSF-020

use super::patterns::run_pattern;
use super::types::MAX_PHASE_RETRIES;
use crate::executor::{AgentEvent, EventCallback};
use crate::llm;

pub(crate) async fn run_phase_with_retry(
    agent_ids: &[&str],
    task: &str,
    phase: &str,
    pattern: &str,
    workspace: &str,
    mission_id: &str,
    phase_id: &str,
    on_event: &EventCallback,
) -> Result<String, String> {
    let mut last_error = String::new();
    let mut current_task = task.to_string();

    for attempt in 0..=MAX_PHASE_RETRIES {
        if attempt > 0 {
            let backoff_secs = 2u64.pow(attempt as u32); // 2s, 4s, 8s
            eprintln!("[engine] Phase {} attempt {} — backoff {}s", phase, attempt + 1, backoff_secs);
            on_event("engine", AgentEvent::Response {
                content: format!("Phase {} failed (attempt {}), retrying in {}s...", phase, attempt, backoff_secs),
            });
            tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;

            // LLM health probe — quick check before burning a retry
            if let Err(probe_err) = llm_health_probe().await {
                eprintln!("[engine] LLM health probe failed: {} — attempting auto-restart", probe_err);
                on_event("engine", AgentEvent::Response {
                    content: format!("LLM down ({}), attempting auto-restart...", probe_err),
                });

                // Try to restart the LLM server
                if let Err(restart_err) = restart_llm_server().await {
                    eprintln!("[engine] LLM restart failed: {}", restart_err);
                    on_event("engine", AgentEvent::Response {
                        content: format!("LLM restart failed: {} — waiting 15s...", restart_err),
                    });
                    tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                } else {
                    on_event("engine", AgentEvent::Response {
                        content: "LLM server restarted successfully".to_string(),
                    });
                }
            }

            // Inject previous error feedback
            current_task = format!(
                "{}\n\n## PREVIOUS ATTEMPT {} FAILED:\n{}\n\nFix the issues and try again.",
                task, attempt, last_error
            );
        }

        match run_pattern(agent_ids, &current_task, phase, pattern, workspace, mission_id, phase_id, on_event).await {
            Ok(output) => return Ok(output),
            Err(e) => {
                eprintln!("[engine] Phase {} failed (attempt {}): {}", phase, attempt + 1, e);
                last_error = e;
            }
        }
    }

    Err(format!("Phase {} failed after {} retries: {}", phase, MAX_PHASE_RETRIES, last_error))
}

/// Quick LLM health check — send a trivial prompt to verify connectivity
async fn llm_health_probe() -> Result<(), String> {
    let config = llm::get_config().ok_or("LLM not configured")?;
    let base = config.base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1") {
        format!("{}/chat/completions", base)
    } else {
        format!("{}/v1/chat/completions", base)
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": config.model,
        "messages": [{"role": "user", "content": "ping"}],
        "max_tokens": 5
    });

    let resp = client.post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("unreachable: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}

/// Restart the LLM server (MLX or compatible) when it crashes.
/// Only applicable for local servers — skips for cloud providers.
async fn restart_llm_server() -> Result<(), String> {
    let config = llm::get_config().ok_or("LLM not configured")?;

    // Cloud providers can't be restarted locally
    let base = config.base_url.to_lowercase();
    if base.contains("minimax") || base.contains("openai.com") || base.contains("anthropic")
        || base.contains("azure") || base.contains("googleapis") || base.contains("nvidia")
    {
        return Err("Cloud provider — cannot restart locally".into());
    }

    // Parse port from base_url
    let port = config.base_url
        .split(':').last()
        .and_then(|p| p.trim_matches('/').split('/').next())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8800);

    eprintln!("[engine] Restarting LLM server on port {}...", port);

    // Kill existing process on port
    let kill_output = tokio::process::Command::new("sh")
        .args(["-c", &format!("lsof -ti:{} | xargs kill -9 2>/dev/null; true", port)])
        .output()
        .await
        .map_err(|e| format!("kill failed: {}", e))?;
    eprintln!("[engine] Kill result: {}", String::from_utf8_lossy(&kill_output.stderr));

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Relaunch MLX server
    let model = &config.model;
    let cmd = format!(
        "nohup mlx_lm.server --model {} --port {} > /tmp/mlx-server.log 2>&1 &",
        model, port
    );
    eprintln!("[engine] Launching: {}", cmd);

    let _ = tokio::process::Command::new("sh")
        .args(["-c", &cmd])
        .output()
        .await
        .map_err(|e| format!("launch failed: {}", e))?;

    // Wait for server to become ready (up to 60s)
    let base = config.base_url.trim_end_matches('/');
    let models_url = if base.ends_with("/v1") {
        format!("{}/models", base)
    } else {
        format!("{}/v1/models", base)
    };

    for i in 0..12 {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;

        match client.get(&models_url).send().await {
            Ok(r) if r.status().is_success() => {
                eprintln!("[engine] LLM server ready after {}s", (i + 1) * 5);
                return Ok(());
            }
            _ => {
                eprintln!("[engine] LLM not ready yet (attempt {}/12)...", i + 1);
            }
        }
    }

    Err("LLM server did not become ready within 60s".to_string())
}
