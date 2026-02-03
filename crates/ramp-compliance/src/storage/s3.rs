use super::{DocumentStorage, DocumentType, StorageError, StorageResult};
use async_trait::async_trait;
use aws_config::SdkConfig;
use aws_sdk_s3::{
    presigning::PresigningConfig,
    primitives::ByteStream, // config::Region unused
    types::ServerSideEncryption,
    Client,
};
use std::time::Duration;
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

    pub async fn from_env(bucket: String) -> StorageResult<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);
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
        let body = ByteStream::from(data);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(body)
            .server_side_encryption(ServerSideEncryption::Aes256)
            .send()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;

        Ok(key)
    }

    async fn download(&self, url: &str) -> StorageResult<Vec<u8>> {
        // Assuming url is the key
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
