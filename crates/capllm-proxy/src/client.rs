//! Connection-pooled HTTP client for forwarding requests to upstream providers.

use std::time::Duration;

use axum::http::HeaderMap;
use capllm_core::GatewayError;

/// A thin wrapper around [`reqwest::Client`] pre-configured for low-latency
/// connection-pooled forwarding.
#[derive(Debug, Clone)]
pub struct ProxyClient {
    inner: reqwest::Client,
}

impl ProxyClient {
    /// Create a new client with aggressive connection pooling.
    pub fn new() -> Result<Self, GatewayError> {
        let inner = reqwest::Client::builder()
            .pool_max_idle_per_host(32)
            .pool_idle_timeout(Duration::from_secs(90))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300)) // 5 min for long generations
            .tcp_nodelay(true) // Disable Nagle for latency
            .build()
            .map_err(|e| GatewayError::ConfigError(e.to_string()))?;

        Ok(Self { inner })
    }

    /// Forward a request and return the raw streaming response.
    ///
    /// The caller is responsible for consuming the byte stream.
    pub async fn forward_streaming(
        &self,
        url: &str,
        body: &serde_json::Value,
        headers: HeaderMap,
    ) -> Result<reqwest::Response, GatewayError> {
        let response = self
            .inner
            .post(url)
            .headers(headers)
            .json(body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(GatewayError::UpstreamError(format!(
                "HTTP {status}: {error_body}"
            )));
        }

        Ok(response)
    }

    /// Forward a request and return the full response body as text.
    pub async fn forward(
        &self,
        url: &str,
        body: &serde_json::Value,
        headers: HeaderMap,
    ) -> Result<String, GatewayError> {
        let response = self
            .inner
            .post(url)
            .headers(headers)
            .json(body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(GatewayError::UpstreamError(format!(
                "HTTP {status}: {text}"
            )));
        }

        Ok(text)
    }
}

impl Default for ProxyClient {
    fn default() -> Self {
        Self::new().expect("failed to create default ProxyClient")
    }
}
