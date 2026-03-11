use super::{DocumentStorage, DocumentType, StorageError, StorageResult};
use async_trait::async_trait;
use aws_config::SdkConfig;
use aws_sdk_s3::{
    presigning::PresigningConfig, primitives::ByteStream, types::ServerSideEncryption, Client,
};
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
pub struct S3DocumentStorage {
    client: Client,
    bucket: String,
}

impl S3DocumentStorage {
    pub fn new(client: Client, bucket: String) -> Self {
        Self { client, bucket }
    }

    /// Create S3 storage from environment variables.
    ///
    /// Uses the standard AWS SDK credential chain:
    /// - `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` for explicit credentials
    /// - `AWS_REGION` for region selection (defaults to us-east-1)
    /// - `AWS_S3_ENDPOINT` for custom endpoints (e.g., MinIO, LocalStack)
    ///
    /// The bucket name is provided as a parameter but can also come from
    /// the `AWS_S3_BUCKET` env var via the factory.
    pub async fn from_env(bucket: String) -> StorageResult<Self> {
        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest());

        // Allow custom endpoint for local development (MinIO, LocalStack)
        if let Ok(endpoint) = std::env::var("AWS_S3_ENDPOINT") {
            info!(endpoint = %endpoint, "Using custom S3 endpoint");
            config_loader = config_loader.endpoint_url(&endpoint);
        }

        let config = config_loader.load().await;
        let client = Client::new(&config);

        info!(bucket = %bucket, "S3 document storage initialized");
        Ok(Self { client, bucket })
    }

    pub fn with_config(config: &SdkConfig, bucket: String) -> Self {
        let client = Client::new(config);
        Self { client, bucket }
    }

    fn generate_key(
        &self,
        tenant_id: String,
        user_id: String,
        doc_type: DocumentType,
        extension: &str,
    ) -> String {
        let file_id = Uuid::new_v4();
        format!(
            "{}/{}/{}/{}.{}",
            tenant_id, user_id, doc_type, file_id, extension
        )
    }

    /// Infer content-type from file extension
    fn content_type_for_extension(extension: &str) -> &'static str {
        match extension.to_lowercase().as_str() {
            "pdf" => "application/pdf",
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "csv" => "text/csv",
            "json" => "application/json",
            "xml" => "application/xml",
            "txt" => "text/plain",
            "html" | "htm" => "text/html",
            "zip" => "application/zip",
            "tiff" | "tif" => "image/tiff",
            _ => "application/octet-stream",
        }
    }
}

#[async_trait]
impl DocumentStorage for S3DocumentStorage {
    async fn upload(
        &self,
        tenant_id: String,
        user_id: String,
        doc_type: DocumentType,
        data: Vec<u8>,
        extension: &str,
    ) -> StorageResult<String> {
        let key = self.generate_key(tenant_id, user_id, doc_type, extension);
        let content_type = Self::content_type_for_extension(extension);
        let body = ByteStream::from(data);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(body)
            .content_type(content_type)
            .server_side_encryption(ServerSideEncryption::Aes256)
            .send()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;

        info!(key = %key, content_type = %content_type, "Document uploaded to S3");
        Ok(key)
    }

    async fn download(&self, url: &str) -> StorageResult<Vec<u8>> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(url)
            .send()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;

        let data = output
            .body
            .collect()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?
            .into_bytes();

        Ok(data.into())
    }

    async fn delete(&self, url: &str) -> StorageResult<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(url)
            .send()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;

        info!(key = %url, "Document deleted from S3");
        Ok(())
    }

    async fn generate_presigned_url(&self, url: &str, expiry: Duration) -> StorageResult<String> {
        let presigning_config = PresigningConfig::expires_in(expiry)
            .map_err(|e| StorageError::ConfigError(e.to_string()))?;

        let presigned_req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(url)
            .presigned(presigning_config)
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;

        Ok(presigned_req.uri().to_string())
    }
}

/// Local filesystem storage for development when no AWS credentials are available.
///
/// Stores documents under a configurable base directory, organized by
/// tenant_id/user_id/doc_type/. Presigned URLs return file:// paths.
#[derive(Clone)]
pub struct LocalFilesystemStorage {
    base_dir: std::path::PathBuf,
}

