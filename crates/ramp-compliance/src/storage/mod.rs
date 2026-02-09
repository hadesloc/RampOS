use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
// use uuid::Uuid; // Unused

pub mod mock;
pub mod s3;

pub use mock::MockDocumentStorage;
pub use s3::{LocalFilesystemStorage, S3DocumentStorage};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Storage backend error: {0}")]
    BackendError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Document not found: {0}")]
    NotFound(String),
    #[error("Invalid path")]
    InvalidPath,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentType {
    IdFront,
    IdBack,
    Selfie,
    ProofOfAddress,
    BankStatement,
    Report,
}

impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentType::IdFront => write!(f, "id_front"),
            DocumentType::IdBack => write!(f, "id_back"),
            DocumentType::Selfie => write!(f, "selfie"),
            DocumentType::ProofOfAddress => write!(f, "proof_of_address"),
            DocumentType::BankStatement => write!(f, "bank_statement"),
            DocumentType::Report => write!(f, "report"),
        }
    }
}

#[async_trait]
pub trait DocumentStorage: Send + Sync {
    /// Uploads a document and returns a unique URL/identifier
    async fn upload(
        &self,
        tenant_id: String,
        user_id: String,
        doc_type: DocumentType,
        data: Vec<u8>,
        extension: &str,
    ) -> StorageResult<String>;

    /// Downloads the document content
    async fn download(&self, url: &str) -> StorageResult<Vec<u8>>;

    /// Deletes the document
    async fn delete(&self, url: &str) -> StorageResult<()>;

    /// Generates a presigned URL for temporary access
    async fn generate_presigned_url(
        &self,
        url: &str,
        expiry: std::time::Duration,
    ) -> StorageResult<String>;
}
