//! Version transformers - adapt request/response payloads between API versions
//!
//! Each transformer handles the conversion between two consecutive versions.
//! The pipeline chains transformers to bridge larger version gaps.

use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::version::ApiVersion;

/// Trait for transforming request and response payloads between API versions.
///
/// Implementors should handle the transformation from one specific version
/// to the next (or previous). The pipeline chains these together to handle
/// multi-version jumps.
pub trait VersionTransformer: Send + Sync {
    /// The source version this transformer converts FROM.
    fn from_version(&self) -> ApiVersion;

    /// The target version this transformer converts TO.
    fn to_version(&self) -> ApiVersion;

    /// Transform a request payload from `from_version` to `to_version` (upgrade).
    /// This is used when a client sends a request using an older version,
    /// and we need to convert it to the internal (latest) format.
    fn transform_request(&self, payload: Value) -> Result<Value, TransformError>;

    /// Transform a response payload from `to_version` to `from_version` (downgrade).
    /// This is used when we need to send a response back in the format
    /// expected by an older API version.
    fn transform_response(&self, payload: Value) -> Result<Value, TransformError>;
}

/// Errors that can occur during version transformation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TransformError {
    #[error("Missing required field '{0}' for version transformation")]
    MissingField(String),

    #[error("Incompatible field type for '{0}': expected {1}")]
    IncompatibleType(String, String),

    #[error("Transformation failed: {0}")]
    Other(String),
}

/// Sample transformer: v2026-02-01 -> v2026-03-01
///
/// Changes introduced in 2026-03-01:
/// - `amount` field renamed to `amount_minor` (value in minor currency units)
/// - Added `currency` field (defaults to "VND" for older requests)
/// - `status` field values changed: "pending" -> "awaiting_confirmation"
/// - Response includes `api_version` field
pub struct V20260201ToV20260301;

impl VersionTransformer for V20260201ToV20260301 {
    fn from_version(&self) -> ApiVersion {
        ApiVersion::parse("2026-02-01").expect("valid version")
    }

    fn to_version(&self) -> ApiVersion {
        ApiVersion::parse("2026-03-01").expect("valid version")
    }

    fn transform_request(&self, mut payload: Value) -> Result<Value, TransformError> {
        if let Some(obj) = payload.as_object_mut() {
            // Rename: amount -> amount_minor
            if let Some(amount) = obj.remove("amount") {
                obj.insert("amount_minor".to_string(), amount);
            }

            // Add default currency if not present
            if !obj.contains_key("currency") {
                obj.insert("currency".to_string(), Value::String("VND".to_string()));
            }
        }
        Ok(payload)
    }

    fn transform_response(&self, mut payload: Value) -> Result<Value, TransformError> {
        if let Some(obj) = payload.as_object_mut() {
            // Rename back: amount_minor -> amount
            if let Some(amount_minor) = obj.remove("amount_minor") {
                obj.insert("amount".to_string(), amount_minor);
            }

            // Map status values back: "awaiting_confirmation" -> "pending"
            if let Some(status) = obj.get_mut("status") {
                if status.as_str() == Some("awaiting_confirmation") {
                    *status = Value::String("pending".to_string());
                }
            }

            // Remove api_version from response for old clients
            obj.remove("api_version");

            // Remove currency field (not present in v1)
            obj.remove("currency");
        }
        Ok(payload)
    }
}

/// Registry of all version transformers, keyed by (from_version, to_version).
///
/// The pipeline looks up the chain of transformers needed to go from
/// a client's version to the server's internal version (latest) and vice versa.
#[derive(Clone)]
pub struct TransformerRegistry {
    /// Transformers keyed by their from_version string, in order.
    transformers: BTreeMap<String, Arc<dyn VersionTransformer>>,
}

impl TransformerRegistry {
    /// Create a new registry with default transformers.
    pub fn new() -> Self {
        let mut registry = Self {
            transformers: BTreeMap::new(),
        };
        // Register the v1-to-v2 transformer
        registry.register(Arc::new(V20260201ToV20260301));
        registry
    }

    /// Register a transformer.
    pub fn register(&mut self, transformer: Arc<dyn VersionTransformer>) {
        let key = transformer.from_version().to_string();
        self.transformers.insert(key, transformer);
    }

