# KYC Provider Integration Guide

This guide explains how to implement a custom KYC (Know Your Customer) provider for RampOS. KYC providers handle identity verification, document validation, and tier management.

## Overview

The KYC system in RampOS consists of:

- **KycProvider**: Core trait for verification logic
- **KycService**: Service layer that orchestrates verification and document storage
- **DocumentStorage**: Interface for secure document storage
- **TierManager**: Manages user KYC tiers and limits

## Architecture

```
                      KycService
                          |
          +---------------+---------------+
          |               |               |
    KycProvider    DocumentStorage   TierManager
          |               |               |
     +----+----+    +-----+-----+         |
     |         |    |           |         |
   Mock     eKYC   Mock        S3      Tier Rules
  Provider  API   Storage   Storage
```

## KycProvider Trait

The core trait for implementing KYC verification:

```rust
use async_trait::async_trait;
use ramp_common::{
    types::{TenantId, UserId},
    Result,
};

/// KYC verification request
#[derive(Debug, Clone)]
pub struct KycVerificationRequest {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub tier: KycTier,
    pub full_name: String,
    pub date_of_birth: String,
    pub id_number: String,
    pub id_type: String,  // CCCD, PASSPORT, DRIVER_LICENSE
    pub documents: Vec<KycDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycDocument {
    pub doc_type: String,
    pub file_hash: String,
    pub storage_url: String,
}

/// KYC verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycVerificationResult {
    pub status: KycStatus,
    pub verified_tier: Option<KycTier>,
    pub rejection_reason: Option<String>,
    pub provider_reference: Option<String>,
}

#[async_trait]
pub trait KycProvider: Send + Sync {
    /// Submit verification request
    async fn verify(
        &self,
        request: &KycVerificationRequest,
    ) -> Result<KycVerificationResult>;

    /// Check verification status by provider reference
    async fn check_status(
        &self,
        reference: &str,
    ) -> Result<KycVerificationResult>;
}
```

## KYC Tiers

RampOS uses a tiered KYC system:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KycTier {
    /// Basic tier: phone verification only
    Tier0,
    /// Standard tier: ID verification
    Tier1,
    /// Enhanced tier: ID + proof of address
    Tier2,
    /// Full tier: all documents + liveness check
    Tier3,
}

impl KycTier {
    pub fn daily_payin_limit_vnd(&self) -> Decimal {
        match self {
            KycTier::Tier0 => Decimal::from(5_000_000),      // 5M VND
            KycTier::Tier1 => Decimal::from(50_000_000),     // 50M VND
            KycTier::Tier2 => Decimal::from(200_000_000),    // 200M VND
            KycTier::Tier3 => Decimal::from(1_000_000_000),  // 1B VND
        }
    }

    pub fn daily_payout_limit_vnd(&self) -> Decimal {
        match self {
            KycTier::Tier0 => Decimal::from(2_000_000),      // 2M VND
            KycTier::Tier1 => Decimal::from(30_000_000),     // 30M VND
            KycTier::Tier2 => Decimal::from(100_000_000),    // 100M VND
            KycTier::Tier3 => Decimal::from(500_000_000),    // 500M VND
        }
    }

    pub fn single_transaction_limit_vnd(&self) -> Decimal {
        match self {
            KycTier::Tier0 => Decimal::from(2_000_000),      // 2M VND
            KycTier::Tier1 => Decimal::from(20_000_000),     // 20M VND
            KycTier::Tier2 => Decimal::from(50_000_000),     // 50M VND
            KycTier::Tier3 => Decimal::from(200_000_000),    // 200M VND
        }
    }
}
```

## Implementing a Custom KYC Provider

### Step 1: Create the Provider Struct

```rust
use async_trait::async_trait;
use ramp_compliance::kyc::{
    KycProvider, KycVerificationRequest, KycVerificationResult,
};
use ramp_compliance::types::{KycStatus, KycTier};
use ramp_common::Result;
use tracing::{info, warn};

pub struct EkycVietnamProvider {
    api_base_url: String,
    api_key: String,
    api_secret: String,
    http_client: reqwest::Client,
}

