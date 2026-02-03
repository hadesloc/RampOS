use crate::config::providers::{
    DocumentStorageConfig, DocumentStorageType, KycProviderConfig, KycProviderType,
    KytProviderConfig, KytProviderType, SanctionsProviderConfig, SanctionsProviderType,
};
use crate::kyc::{KycProvider, MockKycProvider};
use crate::kyt::{KytProvider, MockKytProvider};
use crate::sanctions::{MockSanctionsProvider, OpenSanctionsProvider, SanctionsProvider};
use crate::storage::{DocumentStorage, MockDocumentStorage, S3DocumentStorage};

pub fn create_kyc_provider(config: &KycProviderConfig) -> Box<dyn KycProvider> {
    match config.provider {
        KycProviderType::Mock => Box::new(MockKycProvider::with_default_config()),
        KycProviderType::Onfido => {
            // Placeholder for Onfido integration
            // In a real implementation, we would import OnfidoProvider and initialize it
            panic!("Onfido provider not yet implemented");
        }
        KycProviderType::Jumio => {
            // Placeholder for Jumio integration
            panic!("Jumio provider not yet implemented");
        }
    }
}

pub fn create_kyt_provider(config: &KytProviderConfig) -> Box<dyn KytProvider> {
    match config.provider {
        KytProviderType::Mock => Box::new(MockKytProvider),
        KytProviderType::Chainalysis => {
            panic!("Chainalysis provider not yet implemented");
        }
        KytProviderType::Elliptic => {
            panic!("Elliptic provider not yet implemented");
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

pub fn create_document_storage(config: &DocumentStorageConfig) -> Box<dyn DocumentStorage> {
    match config.provider {
        DocumentStorageType::Mock => Box::new(MockDocumentStorage::new()),
        DocumentStorageType::S3 => {
            let _bucket = config.bucket.clone().expect("S3 requires bucket name");

            // If we have AWS config loaded globally or passed in, we would use it.
            // For now, we'll rely on environment variables if not fully configured here,
            // but S3DocumentStorage::from_env is async, and this factory is sync?
            // Usually factories might need to be async or we block.
            // But S3DocumentStorage has `new` taking `Client`.

            // For simplicity in this sync factory, we might need to handle this carefully.
            // However, S3DocumentStorage has a `from_env` but it is async.
            // Let's assume we can block or we should change factory to async.
            // The plan shows a sync factory function signature.

            // We'll panic for now if we can't do it sync easily, or use `tokio::task::block_in_place` if needed,
            // but creating the client is async.

            // Actually, `S3DocumentStorage` struct definition I saw earlier:
            // pub fn from_env(bucket: String) -> StorageResult<Self> is async.
            // pub fn with_config(config: &SdkConfig, bucket: String) -> Self is sync-ish (Client::new(config) is sync).

            // But getting SdkConfig usually requires async load_defaults.

            // I'll stick to a placeholder or simple implementation if possible.
            // Given I cannot easily make this async without changing the signature (which I can do as I define it),
            // I will change the signature to async or just not implement S3 fully here yet if it's too complex.
            // But the user asked for framework.

            // I'll implement it but note it might panic if run in async context without care?
            // No, creating the provider object itself doesn't need to be async if we have the config.
            // But we don't have the AWS SdkConfig here.

            panic!("S3 provider initialization requires async context or pre-loaded config");
        }
    }
}

// Async version of factory for DocumentStorage if needed
pub async fn create_document_storage_async(
    config: &DocumentStorageConfig,
) -> Box<dyn DocumentStorage> {
    match config.provider {
        DocumentStorageType::Mock => Box::new(MockDocumentStorage::new()),
        DocumentStorageType::S3 => {
            let bucket = config.bucket.clone().expect("S3 requires bucket name");
            // Load AWS config from env
            // This is a bit implicit, ideally we'd pass AWS config in.
            // But for now:
            match S3DocumentStorage::from_env(bucket).await {
                Ok(s) => Box::new(s),
                Err(e) => panic!("Failed to create S3 storage: {:?}", e),
            }
        }
    }
}
