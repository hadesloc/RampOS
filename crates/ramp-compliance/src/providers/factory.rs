use crate::config::providers::{
    DocumentStorageConfig, DocumentStorageType, KycProviderConfig, KycProviderType,
    KytProviderConfig, KytProviderType, SanctionsProviderConfig, SanctionsProviderType,
};
use crate::kyc::{KycProvider, MockKycProvider, OnfidoKycProvider};
use crate::kyt::{ChainalysisKytProvider, KytProvider, MockKytProvider};
use crate::sanctions::{MockSanctionsProvider, OpenSanctionsProvider, SanctionsProvider};
use crate::storage::{DocumentStorage, LocalFilesystemStorage, MockDocumentStorage, S3DocumentStorage, StorageError};

/// Error type for provider factory operations
#[derive(Debug, thiserror::Error)]
pub enum ProviderFactoryError {
    #[error("Provider not implemented: {0}")]
    NotImplemented(String),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Configuration error: {0}")]
    Configuration(String),
}

pub fn create_kyc_provider(config: &KycProviderConfig) -> Result<Box<dyn KycProvider>, ProviderFactoryError> {
    // Auto-detect: if config says Mock but ONFIDO_API_KEY is set, use Onfido
    let effective_provider = match &config.provider {
        KycProviderType::Mock => {
            if std::env::var("ONFIDO_API_KEY").is_ok() {
                tracing::info!("ONFIDO_API_KEY env var detected, auto-selecting Onfido KYC provider");
                KycProviderType::Onfido
            } else {
                tracing::warn!("Using mock KYC provider - set ONFIDO_API_KEY for real Onfido");
                KycProviderType::Mock
            }
        }
        other => other.clone(),
    };

    match effective_provider {
        KycProviderType::Mock => Ok(Box::new(MockKycProvider::with_default_config())),
        KycProviderType::Onfido => {
            let api_key = config
                .api_key
                .clone()
                .or_else(|| std::env::var("ONFIDO_API_KEY").ok())
                .ok_or_else(|| ProviderFactoryError::Configuration("Onfido requires api_key (config or ONFIDO_API_KEY env var)".into()))?;
            Ok(Box::new(OnfidoKycProvider::new(api_key, config.api_url.clone())))
        }
        KycProviderType::Jumio => {
            Err(ProviderFactoryError::NotImplemented("Jumio provider not yet implemented".into()))
        }
    }
}

pub fn create_kyt_provider(config: &KytProviderConfig) -> Result<Box<dyn KytProvider>, ProviderFactoryError> {
    // Auto-detect: if config says Mock but CHAINALYSIS_API_KEY is set, use Chainalysis
    let effective_provider = match &config.provider {
        KytProviderType::Mock => {
            if std::env::var("CHAINALYSIS_API_KEY").is_ok() {
                tracing::info!("CHAINALYSIS_API_KEY env var detected, auto-selecting Chainalysis KYT provider");
                KytProviderType::Chainalysis
            } else {
                tracing::warn!("Using mock KYT provider - set CHAINALYSIS_API_KEY for real Chainalysis");
                KytProviderType::Mock
            }
        }
        other => other.clone(),
    };

    match effective_provider {
        KytProviderType::Mock => Ok(Box::new(MockKytProvider)),
        KytProviderType::Chainalysis => {
            let api_key = config
                .api_key
                .clone()
                .or_else(|| std::env::var("CHAINALYSIS_API_KEY").ok())
                .ok_or_else(|| ProviderFactoryError::Configuration("Chainalysis requires api_key (config or CHAINALYSIS_API_KEY env var)".into()))?;
            Ok(Box::new(ChainalysisKytProvider::new(api_key, config.api_url.clone())))
        }
        KytProviderType::Elliptic => {
            Err(ProviderFactoryError::NotImplemented("Elliptic provider not yet implemented".into()))
        }
    }
}

pub fn create_sanctions_provider(config: &SanctionsProviderConfig) -> Result<Box<dyn SanctionsProvider>, ProviderFactoryError> {
    // Auto-detect: if config says Mock but SANCTIONS_API_KEY is set, use OpenSanctions
    let effective_provider = match &config.provider {
        SanctionsProviderType::Mock => {
            if std::env::var("SANCTIONS_API_KEY").is_ok() {
                tracing::info!("SANCTIONS_API_KEY env var detected, auto-selecting OpenSanctions provider");
                SanctionsProviderType::OpenSanctions
            } else {
                tracing::warn!("Using mock sanctions provider - set SANCTIONS_API_KEY for real OpenSanctions");
                SanctionsProviderType::Mock
            }
        }
        other => other.clone(),
    };

    match effective_provider {
        SanctionsProviderType::Mock => Ok(Box::new(MockSanctionsProvider::new())),
        SanctionsProviderType::OpenSanctions => {
            let api_key = config
                .api_key
                .clone()
                .or_else(|| std::env::var("SANCTIONS_API_KEY").ok())
                .ok_or_else(|| ProviderFactoryError::Configuration("OpenSanctions requires api_key (config or SANCTIONS_API_KEY env var)".into()))?;
            Ok(Box::new(OpenSanctionsProvider::new(
                api_key,
                config.api_url.clone(),
            )))
        }
    }
}