impl EkycVietnamProvider {
    pub fn new(
        api_base_url: String,
        api_key: String,
        api_secret: String,
    ) -> Self {
        Self {
            api_base_url,
            api_key,
            api_secret,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Generate authentication header
    fn auth_header(&self) -> String {
        use base64::Engine;
        let credentials = format!("{}:{}", self.api_key, self.api_secret);
        format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(credentials))
    }
}
```

### Step 2: Implement the KycProvider Trait

```rust
#[async_trait]
impl KycProvider for EkycVietnamProvider {
    async fn verify(
        &self,
        request: &KycVerificationRequest,
    ) -> Result<KycVerificationResult> {
        info!(
            user_id = %request.user_id,
            tier = ?request.tier,
            "Submitting eKYC verification"
        );

        // Build request for eKYC API
        let ekyc_request = serde_json::json!({
            "reference_id": format!("{}_{}", request.tenant_id.0, request.user_id.0),
            "full_name": request.full_name,
            "date_of_birth": request.date_of_birth,
            "id_type": self.map_id_type(&request.id_type),
            "id_number": request.id_number,
            "documents": request.documents.iter().map(|d| {
                serde_json::json!({
                    "type": d.doc_type,
                    "url": d.storage_url,
                    "hash": d.file_hash,
                })
            }).collect::<Vec<_>>(),
            "verification_level": self.map_tier_to_level(request.tier),
        });

        // Submit to eKYC API
        let response = self.http_client
            .post(format!("{}/v1/verifications", self.api_base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&ekyc_request)
            .send()
            .await
            .map_err(|e| ramp_common::Error::External(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!(error = %error_text, "eKYC API error");
            return Err(ramp_common::Error::External(
                format!("eKYC API error: {}", error_text)
            ));
        }

        let ekyc_response: serde_json::Value = response.json().await
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        // Parse response
        let status = self.parse_status(&ekyc_response);
        let verified_tier = if status == KycStatus::Approved {
            Some(request.tier)
        } else {
            None
        };

        Ok(KycVerificationResult {
            status,
            verified_tier,
            rejection_reason: ekyc_response["rejection_reason"]
                .as_str()
                .map(String::from),
            provider_reference: ekyc_response["verification_id"]
                .as_str()
                .map(String::from),
        })
    }

    async fn check_status(
        &self,
        reference: &str,
    ) -> Result<KycVerificationResult> {
        let response = self.http_client
            .get(format!(
                "{}/v1/verifications/{}",
                self.api_base_url, reference
            ))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| ramp_common::Error::External(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ramp_common::Error::External(
                "Failed to check verification status".to_string()
            ));
        }

        let ekyc_response: serde_json::Value = response.json().await
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        let status = self.parse_status(&ekyc_response);
        let tier = ekyc_response["verification_level"]
            .as_str()
            .and_then(|l| self.parse_level_to_tier(l));

        Ok(KycVerificationResult {
            status,
            verified_tier: if status == KycStatus::Approved { tier } else { None },
            rejection_reason: ekyc_response["rejection_reason"]
                .as_str()
                .map(String::from),
            provider_reference: Some(reference.to_string()),
        })
    }
}

impl EkycVietnamProvider {
    fn map_id_type(&self, id_type: &str) -> &str {
        match id_type {
            "CCCD" => "citizen_id",
            "PASSPORT" => "passport",
            "DRIVER_LICENSE" => "driver_license",
            _ => "other",
        }
    }

    fn map_tier_to_level(&self, tier: KycTier) -> &str {
        match tier {
            KycTier::Tier0 => "basic",
            KycTier::Tier1 => "standard",
            KycTier::Tier2 => "enhanced",
            KycTier::Tier3 => "full",
        }
    }

    fn parse_level_to_tier(&self, level: &str) -> Option<KycTier> {
        match level {
            "basic" => Some(KycTier::Tier0),
            "standard" => Some(KycTier::Tier1),
            "enhanced" => Some(KycTier::Tier2),
            "full" => Some(KycTier::Tier3),
            _ => None,
        }
    }

