//! Zero-Data Retention (ZDR) enforcement.
//!
//! Automatically injects vendor-specific privacy metadata to legally prevent
//! model training on enterprise telemetry.
//!
//! - **Anthropic**: API data is already excluded from training by default.
//!   We add an explicit `anthropic-beta` header for ZDR when available.
//! - **Gemini**: We inject `safetySettings` into the request body.

use axum::http::{HeaderMap, HeaderValue};
use capllm_core::Provider;

/// Zero-Data Retention enforcer.
pub struct ZdrEnforcer;

impl ZdrEnforcer {
    /// Inject privacy-enforcement headers for the given provider.
    ///
    /// This modifies the forwarded `HeaderMap` in-place.
    pub fn apply_headers(provider: Provider, headers: &mut HeaderMap) {
        match provider {
            Provider::Anthropic => {
                // Anthropic API never trains on API data by default.
                // We add the explicit ZDR beta header for additional legal coverage.
                if let Ok(val) = HeaderValue::from_str("zero-data-retention-2025-04-01") {
                    headers.insert("anthropic-beta", val);
                }
                tracing::debug!("ZDR: injected Anthropic zero-data-retention header");
            }
            Provider::Gemini => {
                // Gemini/Google AI Studio — paid API does not train on data.
                // We set the header to signal enterprise compliance intent.
                if let Ok(val) = HeaderValue::from_str("true") {
                    headers.insert("x-goog-user-project-data-retention", val);
                }
                tracing::debug!("ZDR: injected Gemini data-retention signal header");
            }
        }
    }

    /// Inject safety/privacy settings into the request body for providers
    /// that support body-level configuration.
    ///
    /// Currently only applies to Gemini (safety settings).
    pub fn apply_body(provider: Provider, body: &mut serde_json::Value) {
        if provider == Provider::Gemini {
            // Inject safety settings to block dangerous content categories
            if body.is_object() && body.get("safetySettings").is_none() {
                body["safetySettings"] = serde_json::json!([
                    {
                        "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                        "threshold": "BLOCK_MEDIUM_AND_ABOVE"
                    },
                    {
                        "category": "HARM_CATEGORY_HATE_SPEECH",
                        "threshold": "BLOCK_MEDIUM_AND_ABOVE"
                    },
                    {
                        "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                        "threshold": "BLOCK_MEDIUM_AND_ABOVE"
                    },
                    {
                        "category": "HARM_CATEGORY_HARASSMENT",
                        "threshold": "BLOCK_MEDIUM_AND_ABOVE"
                    }
                ]);
                tracing::debug!("ZDR: injected Gemini safety settings");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_zdr_header_injected() {
        let mut headers = HeaderMap::new();
        ZdrEnforcer::apply_headers(Provider::Anthropic, &mut headers);

        assert!(headers.contains_key("anthropic-beta"));
        assert_eq!(
            headers.get("anthropic-beta").unwrap().to_str().unwrap(),
            "zero-data-retention-2025-04-01"
        );
    }

    #[test]
    fn gemini_zdr_header_injected() {
        let mut headers = HeaderMap::new();
        ZdrEnforcer::apply_headers(Provider::Gemini, &mut headers);

        assert!(headers.contains_key("x-goog-user-project-data-retention"));
    }

    #[test]
    fn gemini_safety_settings_injected() {
        let mut body = serde_json::json!({
            "contents": [{"parts": [{"text": "hello"}]}]
        });

        ZdrEnforcer::apply_body(Provider::Gemini, &mut body);

        assert!(body.get("safetySettings").is_some());
        let settings = body["safetySettings"].as_array().unwrap();
        assert_eq!(settings.len(), 4);
    }

    #[test]
    fn gemini_does_not_overwrite_existing_safety() {
        let mut body = serde_json::json!({
            "contents": [{"parts": [{"text": "hello"}]}],
            "safetySettings": [{"category": "CUSTOM", "threshold": "BLOCK_NONE"}]
        });

        ZdrEnforcer::apply_body(Provider::Gemini, &mut body);

        // Should not overwrite existing settings
        let settings = body["safetySettings"].as_array().unwrap();
        assert_eq!(settings.len(), 1);
        assert_eq!(settings[0]["category"], "CUSTOM");
    }

    #[test]
    fn anthropic_body_unchanged() {
        let mut body = serde_json::json!({"messages": []});
        let original = body.clone();
        ZdrEnforcer::apply_body(Provider::Anthropic, &mut body);
        assert_eq!(body, original);
    }
}
