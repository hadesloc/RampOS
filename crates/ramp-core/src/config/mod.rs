use ramp_compliance::config::ProvidersConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub nats: NatsConfig,
    pub server: ServerConfig,
    pub webhook: WebhookConfig,
    pub storage: Option<StorageConfig>,
    #[serde(default)]
    pub providers: ProvidersConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://rampos:change_me@localhost:5432/rampos".to_string()
            }),
            max_connections: 100,
            min_connections: 10,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String, // Keep this for backward compatibility/single node
    pub pool_size: u32,
    pub sentinel_urls: Option<Vec<String>>,
    pub sentinel_master_name: Option<String>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://:dev_redis_pass@localhost:6379".to_string(),
            pool_size: 20,
            sentinel_urls: None,
            sentinel_master_name: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NatsConfig {
    pub url: String,
    pub stream_name: String,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            stream_name: "rampos".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub request_timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookConfig {
    pub retry_max_attempts: u32,
    pub retry_initial_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub signature_tolerance_secs: i64,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            retry_max_attempts: 10,
            retry_initial_delay_ms: 1000,
            retry_max_delay_ms: 3600000,   // 1 hour
            signature_tolerance_secs: 300, // 5 minutes
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut builder = config::Config::builder()
            .add_source(config::Environment::with_prefix("RAMPOS").separator("__"));

        // Allow flat environment variables to override provider config
        let overrides = [
            ("RAMPOS_KYC_PROVIDER", "providers.kyc.provider"),
            ("RAMPOS_KYC_API_KEY", "providers.kyc.api_key"),
            ("RAMPOS_KYC_API_URL", "providers.kyc.api_url"),
            ("RAMPOS_KYT_PROVIDER", "providers.kyt.provider"),
            ("RAMPOS_KYT_API_KEY", "providers.kyt.api_key"),
            ("RAMPOS_SANCTIONS_PROVIDER", "providers.sanctions.provider"),
            ("RAMPOS_SANCTIONS_API_KEY", "providers.sanctions.api_key"),
            (
                "RAMPOS_DOCUMENT_STORAGE_PROVIDER",
                "providers.document_storage.provider",
            ),
            (
                "RAMPOS_DOCUMENT_STORAGE_BUCKET",
                "providers.document_storage.bucket",
            ),
        ];

        for (env_key, config_key) in overrides {
            if let Ok(val) = std::env::var(env_key) {
                builder = builder.set_override(config_key, val)?;
            }
        }

        let cfg = builder.build()?;

        cfg.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_default_config() {
        let config = RedisConfig::default();
        assert_eq!(config.url, "redis://:dev_redis_pass@localhost:6379");
    }
}
