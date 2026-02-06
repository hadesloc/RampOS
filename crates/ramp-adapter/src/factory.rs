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
use tracing::{debug, info};

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
        // Register mock adapter
        self.register("mock", |config| {
            Ok(Box::new(MockAdapter::new(
                config.provider_code,
                config.webhook_secret,
            )))
        })?;

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
        let mut constructors = self.constructors.write().map_err(|_| Error::Internal("Failed to acquire write lock on constructors".to_string()))?;
        constructors.insert(adapter_type.to_lowercase(), Box::new(constructor));
        debug!(adapter_type = %adapter_type, "Registered adapter type");
        Ok(())
    }

    /// Register an extended constructor that takes JSON config
    pub fn register_extended<F>(&self, adapter_type: &str, constructor: F) -> Result<()>
    where
        F: Fn(serde_json::Value) -> Result<Box<dyn RailsAdapter>> + Send + Sync + 'static,
    {
        let mut constructors = self.extended_constructors.write().map_err(|_| Error::Internal("Failed to acquire write lock on extended_constructors".to_string()))?;
        constructors.insert(adapter_type.to_lowercase(), Box::new(constructor));
        Ok(())
    }

    /// Create an adapter instance with basic config
    pub fn create(
        &self,
        adapter_type: &str,
        config: AdapterConfig,
    ) -> Result<Box<dyn RailsAdapter>> {
        let constructors = self.constructors.read().map_err(|_| Error::Internal("Failed to acquire read lock on constructors".to_string()))?;
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
        let constructors = self.extended_constructors.read().map_err(|_| Error::Internal("Failed to acquire read lock on extended_constructors".to_string()))?;
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
        let constructors = self.constructors.read().expect("Failed to acquire read lock on constructors");
        constructors.keys().cloned().collect()
    }

    /// Check if an adapter type is registered
    pub fn is_registered(&self, adapter_type: &str) -> bool {
        let constructors = self.constructors.read().expect("Failed to acquire read lock on constructors");
        constructors.contains_key(&adapter_type.to_lowercase())
    }
}

impl Default for AdapterFactory {
    fn default() -> Self {
        Self::new().expect("Failed to initialize AdapterFactory")
    }
}

/// Helper to create a default set of adapters for testing
pub fn create_test_adapters() -> HashMap<String, Arc<dyn RailsAdapter>> {
    let mut adapters: HashMap<String, Arc<dyn RailsAdapter>> = HashMap::new();

    adapters.insert(
        "mock".to_string(),
        Arc::new(MockAdapter::new("mock", "test_webhook_secret")),
    );

    adapters.insert(
        "vietqr".to_string(),
        Arc::new(VietQRAdapter::new("vietqr", "test_webhook_secret").expect("Failed to create test VietQR adapter")),
    );

    adapters.insert(
        "napas".to_string(),
        Arc::new(NapasAdapter::new("napas", "test_webhook_secret").expect("Failed to create test Napas adapter")),
    );

    adapters
}

/// Helper to create production adapters from environment variables
pub fn create_adapters_from_env() -> Result<HashMap<String, Arc<dyn RailsAdapter>>> {
    let mut adapters: HashMap<String, Arc<dyn RailsAdapter>> = HashMap::new();

    // VietQR adapter
    if let Ok(api_key) = std::env::var("VIETQR_API_KEY") {
        let config = VietQRConfig {
            base: AdapterConfig {
                provider_code: "vietqr".to_string(),
                api_base_url: std::env::var("VIETQR_API_URL")
                    .unwrap_or_else(|_| "https://api.vietqr.io".to_string()),
                api_key,
                api_secret: std::env::var("VIETQR_API_SECRET").unwrap_or_default(),
                webhook_secret: std::env::var("VIETQR_WEBHOOK_SECRET").unwrap_or_default(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            client_id: std::env::var("VIETQR_CLIENT_ID").ok(),
            merchant_account_number: std::env::var("VIETQR_MERCHANT_ACCOUNT")
                .unwrap_or_default(),
            merchant_bank_bin: std::env::var("VIETQR_MERCHANT_BANK_BIN").unwrap_or_default(),
            merchant_name: std::env::var("VIETQR_MERCHANT_NAME")
                .unwrap_or_else(|_| "RampOS".to_string()),
            enable_real_api: std::env::var("VIETQR_ENABLE_REAL_API")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        };

        adapters.insert(
            "vietqr".to_string(),
            Arc::new(VietQRAdapter::with_config(config)?),
        );
        info!("VietQR adapter configured from environment");
    }

    // Napas adapter
    if let Ok(api_key) = std::env::var("NAPAS_API_KEY") {
        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: std::env::var("NAPAS_API_URL")
                    .unwrap_or_else(|_| "https://api.napas.com.vn".to_string()),
                api_key,
                api_secret: std::env::var("NAPAS_API_SECRET").unwrap_or_default(),
                webhook_secret: std::env::var("NAPAS_WEBHOOK_SECRET").unwrap_or_default(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: std::env::var("NAPAS_MERCHANT_ID").unwrap_or_default(),
            terminal_id: std::env::var("NAPAS_TERMINAL_ID").unwrap_or_default(),
            partner_code: std::env::var("NAPAS_PARTNER_CODE").unwrap_or_default(),
            enable_real_api: std::env::var("NAPAS_ENABLE_REAL_API")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            private_key_pem: std::env::var("NAPAS_PRIVATE_KEY").ok(),
            napas_public_key_pem: std::env::var("NAPAS_PUBLIC_KEY").ok(),
        };

        adapters.insert(
            "napas".to_string(),
            Arc::new(NapasAdapter::with_config(config)?),
        );
        info!("Napas adapter configured from environment");
    }

    // Always include mock adapter for testing
    adapters.insert(
        "mock".to_string(),
        Arc::new(MockAdapter::new("mock", "mock_webhook_secret")),
    );

    Ok(adapters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
