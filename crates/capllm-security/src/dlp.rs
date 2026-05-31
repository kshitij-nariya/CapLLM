//! Inline Data Loss Prevention (DLP) masking engine.
//!
//! Uses `regex::RegexSet` for O(n) multi-pattern scanning across 7 PII
//! categories. Detected sensitive data is replaced with unique placeholders,
//! and originals are stored in the encrypted [`SensitiveVault`].

use capllm_core::ChatCompletionRequest;
use regex::Regex;

use crate::vault::SensitiveVault;



/// Pattern category labels for placeholder generation.
const LABELS: [&str; 7] = [
    "SSN", "CC", "AWSKEY", "APIKEY", "PRIVKEY", "EMAIL", "PHONE",
];

/// Compiled DLP pattern-matching engine.
///
/// Regex patterns are compiled once at startup and reused for all requests.
pub struct DlpEngine {
    patterns: Vec<Regex>,
}

impl DlpEngine {
    /// Compile all DLP patterns. Call once at startup.
    #[must_use]
    pub fn new() -> Self {
        let raw_patterns = [
            r"\b\d{3}-\d{2}-\d{4}\b",                                      // SSN
            r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b",               // Credit Card
            r"AKIA[0-9A-Z]{16}",                                           // AWS Access Key
            r"sk-[a-zA-Z0-9]{20,}",                                        // Generic API Key
            r"-----BEGIN\s(?:RSA\s|EC\s|DSA\s)?PRIVATE\sKEY-----[\s\S]*?-----END\s(?:RSA\s|EC\s|DSA\s)?PRIVATE\sKEY-----", // PEM
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b",       // Email
            r"\b(?:\+1[\s-]?)?\(?\d{3}\)?[\s.-]\d{3}[\s.-]\d{4}\b",      // US Phone
        ];

        let patterns = raw_patterns
            .iter()
            .map(|p| Regex::new(p).expect("invalid DLP regex"))
            .collect();

        Self { patterns }
    }

    /// Scan all messages in a request and mask sensitive data.
    ///
    /// Returns a new request with redacted content. Original values are stored
    /// in the provided vault, encrypted with AES-256-GCM.
    pub fn scan_and_mask(
        &self,
        request: &ChatCompletionRequest,
        vault: &SensitiveVault,
    ) -> ChatCompletionRequest {
        let mut masked = request.clone();
        for msg in &mut masked.messages {
            msg.content = self.mask_text(&msg.content, vault);
        }
        masked
    }

    /// Mask sensitive patterns in a single text string.
    fn mask_text(&self, text: &str, vault: &SensitiveVault) -> String {
        let mut result = text.to_owned();

        // Apply each pattern in order (SSN first, then CC, etc.)
        for (idx, pattern) in self.patterns.iter().enumerate() {
            let label = LABELS[idx];

            // Collect matches first to avoid borrow issues
            let matches: Vec<String> = pattern
                .find_iter(&result)
                .map(|m| m.as_str().to_owned())
                .collect();

            for matched in matches {
                let id = short_id();
                let placeholder = format!("[REDACTED-{label}-{id}]");

                vault.store(&id, &matched);
                result = result.replacen(&matched, &placeholder, 1);

                tracing::info!(
                    category = label,
                    placeholder = %placeholder,
                    "DLP: masked sensitive data"
                );
            }
        }

        result
    }

    /// Re-hydrate placeholders in response text with original values from vault.
    ///
    /// Scans for `[REDACTED-*-{id}]` patterns and replaces them with the
    /// decrypted originals.
    pub fn rehydrate(text: &str, vault: &SensitiveVault) -> String {
        let placeholder_re =
            Regex::new(r"\[REDACTED-[A-Z]+-([a-f0-9]{8})\]").expect("invalid rehydrate regex");

        let mut result = text.to_owned();
        let captures: Vec<(String, String)> = placeholder_re
            .captures_iter(text)
            .filter_map(|cap| {
                let full = cap.get(0)?.as_str().to_owned();
                let id = cap.get(1)?.as_str().to_owned();
                Some((full, id))
            })
            .collect();

        for (placeholder, id) in captures {
            if let Some(original) = vault.retrieve(&id) {
                result = result.replacen(&placeholder, &original, 1);
                tracing::debug!(id, "DLP: re-hydrated placeholder");
            }
        }

        result
    }

    /// Check if any message content contains sensitive data (for auditing).
    pub fn has_sensitive_data(&self, request: &ChatCompletionRequest) -> bool {
        request
            .messages
            .iter()
            .any(|msg| self.patterns.iter().any(|p| p.is_match(&msg.content)))
    }
}

impl Default for DlpEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DlpEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DlpEngine")
            .field("patterns", &self.patterns.len())
            .finish()
    }
}