    fn parse_status(&self, response: &serde_json::Value) -> KycStatus {
        match response["status"].as_str().unwrap_or("") {
            "approved" | "verified" => KycStatus::Approved,
            "rejected" | "failed" => KycStatus::Rejected,
            "pending" | "in_review" => KycStatus::Pending,
            "submitted" => KycStatus::Submitted,
            "expired" => KycStatus::Expired,
            _ => KycStatus::Pending,
        }
    }
}
```

## Webhook Handling

KYC providers typically send webhooks for async verification results.

### Webhook Event Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KycWebhookEvent {
    VerificationCompleted {
        reference: String,
        status: KycStatus,
        verified_tier: Option<KycTier>,
    },
    VerificationFailed {
        reference: String,
        reason: String,
    },
    DocumentReceived {
        reference: String,
        doc_type: String,
    },
    ManualReviewRequired {
        reference: String,
        reason: String,
    },
}
```

### Webhook Handler

```rust
use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
};
use tracing::{info, error};

pub async fn handle_kyc_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> StatusCode {
    // Verify webhook signature
    let signature = headers
        .get("X-Webhook-Signature")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();

    if !verify_kyc_webhook_signature(
        &state.kyc_webhook_secret,
        signature,
        &payload_bytes,
    ) {
        error!("Invalid KYC webhook signature");
        return StatusCode::UNAUTHORIZED;
    }

    // Parse webhook event
    let event_type = payload["event"].as_str().unwrap_or("");
    let reference = payload["reference_id"].as_str().unwrap_or("");

    info!(
        event_type = %event_type,
        reference = %reference,
        "Received KYC webhook"
    );

    match event_type {
        "verification.completed" => {
            let status = parse_kyc_status(&payload["data"]["status"]);
            let tier = payload["data"]["level"]
                .as_str()
                .and_then(parse_tier);

            // Update user KYC status in database
            if let Err(e) = state.kyc_service
                .update_verification_status(reference, status, tier)
                .await
            {
                error!(error = %e, "Failed to update KYC status");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }

            // Trigger webhook to tenant
            if let Err(e) = state.webhook_service
                .queue_kyc_status_event(reference, status, tier)
                .await
            {
                error!(error = %e, "Failed to queue KYC webhook");
            }
        }
        "verification.failed" => {
            let reason = payload["data"]["reason"]
                .as_str()
                .unwrap_or("Unknown reason");

            if let Err(e) = state.kyc_service
                .mark_verification_failed(reference, reason)
                .await
            {
                error!(error = %e, "Failed to mark verification as failed");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        "manual_review.required" => {
            // Create compliance case for manual review
            if let Err(e) = state.compliance_service
                .create_kyc_review_case(reference, &payload["data"])
                .await
            {
                error!(error = %e, "Failed to create review case");
            }
        }
        _ => {
            info!(event_type = %event_type, "Unknown KYC webhook event type");
        }
    }

    StatusCode::OK
}

fn verify_kyc_webhook_signature(
    secret: &str,
    signature: &str,
    payload: &[u8],
) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Parse signature: "t=<timestamp>,v1=<signature>"
    let parts: Vec<&str> = signature.split(',').collect();
    if parts.len() < 2 {
        return false;
    }

    let timestamp = parts[0].strip_prefix("t=").unwrap_or("");
    let sig = parts[1].strip_prefix("v1=").unwrap_or("");

    // Check timestamp is recent (5 min tolerance)
    let ts: i64 = timestamp.parse().unwrap_or(0);
    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        return false;
    }

    // Verify HMAC
    let message = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());

    let expected = hex::encode(mac.finalize().into_bytes());
    sig == expected
}
```

## Document Storage

The KYC system uses pluggable document storage.

### DocumentStorage Trait

```rust
#[async_trait]
pub trait DocumentStorage: Send + Sync {
    /// Upload a document
    async fn upload(
        &self,
        tenant_id: String,
        user_id: String,
        doc_type: DocumentType,
        data: Vec<u8>,
        extension: &str,
    ) -> StorageResult<String>;

    /// Download document content
    async fn download(&self, url: &str) -> StorageResult<Vec<u8>>;

    /// Delete a document
    async fn delete(&self, url: &str) -> StorageResult<()>;

    /// Generate presigned URL for temporary access
    async fn generate_presigned_url(
        &self,
        url: &str,
        expiry: std::time::Duration,
    ) -> StorageResult<String>;
}
```

### Document Types

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentType {
    IdFront,
    IdBack,
    Selfie,
    ProofOfAddress,
    BankStatement,
    Report,
}
```

### S3 Storage Implementation

```rust
use aws_sdk_s3::{
    Client, presigning::PresigningConfig,
    primitives::ByteStream, types::ServerSideEncryption,
};

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
        let config = aws_config::load_defaults(
            aws_config::BehaviorVersion::latest()
        ).await;
        let client = Client::new(&config);
        Ok(Self { client, bucket })
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
        let output = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(url)
            .send()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;

