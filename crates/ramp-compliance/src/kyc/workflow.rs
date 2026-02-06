use super::KycDocument;
use crate::types::{KycStatus, KycTier};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument}; // warn unused

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycVerificationWorkflowInput {
    pub tenant_id: String,
    pub user_id: String,
    pub tier_requested: KycTier,
    pub documents: Vec<KycDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycVerificationWorkflowResult {
    pub user_id: String,
    pub status: KycStatus,
    pub verified_tier: Option<KycTier>,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KycWorkflowSignal {
    ProviderCallback { result: ProviderCallbackResult },
    ManualApprove { reviewer_id: String },
    ManualReject { reviewer_id: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCallbackResult {
    pub status: KycStatus,
    pub verified_tier: Option<KycTier>,
    pub reason: Option<String>,
    pub provider_reference: Option<String>,
}

pub mod activities {
    use super::*;
    use ramp_common::Result;

    pub async fn validate_documents(_documents: &[KycDocument]) -> Result<bool> {
        info!("Validating {} documents", _documents.len());
        if _documents.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    pub async fn submit_to_provider(
        tenant_id: &str,
        user_id: &str,
        _documents: &[KycDocument],
    ) -> Result<String> {
        info!(%tenant_id, %user_id, "Submitting to KYC provider");
        Ok(format!(
            "VERIFY_{}_{}",
            user_id,
            chrono::Utc::now().timestamp()
        ))
    }

    pub async fn update_user_tier(tenant_id: &str, user_id: &str, tier: KycTier) -> Result<()> {
        info!(%tenant_id, %user_id, ?tier, "Updating user tier");
        Ok(())
    }

    pub async fn create_rejection_case(
        tenant_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<String> {
        info!(%tenant_id, %user_id, %reason, "Creating rejection case");
        Ok(format!("CASE_REJECT_{}", user_id))
    }

    pub async fn create_manual_review_case(
        tenant_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<String> {
        info!(%tenant_id, %user_id, %reason, "Creating manual review case");
        Ok(format!("CASE_REVIEW_{}", user_id))
    }
}

pub struct KycVerificationWorkflow;

impl KycVerificationWorkflow {
    #[instrument(skip(self, signal_provider), fields(user_id = %input.user_id))]
    pub async fn execute<F, Fut>(
        &self,
        input: KycVerificationWorkflowInput,
        signal_provider: F,
    ) -> Result<KycVerificationWorkflowResult, String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Option<KycWorkflowSignal>>,
    {
        info!("Starting KYC verification workflow");

        // Step 1: Validate documents
        match activities::validate_documents(&input.documents).await {
            Ok(valid) if valid => (),
            Ok(_) => {
                let reason = "Invalid documents submitted".to_string();
                let _ =
                    activities::create_rejection_case(&input.tenant_id, &input.user_id, &reason)
                        .await;
                return Ok(KycVerificationWorkflowResult {
                    user_id: input.user_id,
                    status: KycStatus::Rejected,
                    verified_tier: None,
                    rejection_reason: Some(reason),
                });
            }
            Err(e) => return Err(e.to_string()),
        }

        // Step 2: Submit to provider
        let verification_id = match activities::submit_to_provider(
            &input.tenant_id,
            &input.user_id,
            &input.documents,
        )
        .await
        {
            Ok(id) => id,
            Err(e) => return Err(e.to_string()),
        };

        info!(%verification_id, "Submitted to provider, waiting for signal");

        // Step 3: Wait for signal (Provider Callback or Manual Review)

        loop {
            let signal = signal_provider().await;

            match signal {
                Some(KycWorkflowSignal::ProviderCallback { result }) => {
                    match result.status {
                        KycStatus::Approved => {
                            let tier = result.verified_tier.unwrap_or(input.tier_requested);
                            if let Err(e) =
                                activities::update_user_tier(&input.tenant_id, &input.user_id, tier)
                                    .await
                            {
                                error!("Failed to update user tier: {}", e);
                                return Err(e.to_string());
                            }

                            return Ok(KycVerificationWorkflowResult {
                                user_id: input.user_id,
                                status: KycStatus::Approved,
                                verified_tier: Some(tier),
                                rejection_reason: None,
                            });
                        }
                        KycStatus::Rejected | KycStatus::Expired => {
                            let reason = result.reason.unwrap_or("Provider rejected".to_string());
                            let _ = activities::create_rejection_case(
                                &input.tenant_id,
                                &input.user_id,
                                &reason,
                            )
                            .await;

                            return Ok(KycVerificationWorkflowResult {
                                user_id: input.user_id,
                                status: KycStatus::Rejected,
                                verified_tier: None,
                                rejection_reason: Some(reason),
                            });
                        }
                        KycStatus::Pending | KycStatus::InProgress => {
                            // Provider indicates manual review needed
                            let _ = activities::create_manual_review_case(
                                &input.tenant_id,
                                &input.user_id,
                                "Provider requested manual review",
                            )
                            .await;
                            info!("Manual review case created, waiting for reviewer signal");
                            // Continue loop
                        }
                    }
                }
                Some(KycWorkflowSignal::ManualApprove { reviewer_id }) => {
                    info!(%reviewer_id, "Manually approved");
                    if let Err(e) = activities::update_user_tier(
                        &input.tenant_id,
                        &input.user_id,
                        input.tier_requested,
                    )
                    .await
                    {
                        return Err(e.to_string());
                    }
                    return Ok(KycVerificationWorkflowResult {
                        user_id: input.user_id,
                        status: KycStatus::Approved,
                        verified_tier: Some(input.tier_requested),
                        rejection_reason: None,
                    });
                }
                Some(KycWorkflowSignal::ManualReject {
                    reviewer_id,
                    reason,
                }) => {
                    info!(%reviewer_id, %reason, "Manually rejected");
                    let _ = activities::create_rejection_case(
                        &input.tenant_id,
                        &input.user_id,
                        &reason,
                    )
                    .await;
                    return Ok(KycVerificationWorkflowResult {
                        user_id: input.user_id,
                        status: KycStatus::Rejected,
                        verified_tier: None,
                        rejection_reason: Some(reason),
                    });
                }
                None => {
                    return Err("Signal timeout".to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kyc_workflow_approved() {
        let workflow = KycVerificationWorkflow;
        let input = KycVerificationWorkflowInput {
            tenant_id: "tenant_1".to_string(),
            user_id: "user_1".to_string(),
            tier_requested: KycTier::Tier1,
            documents: vec![KycDocument {
                doc_type: "ID_FRONT".to_string(),
                file_hash: "hash".to_string(),
                storage_url: "url".to_string(),
            }],
        };

        // Need to use internal mutability
        use std::sync::Mutex;
        let signals = Mutex::new(vec![Some(KycWorkflowSignal::ProviderCallback {
            result: ProviderCallbackResult {
                status: KycStatus::Approved,
                verified_tier: Some(KycTier::Tier1),
                reason: None,
                provider_reference: Some("ref".to_string()),
            },
        })]);

        let signal_provider = || async {
            let mut signals = signals.lock().unwrap();
            if !signals.is_empty() {
                signals.remove(0)
            } else {
                None
            }
        };

        let result = workflow.execute(input, signal_provider).await.expect("Workflow failed");
        assert_eq!(result.status, KycStatus::Approved);
        assert_eq!(result.verified_tier, Some(KycTier::Tier1));
    }

    #[tokio::test]
    async fn test_kyc_workflow_manual_review_approve() {
        let workflow = KycVerificationWorkflow;
        let input = KycVerificationWorkflowInput {
            tenant_id: "tenant_1".to_string(),
            user_id: "user_1".to_string(),
            tier_requested: KycTier::Tier1,
            documents: vec![KycDocument {
                doc_type: "ID_FRONT".to_string(),
                file_hash: "hash".to_string(),
                storage_url: "url".to_string(),
            }],
        };

        use std::sync::Mutex;
        let signals = Mutex::new(vec![
            Some(KycWorkflowSignal::ProviderCallback {
                result: ProviderCallbackResult {
                    status: KycStatus::Pending,
                    verified_tier: None,
                    reason: Some("Manual review needed".to_string()),
                    provider_reference: Some("ref".to_string()),
                },
            }),
            Some(KycWorkflowSignal::ManualApprove {
                reviewer_id: "reviewer_1".to_string(),
            }),
        ]);

        let signal_provider = || async {
            let mut signals = signals.lock().unwrap();
            if !signals.is_empty() {
                signals.remove(0)
            } else {
                None
            }
        };

        let result = workflow.execute(input, signal_provider).await.expect("Workflow failed");
        assert_eq!(result.status, KycStatus::Approved);
    }
}