/// Generate a short hex ID for placeholders.
fn short_id() -> String {
    let id = uuid::Uuid::new_v4();
    // Take first 8 hex chars for short IDs
    id.simple().to_string()[..8].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use capllm_core::{ChatMessage, ChatRole};

    fn make_request(content: &str) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "test".to_owned(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: content.to_owned(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
        }
    }

    #[test]
    fn masks_ssn() {
        let engine = DlpEngine::new();
        let vault = SensitiveVault::new();
        let req = make_request("My SSN is 123-45-6789, please help");

        let masked = engine.scan_and_mask(&req, &vault);
        assert!(!masked.messages[0].content.contains("123-45-6789"));
        assert!(masked.messages[0].content.contains("[REDACTED-SSN-"));
        assert!(masked.messages[0].content.contains("please help"));
    }

    #[test]
    fn masks_credit_card() {
        let engine = DlpEngine::new();
        let vault = SensitiveVault::new();
        let req = make_request("Card: 4111-1111-1111-1111");

        let masked = engine.scan_and_mask(&req, &vault);
        assert!(!masked.messages[0].content.contains("4111"));
        assert!(masked.messages[0].content.contains("[REDACTED-CC-"));
    }

    #[test]
    fn masks_aws_key() {
        let engine = DlpEngine::new();
        let vault = SensitiveVault::new();
        let req = make_request("AWS key: AKIAIOSFODNN7EXAMPLE");

        let masked = engine.scan_and_mask(&req, &vault);
        assert!(!masked.messages[0].content.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(masked.messages[0].content.contains("[REDACTED-AWSKEY-"));
    }

    #[test]
    fn masks_api_key() {
        let engine = DlpEngine::new();
        let vault = SensitiveVault::new();
        let req = make_request("My key is sk-abcdefghij1234567890extra");

        let masked = engine.scan_and_mask(&req, &vault);
        assert!(!masked.messages[0].content.contains("sk-abcdefghij"));
        assert!(masked.messages[0].content.contains("[REDACTED-APIKEY-"));
    }

    #[test]
    fn masks_pem_private_key() {
        let engine = DlpEngine::new();
        let vault = SensitiveVault::new();
        let pem = "Here is my key:\n-----BEGIN RSA PRIVATE KEY-----\nMIIBogIBAAJBALRiMLAHudeSA/x3hB2f+2NRkJLA\n-----END RSA PRIVATE KEY-----\nDone.";
        let req = make_request(pem);

        let masked = engine.scan_and_mask(&req, &vault);
        assert!(!masked.messages[0].content.contains("BEGIN RSA PRIVATE KEY"));
        assert!(masked.messages[0].content.contains("[REDACTED-PRIVKEY-"));
        assert!(masked.messages[0].content.contains("Done."));
    }

    #[test]
    fn masks_email() {
        let engine = DlpEngine::new();
        let vault = SensitiveVault::new();
        let req = make_request("Contact user@example.com for details");

        let masked = engine.scan_and_mask(&req, &vault);
        assert!(!masked.messages[0].content.contains("user@example.com"));
        assert!(masked.messages[0].content.contains("[REDACTED-EMAIL-"));
    }

    #[test]
    fn masks_phone() {
        let engine = DlpEngine::new();
        let vault = SensitiveVault::new();
        let req = make_request("Call me at (555) 123-4567");

        let masked = engine.scan_and_mask(&req, &vault);
        assert!(!masked.messages[0].content.contains("(555) 123-4567"));
        assert!(masked.messages[0].content.contains("[REDACTED-PHONE-"));
    }

    #[test]
    fn clean_text_unchanged() {
        let engine = DlpEngine::new();
        let vault = SensitiveVault::new();
        let req = make_request("What is the weather like today?");

        let masked = engine.scan_and_mask(&req, &vault);
        assert_eq!(
            masked.messages[0].content,
            "What is the weather like today?"
        );
        assert!(vault.is_empty());
    }

    #[test]
    fn rehydration_restores_originals() {
        let engine = DlpEngine::new();
        let vault = SensitiveVault::new();
        let req = make_request("My SSN is 123-45-6789");

        let masked = engine.scan_and_mask(&req, &vault);
        assert!(masked.messages[0].content.contains("[REDACTED-SSN-"));

        // Simulate LLM echoing the placeholder back
        let response_text = format!(
            "Your SSN {} has been noted.",
            &masked.messages[0].content[10..] // Extract the redacted part
        );

        // Re-hydrate using the vault
        let rehydrated = DlpEngine::rehydrate(&response_text, &vault);
        assert!(rehydrated.contains("123-45-6789"));
    }

    #[test]
    fn multiple_patterns_in_single_message() {
        let engine = DlpEngine::new();
        let vault = SensitiveVault::new();
        let req = make_request(
            "SSN: 123-45-6789, email: test@corp.com, card: 4111 1111 1111 1111",
        );

        let masked = engine.scan_and_mask(&req, &vault);
        let content = &masked.messages[0].content;
        assert!(content.contains("[REDACTED-SSN-"));
        assert!(content.contains("[REDACTED-EMAIL-"));
        assert!(content.contains("[REDACTED-CC-"));
        assert!(!content.contains("123-45-6789"));
        assert!(!content.contains("test@corp.com"));
        assert!(!content.contains("4111"));
    }

    #[test]
    fn has_sensitive_data_detection() {
        let engine = DlpEngine::new();
        assert!(engine.has_sensitive_data(&make_request("SSN: 123-45-6789")));
        assert!(!engine.has_sensitive_data(&make_request("Hello world")));
    }
}
