//! Secrets management abstraction layer
//!
//! Provides a `SecretProvider` trait that abstracts away the source of secrets.
//! Phase 1: `EnvSecretProvider` reads from environment variables (current behavior).
//! Phase 2+: Swap in `VaultSecretProvider` for HashiCorp Vault or AWS Secrets Manager.

use async_trait::async_trait;
use std::sync::Arc;

/// Trait for accessing secrets from any backend.
///
/// All secret access should go through this trait rather than
/// directly calling `std::env::var()` for sensitive values.
#[async_trait]
pub trait SecretProvider: Send + Sync + std::fmt::Debug {
    /// Get a secret value by key.
    /// Returns `Err` if the secret is not found or inaccessible.
    async fn get_secret(&self, key: &str) -> crate::Result<String>;

    /// Check if a secret exists without retrieving its value.
    async fn has_secret(&self, key: &str) -> bool {
        self.get_secret(key).await.is_ok()
    }

    /// Get a secret with a fallback default value.
    async fn get_secret_or(&self, key: &str, default: &str) -> String {
        self.get_secret(key).await.unwrap_or_else(|_| default.to_string())
    }

    /// Provider name for diagnostics (e.g., "env", "vault", "aws-sm")
    fn provider_name(&self) -> &'static str;
}

/// Environment variable-based secret provider.
///
/// Reads secrets from `std::env::var()`. This is the default provider
/// for development and the initial production deployment.
///
/// **Security note**: Environment variables are visible in `/proc/<pid>/environ`
/// on Linux. For production, migrate to Vault or AWS Secrets Manager.
#[derive(Debug, Clone)]
pub struct EnvSecretProvider;

#[async_trait]
impl SecretProvider for EnvSecretProvider {
    async fn get_secret(&self, key: &str) -> crate::Result<String> {
        std::env::var(key).map_err(|_| {
            crate::Error::Config(format!("Secret '{}' not found in environment", key))
        })
    }

    fn provider_name(&self) -> &'static str {
        "env"
    }
}

/// Create the default secret provider based on configuration.
///
/// Currently always returns `EnvSecretProvider`.
/// In the future, this will check `SECRET_PROVIDER` env var to select
/// between "env", "vault", or "aws-sm".
pub fn create_secret_provider() -> Arc<dyn SecretProvider> {
    // Future: match std::env::var("SECRET_PROVIDER").as_deref() {
    //     Ok("vault") => Arc::new(VaultSecretProvider::from_env()),
    //     Ok("aws-sm") => Arc::new(AwsSecretProvider::from_env()),
    //     _ => Arc::new(EnvSecretProvider),
    // }
    Arc::new(EnvSecretProvider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_env_secret_provider_reads_env() {
        std::env::set_var("TEST_SECRET_KEY_12345", "test_value");
        let provider = EnvSecretProvider;
        let result = provider.get_secret("TEST_SECRET_KEY_12345").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_value");
        std::env::remove_var("TEST_SECRET_KEY_12345");
    }

    #[tokio::test]
    async fn test_env_secret_provider_missing_key() {
        let provider = EnvSecretProvider;
        let result = provider.get_secret("NONEXISTENT_SECRET_XYZ_99").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_env_secret_provider_has_secret() {
        std::env::set_var("TEST_HAS_SECRET_KEY", "exists");
        let provider = EnvSecretProvider;
        assert!(provider.has_secret("TEST_HAS_SECRET_KEY").await);
        assert!(!provider.has_secret("NONEXISTENT_KEY_ABC").await);
        std::env::remove_var("TEST_HAS_SECRET_KEY");
    }

    #[tokio::test]
    async fn test_env_secret_provider_get_or_default() {
        let provider = EnvSecretProvider;
        let result = provider.get_secret_or("NONEXISTENT_KEY_DEFAULT", "fallback").await;
        assert_eq!(result, "fallback");
    }

    #[test]
    fn test_provider_name() {
        let provider = EnvSecretProvider;
        assert_eq!(provider.provider_name(), "env");
    }
}
