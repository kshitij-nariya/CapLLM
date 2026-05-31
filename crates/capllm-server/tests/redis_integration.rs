//! Redis integration tests for the `CapLLM` `FinOps` layer.
//!
//! These tests require a running Redis instance on `redis://127.0.0.1:6379`.
//! If Redis is unavailable, all tests are skipped gracefully.
//!
//! Each test uses unique key names to avoid conflicts when running in parallel.

use std::net::SocketAddr;

use axum::extract::Json;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use capllm_core::{GatewayConfig, TenantMeta};
use capllm_redis::{QueryCache, RateLimiter, RedisPool, TenantStore};
use capllm_server::build_router;
use capllm_server::state::AppState;
use serde_json::json;
use tokio::net::TcpListener;

const REDIS_URL: &str = "redis://127.0.0.1:6379";

/// Try to connect to Redis; return None if unavailable.
async fn try_redis() -> Option<RedisPool> {
    RedisPool::connect(REDIS_URL).await.ok()
}

fn test_tenant(provider: &str) -> TenantMeta {
    TenantMeta {
        org: "test-org".to_owned(),
        department: "engineering".to_owned(),
        team: "ml-team".to_owned(),
        user: "tester".to_owned(),
        vendor_key: "sk-test-vendor-key".to_owned(),
        provider: provider.to_owned(),
        tpm_limit: 100,
        spend_cap: 500,
    }
}

fn mock_anthropic() -> Router {
    async fn handler() -> impl IntoResponse {
        Json(json!({
            "content": [{"type": "text", "text": "Hello from mock!"}],
            "model": "claude-test",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 5, "output_tokens": 3}
        }))
    }
    Router::new().route("/v1/messages", post(handler))
}

async fn start_gateway_with_redis(mock_url: &str, redis: RedisPool) -> SocketAddr {
    let config = GatewayConfig {
        port: 0,
        anthropic_base_url: mock_url.to_owned(),
        gemini_base_url: mock_url.to_owned(),
        redis_url: Some(REDIS_URL.to_owned()),
    };
    let state = AppState::new(config, Some(redis));
    let app = build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

// ─── Virtual Key Tests ───────────────────────────────────────────────────────

#[tokio::test]
async fn virtual_key_valid_resolves_tenant() {
    let Some(pool) = try_redis().await else { return };

    let meta = test_tenant("anthropic");
    TenantStore::provision(&pool, "gw-vk-valid-test", &meta).await.unwrap();

    let resolved = TenantStore::resolve(&pool, "gw-vk-valid-test").await.unwrap();
    assert_eq!(resolved.org, "test-org");
    assert_eq!(resolved.team, "ml-team");
    assert_eq!(resolved.vendor_key, "sk-test-vendor-key");
    assert_eq!(resolved.tpm_limit, 100);
}

#[tokio::test]
async fn virtual_key_invalid_returns_unauthorized() {
    let Some(pool) = try_redis().await else { return };

    let result = TenantStore::resolve(&pool, "gw-does-not-exist-xyz").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("invalid virtual key"));
}

// ─── Rate Limiter Tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn rate_limiter_allows_within_tpm() {
    let Some(pool) = try_redis().await else { return };

    let team = format!("rl-allow-team-{}", uuid::Uuid::new_v4());
    let result = RateLimiter::check_and_record(&pool, &team, 10, 100, 10000).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn rate_limiter_rejects_over_tpm() {
    let Some(pool) = try_redis().await else { return };

    let team = format!("rl-tpm-team-{}", uuid::Uuid::new_v4());
    RateLimiter::check_and_record(&pool, &team, 90, 100, 10000)
        .await
        .unwrap();

    let result = RateLimiter::check_and_record(&pool, &team, 20, 100, 10000).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("rate limited"));
}

#[tokio::test]
async fn rate_limiter_rejects_over_spend_cap() {
    let Some(pool) = try_redis().await else { return };

    let team = format!("rl-spend-team-{}", uuid::Uuid::new_v4());
    RateLimiter::check_and_record(&pool, &team, 40, 10000, 50)
        .await
        .unwrap();

    let result = RateLimiter::check_and_record(&pool, &team, 20, 10000, 50).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("rate limited"));
}

