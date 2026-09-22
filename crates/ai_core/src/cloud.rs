//! Cloud chat-completion providers for AI commit messages.
//!
//! Ported from the Qt app (`src/mainwindow.cpp::runAiGeneration`): builds the
//! provider-specific request for the selected provider (OpenRouter, OpenAI,
//! Anthropic or Google AI Studio) and extracts the generated text from the
//! provider-specific response shape.
//!
//! The function is synchronous (blocking the calling thread) to mirror the
//! existing local-GGUF inference API; long-running work should be dispatched
//! onto a background thread by the caller.

use serde_json::{json, Value};

/// Cloud providers supported for AI commit-message generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    OpenRouter,
    OpenAi,
    Anthropic,
    GoogleAiStudio,
}

impl CloudProvider {
    /// All supported providers, in display order.
    pub const ALL: [CloudProvider; 4] = [
        Self::OpenRouter,
        Self::OpenAi,
        Self::Anthropic,
        Self::GoogleAiStudio,
    ];

    /// Stable display name — also used as the persisted `ai/provider` value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenRouter => "OpenRouter",
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Anthropic",
            Self::GoogleAiStudio => "Google AI Studio",
        }
    }

    /// Parse a persisted `ai/provider` value back into an enum variant.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|p| p.as_str() == s)
    }
}

/// Generate text using a cloud chat-completion provider.
///
/// Blocks the calling thread until the provider responds (or times out).
pub fn generate_cloud(
    provider: CloudProvider,
    model: &str,
    api_key: &str,
    system_prompt: &str,
    user_content: &str,
) -> Result<String, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("failed to create async runtime: {e}"))?;
    rt.block_on(generate_cloud_async(
        provider,
        model,
        api_key,
        system_prompt,
        user_content,
    ))
}

async fn generate_cloud_async(
    provider: CloudProvider,
    model: &str,
    api_key: &str,
    system_prompt: &str,
    user_content: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;

    let (url, body) = build_request(provider, model, api_key, system_prompt, user_content)?;

    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body);

    // Provider-specific auth headers (mirrors the Qt implementation).
    match provider {
        CloudProvider::Anthropic => {
            req = req
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01");
        }
        CloudProvider::GoogleAiStudio => {}
        _ => {
            req = req.header("Authorization", format!("Bearer {api_key}"));
        }
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("network error: {e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;

    if !status.is_success() {
        return Err(format!("HTTP {status}: {text}"));
    }

    let obj: Value =
        serde_json::from_str(&text).map_err(|e| format!("invalid JSON response: {e}"))?;
    let content = extract_text(provider, &obj);
    if content.is_empty() {
        return Err("No response from AI.".to_string());
    }
    Ok(content)
}

/// Build the provider-specific request URL and body (via `serde_json::Value`,
/// matching the shapes constructed by the Qt implementation).
fn build_request(
    provider: CloudProvider,
    model: &str,
    api_key: &str,
    system_prompt: &str,
    user_content: &str,
) -> Result<(String, Value), String> {
    let system = json!({ "role": "system", "content": system_prompt });
    let user = json!({ "role": "user", "content": user_content });

    match provider {
        CloudProvider::OpenRouter => Ok((
            "https://openrouter.ai/api/v1/chat/completions".to_string(),
            json!({ "model": model, "messages": [system, user] }),
        )),
        CloudProvider::OpenAi => Ok((
            "https://api.openai.com/v1/chat/completions".to_string(),
            json!({ "model": model, "messages": [system, user] }),
        )),
        CloudProvider::Anthropic => Ok((
            "https://api.anthropic.com/v1/messages".to_string(),
            json!({
                "model": model,
                "max_tokens": 1024,
                "messages": [{
                    "role": "user",
                    "content": format!("System: {system_prompt}\n\n{user_content}")
                }]
            }),
        )),
        CloudProvider::GoogleAiStudio => Ok((
            format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
            ),
            json!({
                "contents": [{
                    "role": "user",
                    "parts": [{
                        "text": format!("System: {system_prompt}\n\n{user_content}")
                    }]
                }]
            }),
        )),
    }
}

/// Extract the generated text from the provider-specific response shape.
fn extract_text(provider: CloudProvider, obj: &Value) -> String {
    match provider {
        CloudProvider::Anthropic => obj["content"]
            .as_array()
            .and_then(|c| c.first())
            .and_then(|item| item["text"].as_str())
            .unwrap_or_default()
            .trim()
            .to_string(),
        CloudProvider::GoogleAiStudio => obj["candidates"]
            .as_array()
            .and_then(|cands| cands.first())
            .and_then(|c| c["content"]["parts"].as_array())
            .and_then(|parts| parts.first())
            .and_then(|p| p["text"].as_str())
            .unwrap_or_default()
            .trim()
            .to_string(),
        // OpenAI-compatible (OpenRouter, OpenAI).
        _ => obj["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .and_then(|c| c["message"]["content"].as_str())
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_roundtrip() {
        for p in CloudProvider::ALL {
            assert_eq!(CloudProvider::parse(p.as_str()), Some(p));
        }
        assert_eq!(CloudProvider::parse("nope"), None);
    }

    #[test]
    fn extract_openai_compatible() {
        let v: Value =
            serde_json::from_str(r#"{"choices":[{"message":{"content":"  feat: add cors  "}}]}"#)
                .unwrap();
        assert_eq!(
            extract_text(CloudProvider::OpenRouter, &v),
            "feat: add cors"
        );
    }

    #[test]
    fn extract_anthropic() {
        let v: Value =
            serde_json::from_str(r#"{"content":[{"type":"text","text":"  hello  "}]}"#).unwrap();
        assert_eq!(extract_text(CloudProvider::Anthropic, &v), "hello");
    }

    #[test]
    fn extract_google() {
        let v: Value =
            serde_json::from_str(r#"{"candidates":[{"content":{"parts":[{"text":"  hi  "}]}}]}"#)
                .unwrap();
        assert_eq!(extract_text(CloudProvider::GoogleAiStudio, &v), "hi");
    }
}