pub fn create_document_storage(config: &DocumentStorageConfig) -> Result<Box<dyn DocumentStorage>, ProviderFactoryError> {
    // Auto-detect: if config says Mock but AWS_S3_BUCKET is set, use S3 (async required)
    // or if DOCUMENT_STORAGE_DIR is set, use local filesystem
    let effective_provider = match &config.provider {
        DocumentStorageType::Mock => {
            if std::env::var("AWS_S3_BUCKET").is_ok() && std::env::var("AWS_ACCESS_KEY_ID").is_ok() {
                tracing::info!("AWS credentials detected, auto-selecting S3 storage (use create_document_storage_async)");
                // S3 requires async initialization, fall through to error
                DocumentStorageType::S3
            } else if std::env::var("DOCUMENT_STORAGE_DIR").is_ok() {
                tracing::info!("DOCUMENT_STORAGE_DIR env var detected, auto-selecting local filesystem storage");
                DocumentStorageType::Local
            } else {
                tracing::warn!("Using mock document storage - set AWS_S3_BUCKET+AWS_ACCESS_KEY_ID for S3, or DOCUMENT_STORAGE_DIR for local");
                DocumentStorageType::Mock
            }
        }
        other => other.clone(),
    };

    match effective_provider {
        DocumentStorageType::Mock => Ok(Box::new(MockDocumentStorage::new())),
        DocumentStorageType::Local => {
            let storage = LocalFilesystemStorage::from_env()?;
            Ok(Box::new(storage))
        }
        DocumentStorageType::S3 => {
            Err(ProviderFactoryError::NotImplemented(
                "S3 provider initialization requires async context - use create_document_storage_async".into()
            ))
        }
    }
}

