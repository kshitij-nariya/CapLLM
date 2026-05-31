/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub port: u16,
    pub anthropic_base_url: String,
    pub gemini_base_url: String,
    /// Redis URL for `FinOps` layer. `None` disables Redis features.
    pub redis_url: Option<String>,
}

impl GatewayConfig {
    /// Build configuration from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        Self {
            port: std::env::var("GATEWAY_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
            anthropic_base_url: std::env::var("ANTHROPIC_BASE_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_owned()),
            gemini_base_url: std::env::var("GEMINI_BASE_URL")
                .unwrap_or_else(|_| {
                    "https://generativelanguage.googleapis.com".to_owned()
                }),
            redis_url: std::env::var("REDIS_URL").ok().or_else(|| {
                // Default to local Redis if REDIS_URL not set
                Some("redis://127.0.0.1:6379".to_owned())
            }),
        }
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
