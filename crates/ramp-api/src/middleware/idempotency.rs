//! Idempotency key middleware for safe request retries using Redis or Memory

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

use crate::middleware::tenant::TenantContext;

/// Idempotency configuration
#[derive(Debug, Clone)]
pub struct IdempotencyConfig {
    /// TTL for idempotency keys in seconds (default: 24 hours)
    pub ttl_seconds: u64,
    /// Redis key prefix
    pub key_prefix: String,
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: 86400, // 24 hours
            key_prefix: "idempotency".to_string(),
        }
    }
}

/// Stored response for idempotency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredResponse {
    pub status_code: u16,
    pub body: String,
    pub content_type: String,
}

/// Abstract store for idempotency
#[async_trait::async_trait]
pub trait IdempotencyStore: Send + Sync {
    async fn get(&self, tenant_id: &str, key: &str, key_prefix: &str) -> Option<StoredResponse>;
    async fn store(
        &self,
        tenant_id: &str,
        key: &str,
        response: &StoredResponse,
        ttl_seconds: u64,
        key_prefix: &str,
    ) -> Result<(), String>;
    async fn try_lock(&self, tenant_id: &str, key: &str, key_prefix: &str) -> Result<bool, String>;
    async fn unlock(&self, tenant_id: &str, key: &str, key_prefix: &str) -> Result<(), String>;
}

/// Redis implementation
pub struct RedisIdempotencyStore {
    redis: Arc<redis::aio::ConnectionManager>,
}

impl RedisIdempotencyStore {
    pub fn new(redis: redis::aio::ConnectionManager) -> Self {
        Self {
            redis: Arc::new(redis),
        }
    }
}

#[async_trait::async_trait]
impl IdempotencyStore for RedisIdempotencyStore {
    async fn get(&self, tenant_id: &str, key: &str, key_prefix: &str) -> Option<StoredResponse> {
        let full_key = format!("{}:{}:{}", key_prefix, tenant_id, key);
        let mut conn = (*self.redis).clone();

        use redis::AsyncCommands;
        match conn.get::<_, Option<String>>(&full_key).await {
            Ok(Some(data)) => serde_json::from_str(&data).ok(),
            _ => None,
        }
    }

