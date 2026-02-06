use super::{DocumentStorage, DocumentType, StorageError, StorageResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct MockDocumentStorage {
    storage: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    delay: Duration,
}

impl MockDocumentStorage {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            delay: Duration::from_millis(0),
        }
    }

    #[cfg(test)] // Only used in tests
    pub fn with_delay(delay: Duration) -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            delay,
        }
    }
}

#[async_trait]
impl DocumentStorage for MockDocumentStorage {
    async fn upload(
        &self,
        tenant_id: String,
        user_id: String,
        doc_type: DocumentType,
        data: Vec<u8>,
        extension: &str,
    ) -> StorageResult<String> {
        if !self.delay.is_zero() {
            sleep(self.delay).await;
        }

        let file_id = Uuid::new_v4();
        let key = format!(
            "{}/{}/{}/{}.{}",
            tenant_id, user_id, doc_type, file_id, extension
        );

        self.storage.lock()
            .map_err(|e| StorageError::BackendError(format!("Storage lock poisoned: {}", e)))?
            .insert(key.clone(), data);
        Ok(key)
    }

    async fn download(&self, url: &str) -> StorageResult<Vec<u8>> {
        if !self.delay.is_zero() {
            sleep(self.delay).await;
        }

        let storage = self.storage.lock()
            .map_err(|e| StorageError::BackendError(format!("Storage lock poisoned: {}", e)))?;
        storage
            .get(url)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(url.to_string()))
    }

    async fn delete(&self, url: &str) -> StorageResult<()> {
        if !self.delay.is_zero() {
            sleep(self.delay).await;
        }

        let mut storage = self.storage.lock()
            .map_err(|e| StorageError::BackendError(format!("Storage lock poisoned: {}", e)))?;
        storage.remove(url);
        Ok(())
    }

    async fn generate_presigned_url(&self, url: &str, _expiry: Duration) -> StorageResult<String> {
        // For mock, just return a fake URL
        Ok(format!("https://mock-s3.local/{}", url))
    }
}
