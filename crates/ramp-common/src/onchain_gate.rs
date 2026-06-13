//! Runtime gate for experimental on-chain execution surfaces.

use std::sync::{Mutex, MutexGuard, Once, OnceLock};
use tracing::warn;

/// Runtime environment variable that explicitly enables experimental on-chain execution.
pub const EXPERIMENTAL_ONCHAIN_EXECUTION_ENV: &str = "EXPERIMENTAL_ONCHAIN_EXECUTION";

static ENV_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static PRODUCTION_ONCHAIN_WARNING: Once = Once::new();

/// Acquires the process-wide environment mutation guard for tests that set or remove env vars.
///
/// Environment variables are process-global, so tests that mutate them must share this guard
/// across crates to remain deterministic under default parallel `cargo test` execution.
pub fn test_env_lock() -> MutexGuard<'static, ()> {
    ENV_TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

/// Returns true when the process is running in production mode.
///
/// Checks non-empty `RUST_ENV` first, then non-empty `RAMPOS_ENV`, for the
/// case-insensitive value `"production"`. Keep this mirrored with
/// `ramp-api::providers::is_production` and `ramp-adapter::factory::is_production`.
pub fn is_production() -> bool {
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

/// Strict env flag parser: only exact `"true"` or `"1"` enables a flag.
pub fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

/// Returns true when experimental on-chain execution has been explicitly enabled.
///
/// Emits a loud one-time warning if enabled in production.
pub fn experimental_onchain_execution_enabled() -> bool {
    let enabled = env_flag_enabled(EXPERIMENTAL_ONCHAIN_EXECUTION_ENV);

    if enabled && is_production() {
        PRODUCTION_ONCHAIN_WARNING.call_once(|| {
            warn!(
                env_var = EXPERIMENTAL_ONCHAIN_EXECUTION_ENV,
                "EXPERIMENTAL ON-CHAIN EXECUTION ENABLED IN PRODUCTION — not launch scope (D-03)"
            );
        });
    }

    enabled
}

/// Standard fail-closed error text for experimental on-chain execution surfaces.
pub fn experimental_onchain_execution_disabled_message(surface: &str) -> String {
    format!(
        "{} is experimental and disabled — set {}=true to enable experimental on-chain execution (D-03)",
        surface, EXPERIMENTAL_ONCHAIN_EXECUTION_ENV
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_flag_parsing_is_strict() {
        let _guard = test_env_lock();
        std::env::remove_var("RAMPOS_TEST_FLAG");
        assert!(!env_flag_enabled("RAMPOS_TEST_FLAG"));

        for disabled in ["", "yes", "TRUE ", "TRUE", " true", "True", "0"] {
            std::env::set_var("RAMPOS_TEST_FLAG", disabled);
            assert!(!env_flag_enabled("RAMPOS_TEST_FLAG"), "{disabled:?}");
        }

        for enabled in ["true", "1"] {
            std::env::set_var("RAMPOS_TEST_FLAG", enabled);
            assert!(env_flag_enabled("RAMPOS_TEST_FLAG"), "{enabled:?}");
        }

        std::env::remove_var("RAMPOS_TEST_FLAG");
    }

    #[test]
    fn production_detection_matches_runtime_semantics() {
        let _guard = test_env_lock();
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("RAMPOS_ENV");
        assert!(!is_production());

        std::env::set_var("RAMPOS_ENV", "production");
        assert!(is_production());

        std::env::set_var("RUST_ENV", "  ");
        assert!(is_production());

        std::env::set_var("RUST_ENV", "Production");
        assert!(is_production());

        std::env::set_var("RUST_ENV", "development");
        assert!(!is_production());

        std::env::remove_var("RUST_ENV");
        std::env::remove_var("RAMPOS_ENV");
    }
}
