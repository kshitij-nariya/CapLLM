//! `CapLLM` — AI Reverse Proxy Gateway binary entry-point.

use capllm_core::GatewayConfig;
use capllm_server::build_router;
use capllm_server::state::AppState;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Tracing ──────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    // ── Config ───────────────────────────────────────────────────────────
    let config = GatewayConfig::from_env();
    let port = config.port;

    // ── Redis ────────────────────────────────────────────────────────────
    let redis = if let Some(url) = &config.redis_url {
        match capllm_redis::RedisPool::connect(url).await {
            Ok(pool) => {
                tracing::info!("✅ Redis FinOps layer enabled");
                Some(pool)
            }
            Err(e) => {
                tracing::warn!("⚠ Redis unavailable, FinOps layer disabled: {e}");
                None
            }
        }
    } else {
        tracing::info!("Redis not configured, FinOps layer disabled");
        None
    };

    let state = AppState::new(config, redis);

    // ── Server ───────────────────────────────────────────────────────────
    let app = build_router(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!("🚀 CapLLM gateway listening on {addr}");

    let listener = TcpListener::bind(&addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("gateway shut down gracefully");
    Ok(())
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => { tracing::info!("received SIGINT"); }
        () = terminate => { tracing::info!("received SIGTERM"); }
    }
}
