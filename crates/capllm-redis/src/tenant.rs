//! Virtual key → tenant metadata resolution.

use capllm_core::{GatewayError, TenantMeta};
use fred::prelude::*;

use crate::pool::RedisPool;

/// Redis-backed virtual key store.
pub struct TenantStore;

impl TenantStore {
    /// Resolve a virtual gateway token (`gw-*`) to tenant metadata.
    ///
    /// Returns `Err(Unauthorized)` if the key doesn't exist.
    pub async fn resolve(pool: &RedisPool, token: &str) -> Result<TenantMeta, GatewayError> {
        let key = format!("vkey:{token}");
        let result: std::collections::HashMap<String, String> = pool
            .client()
            .hgetall(&key)
            .await
            .map_err(|e| GatewayError::RedisError(e.to_string()))?;

        if result.is_empty() {
            return Err(GatewayError::Unauthorized(format!(
                "invalid virtual key: {token}"
            )));
        }

        let get = |field: &str| -> Result<String, GatewayError> {
            result
                .get(field)
                .cloned()
                .ok_or_else(|| GatewayError::Unauthorized(format!("missing field `{field}` in virtual key")))
        };

        let parse_u64 = |field: &str| -> Result<u64, GatewayError> {
            get(field)?
                .parse()
                .map_err(|_| GatewayError::Unauthorized(format!("invalid `{field}` in virtual key")))
        };

        Ok(TenantMeta {
            org: get("org")?,
            department: get("department")?,
            team: get("team")?,
            user: get("user")?,
            vendor_key: get("vendor_key")?,
            provider: get("provider")?,
            tpm_limit: parse_u64("tpm_limit")?,
            spend_cap: parse_u64("spend_cap")?,
        })
    }

    /// Provision a virtual key (for testing / admin use).
    pub async fn provision(
        pool: &RedisPool,
        token: &str,
        meta: &TenantMeta,
    ) -> Result<(), GatewayError> {
        let key = format!("vkey:{token}");
        let fields: Vec<(&str, String)> = vec![
            ("org", meta.org.clone()),
            ("department", meta.department.clone()),
            ("team", meta.team.clone()),
            ("user", meta.user.clone()),
            ("vendor_key", meta.vendor_key.clone()),
            ("provider", meta.provider.clone()),
            ("tpm_limit", meta.tpm_limit.to_string()),
            ("spend_cap", meta.spend_cap.to_string()),
        ];

        pool.client()
            .hset::<(), _, _>(&key, fields)
            .await
            .map_err(|e| GatewayError::RedisError(e.to_string()))?;

        Ok(())
    }
}
