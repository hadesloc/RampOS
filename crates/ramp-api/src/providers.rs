//! Provider factory module
//!
//! Config-driven provider selection. In production mode (`RUST_ENV=production`),
//! startup will fail if any mock/in-memory provider is configured.

use std::sync::Arc;
use tracing::{info, warn};

use ramp_core::{
    billing::{BillingConfig, BillingDataProvider, BillingService},
    event::{EventPublisher, InMemoryEventPublisher},
    stablecoin::{VnstProtocolConfig, VnstProtocolDataProvider, VnstProtocolService},
};

/// Returns true when the process is running in production mode.
///
/// Checks `RUST_ENV` (or `RAMPOS_ENV`) for the value `"production"`.
fn is_production() -> bool {
    std::env::var("RUST_ENV")
        .or_else(|_| std::env::var("RAMPOS_ENV"))
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Event Publisher
// ---------------------------------------------------------------------------

/// Build the event publisher based on `EVENT_PUBLISHER` env var.
///
/// Selection logic:
/// 1. If `EVENT_PUBLISHER` is explicitly set, use that value.
/// 2. If `EVENT_PUBLISHER` is absent but `NATS_URL` (or `RAMPOS__NATS__URL` via
///    the config) is available, auto-select `"nats"`.
/// 3. Otherwise, fall back to `"memory"`.
///
/// Accepted values for `EVENT_PUBLISHER`:
/// - `"nats"` – connects to NATS at the provided URL (feature `nats` must be compiled in)
/// - `"memory"` / absent – uses `InMemoryEventPublisher` (rejected in production)
///
/// In production (`RAMPOS_ENV=production` or `RUST_ENV=production`) the process
/// will fail fast if no NATS URL is configured.
#[allow(unused_variables)] // `nats_stream` and `effective_nats_url` are used only with feature `nats`
pub async fn build_event_publisher(
    nats_url: &str,
    nats_stream: &str,
) -> anyhow::Result<Arc<dyn EventPublisher>> {
    // Check for NATS_URL env var directly (takes precedence over config default)
    let nats_url_from_env = std::env::var("NATS_URL")
        .or_else(|_| std::env::var("RAMPOS__NATS__URL"))
        .ok();

    // Resolve the effective NATS URL: env var first, then config value.
    // When the `nats` feature is disabled this variable is unused, which is expected.
    let effective_nats_url = nats_url_from_env
        .as_deref()
        .unwrap_or(nats_url);

    // Determine publisher kind with auto-detection
    let kind = match std::env::var("EVENT_PUBLISHER") {
        Ok(v) => v,
        Err(_) => {
            // Auto-detect: if a real NATS URL is available, prefer nats
            let has_nats = nats_url_from_env.is_some()
                || (!nats_url.is_empty() && nats_url != "nats://localhost:4222");
            if has_nats {
                info!("EVENT_PUBLISHER not set; auto-selecting 'nats' (NATS URL available)");
                "nats".to_string()
            } else {
                "memory".to_string()
            }
        }
    };

    match kind.to_lowercase().as_str() {
        #[cfg(feature = "nats")]
        "nats" => {
            if effective_nats_url.is_empty() {
                anyhow::bail!(
                    "EVENT_PUBLISHER=nats but no NATS URL configured. \
                     Set NATS_URL or RAMPOS__NATS__URL."
                );
            }
            info!("Connecting to NATS event publisher at {}", effective_nats_url);
            let publisher =
                ramp_core::event::NatsEventPublisher::new(effective_nats_url, nats_stream).await?;
            Ok(Arc::new(publisher))
        }
        #[cfg(not(feature = "nats"))]
        "nats" => {
            anyhow::bail!(
                "EVENT_PUBLISHER=nats requested but binary was compiled without the `nats` feature"
            );
        }
        "memory" | "" => {
            if is_production() {
                anyhow::bail!(
                    "InMemoryEventPublisher is not allowed in production. \
                     Set EVENT_PUBLISHER=nats and provide NATS_URL or RAMPOS__NATS__URL."
                );
            }
            warn!("Using InMemoryEventPublisher – NOT suitable for production");
            Ok(Arc::new(InMemoryEventPublisher::new()))
        }
        other => {
            anyhow::bail!("Unknown EVENT_PUBLISHER value: '{}'. Accepted: nats, memory", other);
        }
    }
}

// ---------------------------------------------------------------------------
// Billing Data Provider
// ---------------------------------------------------------------------------

/// Build the billing data provider based on `BILLING_PROVIDER` env var.
///
/// Accepted values:
/// - `"postgres"` – uses the database-backed provider (TODO: implement PgBillingDataProvider)
/// - `"mock"` / absent – uses `MockBillingDataProvider` (rejected in production)
pub fn build_billing_provider() -> anyhow::Result<Arc<dyn BillingDataProvider>> {
    let kind = std::env::var("BILLING_PROVIDER").unwrap_or_else(|_| "mock".to_string());

    match kind.to_lowercase().as_str() {
        "mock" | "" => {
            if is_production() {
                anyhow::bail!(
                    "MockBillingDataProvider is not allowed in production. \
                     Set BILLING_PROVIDER=postgres."
                );
            }
            warn!("Using MockBillingDataProvider – NOT suitable for production");
            Ok(Arc::new(
                ramp_core::billing::mock::MockBillingDataProvider::new(),
            ))
        }
        "postgres" => {
            anyhow::bail!(
                "BILLING_PROVIDER=postgres selected but PgBillingDataProvider is not yet implemented. \
                 Use BILLING_PROVIDER=mock for dev/test until postgres provider is available."
            );
        }
        other => {
            anyhow::bail!(
                "Unknown BILLING_PROVIDER value: '{}'. Accepted: postgres, mock",
                other
            );
        }
    }
}

/// Build `BillingService` using config-driven provider selection.
pub fn build_billing_service() -> anyhow::Result<BillingService> {
    let provider = build_billing_provider()?;
    Ok(BillingService::new(BillingConfig::default(), provider))
}

// ---------------------------------------------------------------------------
// VNST Protocol Data Provider
// ---------------------------------------------------------------------------

/// Build the VNST protocol data provider based on `VNST_PROVIDER` env var.
///
/// Accepted values:
/// - `"live"` – uses a live on-chain provider (TODO: implement)
/// - `"mock"` / absent – uses `MockVnstProtocolDataProvider` (rejected in production)
pub fn build_vnst_provider() -> anyhow::Result<Arc<dyn VnstProtocolDataProvider>> {
    let kind = std::env::var("VNST_PROVIDER").unwrap_or_else(|_| "mock".to_string());

    match kind.to_lowercase().as_str() {
        "mock" | "" => {
            if is_production() {
                anyhow::bail!(
                    "MockVnstProtocolDataProvider is not allowed in production. \
                     Set VNST_PROVIDER=live."
                );
            }
            warn!("Using MockVnstProtocolDataProvider – NOT suitable for production");
            Ok(Arc::new(
                ramp_core::stablecoin::MockVnstProtocolDataProvider::new(),
            ))
        }
        "live" => {
            anyhow::bail!(
                "VNST_PROVIDER=live selected but live VnstProtocolDataProvider is not yet implemented. \
                 Use VNST_PROVIDER=mock for dev/test until live provider is available."
            );
        }
        other => {
            anyhow::bail!(
                "Unknown VNST_PROVIDER value: '{}'. Accepted: live, mock",
                other
            );
        }
    }
}

/// Build `VnstProtocolService` using config-driven provider selection.
pub fn build_vnst_protocol_service() -> anyhow::Result<VnstProtocolService> {
    let provider = build_vnst_provider()?;
    Ok(VnstProtocolService::new(
        VnstProtocolConfig::default(),
        provider,
    ))
}

// ---------------------------------------------------------------------------
// Startup validation
// ---------------------------------------------------------------------------

/// Validate that no mock providers are configured when running in production.
///
/// This is called during startup and will cause the process to exit with an
/// error if production mode is detected but mock providers are in use.
pub fn validate_production_providers() -> anyhow::Result<()> {
    if !is_production() {
        return Ok(());
    }

    info!("Production mode detected – validating provider configuration");

    let event = std::env::var("EVENT_PUBLISHER").unwrap_or_default();
    let billing = std::env::var("BILLING_PROVIDER").unwrap_or_default();
    let vnst = std::env::var("VNST_PROVIDER").unwrap_or_default();

    let mut errors = Vec::new();

    if event.is_empty() || event.eq_ignore_ascii_case("memory") {
        errors.push("EVENT_PUBLISHER must not be 'memory' in production (set to 'nats')");
    }
    if billing.is_empty() || billing.eq_ignore_ascii_case("mock") {
        errors.push("BILLING_PROVIDER must not be 'mock' in production (set to 'postgres')");
    } else if billing.eq_ignore_ascii_case("postgres") {
        errors.push("BILLING_PROVIDER=postgres is configured but PgBillingDataProvider is not implemented yet");
    }
    if vnst.is_empty() || vnst.eq_ignore_ascii_case("mock") {
        errors.push("VNST_PROVIDER must not be 'mock' in production (set to 'live')");
    } else if vnst.eq_ignore_ascii_case("live") {
        errors.push("VNST_PROVIDER=live is configured but live VnstProtocolDataProvider is not implemented yet");
    }

    if errors.is_empty() {
        info!("All provider configurations valid for production");
        Ok(())
    } else {
        let msg = errors.join("\n  - ");
        anyhow::bail!(
            "Production provider validation failed:\n  - {}",
            msg
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// All tests in this module mutate process-wide environment variables.
    /// This mutex serializes them so parallel test threads don't interfere.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Helper: clear all provider-related env vars to a known baseline.
    fn clear_env() {
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("RAMPOS_ENV");
        std::env::remove_var("EVENT_PUBLISHER");
        std::env::remove_var("BILLING_PROVIDER");
        std::env::remove_var("VNST_PROVIDER");
    }

    #[test]
    fn test_is_production_false_by_default() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        assert!(!is_production());
    }

    #[test]
    fn test_is_production_true() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        assert!(is_production());
        clear_env();
    }

    #[test]
    fn test_is_production_case_insensitive() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("RUST_ENV", "Production");
        assert!(is_production());
        clear_env();
    }

    #[test]
    fn test_production_rejects_mock_billing() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("BILLING_PROVIDER", "mock");
        let result = build_billing_provider();
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("not allowed in production"));
        clear_env();
    }

    #[test]
    fn test_production_rejects_default_billing() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        let result = build_billing_provider();
        assert!(result.is_err());
        clear_env();
    }

    #[test]
    fn test_production_rejects_mock_vnst() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("VNST_PROVIDER", "mock");
        let result = build_vnst_provider();
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("not allowed in production"));
        clear_env();
    }

    #[test]
    fn test_dev_allows_mock_billing() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("BILLING_PROVIDER", "mock");
        let result = build_billing_provider();
        assert!(result.is_ok());
        clear_env();
    }

    #[test]
    fn test_dev_allows_mock_vnst() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("VNST_PROVIDER", "mock");
        let result = build_vnst_provider();
        assert!(result.is_ok());
        clear_env();
    }

    #[test]
    fn test_dev_rejects_unimplemented_postgres_billing() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("BILLING_PROVIDER", "postgres");
        let result = build_billing_provider();
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("not yet implemented"));
        clear_env();
    }

    #[test]
    fn test_dev_rejects_unimplemented_live_vnst() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("VNST_PROVIDER", "live");
        let result = build_vnst_provider();
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("not yet implemented"));
        clear_env();
    }

    #[test]
    fn test_unknown_provider_rejected() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("BILLING_PROVIDER", "invalid");
        let result = build_billing_provider();
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Unknown"));
        clear_env();
    }

    #[test]
    fn test_validate_production_providers_fails_with_defaults() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("RUST_ENV", "production");

        let result = validate_production_providers();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("EVENT_PUBLISHER"));
        assert!(err_msg.contains("BILLING_PROVIDER"));
        assert!(err_msg.contains("VNST_PROVIDER"));

        clear_env();
    }

    #[test]
    fn test_validate_production_providers_fails_when_real_providers_unimplemented() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("EVENT_PUBLISHER", "nats");
        std::env::set_var("BILLING_PROVIDER", "postgres");
        std::env::set_var("VNST_PROVIDER", "live");

        let result = validate_production_providers();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("PgBillingDataProvider is not implemented"));
        assert!(err.contains("VnstProtocolDataProvider is not implemented"));

        clear_env();
    }

    #[test]
    fn test_validate_skips_in_dev() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let result = validate_production_providers();
        assert!(result.is_ok());
    }
}
