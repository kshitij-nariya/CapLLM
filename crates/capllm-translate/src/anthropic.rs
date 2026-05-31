//! Anthropic Messages API translation.
//!
//! Reference: <https://docs.anthropic.com/en/api/messages>

use axum::http::{HeaderMap, HeaderValue};
use capllm_core::types::{gen_completion_id, unix_timestamp};
use capllm_core::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChatRole,
    Choice, ChunkChoice, Delta, GatewayError, Usage,
};
use serde::{Deserialize, Serialize};

// ─── Anthropic Native Types ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub messages: Vec<AnthropicMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

// ─── Anthropic SSE event types (deserialization) ─────────────────────────────

#[derive(Debug, Deserialize)]
struct ContentBlockDelta {
    delta: TextDelta,
}

#[derive(Debug, Deserialize)]
struct TextDelta {
    #[serde(rename = "type")]
    _type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct MessageDelta {
    delta: MessageDeltaInner,
    #[allow(dead_code)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct MessageDeltaInner {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[allow(dead_code)]
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AnthropicFullResponse {
    content: Vec<AnthropicContentBlock>,
    #[allow(dead_code)]
    model: String,
    stop_reason: Option<String>,
    usage: AnthropicFullUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    _type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicFullUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// ─── Request Translation ─────────────────────────────────────────────────────

pub fn translate_request(
    req: &ChatCompletionRequest,
    api_key: &str,
    base_url: &str,
) -> Result<(String, serde_json::Value, HeaderMap), GatewayError> {
    let mut system: Option<String> = None;
    let mut messages = Vec::with_capacity(req.messages.len());

    for msg in &req.messages {
        match msg.role {
            ChatRole::System => {
                system = Some(msg.content.clone());
            }
            ChatRole::User => messages.push(AnthropicMessage {
                role: "user".to_owned(),
                content: msg.content.clone(),
            }),
            ChatRole::Assistant => messages.push(AnthropicMessage {
                role: "assistant".to_owned(),
                content: msg.content.clone(),
            }),
        }
    }

    let anthropic_req = AnthropicRequest {
        model: req.model.clone(),
        max_tokens: req.max_tokens.unwrap_or(4096),
        system,
        messages,
        stream: req.stream,
        temperature: req.temperature,
        top_p: req.top_p,
        stop_sequences: req.stop.clone(),
    };

    let url = format!("{base_url}/v1/messages");
    let body = serde_json::to_value(&anthropic_req)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key).map_err(|e| GatewayError::ConfigError(e.to_string()))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );
    headers.insert("content-type", HeaderValue::from_static("application/json"));

    Ok((url, body, headers))
}

// ─── Streaming Event Translation ─────────────────────────────────────────────

pub fn translate_sse_event(
    event_type: &str,
    data: &str,
    completion_id: &str,
    model: &str,
) -> Result<Option<ChatCompletionChunk>, GatewayError> {
    match event_type {
        "content_block_delta" => {
            let parsed: ContentBlockDelta =
                serde_json::from_str(data).map_err(|e| GatewayError::TranslationError(e.to_string()))?;

            Ok(Some(ChatCompletionChunk {
                id: completion_id.to_owned(),
                object: "chat.completion.chunk".to_owned(),
                created: unix_timestamp(),
                model: model.to_owned(),
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: Delta {
                        role: None,
                        content: Some(parsed.delta.text),
                    },
                    finish_reason: None,
                }],
            }))
        }
        "message_delta" => {
            let parsed: MessageDelta =
                serde_json::from_str(data).map_err(|e| GatewayError::TranslationError(e.to_string()))?;

            let finish = parsed.delta.stop_reason.as_deref().map(|r| {
                if r == "end_turn" { "stop" } else { r }
            });

            Ok(Some(ChatCompletionChunk {
                id: completion_id.to_owned(),
                object: "chat.completion.chunk".to_owned(),
                created: unix_timestamp(),
                model: model.to_owned(),
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: Delta {
                        role: None,
                        content: None,
                    },
                    finish_reason: finish.map(ToOwned::to_owned),
                }],
            }))
        }
        // Silently skip non-content events
        _ => Ok(None),
    }
}

// ─── Non-Streaming Response Translation ──────────────────────────────────────

pub fn translate_response(
    body: &str,
    model: &str,
) -> Result<ChatCompletionResponse, GatewayError> {
    let resp: AnthropicFullResponse =
        serde_json::from_str(body).map_err(|e| GatewayError::TranslationError(e.to_string()))?;

    let content: String = resp
        .content
        .into_iter()
        .map(|b| b.text)
        .collect();

    let finish_reason = resp
        .stop_reason
        .map(|r| if r == "end_turn" { "stop".to_owned() } else { r });

    Ok(ChatCompletionResponse {
        id: gen_completion_id(),
        object: "chat.completion".to_owned(),
        created: unix_timestamp(),
        model: model.to_owned(),
        choices: vec![Choice {
            index: 0,
            message: ChatMessage {
                role: ChatRole::Assistant,
                content,
            },
            finish_reason,
        }],
        usage: Some(Usage {
            prompt_tokens: resp.usage.input_tokens,
            completion_tokens: resp.usage.output_tokens,
            total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
        }),
    })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_openai_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "claude-3-5-sonnet-20241022".to_owned(),
            messages: vec![
                ChatMessage { role: ChatRole::System, content: "Be concise.".to_owned() },
                ChatMessage { role: ChatRole::User, content: "Hello".to_owned() },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1024),
            stream: true,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
        }
    }

    #[test]
    fn request_extracts_system_message() {
        let req = sample_openai_request();
        let (url, body, headers) =
            translate_request(&req, "sk-test-key", "https://api.anthropic.com").unwrap();

        assert_eq!(url, "https://api.anthropic.com/v1/messages");
        assert_eq!(body["system"], "Be concise.");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert!(headers.contains_key("x-api-key"));
    }

    #[test]
    fn translate_content_block_delta() {
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi!"}}"#;
        let chunk = translate_sse_event("content_block_delta", data, "test-id", "claude").unwrap();
        assert!(chunk.is_some());
        let chunk = chunk.unwrap();
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hi!"));
    }

    #[test]
    fn translate_full_response() {
        let body = r#"{
            "content": [{"type":"text","text":"Hello there!"}],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        }"#;
        let resp = translate_response(body, "claude-3-5-sonnet").unwrap();
        assert_eq!(resp.choices[0].message.content, "Hello there!");
        assert_eq!(resp.choices[0].finish_reason.as_deref(), Some("stop"));
        assert_eq!(resp.usage.as_ref().unwrap().total_tokens, 15);
    }
}
