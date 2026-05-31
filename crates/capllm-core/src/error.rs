use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// All error variants that can occur within the gateway pipeline.
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("invalid provider `{0}` — expected `anthropic` or `gemini`")]
    InvalidProvider(String),

    #[error("payload translation failed: {0}")]
    TranslationError(String),

    #[error("upstream provider returned an error: {0}")]
    UpstreamError(String),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("missing required header: {0}")]
    MissingHeader(String),

    #[error("SSE stream error: {0}")]
    StreamError(String),

    #[error("configuration error: {0}")]
    ConfigError(String),

    #[error("rate limited: team `{team}` used {current} tokens, limit is {limit} TPM")]
    RateLimited {
        team: String,
        limit: u64,
        current: u64,
    },

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("redis error: {0}")]
    RedisError(String),

    #[error("prompt injection detected: {0}")]
    PromptInjection(String),

    #[error("DLP violation: {0}")]
    DlpViolation(String),

    #[error("agentic loop detected: {0}")]
    LoopDetected(String),
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::InvalidProvider(_) | Self::MissingHeader(_) | Self::DlpViolation(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            Self::TranslationError(_) | Self::Serde(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
            Self::UpstreamError(_) | Self::HttpClient(_) | Self::StreamError(_) => {
                (StatusCode::BAD_GATEWAY, self.to_string())
            }
            Self::ConfigError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            Self::RateLimited { .. } | Self::LoopDetected(_) => {
                (StatusCode::TOO_MANY_REQUESTS, self.to_string())
            }
            Self::Unauthorized(_) => {
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            Self::RedisError(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
            }
            Self::PromptInjection(_) => {
                (StatusCode::FORBIDDEN, self.to_string())
            }

        };

        let body = json!({
            "error": {
                "message": message,
                "type": "gateway_error",
                "code": status.as_u16(),
            }
        });

        (status, axum::Json(body)).into_response()
    }
}
