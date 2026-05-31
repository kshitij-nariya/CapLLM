//! Agentic Loop-Breaker using Redis.
//!
//! Tracks request velocity and prompt semantic similarity. If a tenant exceeds
//! 15 calls per minute and 90% of those calls share the same semantic hash,
//! the circuit breaker trips.

use capllm_core::{ChatCompletionRequest, GatewayError};
use fred::interfaces::LuaInterface;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::RedisPool;

/// Lua script for agentic loop detection via sliding window.
///
/// Returns `1` if a loop is detected (velocity >= 15 AND max frequency >= 90%).
/// Returns `0` if safe.
const LOOP_BREAKER_SCRIPT: &str = r#"
local key = KEYS[1]
local now = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local prompt_hash = ARGV[3]
local velocity_limit = tonumber(ARGV[4])
local similarity_threshold = tonumber(ARGV[5])
local unique_id = ARGV[6]

local min_time = now - window

-- Remove old entries
redis.call('ZREMRANGEBYSCORE', key, '-inf', min_time)

-- Add current request
local member = prompt_hash .. ":" .. unique_id
redis.call('ZADD', key, now, member)
redis.call('EXPIRE', key, window)

-- Check velocity
local count = redis.call('ZCARD', key)
if count < velocity_limit then
    return 0
end

-- Calculate highest hash frequency
local elements = redis.call('ZRANGE', key, 0, -1)
local hash_counts = {}
local max_freq = 0

for _, elem in ipairs(elements) do
    local idx = string.find(elem, ":")
    if idx then
        local hash = string.sub(elem, 1, idx - 1)
        hash_counts[hash] = (hash_counts[hash] or 0) + 1
        if hash_counts[hash] > max_freq then
            max_freq = hash_counts[hash]
        end
    end
end

if (max_freq / count) >= similarity_threshold then
    return 1
else
    return 0
end
"#;

pub struct LoopBreaker;

impl LoopBreaker {
    /// Compute a basic semantic signature hash from the request.
    ///
    /// Lowers the text and strips non-alphanumeric characters for a basic
    /// exact-match semantic signature.
    pub fn compute_hash(request: &ChatCompletionRequest) -> String {
        let mut hasher = Sha256::new();
        for msg in &request.messages {
            // Very basic normalization: lowercase and filter alphanumeric
            let normalized: String = msg
                .content
                .chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect();
            hasher.update(normalized.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// Check if the current request constitutes an agentic loop.
    ///
    /// Uses a 60-second sliding window. Returns `Err(LoopDetected)` if tripped.
    pub async fn check(
        pool: &RedisPool,
        team_id: &str,
        prompt_hash: &str,
    ) -> Result<(), GatewayError> {
        let key = format!("gateway:loop:{team_id}");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let unique_id = uuid::Uuid::new_v4().to_string();

        let is_loop: i64 = pool
            .client()
            .eval(
                LOOP_BREAKER_SCRIPT,
                vec![key],
                vec![
                    now.to_string(),
                    "60".to_string(),
                    prompt_hash.to_owned(),
                    "15".to_string(),
                    "0.9".to_string(),
                    unique_id,
                ],
            )
            .await
            .map_err(|e| GatewayError::RedisError(e.to_string()))?;

        if is_loop == 1 {
            tracing::warn!(team_id, "agentic loop detected, tripping circuit breaker");
            Err(GatewayError::LoopDetected(
                "Velocity and semantic repetition thresholds exceeded".to_owned(),
            ))
        } else {
            Ok(())
        }
    }
}
