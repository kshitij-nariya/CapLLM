pub mod anthropic;
pub mod gemini;

use axum::http::HeaderMap;
use capllm_core::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, GatewayError, Provider,
};

/// Centralised translation dispatch.
///
/// Each method routes to the correct provider-specific translator based on the
/// [`Provider`] discriminant, keeping the call-site in the server handler clean.
pub struct TranslationEngine;

impl TranslationEngine {
    // ── Request Translation ──────────────────────────────────────────────

    /// Translate an OpenAI-format request into the provider's native schema.
    ///
    /// Returns `(url_path, body_json, extra_headers)` ready for forwarding.
    pub fn translate_request(
        provider: Provider,
        req: &ChatCompletionRequest,
        api_key: &str,
        base_url: &str,
    ) -> Result<(String, serde_json::Value, HeaderMap), GatewayError> {
        match provider {
            Provider::Anthropic => anthropic::translate_request(req, api_key, base_url),
            Provider::Gemini => gemini::translate_request(req, api_key, base_url),
        }
    }

    // ── Streaming SSE Event Translation ──────────────────────────────────

    /// Translate a single raw SSE `data:` payload from the provider's format
    /// into an OpenAI-compatible [`ChatCompletionChunk`].
    ///
    /// Returns `None` for events that should be silently skipped (e.g.
    /// Anthropic's `message_start` which has no content delta).
    pub fn translate_sse_event(
        provider: Provider,
        event_type: &str,
        data: &str,
        completion_id: &str,
        model: &str,
    ) -> Result<Option<ChatCompletionChunk>, GatewayError> {
        match provider {
            Provider::Anthropic => {
                anthropic::translate_sse_event(event_type, data, completion_id, model)
            }
            Provider::Gemini => gemini::translate_sse_event(data, completion_id, model),
        }
    }

    // ── Non-Streaming Response Translation ───────────────────────────────

    /// Translate a full (non-streaming) provider response into the `OpenAI`
    /// canonical [`ChatCompletionResponse`].
    pub fn translate_response(
        provider: Provider,
        body: &str,
        model: &str,
    ) -> Result<ChatCompletionResponse, GatewayError> {
        match provider {
            Provider::Anthropic => anthropic::translate_response(body, model),
            Provider::Gemini => gemini::translate_response(body, model),
        }
    }

    /// Check whether a raw SSE data payload signals end-of-stream.
    pub fn is_stream_done(provider: Provider, event_type: &str, data: &str) -> bool {
        match provider {
            Provider::Anthropic => event_type == "message_stop",
            Provider::Gemini => {
                // Gemini signals done via finishReason in the data payload
                data.contains("\"finishReason\"")
                    && (data.contains("\"STOP\"") || data.contains("\"MAX_TOKENS\""))
            }
        }
    }
}
