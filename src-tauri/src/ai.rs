//! Optional Claude-powered advisor. Raw HTTP to the Messages API (there is no
//! official Rust SDK). The user's financial snapshot rides in the system prompt.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMsg {
    /// "user" | "assistant"
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatReply {
    pub text: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
}

const INSTRUCTIONS: &str = "You are the advisor inside FinCompass, a personal-finance desktop app used by one person for their own money. \
You have their complete financial snapshot below (all amounts in their currency). \
Give specific, numeric, prioritised guidance grounded in that data: name the accounts, cite the figures, and quantify trade-offs. \
Follow the standard priority order unless the numbers argue otherwise: minimum payments, small emergency cushion, employer match, high-interest debt, full emergency fund, tax-advantaged investing, taxable investing, goals. \
Be direct and concise; use short paragraphs or bullets, no headers unless the answer is long. \
When the user asks something the snapshot can't answer, say what's missing and ask one clarifying question. \
You are educational, not a licensed fiduciary; say so once if the user asks for something like a specific security to buy, and never invent market data.";

pub async fn ask(api_key: &str, model: &str, snapshot_json: &str, history: &[ChatMsg]) -> Result<ChatReply, String> {
    if api_key.trim().is_empty() {
        return Err("No API key set. Add an Anthropic API key in Settings.".into());
    }
    if history.is_empty() || history.last().map(|m| m.role.as_str()) != Some("user") {
        return Err("Conversation must end with a user message.".into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;

    let messages: Vec<serde_json::Value> = history
        .iter()
        .map(|m| json!({ "role": m.role, "content": m.content }))
        .collect();

    let body = json!({
        "model": model,
        "max_tokens": 8000,
        "system": [
            { "type": "text", "text": INSTRUCTIONS },
            {
                "type": "text",
                "text": format!("Current financial snapshot (JSON):\n{}", snapshot_json),
                "cache_control": { "type": "ephemeral" }
            }
        ],
        "messages": messages
    });

    let resp = client
        .post(API_URL)
        .header("content-type", "application/json")
        .header("x-api-key", api_key.trim())
        .header("anthropic-version", API_VERSION)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    let val: serde_json::Value = resp.json().await.map_err(|e| format!("Bad response: {e}"))?;
    if !status.is_success() {
        let msg = val
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("API error {}: {}", status.as_u16(), msg));
    }

    let stop = val.get("stop_reason").and_then(|v| v.as_str()).unwrap_or("");
    if stop == "refusal" {
        let why = val
            .pointer("/stop_details/explanation")
            .and_then(|v| v.as_str())
            .unwrap_or("the request was declined by the model's safety system");
        return Err(format!("The advisor declined to answer: {why}"));
    }

    let mut text = String::new();
    if let Some(blocks) = val.get("content").and_then(|c| c.as_array()) {
        for b in blocks {
            if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                    if !text.is_empty() {
                        text.push_str("\n\n");
                    }
                    text.push_str(t);
                }
            }
        }
    }
    if stop == "max_tokens" {
        text.push_str("\n\n[Response was cut off at the token limit.]");
    }
    let usage = val.get("usage").cloned().unwrap_or(json!({}));
    let u = |k: &str| usage.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
    Ok(ChatReply {
        text,
        model: val.get("model").and_then(|m| m.as_str()).unwrap_or(model).to_string(),
        input_tokens: u("input_tokens"),
        output_tokens: u("output_tokens"),
        cache_read_tokens: u("cache_read_input_tokens"),
    })
}