        let data = output.body
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

    async fn generate_presigned_url(
        &self,
        url: &str,
        expiry: Duration,
    ) -> StorageResult<String> {
        let presigning_config = PresigningConfig::expires_in(expiry)
            .map_err(|e| StorageError::ConfigError(e.to_string()))?;

        let presigned_req = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(url)
            .presigned(presigning_config)
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;

        Ok(presigned_req.uri().to_string())
    }
}
```

## KYC Workflow State Machine

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KycWorkflowState {
    Created,
    Submitted,
    InProgress,
    PendingReview,
    Approved,
    Rejected,
    Expired,
}

impl KycWorkflowState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            KycWorkflowState::Approved |
            KycWorkflowState::Rejected |
            KycWorkflowState::Expired
        )
    }

    pub fn allowed_transitions(&self) -> Vec<KycWorkflowState> {
        match self {
            KycWorkflowState::Created => vec![
                KycWorkflowState::Submitted
            ],
            KycWorkflowState::Submitted => vec![
                KycWorkflowState::InProgress,
                KycWorkflowState::Expired,
            ],
            KycWorkflowState::InProgress => vec![
                KycWorkflowState::PendingReview,
                KycWorkflowState::Approved,
                KycWorkflowState::Rejected,
            ],
            KycWorkflowState::PendingReview => vec![
                KycWorkflowState::Approved,
                KycWorkflowState::Rejected,
                KycWorkflowState::Expired,
            ],
            // Terminal states can retry
            KycWorkflowState::Approved => vec![],
            KycWorkflowState::Rejected => vec![KycWorkflowState::Created],
            KycWorkflowState::Expired => vec![KycWorkflowState::Created],
        }
    }

    pub fn can_transition_to(&self, target: KycWorkflowState) -> bool {
        self.allowed_transitions().contains(&target)
    }
}
```

## Using KycService

```rust
use ramp_compliance::kyc::{KycService, KycVerificationRequest};
use ramp_compliance::storage::S3DocumentStorage;

// Create the service
let storage = S3DocumentStorage::from_env("kyc-documents-bucket".to_string())
    .await?;
let provider = EkycVietnamProvider::new(
    "https://api.ekyc.vn".to_string(),
    env::var("EKYC_API_KEY")?,
    env::var("EKYC_API_SECRET")?,
);

let kyc_service = KycService::new(
    Box::new(provider),
    Box::new(storage),
);

// Upload documents
let id_front_url = kyc_service.upload_document(
    tenant_id.clone(),
    user_id.clone(),
    DocumentType::IdFront,
    id_front_bytes,
    "jpg",
).await?;

let selfie_url = kyc_service.upload_document(
    tenant_id.clone(),
    user_id.clone(),
    DocumentType::Selfie,
    selfie_bytes,
    "jpg",
).await?;

// Submit verification
let result = kyc_service.submit_verification(KycVerificationRequest {
    tenant_id,
    user_id,
    tier: KycTier::Tier1,
    full_name: "Nguyen Van A".to_string(),
    date_of_birth: "1990-01-15".to_string(),
    id_number: "001234567890".to_string(),
    id_type: "CCCD".to_string(),
    documents: vec![
        KycDocument {
            doc_type: "id_front".to_string(),
            file_hash: calculate_hash(&id_front_bytes),
            storage_url: id_front_url,
        },
        KycDocument {
            doc_type: "selfie".to_string(),
            file_hash: calculate_hash(&selfie_bytes),
            storage_url: selfie_url,
        },
    ],
}).await?;

println!("Verification status: {:?}", result.status);
println!("Reference: {:?}", result.provider_reference);
```

## Testing

### Mock Provider for Testing

```rust
pub struct MockKycProvider {
    config: MockKycConfig,
    verifications: Arc<Mutex<HashMap<String, MockVerificationRecord>>>,
}

impl MockKycProvider {
    pub fn new(config: MockKycConfig) -> Self {
        Self {
            config,
            verifications: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Manually set verification status for testing
    pub fn set_verification_status(
        &self,
        reference: &str,
        status: KycStatus,
        reason: Option<String>,
    ) {
        if let Some(record) = self.verifications.lock().unwrap().get_mut(reference) {
            record.status = status;
            record.rejection_reason = reason;
        }
    }
}

// Test configuration
#[derive(Debug, Clone)]
pub struct MockKycConfig {
    pub approval_probability: f64,
    pub pending_probability: f64,
    pub processing_delay_ms: u64,
    pub blocked_ids: Vec<String>,
    pub pending_ids: Vec<String>,
    pub approved_ids: Vec<String>,
}
```

### Unit Tests

```rust
#[tokio::test]
async fn test_kyc_verification_approved() {
    let provider = MockKycProvider::new(MockKycConfig {
        approved_ids: vec!["TEST123".to_string()],
        ..Default::default()
    });

    let request = KycVerificationRequest {
        tenant_id: TenantId::new("tenant_1"),
        user_id: UserId::new("user_1"),
        tier: KycTier::Tier1,
        full_name: "Test User".to_string(),
        date_of_birth: "1990-01-01".to_string(),
        id_number: "TEST123".to_string(),
        id_type: "CCCD".to_string(),
        documents: vec![],
    };

    let result = provider.verify(&request).await.unwrap();
    assert_eq!(result.status, KycStatus::Approved);
    assert_eq!(result.verified_tier, Some(KycTier::Tier1));
}

#[tokio::test]
async fn test_kyc_verification_rejected() {
    let provider = MockKycProvider::new(MockKycConfig {
        blocked_ids: vec!["BLOCKED".to_string()],
        ..Default::default()
    });

    let request = KycVerificationRequest {
        tenant_id: TenantId::new("tenant_1"),
        user_id: UserId::new("user_1"),
        tier: KycTier::Tier1,
        full_name: "Test User".to_string(),
        date_of_birth: "1990-01-01".to_string(),
        id_number: "BLOCKED".to_string(),
        id_type: "CCCD".to_string(),
        documents: vec![],
    };

    let result = provider.verify(&request).await.unwrap();
    assert_eq!(result.status, KycStatus::Rejected);
    assert!(result.rejection_reason.is_some());
}
```

## Configuration

```bash
# .env
KYC_PROVIDER=ekyc_vietnam
EKYC_API_BASE_URL=https://api.ekyc.vn
EKYC_API_KEY=your_api_key
EKYC_API_SECRET=your_api_secret
EKYC_WEBHOOK_SECRET=webhook_secret

# Document storage
KYC_STORAGE_BUCKET=your-kyc-bucket
AWS_REGION=ap-southeast-1
```

## Best Practices

1. **Document Security**: Always encrypt documents at rest (S3 SSE)
2. **Presigned URLs**: Use short-lived presigned URLs (15-60 minutes)
3. **Data Retention**: Implement document retention policies per regulation
4. **Audit Logging**: Log all KYC operations for compliance
5. **Error Handling**: Handle provider downtime gracefully
6. **Webhook Idempotency**: Use reference ID to prevent duplicate processing
7. **PII Protection**: Never log sensitive personal information
