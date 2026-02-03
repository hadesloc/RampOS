//! Adapter factory for managing multiple banking adapters

use crate::adapters::mock::MockAdapter;
use crate::adapters::napas::NapasAdapter;
use crate::adapters::vietqr::VietQRAdapter;
use crate::traits::RailsAdapter;
use crate::types::AdapterConfig;
use ramp_common::{Error, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Constructor function type
type AdapterConstructor = Box<dyn Fn(AdapterConfig) -> Box<dyn RailsAdapter> + Send + Sync>;

/// Factory for creating banking adapters
pub struct AdapterFactory {
    constructors: Arc<RwLock<HashMap<String, AdapterConstructor>>>,
}

impl AdapterFactory {
    /// Create a new adapter factory with built-in adapters registered
    pub fn new() -> Self {
        let factory = Self {
            constructors: Arc::new(RwLock::new(HashMap::new())),
        };

        // Register built-in adapters
        factory.register_builtin();

        factory
    }

    fn register_builtin(&self) {
        // Register mock adapter
        self.register("mock", |config| {
            Box::new(MockAdapter::new(
                config.provider_code,
                config.webhook_secret,
            ))
        });

        // Register vietqr adapter
        self.register("vietqr", |config| {
            Box::new(VietQRAdapter::new(
                config.provider_code,
                config.webhook_secret,
            ))
        });

        // Register napas adapter
        self.register("napas", |config| {
            Box::new(NapasAdapter::new(
                config.provider_code,
                config.webhook_secret,
            ))
        });
    }

    /// Register a new adapter type
    pub fn register<F>(&self, adapter_type: &str, constructor: F)
    where
        F: Fn(AdapterConfig) -> Box<dyn RailsAdapter> + Send + Sync + 'static,
    {
        let mut constructors = self.constructors.write().unwrap();
        constructors.insert(adapter_type.to_string(), Box::new(constructor));
    }

    /// Create an adapter instance
    pub fn create(
        &self,
        adapter_type: &str,
        config: AdapterConfig,
    ) -> Result<Box<dyn RailsAdapter>> {
        let constructors = self.constructors.read().unwrap();
        if let Some(constructor) = constructors.get(adapter_type) {
            Ok(constructor(config))
        } else {
            Err(Error::Validation(format!(
                "Unknown adapter type: {}",
                adapter_type
            )))
        }
    }

    /// List registered adapter types
    pub fn list_types(&self) -> Vec<String> {
        let constructors = self.constructors.read().unwrap();
        constructors.keys().cloned().collect()
    }
}

impl Default for AdapterFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_factory_registration() {
        let factory = AdapterFactory::new();
        let types = factory.list_types();
        assert!(types.contains(&"mock".to_string()));
    }

    #[test]
    fn test_create_mock_adapter() {
        let factory = AdapterFactory::new();
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
    fn test_create_unknown_adapter() {
        let factory = AdapterFactory::new();
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
}
