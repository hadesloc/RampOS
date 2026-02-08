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
/// Accepted values:
/// - `"nats"` – requires `RAMPOS__NATS__URL` (feature `nats` must be compiled in)
/// - `"memory"` / absent – uses `InMemoryEventPublisher` (rejected in production)
pub async fn build_event_publisher(
    nats_url: &str,
    nats_stream: &str,
) -> anyhow::Result<Arc<dyn EventPublisher>> {
    let kind = std::env::var("EVENT_PUBLISHER").unwrap_or_else(|_| "memory".to_string());

    match kind.to_lowercase().as_str() {
        #[cfg(feature = "nats")]
        "nats" => {
            info!("Connecting to NATS event publisher at {}", nats_url);
            let publisher =
                ramp_core::event::NatsEventPublisher::new(nats_url, nats_stream).await?;
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
                     Set EVENT_PUBLISHER=nats and provide RAMPOS__NATS__URL."
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
            // For now, postgres billing falls back to mock with a warning.
            // A real PgBillingDataProvider should be implemented separately.
            warn!("BILLING_PROVIDER=postgres selected but PgBillingDataProvider not yet implemented; using mock fallback");
            if is_production() {
                anyhow::bail!(
                    "PgBillingDataProvider is not yet implemented. \
                     Cannot start in production without a real billing provider."
                );
            }
            Ok(Arc::new(
                ramp_core::billing::mock::MockBillingDataProvider::new(),
            ))
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
            warn!("VNST_PROVIDER=live selected but live provider not yet implemented; using mock fallback");
            if is_production() {
                anyhow::bail!(
                    "Live VnstProtocolDataProvider is not yet implemented. \
                     Cannot start in production without a real VNST provider."
                );
            }
            Ok(Arc::new(
                ramp_core::stablecoin::MockVnstProtocolDataProvider::new(),
            ))
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
    }
    if vnst.is_empty() || vnst.eq_ignore_ascii_case("mock") {
        errors.push("VNST_PROVIDER must not be 'mock' in production (set to 'live')");
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

    #[test]
    fn test_is_production_false_by_default() {
        // In test environment, RUST_ENV is typically not set
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("RAMPOS_ENV");
        assert!(!is_production());
    }

    #[test]
    fn test_is_production_true() {
        std::env::set_var("RUST_ENV", "production");
        assert!(is_production());
        std::env::remove_var("RUST_ENV");
    }

    #[test]
    fn test_is_production_case_insensitive() {
        std::env::set_var("RUST_ENV", "Production");
        assert!(is_production());
        std::env::remove_var("RUST_ENV");
    }

    #[test]
    fn test_production_rejects_mock_billing() {
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("BILLING_PROVIDER", "mock");
        let result = build_billing_provider();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not allowed in production"));
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("BILLING_PROVIDER");
    }

    #[test]
    fn test_production_rejects_default_billing() {
        std::env::set_var("RUST_ENV", "production");
        std::env::remove_var("BILLING_PROVIDER");
        let result = build_billing_provider();
        assert!(result.is_err());
        std::env::remove_var("RUST_ENV");
    }

    #[test]
    fn test_production_rejects_mock_vnst() {
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("VNST_PROVIDER", "mock");
        let result = build_vnst_provider();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not allowed in production"));
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("VNST_PROVIDER");
    }

    #[test]
    fn test_dev_allows_mock_billing() {
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("RAMPOS_ENV");
        std::env::set_var("BILLING_PROVIDER", "mock");
        let result = build_billing_provider();
        assert!(result.is_ok());
        std::env::remove_var("BILLING_PROVIDER");
    }

    #[test]
    fn test_dev_allows_mock_vnst() {
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("RAMPOS_ENV");
        std::env::set_var("VNST_PROVIDER", "mock");
        let result = build_vnst_provider();
        assert!(result.is_ok());
        std::env::remove_var("VNST_PROVIDER");
    }

    #[test]
    fn test_unknown_provider_rejected() {
        std::env::set_var("BILLING_PROVIDER", "invalid");
        let result = build_billing_provider();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown"));
        std::env::remove_var("BILLING_PROVIDER");
    }

    #[test]
    fn test_validate_production_providers_fails_with_defaults() {
        std::env::set_var("RUST_ENV", "production");
        std::env::remove_var("EVENT_PUBLISHER");
        std::env::remove_var("BILLING_PROVIDER");
        std::env::remove_var("VNST_PROVIDER");

        let result = validate_production_providers();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("EVENT_PUBLISHER"));
        assert!(err_msg.contains("BILLING_PROVIDER"));
        assert!(err_msg.contains("VNST_PROVIDER"));

        std::env::remove_var("RUST_ENV");
    }

    #[test]
    fn test_validate_production_providers_passes_with_real_providers() {
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("EVENT_PUBLISHER", "nats");
        std::env::set_var("BILLING_PROVIDER", "postgres");
        std::env::set_var("VNST_PROVIDER", "live");

        let result = validate_production_providers();
        assert!(result.is_ok());

        std::env::remove_var("RUST_ENV");
        std::env::remove_var("EVENT_PUBLISHER");
        std::env::remove_var("BILLING_PROVIDER");
        std::env::remove_var("VNST_PROVIDER");
    }

    #[test]
    fn test_validate_skips_in_dev() {
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("RAMPOS_ENV");
        // Even with no providers set, dev should pass
        let result = validate_production_providers();
        assert!(result.is_ok());
    }
}
