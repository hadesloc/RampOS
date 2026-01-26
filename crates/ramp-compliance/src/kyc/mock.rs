use async_trait::async_trait;
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;

use super::{KycProvider, KycVerificationRequest, KycVerificationResult};
use crate::types::{KycStatus, KycTier};
use ramp_common::Result;

/// Mock verification record stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockVerificationRecord {
    pub reference: String,
    pub user_id: String,
    pub tier: KycTier,
    pub status: KycStatus,
    pub full_name: String,
    pub id_number: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub rejection_reason: Option<String>,
}

/// Configuration for mock KYC provider behavior
#[derive(Debug, Clone)]
pub struct MockKycConfig {
    /// Probability of auto-approval (0.0 - 1.0)
    pub approval_probability: f64,
    /// Probability of pending (needs manual review)
    pub pending_probability: f64,
    /// Simulated processing delay in milliseconds
    pub processing_delay_ms: u64,
    /// ID numbers that should always be rejected
    pub blocked_ids: Vec<String>,
    /// ID numbers that should always be pending
    pub pending_ids: Vec<String>,
    /// ID numbers that should always be approved
    pub approved_ids: Vec<String>,
}

impl Default for MockKycConfig {
    fn default() -> Self {
        Self {
            approval_probability: 0.8,
            pending_probability: 0.15,
            processing_delay_ms: 100,
            blocked_ids: vec!["000000000000".to_string()], // Test blocked ID
            pending_ids: vec!["111111111111".to_string()], // Test pending ID
            approved_ids: vec!["123456789012".to_string()], // Test approved ID
        }
    }
}

/// Mock KYC provider for testing
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

    pub fn with_default_config() -> Self {
        Self::new(MockKycConfig::default())
    }

    /// Get all stored verifications (for testing)
    pub fn get_all_verifications(&self) -> Vec<MockVerificationRecord> {
        self.verifications
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    /// Manually set verification status (for testing)
    pub fn set_verification_status(
        &self,
        reference: &str,
        status: KycStatus,
        reason: Option<String>,
    ) {
        if let Some(record) = self.verifications.lock().unwrap().get_mut(reference) {
            record.status = status;
            record.rejection_reason = reason;
            record.updated_at = Utc::now();
        }
    }

    fn determine_status(&self, id_number: &str) -> (KycStatus, Option<String>) {
        // Check deterministic lists first
        if self.config.blocked_ids.contains(&id_number.to_string()) {
            return (
                KycStatus::Rejected,
                Some("ID number is on blocked list".to_string()),
            );
        }

        if self.config.pending_ids.contains(&id_number.to_string()) {
            return (KycStatus::Pending, None);
        }

        if self.config.approved_ids.contains(&id_number.to_string()) {
            return (KycStatus::Approved, None);
        }

        // Otherwise use probability
        let mut rng = rand::thread_rng();
        let roll: f64 = rng.gen();

        if roll < self.config.approval_probability {
            (KycStatus::Approved, None)
        } else if roll < self.config.approval_probability + self.config.pending_probability {
            (KycStatus::Pending, None)
        } else {
            let reasons = [
                "Document image quality too low",
                "Name mismatch between ID and selfie",
                "ID document appears to be expired",
                "Face verification failed",
                "Suspected fraudulent document",
            ];
            let reason = reasons[rng.gen_range(0..reasons.len())].to_string();
            (KycStatus::Rejected, Some(reason))
        }
    }
}

#[async_trait]
impl KycProvider for MockKycProvider {
    async fn verify(&self, request: &KycVerificationRequest) -> Result<KycVerificationResult> {
        // Simulate processing delay
        if self.config.processing_delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.processing_delay_ms,
            ))
            .await;
        }

        let reference = format!("mock_kyc_{}", uuid::Uuid::now_v7());
        let (status, rejection_reason) = self.determine_status(&request.id_number);

        let record = MockVerificationRecord {
            reference: reference.clone(),
            user_id: request.user_id.to_string(),
            tier: request.tier,
            status,
            full_name: request.full_name.clone(),
            id_number: request.id_number.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            rejection_reason: rejection_reason.clone(),
        };

        // Store for later lookup
        self.verifications
            .lock()
            .unwrap()
            .insert(reference.clone(), record);

        info!(
            reference = %reference,
            user_id = %request.user_id,
            tier = ?request.tier,
            status = ?status,
            "Mock KYC verification processed"
        );

        Ok(KycVerificationResult {
            status,
            verified_tier: if status == KycStatus::Approved {
                Some(request.tier)
            } else {
                None
            },
            rejection_reason,
            provider_reference: Some(reference),
        })
    }

    async fn check_status(&self, reference: &str) -> Result<KycVerificationResult> {
        let verifications = self.verifications.lock().unwrap();

        if let Some(record) = verifications.get(reference) {
            Ok(KycVerificationResult {
                status: record.status,
                verified_tier: if record.status == KycStatus::Approved {
                    Some(record.tier)
                } else {
                    None
                },
                rejection_reason: record.rejection_reason.clone(),
                provider_reference: Some(reference.to_string()),
            })
        } else {
            Ok(KycVerificationResult {
                status: KycStatus::Pending,
                verified_tier: None,
                rejection_reason: None,
                provider_reference: None,
            })
        }
    }
}

/// KYC Workflow states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KycWorkflowState {
    /// Initial state
    Created,
    /// Documents submitted, waiting for verification
    Submitted,
    /// Verification in progress
    InProgress,
    /// Waiting for manual review
    PendingReview,
    /// Approved, tier upgraded
    Approved,
    /// Rejected
    Rejected,
    /// Expired (documents too old or timeout)
    Expired,
}

impl KycWorkflowState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            KycWorkflowState::Approved | KycWorkflowState::Rejected | KycWorkflowState::Expired
        )
    }

    pub fn allowed_transitions(&self) -> Vec<KycWorkflowState> {
        match self {
            KycWorkflowState::Created => vec![KycWorkflowState::Submitted],
            KycWorkflowState::Submitted => {
                vec![KycWorkflowState::InProgress, KycWorkflowState::Expired]
            }
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
            // Terminal states
            KycWorkflowState::Approved => vec![],
            KycWorkflowState::Rejected => vec![KycWorkflowState::Created], // Allow retry
            KycWorkflowState::Expired => vec![KycWorkflowState::Created],  // Allow retry
        }
    }

    pub fn can_transition_to(&self, target: KycWorkflowState) -> bool {
        self.allowed_transitions().contains(&target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ramp_common::types::{TenantId, UserId};

    #[tokio::test]
    async fn test_mock_kyc_approved() {
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
    async fn test_mock_kyc_rejected() {
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

    #[test]
    fn test_workflow_transitions() {
        assert!(KycWorkflowState::Created.can_transition_to(KycWorkflowState::Submitted));
        assert!(!KycWorkflowState::Created.can_transition_to(KycWorkflowState::Approved));
        assert!(KycWorkflowState::Approved.is_terminal());
    }
}