    /// Transform a request payload from `client_version` up to `target_version`.
    ///
    /// Chains multiple transformers if needed (e.g., v1 -> v2 -> v3).
    pub fn upgrade_request(
        &self,
        client_version: &ApiVersion,
        target_version: &ApiVersion,
        payload: Value,
    ) -> Result<Value, TransformError> {
        if client_version >= target_version {
            return Ok(payload);
        }

        let mut current = payload;
        let mut current_version = client_version.clone();

        while current_version < *target_version {
            let key = current_version.to_string();
            if let Some(transformer) = self.transformers.get(&key) {
                current = transformer.transform_request(current)?;
                current_version = transformer.to_version();
            } else {
                // No transformer for this version gap - skip ahead
                break;
            }
        }

        Ok(current)
    }

    /// Transform a response payload from `target_version` down to `client_version`.
    ///
    /// Applies transformers in reverse order.
    pub fn downgrade_response(
        &self,
        client_version: &ApiVersion,
        target_version: &ApiVersion,
        payload: Value,
    ) -> Result<Value, TransformError> {
        if client_version >= target_version {
            return Ok(payload);
        }

        // Collect the chain of transformers we need to apply in reverse
        let mut chain: Vec<&Arc<dyn VersionTransformer>> = Vec::new();
        let mut current_version = client_version.clone();

        while current_version < *target_version {
            let key = current_version.to_string();
            if let Some(transformer) = self.transformers.get(&key) {
                chain.push(transformer);
                current_version = transformer.to_version();
            } else {
                break;
            }
        }

        // Apply in reverse order (newest transformation first)
        let mut current = payload;
        for transformer in chain.iter().rev() {
            current = transformer.transform_response(current)?;
        }

        Ok(current)
    }
}

impl Default for TransformerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_upgrade_amount_renamed() {
        let transformer = V20260201ToV20260301;
        let input = json!({ "amount": 50000, "description": "test" });
        let output = transformer.transform_request(input).unwrap();

