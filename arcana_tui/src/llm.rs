use futures::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::event::AppEvent;
use crate::types::ResponseStats;

/// Spawn a streaming LLM request. Sends AppEvents (ThinkStart, Token, ThinkEnd, ResponseComplete)
/// back through the provided sender.
pub fn spawn_stream(
    config: &Config,
    messages: Vec<serde_json::Value>,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let provider = config.agents.main.provider.clone();
    let model = config.agents.main.model.clone();
    let thinking = config.agents.main.thinking.clone();

    let api_key = config.resolve_api_key(&provider).unwrap_or_default();
    let base_url = match provider.as_str() {
        "deepseek" => {
            let url = &config.providers.deepseek.base_url;
            if url.is_empty() { "https://api.deepseek.com".to_string() } else { url.clone() }
        }
        _ => "https://api.deepseek.com".to_string(),
    };

    tokio::spawn(async move {
        if let Err(e) = do_stream(&base_url, &api_key, &model, &thinking, &messages, &tx).await {
            let _ = tx.send(AppEvent::LlmError(crate::types::LlmError::NetworkError {
                message: e.to_string(),
            }));
        }
    });
}

async fn do_stream(
    base_url: &str,
    api_key: &str,
    model: &str,
    thinking: &crate::config::ThinkingConfig,
    messages: &[serde_json::Value],
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "stream_options": {"include_usage": true}
    });
    if thinking.enabled {
        body["thinking"] = serde_json::json!({"type": "enabled"});
        body["reasoning_effort"] = serde_json::json!(thinking.reasoning_effort);
    }

    let client = Client::new();
    let resp = client
        .post(format!("{}/chat/completions", base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("API error ({}): {}", status, text).into());
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut in_thinking = false;
    let mut usage_data: Option<serde_json::Value> = None;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete SSE lines
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim_end_matches('\r').to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() {
                continue;
            }
            if line == "data: [DONE]" {
                // Stream complete
                let stats = usage_data.map(|u| ResponseStats {
                    input_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as usize,
                    output_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as usize,
                    cost: 0.0,
                    duration_secs: 0.0,
                });
                let _ = tx.send(AppEvent::ResponseComplete(stats));
                return Ok(());
            }
            if let Some(json_str) = line.strip_prefix("data: ") {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str) {
                    // Extract usage if present
                    if let Some(usage) = data.get("usage") {
                        if !usage.is_null() {
                            usage_data = Some(usage.clone());
                        }
                    }

                    if let Some(delta) = data["choices"].get(0).and_then(|c| c.get("delta")) {
                        // Handle reasoning_content (thinking)
                        if let Some(reasoning) = delta.get("reasoning_content") {
                            if let Some(text) = reasoning.as_str() {
                                if !text.is_empty() {
                                    if !in_thinking {
                                        in_thinking = true;
                                        let _ = tx.send(AppEvent::ThinkStart);
                                    }
                                    let _ = tx.send(AppEvent::Token(format!("\x00THINK:{}", text)));
                                }
                            }
                        }

                        // Handle content
                        if let Some(content) = delta.get("content") {
                            if let Some(text) = content.as_str() {
                                if !text.is_empty() {
                                    if in_thinking {
                                        in_thinking = false;
                                        let _ = tx.send(AppEvent::ThinkEnd);
                                    }
                                    let _ = tx.send(AppEvent::Token(text.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If stream ended without [DONE]
    if in_thinking {
        let _ = tx.send(AppEvent::ThinkEnd);
    }
    let stats = usage_data.map(|u| ResponseStats {
        input_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as usize,
        output_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as usize,
        cost: 0.0,
        duration_secs: 0.0,
    });
    let _ = tx.send(AppEvent::ResponseComplete(stats));
    Ok(())
}