    async fn store(
        &self,
        tenant_id: &str,
        key: &str,
        response: &StoredResponse,
        ttl_seconds: u64,
        key_prefix: &str,
    ) -> Result<(), String> {
        let full_key = format!("{}:{}:{}", key_prefix, tenant_id, key);
        let mut conn = (*self.redis).clone();

        let data = serde_json::to_string(response).map_err(|e| e.to_string())?;

        use redis::AsyncCommands;
        conn.set_ex::<_, _, ()>(&full_key, &data, ttl_seconds)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn try_lock(&self, tenant_id: &str, key: &str, key_prefix: &str) -> Result<bool, String> {
        let lock_key = format!("{}:{}:{}:lock", key_prefix, tenant_id, key);
        let mut conn = (*self.redis).clone();

        // SET NX with 60 second expiry for lock
        let result: Option<String> = redis::cmd("SET")
            .arg(&lock_key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(60)
            .query_async(&mut conn)
            .await
            .map_err(|e| e.to_string())?;

        Ok(result.is_some())
    }

    async fn unlock(&self, tenant_id: &str, key: &str, key_prefix: &str) -> Result<(), String> {
        let lock_key = format!("{}:{}:{}:lock", key_prefix, tenant_id, key);
        let mut conn = (*self.redis).clone();

        use redis::AsyncCommands;
        conn.del::<_, ()>(&lock_key)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

/// In-memory implementation
pub struct MemoryIdempotencyStore {
    // key -> (StoredResponse, expires_at_timestamp)
    responses: Arc<Mutex<HashMap<String, (StoredResponse, u64)>>>,
    // key -> expires_at_timestamp
    locks: Arc<Mutex<HashMap<String, u64>>>,
}

impl Default for MemoryIdempotencyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryIdempotencyStore {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl IdempotencyStore for MemoryIdempotencyStore {
    async fn get(&self, tenant_id: &str, key: &str, key_prefix: &str) -> Option<StoredResponse> {
        let full_key = format!("{}:{}:{}", key_prefix, tenant_id, key);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut responses = self.responses.lock().unwrap_or_else(|e| {
            warn!("Idempotency in-memory responses mutex poisoned");
            e.into_inner()
        });
        if let Some((response, expires_at)) = responses.get(&full_key) {
            if *expires_at > now {
                return Some(response.clone());
            } else {
                responses.remove(&full_key);
            }
        }
        None
    }

    async fn store(
        &self,
        tenant_id: &str,
        key: &str,
        response: &StoredResponse,
        ttl_seconds: u64,
        key_prefix: &str,
    ) -> Result<(), String> {
        let full_key = format!("{}:{}:{}", key_prefix, tenant_id, key);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut responses = self.responses.lock().unwrap_or_else(|e| {
            warn!("Idempotency in-memory responses mutex poisoned");
            e.into_inner()
        });
        responses.insert(full_key, (response.clone(), now + ttl_seconds));
        Ok(())
    }

    async fn try_lock(&self, tenant_id: &str, key: &str, key_prefix: &str) -> Result<bool, String> {
        let lock_key = format!("{}:{}:{}:lock", key_prefix, tenant_id, key);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut locks = self.locks.lock().unwrap_or_else(|e| {
            warn!("Idempotency in-memory locks mutex poisoned");
            e.into_inner()
        });
        // Check if locked
        if let Some(expires_at) = locks.get(&lock_key) {
            if *expires_at > now {
                return Ok(false);
            }
        }

        // Acquire lock
        locks.insert(lock_key, now + 60);
        Ok(true)
    }

    async fn unlock(&self, tenant_id: &str, key: &str, key_prefix: &str) -> Result<(), String> {
        let lock_key = format!("{}:{}:{}:lock", key_prefix, tenant_id, key);
        let mut locks = self.locks.lock().unwrap_or_else(|e| {
            warn!("Idempotency in-memory locks mutex poisoned");
            e.into_inner()
        });
        locks.remove(&lock_key);
        Ok(())
    }
}

/// Idempotency handler
#[derive(Clone)]
pub struct IdempotencyHandler {
    store: Arc<dyn IdempotencyStore>,
    config: IdempotencyConfig,
}

impl IdempotencyHandler {
    pub fn new(store: Arc<dyn IdempotencyStore>, config: IdempotencyConfig) -> Self {
        Self { store, config }
    }

    pub fn with_redis(redis: redis::aio::ConnectionManager, config: IdempotencyConfig) -> Self {
        Self::new(Arc::new(RedisIdempotencyStore::new(redis)), config)
    }

    pub fn with_memory(config: IdempotencyConfig) -> Self {
        Self::new(Arc::new(MemoryIdempotencyStore::new()), config)
    }

    /// Get stored response for idempotency key
    pub async fn get(&self, tenant_id: &str, key: &str) -> Option<StoredResponse> {
        self.store
            .get(tenant_id, key, &self.config.key_prefix)
            .await
    }

    /// Store response for idempotency key
    pub async fn store(
        &self,
        tenant_id: &str,
        key: &str,
        response: &StoredResponse,
    ) -> Result<(), String> {
        self.store
            .store(
                tenant_id,
                key,
                response,
                self.config.ttl_seconds,
                &self.config.key_prefix,
            )
            .await
    }

    /// Mark request as in-progress (to prevent concurrent requests with same key)
    pub async fn try_lock(&self, tenant_id: &str, key: &str) -> Result<bool, String> {
        self.store
            .try_lock(tenant_id, key, &self.config.key_prefix)
            .await
    }

    /// Release lock
    pub async fn unlock(&self, tenant_id: &str, key: &str) -> Result<(), String> {
        self.store
            .unlock(tenant_id, key, &self.config.key_prefix)
            .await
    }
}

/// Idempotency middleware
///
/// Expects header: Idempotency-Key: <unique-key>
///
/// Behavior:
/// 1. If no key provided, proceed normally (no idempotency)
/// 2. If key exists and has stored response, return stored response
/// 3. If key exists but request in progress, return 409 Conflict
/// 4. Otherwise, process request and store response
pub async fn idempotency_middleware(
    State(handler): State<Arc<IdempotencyHandler>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Only apply to POST and PATCH methods
    if req.method() != axum::http::Method::POST && req.method() != axum::http::Method::PATCH {
        return Ok(next.run(req).await);
    }

    // Extract idempotency key from header
    let idempotency_key = req
        .headers()
        .get("X-Idempotency-Key")
        .or_else(|| req.headers().get("Idempotency-Key"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // If no idempotency key, proceed normally
    let idempotency_key = match idempotency_key {
        Some(key) if !key.is_empty() => key,
        _ => return Ok(next.run(req).await),
    };

    // Get tenant ID
    let tenant_id = req
        .extensions()
        .get::<TenantContext>()
        .map(|ctx| ctx.tenant_id.0.clone())
        .unwrap_or_else(|| "anonymous".to_string());

    // Check if we have a stored response
    if let Some(stored) = handler.get(&tenant_id, &idempotency_key).await {
        info!(
            tenant = %tenant_id,
            key = %idempotency_key,
            "Returning cached idempotent response"
        );

        let status = StatusCode::from_u16(stored.status_code).unwrap_or(StatusCode::OK);
        let response = Response::builder()
            .status(status)
            .header("Content-Type", stored.content_type)
            .header("Idempotent-Replayed", "true")
            .body(Body::from(stored.body))
            .unwrap();

        return Ok(response);
    }

    // Try to acquire lock
    match handler.try_lock(&tenant_id, &idempotency_key).await {
        Ok(true) => {
            // Got the lock, proceed with request
        }
        Ok(false) => {
            // Another request with same key is in progress
            warn!(
                tenant = %tenant_id,
                key = %idempotency_key,
                "Concurrent request with same idempotency key"
            );
            return Err(StatusCode::CONFLICT);
        }
        Err(e) => {
            warn!(error = %e, "Idempotency lock error");
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    // Process the request
    let response = next.run(req).await;

    // Store the response
    let (parts, body) = response.into_parts();
    let body_bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .unwrap_or_default();
    let body_string = String::from_utf8_lossy(&body_bytes).to_string();

    let content_type = parts
        .headers
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    let stored = StoredResponse {
        status_code: parts.status.as_u16(),
        body: body_string.clone(),
        content_type: content_type.clone(),
    };

    // Store response (best effort)
    if let Err(e) = handler.store(&tenant_id, &idempotency_key, &stored).await {
        warn!(error = %e, "Failed to store idempotent response; lock retained");
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "idempotency_store_unavailable",
                "message": "Idempotency store error; request may have been processed"
            })),
        )
            .into_response());
    }

    // Release lock
    let _ = handler.unlock(&tenant_id, &idempotency_key).await;

    // Rebuild response
    let response = Response::from_parts(parts, Body::from(body_bytes));

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotency_config_default() {
        let config = IdempotencyConfig::default();
        assert_eq!(config.ttl_seconds, 86400);
    }

    #[test]
    fn test_stored_response_serialize() {
        let stored = StoredResponse {
            status_code: 200,
            body: r#"{"id": "123"}"#.to_string(),
            content_type: "application/json".to_string(),
        };

        let json = serde_json::to_string(&stored).unwrap();
        let parsed: StoredResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.status_code, 200);
        assert_eq!(parsed.body, r#"{"id": "123"}"#);
    }
}