/// Async version of factory for DocumentStorage.
///
/// Auto-detects the best available storage backend:
/// 1. If config specifies S3 and credentials are available -> S3
/// 2. If config specifies Mock but AWS credentials exist -> S3
/// 3. If config specifies Mock but DOCUMENT_STORAGE_DIR exists -> Local filesystem
/// 4. Otherwise -> Mock (in-memory)
///
/// Environment variables used:
/// - `AWS_S3_BUCKET`: S3 bucket name (overrides config.bucket)
/// - `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`: AWS credentials
/// - `AWS_REGION`: AWS region (defaults to us-east-1)
/// - `AWS_S3_ENDPOINT`: Custom S3 endpoint (MinIO, LocalStack)
/// - `DOCUMENT_STORAGE_DIR`: Local filesystem base directory
pub async fn create_document_storage_async(
    config: &DocumentStorageConfig,
) -> Result<Box<dyn DocumentStorage>, ProviderFactoryError> {
    // Auto-detect the best available storage
    let effective_provider = match &config.provider {
        DocumentStorageType::Mock => {
            if std::env::var("AWS_S3_BUCKET").is_ok() && std::env::var("AWS_ACCESS_KEY_ID").is_ok() {
                tracing::info!("AWS credentials detected, auto-selecting S3 document storage");
                DocumentStorageType::S3
            } else if std::env::var("DOCUMENT_STORAGE_DIR").is_ok() {
                tracing::info!("DOCUMENT_STORAGE_DIR detected, auto-selecting local filesystem storage");
                DocumentStorageType::Local
            } else {
                tracing::warn!("Using mock document storage - set AWS_S3_BUCKET for S3 or DOCUMENT_STORAGE_DIR for local");
                DocumentStorageType::Mock
            }
        }
        other => other.clone(),
    };

    match effective_provider {
        DocumentStorageType::Mock => Ok(Box::new(MockDocumentStorage::new())),
        DocumentStorageType::Local => {
            let storage = LocalFilesystemStorage::from_env()?;
            Ok(Box::new(storage))
        }
        DocumentStorageType::S3 => {
            // Bucket from config, env var, or error
            let bucket = config.bucket.clone()
                .or_else(|| std::env::var("AWS_S3_BUCKET").ok())
                .ok_or_else(|| {
                    ProviderFactoryError::Configuration(
                        "S3 requires bucket name (config.bucket or AWS_S3_BUCKET env var)".into()
                    )
                })?;
            let storage = S3DocumentStorage::from_env(bucket).await?;
            Ok(Box::new(storage))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_kyc_provider_mock() {
        std::env::remove_var("ONFIDO_API_KEY");
        let config = KycProviderConfig::default();
        let provider = create_kyc_provider(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_create_kyc_provider_onfido_with_config_key() {
        let config = KycProviderConfig {
            provider: KycProviderType::Onfido,
            api_key: Some("test-api-key".to_string()),
            api_url: None,
        };
        let provider = create_kyc_provider(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_create_kyc_provider_onfido_missing_key() {
        std::env::remove_var("ONFIDO_API_KEY");
        let config = KycProviderConfig {
            provider: KycProviderType::Onfido,
            api_key: None,
            api_url: None,
        };
        let provider = create_kyc_provider(&config);
        assert!(provider.is_err());
    }

    #[test]
    fn test_create_kyt_provider_mock() {
        std::env::remove_var("CHAINALYSIS_API_KEY");
        let config = KytProviderConfig::default();
        let provider = create_kyt_provider(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_create_kyt_provider_chainalysis_with_config_key() {
        let config = KytProviderConfig {
            provider: KytProviderType::Chainalysis,
            api_key: Some("test-api-key".to_string()),
            api_url: None,
        };
        let provider = create_kyt_provider(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_create_kyt_provider_chainalysis_missing_key() {
        std::env::remove_var("CHAINALYSIS_API_KEY");
        let config = KytProviderConfig {
            provider: KytProviderType::Chainalysis,
            api_key: None,
            api_url: None,
        };
        let provider = create_kyt_provider(&config);
        assert!(provider.is_err());
    }

    #[test]
    fn test_create_sanctions_provider_mock() {
        std::env::remove_var("SANCTIONS_API_KEY");
        let config = SanctionsProviderConfig::default();
        let provider = create_sanctions_provider(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_create_sanctions_provider_opensanctions_with_config_key() {
        let config = SanctionsProviderConfig {
            provider: SanctionsProviderType::OpenSanctions,
            api_key: Some("test-api-key".to_string()),
            api_url: None,
        };
        let provider = create_sanctions_provider(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_create_sanctions_provider_opensanctions_missing_key() {
        std::env::remove_var("SANCTIONS_API_KEY");
        let config = SanctionsProviderConfig {
            provider: SanctionsProviderType::OpenSanctions,
            api_key: None,
            api_url: None,
        };
        let provider = create_sanctions_provider(&config);
        assert!(provider.is_err());
    }

    #[test]
    fn test_create_jumio_not_implemented() {
        let config = KycProviderConfig {
            provider: KycProviderType::Jumio,
            api_key: Some("test-key".to_string()),
            api_url: None,
        };
        let result = create_kyc_provider(&config);
        assert!(result.is_err());
        match result {
            Err(ProviderFactoryError::NotImplemented(_)) => {}
            _ => panic!("Expected NotImplemented error"),
        }
    }

    #[test]
    fn test_create_document_storage_mock() {
        std::env::remove_var("AWS_S3_BUCKET");
        std::env::remove_var("AWS_ACCESS_KEY_ID");
        std::env::remove_var("DOCUMENT_STORAGE_DIR");
        let config = DocumentStorageConfig::default();
        let result = create_document_storage(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_document_storage_local_explicit() {
        let config = DocumentStorageConfig {
            provider: DocumentStorageType::Local,
            bucket: None,
            region: None,
            endpoint: None,
        };
        // Set a temp dir for local storage
        std::env::set_var("DOCUMENT_STORAGE_DIR", std::env::temp_dir().join("rampos-test-docs").to_string_lossy().as_ref());
        let result = create_document_storage(&config);
        std::env::remove_var("DOCUMENT_STORAGE_DIR");
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_document_storage_s3_sync_fails() {
        let config = DocumentStorageConfig {
            provider: DocumentStorageType::S3,
            bucket: Some("test-bucket".to_string()),
            region: None,
            endpoint: None,
        };
        let result = create_document_storage(&config);
        // S3 requires async, so sync factory should return NotImplemented
        assert!(result.is_err());
    }

    #[test]
    fn test_create_document_storage_auto_detect_local() {
        std::env::remove_var("AWS_S3_BUCKET");
        std::env::remove_var("AWS_ACCESS_KEY_ID");
        std::env::set_var("DOCUMENT_STORAGE_DIR", std::env::temp_dir().join("rampos-test-docs-auto").to_string_lossy().as_ref());
        let config = DocumentStorageConfig::default(); // Mock provider
        let result = create_document_storage(&config);
        std::env::remove_var("DOCUMENT_STORAGE_DIR");
        // Should auto-detect DOCUMENT_STORAGE_DIR and use local filesystem
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_kyt_provider_not_implemented() {
        let config = KytProviderConfig {
            provider: KytProviderType::Elliptic,
            api_key: Some("test-key".to_string()),
            api_url: None,
        };
        let result = create_kyt_provider(&config);
        assert!(result.is_err());
        match result {
            Err(ProviderFactoryError::NotImplemented(_)) => {}
            _ => panic!("Expected NotImplemented error"),
        }
    }
}
