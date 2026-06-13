//! Adapter factory for managing multiple banking adapters
//!
//! The factory provides a centralized way to create and manage
//! RailsAdapter instances based on configuration.

use crate::adapters::mock::MockAdapter;
use crate::adapters::napas::NapasAdapter;
use crate::adapters::vietqr::VietQRAdapter;
use crate::traits::RailsAdapter;
use crate::types::{AdapterConfig, NapasConfig, VietQRConfig};
use ramp_common::{Error, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

/// Constructor function type
type AdapterConstructor = Box<dyn Fn(AdapterConfig) -> Result<Box<dyn RailsAdapter>> + Send + Sync>;

/// Extended constructor that takes JSON config
type ExtendedConstructor =
    Box<dyn Fn(serde_json::Value) -> Result<Box<dyn RailsAdapter>> + Send + Sync>;

/// Factory for creating banking adapters
pub struct AdapterFactory {
    constructors: Arc<RwLock<HashMap<String, AdapterConstructor>>>,
    extended_constructors: Arc<RwLock<HashMap<String, ExtendedConstructor>>>,
}

impl AdapterFactory {
    /// Create a new adapter factory with built-in adapters registered
    pub fn new() -> Result<Self> {
        let factory = Self {
            constructors: Arc::new(RwLock::new(HashMap::new())),
            extended_constructors: Arc::new(RwLock::new(HashMap::new())),
        };

        // Register built-in adapters
        factory.register_builtin()?;

        Ok(factory)
    }

    fn register_builtin(&self) -> Result<()> {
        if is_production() {
            warn!("Production mode detected; mock rails adapter is not registered");
        } else {
            // Register mock adapter
            self.register("mock", |config| {
                Ok(Box::new(MockAdapter::new(
                    config.provider_code,
                    config.webhook_secret,
                )))
            })?;
        }

        // Register vietqr adapter (basic config)
        self.register("vietqr", |config| {
            Ok(Box::new(VietQRAdapter::new(
                config.provider_code,
                config.webhook_secret,
            )?))
        })?;

        // Register napas adapter (basic config)
        self.register("napas", |config| {
            Ok(Box::new(NapasAdapter::new(
                config.provider_code,
                config.webhook_secret,
            )?))
        })?;

        // Register extended constructors for full config
        self.register_extended("vietqr", |config_value| {
            let config: VietQRConfig = serde_json::from_value(config_value)
                .map_err(|e| Error::Validation(format!("Invalid VietQR config: {}", e)))?;
            Ok(Box::new(VietQRAdapter::with_config(config)?) as Box<dyn RailsAdapter>)
        })?;

        self.register_extended("napas", |config_value| {
            let config: NapasConfig = serde_json::from_value(config_value)
                .map_err(|e| Error::Validation(format!("Invalid Napas config: {}", e)))?;
            Ok(Box::new(NapasAdapter::with_config(config)?) as Box<dyn RailsAdapter>)
        })?;

        Ok(())
    }

    /// Register a new adapter type with basic constructor
    pub fn register<F>(&self, adapter_type: &str, constructor: F) -> Result<()>
    where
        F: Fn(AdapterConfig) -> Result<Box<dyn RailsAdapter>> + Send + Sync + 'static,
    {
        let mut constructors = self.constructors.write().map_err(|_| {
            Error::Internal("Failed to acquire write lock on constructors".to_string())
        })?;
        constructors.insert(adapter_type.to_lowercase(), Box::new(constructor));
        debug!(adapter_type = %adapter_type, "Registered adapter type");
        Ok(())
    }

    /// Register an extended constructor that takes JSON config
    pub fn register_extended<F>(&self, adapter_type: &str, constructor: F) -> Result<()>
    where
        F: Fn(serde_json::Value) -> Result<Box<dyn RailsAdapter>> + Send + Sync + 'static,
    {
        let mut constructors = self.extended_constructors.write().map_err(|_| {
            Error::Internal("Failed to acquire write lock on extended_constructors".to_string())
        })?;
        constructors.insert(adapter_type.to_lowercase(), Box::new(constructor));
        Ok(())
    }

    /// Create an adapter instance with basic config
    pub fn create(
        &self,
        adapter_type: &str,
        config: AdapterConfig,
    ) -> Result<Box<dyn RailsAdapter>> {
        let constructors = self.constructors.read().map_err(|_| {
            Error::Internal("Failed to acquire read lock on constructors".to_string())
        })?;
        let adapter_type_lower = adapter_type.to_lowercase();

        if let Some(constructor) = constructors.get(&adapter_type_lower) {
            info!(adapter_type = %adapter_type, "Creating adapter");
            constructor(config)
        } else {
            Err(Error::Validation(format!(
                "Unknown adapter type: {}",
                adapter_type
            )))
        }
    }

    /// Create an adapter instance with extended JSON config
    pub fn create_from_json(
        &self,
        adapter_type: &str,
        config: serde_json::Value,
    ) -> Result<Box<dyn RailsAdapter>> {
        let constructors = self.extended_constructors.read().map_err(|_| {
            Error::Internal("Failed to acquire read lock on extended_constructors".to_string())
        })?;
        let adapter_type_lower = adapter_type.to_lowercase();

        if let Some(constructor) = constructors.get(&adapter_type_lower) {
            info!(adapter_type = %adapter_type, "Creating adapter from JSON config");
            constructor(config)
        } else {
            // Fall back to basic constructor if extended not available
            drop(constructors);
            let basic_config: AdapterConfig = serde_json::from_value(config)
                .map_err(|e| Error::Validation(format!("Invalid adapter config: {}", e)))?;
            self.create(adapter_type, basic_config)
        }
    }

    /// Create adapters from a configuration map
    ///
    /// The configuration should be a map of provider codes to their configs.
    /// Returns a map of provider codes to adapter instances.
    pub fn create_from_config_map(
        &self,
        config_map: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, Arc<dyn RailsAdapter>>> {
        let mut adapters = HashMap::new();

        for (provider_code, config) in config_map {
            // Extract adapter type from config or use provider_code
            let adapter_type = config
                .get("adapter_type")
                .and_then(|v| v.as_str())
                .unwrap_or(provider_code);

            let adapter = self.create_from_json(adapter_type, config.clone())?;
            adapters.insert(provider_code.clone(), Arc::from(adapter));

            info!(
                provider_code = %provider_code,
                adapter_type = %adapter_type,
                "Created adapter from config"
            );
        }

        Ok(adapters)
    }

    /// List registered adapter types
    pub fn list_types(&self) -> Vec<String> {
        match self.constructors.read() {
            Ok(constructors) => constructors.keys().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Check if an adapter type is registered
    pub fn is_registered(&self, adapter_type: &str) -> bool {
        match self.constructors.read() {
            Ok(constructors) => constructors.contains_key(&adapter_type.to_lowercase()),
            Err(_) => false,
        }
    }
}

impl Default for AdapterFactory {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            constructors: Arc::new(RwLock::new(HashMap::new())),
            extended_constructors: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

/// Helper to create a default set of adapters for test-only use.
pub fn create_test_adapters() -> HashMap<String, Arc<dyn RailsAdapter>> {
    let mut adapters: HashMap<String, Arc<dyn RailsAdapter>> = HashMap::new();

    adapters.insert(
        "mock".to_string(),
        Arc::new(MockAdapter::new("mock", "test_webhook_secret")),
    );

    adapters.insert(
        "vietqr".to_string(),
        Arc::new(
            VietQRAdapter::new("vietqr", "test_webhook_secret")
                .expect("Failed to create test VietQR adapter"),
        ),
    );

    adapters.insert(
        "napas".to_string(),
        Arc::new(
            NapasAdapter::new("napas", "test_webhook_secret")
                .expect("Failed to create test Napas adapter"),
        ),
    );

    adapters
}

/// Returns true when the process is running in production mode.
///
/// Keep this in sync with `ramp-api::providers::is_production`: both check
/// non-empty `RUST_ENV` first, then non-empty `RAMPOS_ENV`, for the
/// case-insensitive value `"production"`. This crate cannot depend on
/// `ramp-api`, so the adapter factory duplicates the startup-safety
/// environment check locally.
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

fn required_secret(name: &str, provider: &str) -> Result<String> {
    std::env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            Error::Validation(format!(
                "{} is required when {}_ENABLE_REAL_API=true",
                name, provider
            ))
        })
}

/// Helper to create production adapters from environment variables
pub fn create_adapters_from_env() -> Result<HashMap<String, Arc<dyn RailsAdapter>>> {
    let mut adapters: HashMap<String, Arc<dyn RailsAdapter>> = HashMap::new();

    // VietQR adapter
    if let Ok(api_key) = std::env::var("VIETQR_API_KEY") {
        let enable_real_api = env_flag_enabled("VIETQR_ENABLE_REAL_API");
        let (api_secret, webhook_secret) = if enable_real_api {
            (
                required_secret("VIETQR_API_SECRET", "VIETQR")?,
                required_secret("VIETQR_WEBHOOK_SECRET", "VIETQR")?,
            )
        } else {
            (
                std::env::var("VIETQR_API_SECRET").unwrap_or_default(),
                std::env::var("VIETQR_WEBHOOK_SECRET").unwrap_or_default(),
            )
        };

        let config = VietQRConfig {
            base: AdapterConfig {
                provider_code: "vietqr".to_string(),
                api_base_url: std::env::var("VIETQR_API_URL")
                    .unwrap_or_else(|_| "https://api.vietqr.io".to_string()),
                api_key,
                api_secret,
                webhook_secret,
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            client_id: std::env::var("VIETQR_CLIENT_ID").ok(),
            merchant_account_number: std::env::var("VIETQR_MERCHANT_ACCOUNT").unwrap_or_default(),
            merchant_bank_bin: std::env::var("VIETQR_MERCHANT_BANK_BIN").unwrap_or_default(),
            merchant_name: std::env::var("VIETQR_MERCHANT_NAME")
                .unwrap_or_else(|_| "RampOS".to_string()),
            enable_real_api,
        };

        adapters.insert(
            "vietqr".to_string(),
            Arc::new(VietQRAdapter::with_config(config)?),
        );
        info!("VietQR adapter configured from environment");
    }

    // Napas adapter
    if let Ok(api_key) = std::env::var("NAPAS_API_KEY") {
        let enable_real_api = env_flag_enabled("NAPAS_ENABLE_REAL_API");
        let (api_secret, webhook_secret) = if enable_real_api {
            (
                required_secret("NAPAS_API_SECRET", "NAPAS")?,
                required_secret("NAPAS_WEBHOOK_SECRET", "NAPAS")?,
            )
        } else {
            (
                std::env::var("NAPAS_API_SECRET").unwrap_or_default(),
                std::env::var("NAPAS_WEBHOOK_SECRET").unwrap_or_default(),
            )
        };

        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: std::env::var("NAPAS_API_URL")
                    .unwrap_or_else(|_| "https://api.napas.com.vn".to_string()),
                api_key,
                api_secret,
                webhook_secret,
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: std::env::var("NAPAS_MERCHANT_ID").unwrap_or_default(),
            terminal_id: std::env::var("NAPAS_TERMINAL_ID").unwrap_or_default(),
            partner_code: std::env::var("NAPAS_PARTNER_CODE").unwrap_or_default(),
            enable_real_api,
            private_key_pem: std::env::var("NAPAS_PRIVATE_KEY").ok(),
            napas_public_key_pem: std::env::var("NAPAS_PUBLIC_KEY").ok(),
        };

        adapters.insert(
            "napas".to_string(),
            Arc::new(NapasAdapter::with_config(config)?),
        );
        info!("Napas adapter configured from environment");
    }

    if is_production() {
        warn!("Production mode detected; mock rails adapter is not registered");
    } else {
        adapters.insert(
            "mock".to_string(),
            Arc::new(MockAdapter::new("mock", "mock_webhook_secret")),
        );
    }

    Ok(adapters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;

    /// All tests below that touch env vars must hold this process-wide lock.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn clear_env() {
        for name in [
            "RUST_ENV",
            "RAMPOS_ENV",
            "VIETQR_API_KEY",
            "VIETQR_API_SECRET",
            "VIETQR_WEBHOOK_SECRET",
            "VIETQR_ENABLE_REAL_API",
            "VIETQR_CLIENT_ID",
            "VIETQR_MERCHANT_ACCOUNT",
            "VIETQR_MERCHANT_BANK_BIN",
            "VIETQR_MERCHANT_NAME",
            "NAPAS_API_KEY",
            "NAPAS_API_SECRET",
            "NAPAS_WEBHOOK_SECRET",
            "NAPAS_ENABLE_REAL_API",
            "NAPAS_MERCHANT_ID",
            "NAPAS_TERMINAL_ID",
            "NAPAS_PARTNER_CODE",
            "NAPAS_PRIVATE_KEY",
            "NAPAS_PUBLIC_KEY",
        ] {
            std::env::remove_var(name);
        }
    }

    #[test]
    fn test_factory_registration() {
        let factory = AdapterFactory::new().unwrap();
        let types = factory.list_types();
        assert!(types.contains(&"mock".to_string()));
        assert!(types.contains(&"vietqr".to_string()));
        assert!(types.contains(&"napas".to_string()));
    }

    #[test]
    fn test_create_mock_adapter() {
        let factory = AdapterFactory::new().unwrap();
        let config = AdapterConfig {
            provider_code: "MOCK".to_string(),
            api_base_url: "http://localhost".to_string(),
            api_key: "key".to_string(),
            api_secret: "secret".to_string(),
            webhook_secret: "webhook_secret".to_string(),
            timeout_secs: 30,
            extra: json!({}),
        };

        let adapter = factory.create("mock", config).unwrap();
        assert_eq!(adapter.provider_code(), "MOCK");
        assert_eq!(adapter.provider_name(), "Mock Bank");
    }

    #[test]
    fn test_create_vietqr_from_json() {
        let factory = AdapterFactory::new().unwrap();
        let config = json!({
            "provider_code": "vietqr",
            "api_base_url": "https://api.vietqr.io",
            "api_key": "test_key",
            "api_secret": "test_secret",
            "webhook_secret": "webhook_secret",
            "timeout_secs": 30,
            "extra": {},
            "merchant_account_number": "1234567890",
            "merchant_bank_bin": "970436",
            "merchant_name": "Test Merchant",
            "enable_real_api": false
        });

        let adapter = factory.create_from_json("vietqr", config).unwrap();
        assert_eq!(adapter.provider_code(), "vietqr");
        assert_eq!(adapter.provider_name(), "VietQR");
        assert!(adapter.is_simulation_mode());
    }

    #[test]
    fn test_create_unknown_adapter() {
        let factory = AdapterFactory::new().unwrap();
        let config = AdapterConfig {
            provider_code: "UNKNOWN".to_string(),
            api_base_url: "http://localhost".to_string(),
            api_key: "key".to_string(),
            api_secret: "secret".to_string(),
            webhook_secret: "webhook_secret".to_string(),
            timeout_secs: 30,
            extra: json!({}),
        };

        let result = factory.create("unknown", config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_test_adapters() {
        let adapters = create_test_adapters();
        assert!(adapters.contains_key("mock"));
        assert!(adapters.contains_key("vietqr"));
        assert!(adapters.contains_key("napas"));
    }

    #[test]
    fn test_is_registered() {
        let factory = AdapterFactory::new().unwrap();
        assert!(factory.is_registered("mock"));
        assert!(factory.is_registered("VIETQR")); // Case insensitive
        assert!(!factory.is_registered("unknown"));
    }

    #[test]
    fn test_create_adapters_from_env_includes_mock_outside_production() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();

        let adapters = create_adapters_from_env().unwrap();
        assert!(adapters.contains_key("mock"));

        clear_env();
    }

    #[test]
    fn test_adapter_factory_new_excludes_mock_in_production() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("RUST_ENV", "production");

        let factory = AdapterFactory::new().unwrap();
        assert!(!factory.is_registered("mock"));
        assert!(factory.is_registered("vietqr"));
        assert!(factory.is_registered("napas"));

        clear_env();
    }

    #[test]
    fn test_create_adapters_from_env_excludes_mock_in_production() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("RUST_ENV", "production");

        let adapters = create_adapters_from_env().unwrap();
        assert!(!adapters.contains_key("mock"));

        clear_env();
    }

    #[test]
    fn test_create_adapters_from_env_allows_empty_secrets_in_simulation() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("NAPAS_API_KEY", "napas_key");
        std::env::set_var("NAPAS_ENABLE_REAL_API", "false");

        let adapters = create_adapters_from_env().unwrap();
        assert!(adapters.contains_key("napas"));
        assert!(adapters.get("napas").unwrap().is_simulation_mode());

        clear_env();
    }

    #[test]
    fn test_create_adapters_from_env_requires_napas_secrets_when_real_api_enabled() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("NAPAS_API_KEY", "napas_key");
        std::env::set_var("NAPAS_ENABLE_REAL_API", "true");

        let result = create_adapters_from_env();
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("NAPAS_API_SECRET"));

        clear_env();
    }

    #[test]
    fn test_create_adapters_from_env_rejects_empty_secret_when_real_api_enabled() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("NAPAS_API_KEY", "napas_key");
        std::env::set_var("NAPAS_ENABLE_REAL_API", "true");
        std::env::set_var("NAPAS_API_SECRET", "   ");
        std::env::set_var("NAPAS_WEBHOOK_SECRET", "webhook_secret");

        let result = create_adapters_from_env();
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("NAPAS_API_SECRET"));

        clear_env();
    }

    #[test]
    fn test_is_production_ignores_empty_rust_env_and_falls_through_to_rampos_env() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("RUST_ENV", "   ");
        std::env::set_var("RAMPOS_ENV", "production");

        assert!(is_production());

        clear_env();
    }

    #[test]
    fn test_create_adapters_from_env_requires_vietqr_secrets_when_real_api_enabled() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        std::env::set_var("VIETQR_API_KEY", "vietqr_key");
        std::env::set_var("VIETQR_ENABLE_REAL_API", "1");
        std::env::set_var("VIETQR_API_SECRET", "vietqr_secret");

        let result = create_adapters_from_env();
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("VIETQR_WEBHOOK_SECRET"));

        clear_env();
    }
}
