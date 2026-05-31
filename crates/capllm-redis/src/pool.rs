//! Async Redis connection pool wrapper around `fred`.

use capllm_core::GatewayError;
use fred::prelude::*;

/// Async Redis connection pool.
#[derive(Clone)]
pub struct RedisPool {
    client: Client,
}

impl RedisPool {
    /// Connect to Redis at the given URL.
    pub async fn connect(url: &str) -> Result<Self, GatewayError> {
        let config = Config::from_url(url)
            .map_err(|e| GatewayError::RedisError(format!("invalid redis url: {e}")))?;

        let client = Builder::from_config(config)
            .build()
            .map_err(|e| GatewayError::RedisError(e.to_string()))?;

        client
            .init()
            .await
            .map_err(|e| GatewayError::RedisError(format!("redis connect failed: {e}")))?;

        tracing::info!("connected to Redis");
        Ok(Self { client })
    }

    /// Get a reference to the underlying fred client.
    pub const fn client(&self) -> &Client {
        &self.client
    }

    /// Health check — PING the server.
    pub async fn ping(&self) -> Result<(), GatewayError> {
        let _: String = self
            .client
            .ping(None)
            .await
            .map_err(|e| GatewayError::RedisError(e.to_string()))?;
        Ok(())
    }
}

impl std::fmt::Debug for RedisPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisPool").finish()
    }
}