        assert_eq!(output["amount_minor"], 50000);
        assert!(output.get("amount").is_none());
        assert_eq!(output["currency"], "VND");
        assert_eq!(output["description"], "test");
    }

    #[test]
    fn test_request_upgrade_preserves_explicit_currency() {
        let transformer = V20260201ToV20260301;
        let input = json!({ "amount": 100, "currency": "USD" });
        let output = transformer.transform_request(input).unwrap();

        assert_eq!(output["currency"], "USD");
    }

    #[test]
    fn test_response_downgrade_amount_renamed_back() {
        let transformer = V20260201ToV20260301;
        let input = json!({
            "amount_minor": 50000,
            "currency": "VND",
            "status": "awaiting_confirmation",
            "api_version": "2026-03-01"
        });
        let output = transformer.transform_response(input).unwrap();

        assert_eq!(output["amount"], 50000);
        assert!(output.get("amount_minor").is_none());
        assert_eq!(output["status"], "pending");
        assert!(output.get("api_version").is_none());
        assert!(output.get("currency").is_none());
    }

    #[test]
    fn test_response_downgrade_preserves_other_statuses() {
        let transformer = V20260201ToV20260301;
        let input = json!({ "status": "completed" });
        let output = transformer.transform_response(input).unwrap();
        assert_eq!(output["status"], "completed");
    }

    #[test]
    fn test_registry_upgrade_chain() {
        let registry = TransformerRegistry::new();
        let v1 = ApiVersion::parse("2026-02-01").unwrap();
        let v2 = ApiVersion::parse("2026-03-01").unwrap();

        let input = json!({ "amount": 1000 });
        let output = registry.upgrade_request(&v1, &v2, input).unwrap();
        assert_eq!(output["amount_minor"], 1000);
        assert_eq!(output["currency"], "VND");
    }

    #[test]
    fn test_registry_downgrade_chain() {
        let registry = TransformerRegistry::new();
        let v1 = ApiVersion::parse("2026-02-01").unwrap();
        let v2 = ApiVersion::parse("2026-03-01").unwrap();

        let input = json!({
            "amount_minor": 2000,
            "currency": "VND",
            "status": "awaiting_confirmation"
        });
        let output = registry.downgrade_response(&v1, &v2, input).unwrap();
        assert_eq!(output["amount"], 2000);
        assert_eq!(output["status"], "pending");
    }

    #[test]
    fn test_registry_same_version_noop() {
        let registry = TransformerRegistry::new();
        let v = ApiVersion::parse("2026-03-01").unwrap();

        let input = json!({ "foo": "bar" });
        let output = registry.upgrade_request(&v, &v, input.clone()).unwrap();
        assert_eq!(output, input);

        let output = registry.downgrade_response(&v, &v, input.clone()).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn test_registry_newer_client_noop() {
        let registry = TransformerRegistry::new();
        let v1 = ApiVersion::parse("2026-02-01").unwrap();
        let v2 = ApiVersion::parse("2026-03-01").unwrap();

        let input = json!({ "foo": "bar" });
        // Client is newer than target - no transform needed
        let output = registry.upgrade_request(&v2, &v1, input.clone()).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn test_transform_empty_object() {
        let transformer = V20260201ToV20260301;
        let input = json!({});
        let output = transformer.transform_request(input).unwrap();
        assert_eq!(output["currency"], "VND");
    }

    #[test]
    fn test_transform_non_object() {
        let transformer = V20260201ToV20260301;
        let input = json!("string_value");
        let output = transformer.transform_request(input.clone()).unwrap();
        // Non-objects pass through unchanged
        assert_eq!(output, input);
    }

    #[test]
    fn test_transformer_applies_to_matching_version() {
        let transformer = V20260201ToV20260301;
        let from = transformer.from_version();
        let to = transformer.to_version();

        // Verify transformer version bounds
        assert_eq!(from.to_string(), "2026-02-01");
        assert_eq!(to.to_string(), "2026-03-01");

        // When client version matches from_version, transformer should apply
        let registry = TransformerRegistry::new();
        let input = json!({ "amount": 5000 });
        let output = registry.upgrade_request(&from, &to, input).unwrap();
        assert_eq!(output["amount_minor"], 5000);
        assert!(output.get("amount").is_none());
    }

    #[test]
    fn test_transformer_skips_newer_version() {
        let transformer = V20260201ToV20260301;
        let to = transformer.to_version(); // 2026-03-01

        // When client is already at to_version, no transformation should happen
        let registry = TransformerRegistry::new();
        let input = json!({ "amount_minor": 5000, "currency": "VND" });
        let output = registry.upgrade_request(&to, &to, input.clone()).unwrap();
        assert_eq!(output, input, "No transformation for same version");
    }

    #[test]
    fn test_transformer_downgrade_applies_for_older_client() {
        let registry = TransformerRegistry::new();
        let v1 = ApiVersion::parse("2026-02-01").unwrap();
        let v2 = ApiVersion::parse("2026-03-01").unwrap();

        let response = json!({
            "amount_minor": 10000,
            "currency": "VND",
            "status": "awaiting_confirmation",
            "api_version": "2026-03-01",
            "id": "intent_123"
        });

        let downgraded = registry.downgrade_response(&v1, &v2, response).unwrap();
        // Old fields should be restored
        assert_eq!(downgraded["amount"], 10000);
        assert_eq!(downgraded["status"], "pending");
        // New fields should be removed
        assert!(downgraded.get("amount_minor").is_none());
        assert!(downgraded.get("api_version").is_none());
        assert!(downgraded.get("currency").is_none());
        // Unrelated fields should be preserved
        assert_eq!(downgraded["id"], "intent_123");
    }

    #[test]
    fn test_transformer_version_bounds() {
        let transformer = V20260201ToV20260301;
        assert!(transformer.from_version() < transformer.to_version());
        assert!(transformer.from_version().is_compatible());
        assert!(transformer.to_version().is_compatible());
    }

    #[test]
    fn test_registry_default_has_transformers() {
        let registry = TransformerRegistry::default();
        let v1 = ApiVersion::parse("2026-02-01").unwrap();
        let v2 = ApiVersion::parse("2026-03-01").unwrap();

        // Should be able to transform between known versions
        let input = json!({ "amount": 100 });
        let result = registry.upgrade_request(&v1, &v2, input);
        assert!(result.is_ok());
    }
}
