//! Asynchronous Telemetry Pipeline via OpenTelemetry and Prometheus.
//!
//! Provides a non-blocking metrics exporter.

use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::Resource;
use prometheus::Registry;
use std::sync::Arc;

/// Telemetry pipeline and Prometheus registry.
#[derive(Clone, Debug)]
pub struct Telemetry {
    registry: Arc<Registry>,
}

impl Telemetry {
    /// Initialize the OTEL meter provider and Prometheus exporter.
    pub fn new() -> Self {
        let registry = prometheus::Registry::new();
        let exporter = opentelemetry_prometheus::exporter()
            .with_registry(registry.clone())
            .build()
            .expect("Failed to initialize Prometheus exporter");

        let resource = Resource::new(vec![KeyValue::new("service.name", "capllm-gateway")]);

        let provider = SdkMeterProvider::builder()
            .with_reader(exporter)
            .with_resource(resource)
            .build();

        global::set_meter_provider(provider);

        Self {
            registry: Arc::new(registry),
        }
    }

    /// Retrieve the Prometheus metrics payload.
    pub fn export_metrics(&self) -> String {
        use prometheus::Encoder;
        let mut buffer = vec![];
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions to record metrics globally.
pub mod metrics {
    use opentelemetry::{global, KeyValue};
    use std::time::Duration;

    /// Increment the global requests counter.
    pub fn record_request(provider: &str, status: u16, team: &str) {
        let meter = global::meter("capllm.gateway");
        let counter = meter
            .u64_counter("gateway_requests_total")
            .with_description("Total gateway requests")
            .init();

        counter.add(
            1,
            &[
                KeyValue::new("provider", provider.to_owned()),
                KeyValue::new("status", status.to_string()),
                KeyValue::new("team", team.to_owned()),
            ],
        );
    }

    /// Record request latency histogram.
    pub fn record_latency(provider: &str, duration: Duration) {
        let meter = global::meter("capllm.gateway");
        let histogram = meter
            .f64_histogram("gateway_latency_seconds")
            .with_description("Gateway proxy latency in seconds")
            .init();

        histogram.record(
            duration.as_secs_f64(),
            &[KeyValue::new("provider", provider.to_owned())],
        );
    }

    /// Record consumed tokens (estimated or exact).
    pub fn record_tokens(provider: &str, team: &str, tokens: u64) {
        let meter = global::meter("capllm.gateway");
        let counter = meter
            .u64_counter("gateway_tokens_total")
            .with_description("Total tokens processed (estimated)")
            .init();

        counter.add(
            tokens,
            &[
                KeyValue::new("provider", provider.to_owned()),
                KeyValue::new("team", team.to_owned()),
            ],
        );
    }
}
