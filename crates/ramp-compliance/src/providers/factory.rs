use crate::config::providers::{
    DocumentStorageConfig, DocumentStorageType, KycProviderConfig, KycProviderType,
    KytProviderConfig, KytProviderType, SanctionsProviderConfig, SanctionsProviderType,
};
use crate::kyc::{KycProvider, MockKycProvider};
use crate::kyt::{KytProvider, MockKytProvider};
use crate::sanctions::{MockSanctionsProvider, OpenSanctionsProvider, SanctionsProvider};
use crate::storage::{DocumentStorage, MockDocumentStorage, S3DocumentStorage, StorageError};

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
    match config.provider {
        KycProviderType::Mock => Ok(Box::new(MockKycProvider::with_default_config())),
        KycProviderType::Onfido => {
            Err(ProviderFactoryError::NotImplemented("Onfido provider not yet implemented".into()))
        }
        KycProviderType::Jumio => {
            Err(ProviderFactoryError::NotImplemented("Jumio provider not yet implemented".into()))
        }
    }
}

pub fn create_kyt_provider(config: &KytProviderConfig) -> Result<Box<dyn KytProvider>, ProviderFactoryError> {
    match config.provider {
        KytProviderType::Mock => Ok(Box::new(MockKytProvider)),
        KytProviderType::Chainalysis => {
            Err(ProviderFactoryError::NotImplemented("Chainalysis provider not yet implemented".into()))
        }
        KytProviderType::Elliptic => {
            Err(ProviderFactoryError::NotImplemented("Elliptic provider not yet implemented".into()))
        }
    }
}

pub fn create_sanctions_provider(config: &SanctionsProviderConfig) -> Box<dyn SanctionsProvider> {
    match config.provider {
        SanctionsProviderType::Mock => Box::new(MockSanctionsProvider::new()),
        SanctionsProviderType::OpenSanctions => Box::new(OpenSanctionsProvider::new(
            config
                .api_key
                .clone()
                .expect("OpenSanctions requires api_key"),
            config.api_url.clone(),
        )),
    }
}

pub fn create_document_storage(config: &DocumentStorageConfig) -> Result<Box<dyn DocumentStorage>, ProviderFactoryError> {
    match config.provider {
        DocumentStorageType::Mock => Ok(Box::new(MockDocumentStorage::new())),
        DocumentStorageType::S3 => {
            Err(ProviderFactoryError::NotImplemented(
                "S3 provider initialization requires async context - use create_document_storage_async".into()
            ))
        }
    }
}

// Async version of factory for DocumentStorage if needed
pub async fn create_document_storage_async(
    config: &DocumentStorageConfig,
) -> Result<Box<dyn DocumentStorage>, ProviderFactoryError> {
    match config.provider {
        DocumentStorageType::Mock => Ok(Box::new(MockDocumentStorage::new())),
        DocumentStorageType::S3 => {
            let bucket = config.bucket.clone().ok_or_else(|| {
                ProviderFactoryError::Configuration("S3 requires bucket name".into())
            })?;
            let storage = S3DocumentStorage::from_env(bucket).await?;
            Ok(Box::new(storage))
        }
    }
}
