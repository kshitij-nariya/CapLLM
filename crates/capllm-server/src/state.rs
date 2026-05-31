use std::sync::Arc;

use capllm_core::GatewayConfig;
use capllm_proxy::ProxyClient;
use capllm_redis::RedisPool;
use capllm_security::{DlpEngine, SensitiveVault};

/// Shared application state available to all handlers via Axum's `State` extractor.
#[derive(Debug, Clone)]
pub struct AppState {
    pub proxy: Arc<ProxyClient>,
    pub config: Arc<GatewayConfig>,
    /// Redis pool for `FinOps` layer. `None` if Redis is not configured.
    pub redis: Option<Arc<RedisPool>>,
    /// DLP pattern-matching engine (compiled regexes).
    pub dlp: Arc<DlpEngine>,
    /// Encrypted transient vault for DLP re-hydration.
    pub vault: Arc<SensitiveVault>,
    /// OpenTelemetry Prometheus pipeline.
    pub telemetry: crate::telemetry::Telemetry,
}

impl AppState {
    pub fn new(config: GatewayConfig, redis: Option<RedisPool>) -> Self {
        Self {
            proxy: Arc::new(ProxyClient::default()),
            config: Arc::new(config),
            redis: redis.map(Arc::new),
            dlp: Arc::new(DlpEngine::new()),
            vault: Arc::new(SensitiveVault::new()),
            telemetry: crate::telemetry::Telemetry::new(),
        }
    }
}
