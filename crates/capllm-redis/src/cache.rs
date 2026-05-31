//! Exact-match query cache using SHA-256 payload hashing.
//!
//! Before forwarding a request to an upstream provider, the gateway hashes the
//! canonical payload. If an identical hash exists in Redis with a valid TTL,
//! the cached response is returned instantly for $0 token cost.

use capllm_core::{ChatCompletionRequest, GatewayError};
use fred::prelude::*;
use sha2::{Digest, Sha256};

use crate::pool::RedisPool;

/// Default cache TTL in seconds (5 minutes).
const DEFAULT_TTL_SECS: i64 = 300;

/// Exact-match query cache.
pub struct QueryCache;

impl QueryCache {
    /// Compute a cache key by SHA-256 hashing the canonicalized request.
    ///
    /// The request is serialized with sorted keys and no whitespace to ensure
    /// deterministic hashing regardless of field ordering.
    pub fn cache_key(request: &ChatCompletionRequest) -> String {
        // Serialize to Value first (sorts keys in serde_json), then to string
        let canonical = serde_json::to_string(request).unwrap_or_default();
        let hash = Sha256::digest(canonical.as_bytes());
        format!("cache:{hash:x}")
    }

    /// Check if a cached response exists for the given cache key.
    pub async fn get(pool: &RedisPool, key: &str) -> Result<Option<String>, GatewayError> {
        let result: Option<String> = pool
            .client()
            .get(key)
            .await
            .map_err(|e| GatewayError::RedisError(e.to_string()))?;

        if result.is_some() {
            tracing::info!(key, "cache HIT");
        }

        Ok(result)
    }

    /// Store a response in the cache with a TTL.
    pub async fn set(
        pool: &RedisPool,
        key: &str,
        response: &str,
        ttl_secs: Option<i64>,
    ) -> Result<(), GatewayError> {
        let ttl = Expiration::EX(ttl_secs.unwrap_or(DEFAULT_TTL_SECS));
        pool.client()
            .set::<(), _, _>(key, response, Some(ttl), None, false)
            .await
            .map_err(|e| GatewayError::RedisError(e.to_string()))?;

        tracing::debug!(key, "cache SET");
        Ok(())
    }
}