// ─── Query Cache Tests ──────────────────────────────────────────────────────

#[tokio::test]
async fn cache_miss_then_hit() {
    let Some(pool) = try_redis().await else { return };

    let key_str = format!("cache:test-miss-hit-unique-{}", uuid::Uuid::new_v4());
    let key = &key_str;

    let result = QueryCache::get(&pool, key).await.unwrap();
    assert!(result.is_none());

    QueryCache::set(&pool, key, r#"{"result":"cached"}"#, Some(60))
        .await
        .unwrap();

    let result = QueryCache::get(&pool, key).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), r#"{"result":"cached"}"#);
}

#[tokio::test]
async fn cache_key_deterministic() {
    use capllm_core::{ChatCompletionRequest, ChatMessage, ChatRole};

    let req = ChatCompletionRequest {
        model: "test-model".to_owned(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Hello".to_owned(),
        }],
        temperature: Some(0.7),
        max_tokens: Some(100),
        stream: false,
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop: None,
    };

    let key1 = QueryCache::cache_key(&req);
    let key2 = QueryCache::cache_key(&req);
    assert_eq!(key1, key2);
    assert!(key1.starts_with("cache:"));
}

// ─── Concurrent Tenant Isolation ─────────────────────────────────────────────

#[tokio::test]
async fn concurrent_teams_isolated() {
    let Some(pool) = try_redis().await else { return };

    let pool_a = pool.clone();
    let pool_b = pool.clone();

    let team_a = format!("iso-team-a-{}", uuid::Uuid::new_v4());
    let team_b = format!("iso-team-b-{}", uuid::Uuid::new_v4());

    let (res_a, res_b) = tokio::join!(
        async {
            RateLimiter::check_and_record(&pool_a, &team_a, 40, 50, 10000).await.unwrap();
            RateLimiter::check_and_record(&pool_a, &team_a, 20, 50, 10000).await
        },
        async {
            RateLimiter::check_and_record(&pool_b, &team_b, 40, 200, 10000).await.unwrap();
            RateLimiter::check_and_record(&pool_b, &team_b, 20, 200, 10000).await
        }
    );

    assert!(res_a.is_err(), "team-a should be rate limited");
    assert!(res_b.is_ok(), "team-b should be within limits");
}

// ─── E2E: Virtual Key → Gateway → Mock Provider ─────────────────────────────

#[tokio::test]
async fn e2e_virtual_key_non_streaming() {
    let Some(pool) = try_redis().await else { return };

    let mock = mock_anthropic();
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(mock_listener, mock).await.unwrap() });
    let mock_url = format!("http://{mock_addr}");

    let meta = test_tenant("anthropic");
    TenantStore::provision(&pool, "gw-e2e-ns-key", &meta).await.unwrap();

    let gw_addr = start_gateway_with_redis(&mock_url, pool).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gw_addr}/v1/chat/completions"))
        .header("Authorization", "Bearer gw-e2e-ns-key")
        .json(&json!({
            "model": "claude-test",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": false,
            "max_tokens": 100
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["choices"][0]["message"]["content"], "Hello from mock!");
}

#[tokio::test]
async fn e2e_virtual_key_rate_limited() {
    let Some(pool) = try_redis().await else { return };

    let mock = mock_anthropic();
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(mock_listener, mock).await.unwrap() });
    let mock_url = format!("http://{mock_addr}");

    let meta = TenantMeta {
        tpm_limit: 5,
        spend_cap: 500,
        ..test_tenant("anthropic")
    };
    TenantStore::provision(&pool, "gw-e2e-rl-key", &meta).await.unwrap();

    let gw_addr = start_gateway_with_redis(&mock_url, pool).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("http://{gw_addr}/v1/chat/completions"))
        .header("Authorization", "Bearer gw-e2e-rl-key")
        .json(&json!({
            "model": "claude-test",
            "messages": [{"role": "user", "content": "This is a long message that will have many estimated tokens and exceed our very low limit"}],
            "stream": false,
            "max_tokens": 100
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 429, "expected rate limit response");
}
