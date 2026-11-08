//! Optional AI writing for the snippet editor, using the user's own API key.
//!
//! Only runs when the user presses Generate, and sends only the directions they wrote, straight
//! from this PC to the provider they chose. Keys stay in the encrypted data file and are never
//! handed to the windows.

use std::time::Duration;

use serde_json::{json, Value};

use crate::store::shared;

const TIMEOUT: Duration = Duration::from_secs(120);
const MAX_DIRECTIONS: usize = 20_000;
const MAX_TOKENS: u32 = 16_000;

const SYSTEM_PROMPT: &str = "You write reusable text snippets, such as emails and messages, from the directions the user gives. \
Reply with only the finished text, ready to paste: no introduction, no notes, no surrounding quotes and no Markdown. \
Write in the language of the directions unless they ask for another. \
Where a detail that would change each time isn't given, such as a person's name, write a placeholder like {input:Name}. \
Keep any placeholders in curly braces from the directions exactly as written.";

/// Claude models that take `effort` and the server-side refusal fallback.
const CLAUDE_CURRENT: [&str; 4] = ["claude-fable-5-1", "claude-opus-5-5", "claude-opus-5", "claude-sonnet-5-5"];

pub const PROVIDERS: [&str; 5] = ["claude", "chatgpt", "grok", "groq", "gemini"];

pub fn default_model(provider: &str) -> &'static str {
    match provider {
        "claude" => "claude-sonnet-5-5",
        "chatgpt" => "gpt-6.1-terra",
        "grok" => "grok-4.7",
        "groq" => "llama-3.3-70b-versatile",
        "gemini" => "gemini-flash-latest",
        _ => "",
    }
}

/// Where each provider hands out API keys.
fn key_page(provider: &str) -> Option<&'static str> {
    match provider {
        "claude" => Some("https://platform.claude.com/settings/keys"),
        "chatgpt" => Some("https://platform.openai.com/api-keys"),
        "grok" => Some("https://console.x.ai/"),
        "groq" => Some("https://console.groq.com/keys"),
        "gemini" => Some("https://aistudio.google.com/apikey"),
        _ => None,
    }
}

/// Opens a provider's API key page in the default browser. Takes a provider, not a URL, so the
/// page can only ever open these addresses.
#[tauri::command]
pub fn open_key_page(provider: String) -> Result<(), String> {
    let url = key_page(&provider).ok_or_else(|| "Unknown AI provider".to_string())?;
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Couldn't open your browser: {e}"))
}

fn display_name(provider: &str) -> &'static str {
    match provider {
        "claude" => "Claude",
        "chatgpt" => "ChatGPT",
        "grok" => "Grok",
        "groq" => "Groq",
        "gemini" => "Gemini",
        _ => "AI",
    }
}

