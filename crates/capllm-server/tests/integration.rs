//! Integration tests for the `CapLLM` gateway.
//!
//! These tests spin up a mock upstream provider (using Axum) and the gateway
//! itself, then verify end-to-end request translation, streaming, and error
//! handling without touching any real LLM APIs.

use std::convert::Infallible;
use std::net::SocketAddr;

use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use capllm_core::GatewayConfig;
use capllm_server::{build_router, state::AppState};
use serde_json::json;
use tokio::net::TcpListener;

// ─── Mock Upstream Providers ─────────────────────────────────────────────────

/// Start a mock server on a random port and return its address.
async fn start_mock(router: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    addr
}

/// Build a mock Anthropic messages endpoint (non-streaming).
fn mock_anthropic_non_streaming() -> Router {
    async fn handler() -> impl IntoResponse {
        Json(json!({
            "content": [{"type": "text", "text": "Hello from mock Anthropic!"}],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 12, "output_tokens": 6}
        }))
    }
    Router::new().route("/v1/messages", post(handler))
}

/// Build a mock Anthropic messages endpoint (streaming SSE).
fn mock_anthropic_streaming() -> Router {
    async fn handler() -> impl IntoResponse {
        let stream = futures::stream::iter(vec![
            Ok::<Event, Infallible>(
                Event::default()
                    .event("message_start")
                    .data(r#"{"type":"message_start","message":{"id":"msg_test","model":"claude-3-5-sonnet-20241022"}}"#),
            ),
            Ok(Event::default()
                .event("content_block_delta")
                .data(r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#)),
            Ok(Event::default()
                .event("content_block_delta")
                .data(r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world!"}}"#)),
            Ok(Event::default()
                .event("message_delta")
                .data(r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":3}}"#)),
            Ok(Event::default()
                .event("message_stop")
                .data(r#"{"type":"message_stop"}"#)),
        ]);
        Sse::new(stream)
    }
    Router::new().route("/v1/messages", post(handler))
}

/// Build a mock Gemini `generateContent` endpoint (non-streaming).
fn mock_gemini_non_streaming() -> Router {
    async fn handler() -> impl IntoResponse {
        Json(json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Hello from mock Gemini!"}]},
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        }))
    }
    // Gemini URLs contain colons (model:action) which Axum can't match directly.
    // Use a wildcard fallback to catch all POST requests.
    Router::new().fallback(post(handler))
}

/// Build a mock Gemini `streamGenerateContent` endpoint (streaming SSE).
fn mock_gemini_streaming() -> Router {
    async fn handler() -> impl IntoResponse {
        let stream = futures::stream::iter(vec![
            Ok::<Event, Infallible>(Event::default().data(
                r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"Hi "}]},"finishReason":null}]}"#,
            )),
            Ok(Event::default().data(
                r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"there!"}]},"finishReason":"STOP"}]}"#,
            )),
        ]);
        Sse::new(stream)
    }
    Router::new().fallback(post(handler))
}

/// Start the gateway pointing at the given mock base URL.
async fn start_gateway(anthropic_url: &str, gemini_url: &str) -> SocketAddr {
    let config = GatewayConfig {
        port: 0,
        anthropic_base_url: anthropic_url.to_owned(),
        gemini_base_url: gemini_url.to_owned(),
        redis_url: None,
    };
    let state = AppState::new(config, None);
    let app = build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

// ─── Helper ──────────────────────────────────────────────────────────────────

fn openai_request_body(stream: bool) -> serde_json::Value {
    json!({
        "model": "test-model",
        "messages": [
            {"role": "system", "content": "Be helpful."},
            {"role": "user", "content": "Say hello."}
        ],
        "stream": stream,
        "max_tokens": 100
    })
}

// ─── Integration Tests ──────────────────────────────────────────────────────

#[tokio::test]
async fn anthropic_non_streaming_e2e() {
    let mock_addr = start_mock(mock_anthropic_non_streaming()).await;
    let mock_url = format!("http://{mock_addr}");
    let gw_addr = start_gateway(&mock_url, "http://unused").await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gw_addr}/v1/chat/completions"))
        .header("X-Gateway-Provider", "anthropic")
        .header("Authorization", "Bearer test-key")
        .json(&openai_request_body(false))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Hello from mock Anthropic!"
    );
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
    assert_eq!(body["usage"]["total_tokens"], 18);
}

#[tokio::test]
async fn gemini_non_streaming_e2e() {
    let mock_addr = start_mock(mock_gemini_non_streaming()).await;
    let mock_url = format!("http://{mock_addr}");
    let gw_addr = start_gateway("http://unused", &mock_url).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gw_addr}/v1/chat/completions"))
        .header("X-Gateway-Provider", "gemini")
        .header("Authorization", "Bearer test-key")
        .json(&openai_request_body(false))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Hello from mock Gemini!"
    );
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
}

#[tokio::test]
async fn anthropic_streaming_e2e() {
    let mock_addr = start_mock(mock_anthropic_streaming()).await;
    let mock_url = format!("http://{mock_addr}");
    let gw_addr = start_gateway(&mock_url, "http://unused").await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gw_addr}/v1/chat/completions"))
        .header("X-Gateway-Provider", "anthropic")
        .header("Authorization", "Bearer test-key")
        .json(&openai_request_body(true))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let text = resp.text().await.unwrap();
    // Should contain translated SSE data events
    assert!(text.contains("Hello"), "expected 'Hello' in SSE stream: {text}");
    assert!(text.contains("world!"), "expected 'world!' in SSE stream: {text}");
    assert!(text.contains("[DONE]"), "expected [DONE] sentinel: {text}");
}

#[tokio::test]
async fn gemini_streaming_e2e() {
    let mock_addr = start_mock(mock_gemini_streaming()).await;
    let mock_url = format!("http://{mock_addr}");
    let gw_addr = start_gateway("http://unused", &mock_url).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gw_addr}/v1/chat/completions"))
        .header("X-Gateway-Provider", "gemini")
        .header("Authorization", "Bearer test-key")
        .json(&openai_request_body(true))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let text = resp.text().await.unwrap();
    assert!(text.contains("Hi "), "expected 'Hi ' in SSE stream: {text}");
    assert!(text.contains("there!"), "expected 'there!' in SSE stream: {text}");
    assert!(text.contains("[DONE]"), "expected [DONE] sentinel: {text}");
}

#[tokio::test]
async fn missing_provider_header_returns_400() {
    let gw_addr = start_gateway("http://unused", "http://unused").await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gw_addr}/v1/chat/completions"))
        .header("Authorization", "Bearer test-key")
        .json(&openai_request_body(false))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("X-Gateway-Provider"));
}

#[tokio::test]
async fn invalid_provider_returns_400() {
    let gw_addr = start_gateway("http://unused", "http://unused").await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gw_addr}/v1/chat/completions"))
        .header("X-Gateway-Provider", "openai")
        .header("Authorization", "Bearer test-key")
        .json(&openai_request_body(false))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn health_endpoint_works() {
    let gw_addr = start_gateway("http://unused", "http://unused").await;

    let resp = reqwest::get(format!("http://{gw_addr}/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.text().await.unwrap(), "ok");
}
