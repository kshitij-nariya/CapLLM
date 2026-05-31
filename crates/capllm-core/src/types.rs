use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::GatewayError;

// ─── Provider ────────────────────────────────────────────────────────────────

/// Supported LLM provider backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    Anthropic,
    Gemini,
}

impl FromStr for Provider {
    type Err = GatewayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "anthropic" => Ok(Self::Anthropic),
            "gemini" => Ok(Self::Gemini),
            other => Err(GatewayError::InvalidProvider(other.to_owned())),
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Anthropic => write!(f, "anthropic"),
            Self::Gemini => write!(f, "gemini"),
        }
    }
}

// ─── OpenAI Canonical Request ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

// ─── OpenAI Canonical Response (non-streaming) ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ─── OpenAI Canonical Streaming Chunk ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<ChatRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

// ─── Tenant / Virtual Key Metadata ───────────────────────────────────────────

/// Tenant metadata resolved from a virtual gateway key (`gw-*`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMeta {
    pub org: String,
    pub department: String,
    pub team: String,
    pub user: String,
    /// The real vendor API key to forward to the upstream provider.
    pub vendor_key: String,
    /// Provider name (anthropic, gemini).
    pub provider: String,
    /// Tokens-per-minute limit for this team.
    pub tpm_limit: u64,
    /// Hard cumulative token spend cap.
    pub spend_cap: u64,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Generate a unique completion ID in the `chatcmpl-*` format.
pub fn gen_completion_id() -> String {
    format!("chatcmpl-{}", uuid::Uuid::new_v4())
}

/// Current unix timestamp.
pub fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_from_str_valid() {
        assert_eq!("anthropic".parse::<Provider>().unwrap(), Provider::Anthropic);
        assert_eq!("Gemini".parse::<Provider>().unwrap(), Provider::Gemini);
        assert_eq!("ANTHROPIC".parse::<Provider>().unwrap(), Provider::Anthropic);
    }

    #[test]
    fn provider_from_str_invalid() {
        assert!("openai".parse::<Provider>().is_err());
    }

    #[test]
    fn deserialize_chat_request() {
        let json = r#"{
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ],
            "stream": true,
            "max_tokens": 1024
        }"#;
        let req: ChatCompletionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.model, "gpt-4");
        assert_eq!(req.messages.len(), 2);
        assert!(req.stream);
        assert_eq!(req.max_tokens, Some(1024));
    }
}