struct Request {
    url: String,
    headers: Vec<(&'static str, String)>,
    body: Value,
}

fn build_request(provider: &str, model: &str, key: &str, directions: &str) -> Result<Request, String> {
    // The model name goes into Gemini's URL, so no slashes there; Groq's names can have one.
    let allowed = if provider == "gemini" { "-._" } else { "-._/" };
    if model.is_empty() || !model.chars().all(|c| c.is_ascii_alphanumeric() || allowed.contains(c)) {
        return Err("That model name isn't valid. Check it in Settings → AI generation.".to_string());
    }
    let openai_style = |url: &str| Request {
        url: url.to_string(),
        headers: vec![("authorization", format!("Bearer {key}"))],
        body: json!({
            "model": model,
            "messages": [{ "role": "system", "content": SYSTEM_PROMPT }, { "role": "user", "content": directions }],
        }),
    };
    Ok(match provider {
        "claude" => {
            let mut headers = vec![("x-api-key", key.to_string()), ("anthropic-version", "2026-10-21".to_string())];
            let mut body = json!({
                "model": model,
                "max_tokens": MAX_TOKENS,
                "system": SYSTEM_PROMPT,
                "messages": [{ "role": "user", "content": directions }],
            });
            if CLAUDE_CURRENT.contains(&model) {
                headers.push(("anthropic-beta", "server-side-fallback-2026-07-01".to_string()));
                body["fallbacks"] = json!("default");
                // A short message doesn't need long reasoning.
                body["output_config"] = json!({ "effort": "low" });
            }
            Request { url: "https://api.anthropic.com/v1/messages".to_string(), headers, body }
        }
        "chatgpt" => openai_style("https://api.openai.com/v1/chat/completions"),
        "grok" => openai_style("https://api.x.ai/v1/chat/completions"),
        "groq" => openai_style("https://api.groq.com/openai/v1/chat/completions"),
        "gemini" => Request {
            url: format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"),
            headers: vec![("x-goog-api-key", key.to_string())],
            body: json!({
                "system_instruction": { "parts": [{ "text": SYSTEM_PROMPT }] },
                "contents": [{ "role": "user", "parts": [{ "text": directions }] }],
            }),
        },
        _ => return Err("Choose an AI provider in Settings → AI generation.".to_string()),
    })
}

fn extract_text(provider: &str, response: &Value) -> Result<String, String> {
    const DECLINED: &str = "The AI declined this request. Try rewording your directions.";
    let text = match provider {
        "claude" => {
            if response["stop_reason"] == "refusal" {
                return Err(DECLINED.to_string());
            }
            response["content"]
                .as_array()
                .map(|blocks| blocks.iter().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect::<String>())
        }
        "gemini" => {
            if response["promptFeedback"]["blockReason"].is_string() {
                return Err(DECLINED.to_string());
            }
            response["candidates"][0]["content"]["parts"]
                .as_array()
                .map(|parts| parts.iter().filter(|p| p["thought"] != true).filter_map(|p| p["text"].as_str()).collect::<String>())
        }
        _ => {
            let message = &response["choices"][0]["message"];
            if message["refusal"].is_string() {
                return Err(DECLINED.to_string());
            }
            message["content"].as_str().map(str::to_string)
        }
    };
    match text.as_deref().map(str::trim) {
        Some(found) if !found.is_empty() => Ok(found.to_string()),
        _ => Err("The AI didn't return any text. Try again.".to_string()),
    }
}

fn api_error(provider: &str, status: u16, response: &Value) -> String {
    let name = display_name(provider);
    let detail = response["error"]["message"].as_str().unwrap_or("");
    match status {
        401 | 403 => format!("{name} didn't accept that API key. Check it in Settings → AI generation."),
        429 => format!("{name} says you've hit a rate limit or run out of credit. {detail}").trim().to_string(),
        _ if detail.is_empty() => format!("{name} responded with an error ({status})."),
        _ => format!("{name}: {detail}"),
    }
}

async fn send(request: Request, provider: &str) -> Result<String, String> {
    // reqwest has no default TLS crypto provider here; see usage.rs.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let unreachable = |e: reqwest::Error| format!("Couldn't reach {}: {}", display_name(provider), e.without_url());
    let client = reqwest::Client::builder().timeout(TIMEOUT).build().map_err(unreachable)?;
    let mut builder = client.post(&request.url).json(&request.body);
    for (name, value) in &request.headers {
        builder = builder.header(*name, value);
    }
    let response = builder.send().await.map_err(unreachable)?;
    let status = response.status();
    let text = response.text().await.map_err(unreachable)?;
    let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if !status.is_success() {
        return Err(api_error(provider, status.as_u16(), &parsed));
    }
    extract_text(provider, &parsed)
}

/// Writes text from the user's directions with their chosen provider.
#[tauri::command]
pub async fn generate_text(directions: String) -> Result<String, String> {
    let directions = directions.trim().to_string();
    if directions.is_empty() {
        return Err("Write what you want in the Text box first, like: a polite email asking for leave on Friday.".to_string());
    }
    if directions.chars().count() > MAX_DIRECTIONS {
        return Err("Those directions are too long. Keep them under 20,000 characters.".to_string());
    }
    let (provider, model, key) = {
        let data = shared().data.read().unwrap();
        let provider = data.settings.ai_provider.clone();
        if !PROVIDERS.contains(&provider.as_str()) {
            return Err("Turn on AI generation and add an API key in Settings → AI generation.".to_string());
        }
        let Some(key) = data.ai_keys.get(&provider).cloned() else {
            return Err(format!("Add your {} API key in Settings → AI generation.", display_name(&provider)));
        };
        let model = if data.settings.ai_model.is_empty() { default_model(&provider).to_string() } else { data.settings.ai_model.clone() };
        (provider, model, key)
    };
    let request = build_request(&provider, &model, &key, &directions)?;
    send(request, &provider).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{api_error, build_request, default_model, extract_text, key_page, PROVIDERS};

    #[test]
    fn every_provider_has_a_secure_key_page() {
        for provider in PROVIDERS {
            assert!(key_page(provider).unwrap().starts_with("https://"));
        }
        assert_eq!(key_page("https://example.com"), None);
    }

    #[test]
    fn every_provider_has_a_default_model_and_builds_a_request() {
        for provider in PROVIDERS {
            let request = build_request(provider, default_model(provider), "KEY", "write hi").unwrap();
            assert!(request.url.starts_with("https://"));
            assert!(request.headers.iter().any(|(_, v)| v.contains("KEY")));
            assert!(!request.url.contains("KEY"), "keys stay out of URLs");
            assert!(!request.body.to_string().contains("KEY"), "keys stay out of bodies");
            assert!(request.body.to_string().contains("write hi"));
        }
    }

    #[test]
    fn claude_requests_match_the_model() {
        let current = build_request("claude", "claude-opus-5-5", "k", "x").unwrap();
        assert_eq!(current.body["fallbacks"], "default");
        assert_eq!(current.body["output_config"]["effort"], "low");
        assert!(current.headers.iter().any(|(n, _)| *n == "anthropic-beta"));
        assert!(current.body.get("temperature").is_none());
        let other = build_request("claude", "claude-haiku-5-5", "k", "x").unwrap();
        assert!(other.body.get("fallbacks").is_none());
        assert!(!other.headers.iter().any(|(n, _)| *n == "anthropic-beta"));
    }

    #[test]
    fn groq_model_names_can_have_a_slash() {
        let request = build_request("groq", "openai/gpt-oss-120b", "k", "x").unwrap();
        assert_eq!(request.url, "https://api.groq.com/openai/v1/chat/completions");
        assert_eq!(extract_text("groq", &json!({ "choices": [{ "message": { "content": "Hi" } }] })).unwrap(), "Hi");
    }

    #[test]
    fn odd_model_names_and_providers_are_rejected() {
        assert!(build_request("gemini", "a/../b", "k", "x").is_err());
        assert!(build_request("gemini", "", "k", "x").is_err());
        assert!(build_request("other", "m", "k", "x").is_err());
    }

    #[test]
    fn text_is_read_from_each_providers_reply() {
        let claude = json!({ "stop_reason": "end_turn", "content": [{ "type": "thinking", "thinking": "" }, { "type": "text", "text": " Hello " }] });
        assert_eq!(extract_text("claude", &claude).unwrap(), "Hello");
        let openai = json!({ "choices": [{ "message": { "content": "Hi", "refusal": null } }] });
        assert_eq!(extract_text("chatgpt", &openai).unwrap(), "Hi");
        assert_eq!(extract_text("grok", &openai).unwrap(), "Hi");
        let gemini = json!({ "candidates": [{ "content": { "parts": [{ "text": "plan", "thought": true }, { "text": "Hey" }] } }] });
        assert_eq!(extract_text("gemini", &gemini).unwrap(), "Hey");
    }

    #[test]
    fn refusals_and_empty_replies_are_errors() {
        assert!(extract_text("claude", &json!({ "stop_reason": "refusal", "content": [] })).is_err());
        assert!(extract_text("chatgpt", &json!({ "choices": [{ "message": { "content": null, "refusal": "no" } }] })).is_err());
        assert!(extract_text("gemini", &json!({ "promptFeedback": { "blockReason": "SAFETY" } })).is_err());
        assert!(extract_text("claude", &json!({ "content": [] })).is_err());
        assert!(extract_text("grok", &json!({})).is_err());
    }

    #[test]
    fn errors_are_readable() {
        assert!(api_error("claude", 401, &json!({})).contains("API key"));
        assert!(api_error("gemini", 429, &json!({ "error": { "message": "quota" } })).contains("quota"));
        assert_eq!(api_error("grok", 400, &json!({ "error": { "message": "bad model" } })), "Grok: bad model");
        assert!(api_error("chatgpt", 500, &serde_json::Value::Null).contains("500"));
    }
}