impl LocalFilesystemStorage {
    /// Create a new local filesystem storage backend.
    ///
    /// The base_dir will be created if it does not exist.
    pub fn new(base_dir: std::path::PathBuf) -> StorageResult<Self> {
        std::fs::create_dir_all(&base_dir)?;
        info!(base_dir = %base_dir.display(), "Local filesystem document storage initialized");
        Ok(Self { base_dir })
    }

    /// Validate that a URL/key does not escape the base directory via path traversal.
    fn validate_path(&self, url: &str) -> StorageResult<std::path::PathBuf> {
        // Reject obvious path traversal attempts
        if url.contains("..") || url.starts_with('/') || url.starts_with('\\') {
            return Err(StorageError::InvalidPath);
        }
        let filepath = self.base_dir.join(url);
        // Canonicalize to resolve any symlinks or remaining traversal
        // For non-existent files, check the parent directory
        let canonical_base = self
            .base_dir
            .canonicalize()
            .map_err(|_| StorageError::InvalidPath)?;
        let check_path = if filepath.exists() {
            filepath
                .canonicalize()
                .map_err(|_| StorageError::InvalidPath)?
        } else {
            // For new files, check that the parent is under base_dir
            let parent = filepath.parent().ok_or(StorageError::InvalidPath)?;
            if parent.exists() {
                let canonical_parent = parent
                    .canonicalize()
                    .map_err(|_| StorageError::InvalidPath)?;
                if !canonical_parent.starts_with(&canonical_base) {
                    return Err(StorageError::InvalidPath);
                }
                return Ok(filepath);
            }
            return Ok(filepath);
        };
        if !check_path.starts_with(&canonical_base) {
            return Err(StorageError::InvalidPath);
        }
        Ok(check_path)
    }

    /// Create from environment, using `DOCUMENT_STORAGE_DIR` or defaulting to `./data/documents`.
    pub fn from_env() -> StorageResult<Self> {
        let base_dir = std::env::var("DOCUMENT_STORAGE_DIR")
            .unwrap_or_else(|_| "./data/documents".to_string());
        Self::new(std::path::PathBuf::from(base_dir))
    }
}

#[async_trait]
impl DocumentStorage for LocalFilesystemStorage {
    async fn upload(
        &self,
        tenant_id: String,
        user_id: String,
        doc_type: DocumentType,
        data: Vec<u8>,
        extension: &str,
    ) -> StorageResult<String> {
        let file_id = Uuid::new_v4();
        let dir = self
            .base_dir
            .join(&tenant_id)
            .join(&user_id)
            .join(doc_type.to_string());
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| StorageError::IoError(e))?;

        let filename = format!("{}.{}", file_id, extension);
        let filepath = dir.join(&filename);

        tokio::fs::write(&filepath, &data)
            .await
            .map_err(|e| StorageError::IoError(e))?;

        // Return a relative key similar to S3 format
        let key = format!("{}/{}/{}/{}", tenant_id, user_id, doc_type, filename);
        info!(key = %key, "Document stored to local filesystem");
        Ok(key)
    }

    async fn download(&self, url: &str) -> StorageResult<Vec<u8>> {
        let filepath = self.validate_path(url)?;
        if !filepath.exists() {
            return Err(StorageError::NotFound(url.to_string()));
        }
        tokio::fs::read(&filepath)
            .await
            .map_err(|e| StorageError::IoError(e))
    }

    async fn delete(&self, url: &str) -> StorageResult<()> {
        let filepath = self.validate_path(url)?;
        if filepath.exists() {
            tokio::fs::remove_file(&filepath)
                .await
                .map_err(|e| StorageError::IoError(e))?;
        }
        Ok(())
    }

    async fn generate_presigned_url(&self, url: &str, _expiry: Duration) -> StorageResult<String> {
        let filepath = self.validate_path(url)?;
        if !filepath.exists() {
            return Err(StorageError::NotFound(url.to_string()));
        }
        // Return a file:// URL for local development
        Ok(format!("file://{}", filepath.display()))
    }
}
