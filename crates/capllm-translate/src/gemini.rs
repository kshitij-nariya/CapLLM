//! Gemini `GenerateContent` API translation.
//!
//! Reference: <https://ai.google.dev/api/generate-content>

use axum::http::{HeaderMap, HeaderValue};
use capllm_core::types::{gen_completion_id, unix_timestamp};
use capllm_core::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChatRole,
    Choice, ChunkChoice, Delta, GatewayError, Usage,
};
use serde::{Deserialize, Serialize};

// ─── Gemini Native Types ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

// ─── Gemini Response Types (deserialization) ─────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: GeminiContent,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
struct GeminiUsageMetadata {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    total_token_count: Option<u32>,
}

// ─── Request Translation ─────────────────────────────────────────────────────

pub fn translate_request(
    req: &ChatCompletionRequest,
    api_key: &str,
    base_url: &str,
) -> Result<(String, serde_json::Value, HeaderMap), GatewayError> {
    let mut system_instruction: Option<GeminiContent> = None;
    let mut contents = Vec::with_capacity(req.messages.len());

    for msg in &req.messages {
        match msg.role {
            ChatRole::System => {
                system_instruction = Some(GeminiContent {
                    role: None,
                    parts: vec![GeminiPart {
                        text: msg.content.clone(),
                    }],
                });
            }
            ChatRole::User => {
                contents.push(GeminiContent {
                    role: Some("user".to_owned()),
                    parts: vec![GeminiPart {
                        text: msg.content.clone(),
                    }],
                });
            }
            ChatRole::Assistant => {
                contents.push(GeminiContent {
                    role: Some("model".to_owned()),
                    parts: vec![GeminiPart {
                        text: msg.content.clone(),
                    }],
                });
            }
        }
    }

    let generation_config = GeminiGenerationConfig {
        temperature: req.temperature,
        max_output_tokens: req.max_tokens,
        top_p: req.top_p,
        stop_sequences: req.stop.clone(),
    };

    let gemini_req = GeminiRequest {
        contents,
        system_instruction,
        generation_config: Some(generation_config),
    };

    // Gemini model is part of the URL path
    let action = if req.stream {
        "streamGenerateContent?alt=sse"
    } else {
        "generateContent"
    };
    let url = format!(
        "{base_url}/v1beta/models/{}:{action}",
        req.model
    );

    let body = serde_json::to_value(&gemini_req)?;

    let mut headers = HeaderMap::new();
    let key_value = format!("Bearer {api_key}");
    headers.insert(
        "authorization",
        HeaderValue::from_str(&key_value)
            .map_err(|e| GatewayError::ConfigError(e.to_string()))?,
    );
    headers.insert("content-type", HeaderValue::from_static("application/json"));

    Ok((url, body, headers))
}

// ─── Streaming Event Translation ─────────────────────────────────────────────

pub fn translate_sse_event(
    data: &str,
    completion_id: &str,
    model: &str,
) -> Result<Option<ChatCompletionChunk>, GatewayError> {
    let resp: GeminiResponse =
        serde_json::from_str(data).map_err(|e| GatewayError::TranslationError(e.to_string()))?;

    let candidate = resp.candidates.first().ok_or_else(|| {
        GatewayError::TranslationError("no candidates in Gemini response".to_owned())
    })?;

    let text = candidate
        .content
        .parts
        .first()
        .map(|p| p.text.clone())
        .unwrap_or_default();

    let finish_reason = candidate
        .finish_reason
        .as_deref()
        .map(|r| if r == "STOP" { "stop" } else { r })
        .map(ToOwned::to_owned);

    Ok(Some(ChatCompletionChunk {
        id: completion_id.to_owned(),
        object: "chat.completion.chunk".to_owned(),
        created: unix_timestamp(),
        model: model.to_owned(),
        choices: vec![ChunkChoice {
            index: 0,
            delta: Delta {
                role: None,
                content: Some(text),
            },
            finish_reason,
        }],
    }))
}

// ─── Non-Streaming Response Translation ──────────────────────────────────────

pub fn translate_response(
    body: &str,
    model: &str,
) -> Result<ChatCompletionResponse, GatewayError> {
    let resp: GeminiResponse =
        serde_json::from_str(body).map_err(|e| GatewayError::TranslationError(e.to_string()))?;

    let candidate = resp.candidates.first().ok_or_else(|| {
        GatewayError::TranslationError("no candidates in Gemini response".to_owned())
    })?;

    let content: String = candidate
        .content
        .parts
        .iter()
        .map(|p| p.text.as_str())
        .collect();

    let finish_reason = candidate
        .finish_reason
        .as_deref()
        .map(|r| if r == "STOP" { "stop" } else { r })
        .map(ToOwned::to_owned);

    let usage = resp.usage_metadata.map(|u| Usage {
        prompt_tokens: u.prompt_token_count.unwrap_or(0),
        completion_tokens: u.candidates_token_count.unwrap_or(0),
        total_tokens: u.total_token_count.unwrap_or(0),
    });

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
        usage,
    })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_openai_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "gemini-2.0-flash".to_owned(),
            messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: "Be brief.".to_owned(),
                },
                ChatMessage {
                    role: ChatRole::User,
                    content: "Hi".to_owned(),
                },
            ],
            temperature: Some(0.5),
            max_tokens: Some(512),
            stream: false,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
        }
    }

    #[test]
    fn request_builds_correct_url() {
        let req = sample_openai_request();
        let (url, body, _headers) = translate_request(
            &req,
            "test-key",
            "https://generativelanguage.googleapis.com",
        )
        .unwrap();

        assert!(url.contains("gemini-2.0-flash:generateContent"));
        assert!(body["systemInstruction"].is_object());
        assert_eq!(body["contents"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn request_streaming_url() {
        let mut req = sample_openai_request();
        req.stream = true;
        let (url, _body, _headers) = translate_request(
            &req,
            "test-key",
            "https://generativelanguage.googleapis.com",
        )
        .unwrap();

        assert!(url.contains("streamGenerateContent?alt=sse"));
    }

    #[test]
    fn translate_streaming_chunk() {
        let data = r#"{
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Hello!"}]},
                "finishReason": null
            }]
        }"#;
        let chunk = translate_sse_event(data, "test-id", "gemini").unwrap();
        assert!(chunk.is_some());
        assert_eq!(
            chunk.unwrap().choices[0].delta.content.as_deref(),
            Some("Hello!")
        );
    }

    #[test]
    fn translate_full_gemini_response() {
        let body = r#"{
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Hi there!"}]},
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 8,
                "candidatesTokenCount": 3,
                "totalTokenCount": 11
            }
        }"#;
        let resp = translate_response(body, "gemini-2.0-flash").unwrap();
        assert_eq!(resp.choices[0].message.content, "Hi there!");
        assert_eq!(resp.choices[0].finish_reason.as_deref(), Some("stop"));
        assert_eq!(resp.usage.as_ref().unwrap().total_tokens, 11);
    }
}
