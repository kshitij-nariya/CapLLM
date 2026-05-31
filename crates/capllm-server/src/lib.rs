//! `CapLLM` Server — library root.
//!
//! Exposes [`build_router`] and [`state::AppState`] for integration testing.

pub mod handler;
pub mod state;
pub mod telemetry;

use std::time::Duration;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

/// Build the Axum [`Router`] with all middleware and routes.
///
/// Exposed publicly so integration tests can construct the app without binding
/// to a port.
#[allow(deprecated)]
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handler::dashboard))
        .route("/v1/chat/completions", post(handler::chat_completions))
        .route("/health", get(handler::health))
        .route("/metrics", get(handler::metrics))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .layer(TimeoutLayer::new(Duration::from_secs(300)))
        .with_state(state)
}
