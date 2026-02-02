//! OpenTelemetry instrumentation for RampOS
//!
//! Provides distributed tracing and metrics collection.

use opentelemetry::{global, trace::TracerProvider as _};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{Config, Sampler, TracerProvider},
    Resource,
};
use std::time::Duration;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Telemetry configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Service name
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Environment (dev, staging, prod)
    pub environment: String,
    /// OTLP endpoint
    pub otlp_endpoint: Option<String>,
    /// Sampling ratio (0.0 - 1.0)
    pub sampling_ratio: f64,
    /// Log level
    pub log_level: String,
    /// Enable JSON logging
    pub json_logs: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "rampos-api".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: "dev".to_string(),
            otlp_endpoint: None,
            sampling_ratio: 1.0,
            log_level: "info".to_string(),
            json_logs: false,
        }
    }
}

impl TelemetryConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        Self {
            service_name: std::env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "rampos-api".to_string()),
            service_version: std::env::var("SERVICE_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()),
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
            sampling_ratio: std::env::var("OTEL_SAMPLING_RATIO")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0),
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            json_logs: std::env::var("JSON_LOGS")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(false),
        }
    }
}

/// Initialize telemetry (tracing + metrics)
pub fn init_telemetry(config: TelemetryConfig) -> anyhow::Result<()> {
    // Build resource
    let resource = Resource::default()
        .merge(&opentelemetry_sdk::resource::ResourceDetector::detect(
            &opentelemetry_sdk::resource::SdkProvidedResourceDetector,
            Duration::from_secs(0),
        ))
        .merge(&Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", config.service_name.clone()),
            opentelemetry::KeyValue::new("service.version", config.service_version.clone()),
            opentelemetry::KeyValue::new("deployment.environment", config.environment.clone()),
        ]));

    // Set up tracing layers
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // OTLP exporter (if configured)
    if let Some(endpoint) = &config.otlp_endpoint {
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint);

        let tracer_provider = TracerProvider::builder()
            .with_config(
                Config::default()
                    .with_sampler(Sampler::TraceIdRatioBased(config.sampling_ratio))
                    .with_resource(resource),
            )
            .with_batch_exporter(exporter.build_span_exporter()?, runtime::Tokio)
            .build();

        let tracer = tracer_provider.tracer(config.service_name.clone());
        global::set_tracer_provider(tracer_provider);
        global::set_text_map_propagator(
            opentelemetry_sdk::propagation::TraceContextPropagator::new(),
        );

        // Set up OpenTelemetry tracing layer
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json())
            .with(otel_layer)
            .init();
    } else {
        // Local logging only
        if config.json_logs {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().json())
                .init();
        } else {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer())
                .init();
        }
    }

    Ok(())
}

/// Shutdown telemetry
pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}

/// Span extension for adding RampOS-specific attributes
pub trait RampOsSpanExt {
    fn set_tenant_id(&self, tenant_id: &str);
    fn set_user_id(&self, user_id: &str);
    fn set_intent_id(&self, intent_id: &str);
    fn set_intent_type(&self, intent_type: &str);
}

impl RampOsSpanExt for tracing::Span {
    fn set_tenant_id(&self, tenant_id: &str) {
        self.record("tenant_id", tenant_id);
    }

    fn set_user_id(&self, user_id: &str) {
        self.record("user_id", user_id);
    }

    fn set_intent_id(&self, intent_id: &str) {
        self.record("intent_id", intent_id);
    }

    fn set_intent_type(&self, intent_type: &str) {
        self.record("intent_type", intent_type);
    }
}

/// Metrics for RampOS
pub struct Metrics {
    /// Counter for intents created
    pub intents_created: opentelemetry::metrics::Counter<u64>,
    /// Counter for intents completed
    pub intents_completed: opentelemetry::metrics::Counter<u64>,
    /// Counter for intents failed
    pub intents_failed: opentelemetry::metrics::Counter<u64>,
    /// Histogram for intent processing time
    pub intent_processing_time: opentelemetry::metrics::Histogram<f64>,
    /// Counter for API requests
    pub api_requests: opentelemetry::metrics::Counter<u64>,
    /// Histogram for API response time
    pub api_response_time: opentelemetry::metrics::Histogram<f64>,
}

impl Metrics {
    /// Create new metrics
    pub fn new(meter: opentelemetry::metrics::Meter) -> Self {
        Self {
            intents_created: meter
                .u64_counter("rampos.intents.created")
                .with_description("Number of intents created")
                .init(),
            intents_completed: meter
                .u64_counter("rampos.intents.completed")
                .with_description("Number of intents completed")
                .init(),
            intents_failed: meter
                .u64_counter("rampos.intents.failed")
                .with_description("Number of intents failed")
                .init(),
            intent_processing_time: meter
                .f64_histogram("rampos.intents.processing_time")
                .with_description("Intent processing time in seconds")
                .with_unit(opentelemetry::metrics::Unit::new("s"))
                .init(),
            api_requests: meter
                .u64_counter("rampos.api.requests")
                .with_description("Number of API requests")
                .init(),
            api_response_time: meter
                .f64_histogram("rampos.api.response_time")
                .with_description("API response time in seconds")
                .with_unit(opentelemetry::metrics::Unit::new("s"))
                .init(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, "rampos-api");
        assert_eq!(config.sampling_ratio, 1.0);
    }
}
