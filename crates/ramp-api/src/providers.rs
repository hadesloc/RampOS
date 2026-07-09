//! Provider factory module
//!
//! Config-driven provider selection. In production mode (`RUST_ENV=production`),
//! startup will fail if any mock/in-memory provider is configured.

use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, warn};

use ramp_core::{
    billing::{BillingConfig, BillingDataProvider, BillingService, PgBillingDataProvider},
    event::{EventPublisher, InMemoryEventPublisher},
    stablecoin::{
        LiveVnstProtocolDataProvider, VnstProtocolConfig, VnstProtocolDataProvider,
        VnstProtocolService,
    },
};

/// Returns true when the process is running in production mode.
///
/// Checks non-empty `RUST_ENV` first, then non-empty `RAMPOS_ENV`, for the
/// case-insensitive value `"production"`. Keep this mirrored with
/// `ramp-adapter::factory::is_production`.
fn is_production() -> bool {
    std::env::var("RUST_ENV")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("RAMPOS_ENV")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false)
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "true" || v == "1")
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
    let effective_nats_url = nats_url_from_env.as_deref().unwrap_or(nats_url);

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
            info!(
                "Connecting to NATS event publisher at {}",
                effective_nats_url
            );
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
            anyhow::bail!(
                "Unknown EVENT_PUBLISHER value: '{}'. Accepted: nats, memory",
                other
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Billing Data Provider
// ---------------------------------------------------------------------------

/// Build the billing data provider based on `BILLING_PROVIDER` env var.
///
/// Accepted values:
/// - `"postgres"` – uses the database-backed provider (requires a `PgPool`)
/// - `"mock"` / absent – uses `MockBillingDataProvider` (rejected in production)
pub fn build_billing_provider(
    pool: Option<PgPool>,
) -> anyhow::Result<Arc<dyn BillingDataProvider>> {
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
            let pool = pool.ok_or_else(|| {
                anyhow::anyhow!(
                    "BILLING_PROVIDER=postgres requires a PostgreSQL pool. \
                     Pass the application PgPool into build_billing_provider/build_billing_service."
                )
            })?;
            Ok(Arc::new(PgBillingDataProvider::with_pool(pool)))
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
pub fn build_billing_service(pool: Option<PgPool>) -> anyhow::Result<BillingService> {
    let provider = build_billing_provider(pool)?;
    Ok(BillingService::new(BillingConfig::default(), provider))
}

// ---------------------------------------------------------------------------
// VNST Protocol Data Provider
// ---------------------------------------------------------------------------

/// Build the VNST protocol data provider based on `VNST_PROVIDER` env var.
///
/// Accepted values:
/// - `"live"` – uses a live on-chain provider
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
            warn!(
                "Using LiveVnstProtocolDataProvider with read-only capability: on-chain supply only; VNST issuance, reserves, and peg/oracle endpoints require additional providers"
            );
            Ok(Arc::new(LiveVnstProtocolDataProvider::from_env()?))
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
    } else if vnst.eq_ignore_ascii_case("live") {
        warn!(
            "Production VNST_PROVIDER=live is accepted with read-only capability: on-chain supply only; issuance, reserves, and peg/oracle require additional providers"
        );
    }

    let vietqr_configured = std::env::var("VIETQR_API_KEY").is_ok();
    let napas_configured = std::env::var("NAPAS_API_KEY").is_ok();
    let vietqr_real_api = env_flag_enabled("VIETQR_ENABLE_REAL_API");
    let napas_real_api = env_flag_enabled("NAPAS_ENABLE_REAL_API");

    if vietqr_configured && !vietqr_real_api {
        errors.push(
            "VIETQR_API_KEY is configured but VIETQR_ENABLE_REAL_API is not true in production",
        );
    }
    if napas_configured && !napas_real_api {
        errors.push(
            "NAPAS_API_KEY is configured but NAPAS_ENABLE_REAL_API is not true in production",
        );
    }
    if vietqr_real_api && !vietqr_configured {
        errors.push("VIETQR_ENABLE_REAL_API=true but VIETQR_API_KEY is not configured");
    }
    if napas_real_api && !napas_configured {
        errors.push("NAPAS_ENABLE_REAL_API=true but NAPAS_API_KEY is not configured");
    }
    if !(vietqr_configured && vietqr_real_api) && !(napas_configured && napas_real_api) {
        errors.push(
            "At least one real rails adapter must be configured in production \
             (set NAPAS_API_KEY with NAPAS_ENABLE_REAL_API=true or \
             VIETQR_API_KEY with VIETQR_ENABLE_REAL_API=true)",
        );
    }

    if errors.is_empty() {
        info!("All provider configurations valid for production");
        Ok(())
    } else {
        let msg = errors.join("\n  - ");
        anyhow::bail!("Production provider validation failed:\n  - {}", msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // All tests in this module mutate process-wide environment variables
    // (RUST_ENV/RAMPOS_ENV/provider config). They serialize on the shared
    // `ramp_common::onchain_gate::test_env_lock()` so they don't interfere with
    // each other or with env-sensitive tests in sibling modules across this binary.

    /// Helper: clear all provider-related env vars to a known baseline.
    fn clear_env() {
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("RAMPOS_ENV");
        std::env::remove_var("EVENT_PUBLISHER");
        std::env::remove_var("BILLING_PROVIDER");
        std::env::remove_var("VNST_PROVIDER");
        std::env::remove_var("VNST_RPC_URL");
        std::env::remove_var("VNST_CHAIN_ID");
        std::env::remove_var("VNST_CONTRACT_ADDRESS");
        std::env::remove_var("BSC_RPC_URL");
        std::env::remove_var("VIETQR_API_KEY");
        std::env::remove_var("VIETQR_ENABLE_REAL_API");
        std::env::remove_var("NAPAS_API_KEY");
        std::env::remove_var("NAPAS_ENABLE_REAL_API");
    }

    #[test]
    fn test_is_production_false_by_default() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        assert!(!is_production());
    }

    #[test]
    fn test_is_production_true() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        assert!(is_production());
        clear_env();
    }

    #[test]
    fn test_is_production_case_insensitive() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("RUST_ENV", "Production");
        assert!(is_production());
        clear_env();
    }

    #[test]
    fn test_is_production_ignores_empty_rust_env_and_falls_through_to_rampos_env() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("RUST_ENV", "   ");
        std::env::set_var("RAMPOS_ENV", "production");
        assert!(is_production());
        clear_env();
    }

    #[test]
    fn test_production_rejects_mock_billing() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("BILLING_PROVIDER", "mock");
        let result = build_billing_provider(None);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("not allowed in production"));
        clear_env();
    }

    #[test]
    fn test_production_rejects_default_billing() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        let result = build_billing_provider(None);
        assert!(result.is_err());
        clear_env();
    }

    #[test]
    fn test_production_rejects_mock_vnst() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
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
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("BILLING_PROVIDER", "mock");
        let result = build_billing_provider(None);
        assert!(result.is_ok());
        clear_env();
    }

    #[test]
    fn test_dev_allows_mock_vnst() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("VNST_PROVIDER", "mock");
        let result = build_vnst_provider();
        assert!(result.is_ok());
        clear_env();
    }

    #[test]
    fn test_postgres_billing_requires_pool() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("BILLING_PROVIDER", "postgres");
        let result = build_billing_provider(None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("requires a PostgreSQL pool"));
        clear_env();
    }

    #[tokio::test]
    async fn test_dev_builds_postgres_billing_with_pool() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("BILLING_PROVIDER", "postgres");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://user:password@localhost/rampos")
            .expect("lazy pool URL should parse");
        let result = build_billing_provider(Some(pool));
        assert!(result.is_ok());
        clear_env();
    }

    #[test]
    fn test_live_vnst_requires_rpc_url() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("VNST_PROVIDER", "live");
        std::env::set_var(
            "VNST_CONTRACT_ADDRESS",
            "0x1234567890123456789012345678901234567890",
        );
        let result = build_vnst_provider();
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("VNST_RPC_URL") || err.contains("BSC_RPC_URL"));
        clear_env();
    }

    #[test]
    fn test_live_vnst_requires_contract_address() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("VNST_PROVIDER", "live");
        std::env::set_var("VNST_RPC_URL", "https://rpc.example.invalid");
        let result = build_vnst_provider();
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("VNST_CONTRACT_ADDRESS"));
        clear_env();
    }

    #[test]
    fn test_dev_builds_live_vnst_with_config() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("VNST_PROVIDER", "live");
        std::env::set_var("VNST_RPC_URL", "https://rpc.example.invalid");
        std::env::set_var(
            "VNST_CONTRACT_ADDRESS",
            "0x1234567890123456789012345678901234567890",
        );
        let result = build_vnst_provider();
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().capabilities(),
            ramp_core::stablecoin::VnstProviderCapability::ReadOnlySupply
        );
        clear_env();
    }

    #[test]
    fn test_live_vnst_rejects_zero_contract_address() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("VNST_PROVIDER", "live");
        std::env::set_var("VNST_RPC_URL", "https://rpc.example.invalid");
        std::env::set_var(
            "VNST_CONTRACT_ADDRESS",
            "0x0000000000000000000000000000000000000000",
        );
        let result = build_vnst_provider();
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("non-zero"));
        clear_env();
    }

    #[test]
    fn test_unknown_provider_rejected() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("BILLING_PROVIDER", "invalid");
        let result = build_billing_provider(None);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Unknown"));
        clear_env();
    }

    #[test]
    fn test_validate_production_providers_fails_with_defaults() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
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
    fn test_validate_production_providers_allows_postgres_billing_and_live_vnst() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("EVENT_PUBLISHER", "nats");
        std::env::set_var("BILLING_PROVIDER", "postgres");
        std::env::set_var("VNST_PROVIDER", "live");
        std::env::set_var("NAPAS_API_KEY", "napas_key");
        std::env::set_var("NAPAS_ENABLE_REAL_API", "true");

        let result = validate_production_providers();
        assert!(result.is_ok());

        clear_env();
    }

    #[test]
    fn test_validate_production_providers_rejects_mock_vnst_even_with_real_rails() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("EVENT_PUBLISHER", "nats");
        std::env::set_var("BILLING_PROVIDER", "postgres");
        std::env::set_var("VNST_PROVIDER", "mock");
        std::env::set_var("NAPAS_API_KEY", "napas_key");
        std::env::set_var("NAPAS_ENABLE_REAL_API", "true");

        let result = validate_production_providers();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("VNST_PROVIDER"));
        assert!(err.contains("mock"));

        clear_env();
    }

    #[test]
    fn test_validate_production_providers_rejects_simulation_rails() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("EVENT_PUBLISHER", "nats");
        std::env::set_var("BILLING_PROVIDER", "postgres");
        std::env::set_var("VNST_PROVIDER", "live");
        std::env::set_var("NAPAS_API_KEY", "napas_key");
        std::env::set_var("NAPAS_ENABLE_REAL_API", "false");

        let result = validate_production_providers();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("NAPAS_ENABLE_REAL_API"));

        clear_env();
    }

    #[test]
    fn test_validate_production_providers_rejects_no_real_rails() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("EVENT_PUBLISHER", "nats");
        std::env::set_var("BILLING_PROVIDER", "postgres");
        std::env::set_var("VNST_PROVIDER", "live");

        let result = validate_production_providers();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("At least one real rails adapter"));

        clear_env();
    }

    #[test]
    fn test_validate_production_providers_allows_configured_real_rail() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        std::env::set_var("RUST_ENV", "production");
        std::env::set_var("EVENT_PUBLISHER", "nats");
        std::env::set_var("BILLING_PROVIDER", "postgres");
        std::env::set_var("VNST_PROVIDER", "live");
        std::env::set_var("VIETQR_API_KEY", "vietqr_key");
        std::env::set_var("VIETQR_ENABLE_REAL_API", "true");

        let result = validate_production_providers();
        assert!(result.is_ok());

        clear_env();
    }

    #[test]
    fn test_validate_skips_in_dev() {
        let _lock = ramp_common::onchain_gate::test_env_lock();
        clear_env();
        let result = validate_production_providers();
        assert!(result.is_ok());
    }
}
