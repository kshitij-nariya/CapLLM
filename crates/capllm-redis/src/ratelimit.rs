//! Token-aware sliding window rate limiter.
//!
//! Uses a 60-second sliding window tracked in Redis. Tokens are estimated from
//! the request payload (~4 characters per token) and checked against the team's
//! TPM limit and cumulative spend cap.

use capllm_core::{ChatCompletionRequest, ChatRole, GatewayError};
use fred::interfaces::LuaInterface;

use crate::pool::RedisPool;

/// Token-aware rate limiter backed by Redis.
pub struct RateLimiter;

/// Lua script for atomic sliding-window check + record.
const RATE_LIMIT_LUA: &str = r"
redis.call('ZREMRANGEBYSCORE', KEYS[1], '-inf', ARGV[1])

local members = redis.call('ZRANGE', KEYS[1], 0, -1, 'WITHSCORES')
local current_tpm = 0
for i = 1, #members, 2 do
    local parts = members[i]
    local sep = string.find(parts, ':')
    if sep then
        current_tpm = current_tpm + tonumber(string.sub(parts, sep + 1))
    end
end

local est = tonumber(ARGV[3])
local tpm_limit = tonumber(ARGV[4])
local spend_cap = tonumber(ARGV[5])

if current_tpm + est > tpm_limit then
    return {-1, current_tpm}
end

local current_spend = tonumber(redis.call('GET', KEYS[2]) or '0')
if current_spend + est > spend_cap then
    return {-2, current_spend}
end

redis.call('ZADD', KEYS[1], ARGV[2], ARGV[6] .. ':' .. ARGV[3])
redis.call('EXPIRE', KEYS[1], 120)
redis.call('INCRBY', KEYS[2], est)

return {0, current_tpm + est}
";

const fn role_len(role: ChatRole) -> usize {
    match role {
        ChatRole::System => 6,
        ChatRole::User => 4,
        ChatRole::Assistant => 9,
    }
}

impl RateLimiter {
    /// Estimate the number of tokens in a chat completion request.
    ///
    /// Uses the ~4 characters per token heuristic across all message content.
    pub fn estimate_tokens(request: &ChatCompletionRequest) -> u64 {
        let total_chars: usize = request
            .messages
            .iter()
            .map(|m| m.content.len() + role_len(m.role))
            .sum();
        (total_chars / 4).max(1) as u64
    }

    /// Check the team's TPM and spend cap, and record usage if within limits.
    pub async fn check_and_record(
        pool: &RedisPool,
        team_id: &str,
        estimated_tokens: u64,
        tpm_limit: u64,
        spend_cap: u64,
    ) -> Result<(), GatewayError> {
        let rate_key = format!("rate:{team_id}:tpm");
        let spend_key = format!("spend:{team_id}:total");

        #[allow(clippy::cast_possible_truncation)]
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let window_start_ms = now_ms - 60_000;

        let member_id = uuid::Uuid::new_v4().to_string();

        let result: Vec<i64> = pool
            .client()
            .eval(
                RATE_LIMIT_LUA,
                vec![rate_key, spend_key],
                vec![
                    window_start_ms.to_string(),
                    now_ms.to_string(),
                    estimated_tokens.to_string(),
                    tpm_limit.to_string(),
                    spend_cap.to_string(),
                    member_id,
                ],
            )
            .await
            .map_err(|e| GatewayError::RedisError(e.to_string()))?;

        match result.first().copied().unwrap_or(0) {
            0 => {
                tracing::debug!(
                    team = team_id,
                    estimated_tokens,
                    current_tpm = result.get(1).copied().unwrap_or(0),
                    "rate limit check passed"
                );
                Ok(())
            }
            -1 => {
                #[allow(clippy::cast_sign_loss)]
                let current = result.get(1).copied().unwrap_or(0) as u64;
                Err(GatewayError::RateLimited {
                    team: team_id.to_owned(),
                    limit: tpm_limit,
                    current,
                })
            }
            -2 => {
                #[allow(clippy::cast_sign_loss)]
                let current = result.get(1).copied().unwrap_or(0) as u64;
                Err(GatewayError::RateLimited {
                    team: team_id.to_owned(),
                    limit: spend_cap,
                    current,
                })
            }
            other => Err(GatewayError::RedisError(format!(
                "unexpected rate limit script result: {other}"
            ))),
        }
    }
}
