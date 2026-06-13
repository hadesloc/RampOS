use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use ramp_common::Result;

use crate::repository::{
    BeneficiaryProfileFilter, BeneficiaryProfileRecord, EnsureWalletAttestationRequest,
    PgVenueTrustRepository, SourceOfFundsPackageFilter, SourceOfFundsPackageRecord,
    UpsertBeneficiaryProfileRequest, UpsertSourceOfFundsPackageRequest, UpsertVenueAccountRequest,
    UpsertVenueConnectionRequest, UpsertVenueTransferRequest, VenueAccountFilter,
    VenueAccountRecord, VenueConnectionFilter, VenueConnectionRecord, VenueTransferRecord,
    VenueTrustRepository, WalletAttestationFilter, WalletAttestationRecord,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueSubjectSnapshot {
    pub source: String,
    pub subject_type: String,
    pub subject_id: String,
    pub connections: Vec<VenueConnectionRecord>,
    pub accounts: Vec<VenueAccountRecord>,
    pub beneficiary_profiles: Vec<BeneficiaryProfileRecord>,
    pub wallet_attestations: Vec<WalletAttestationRecord>,
    pub source_of_funds_packages: Vec<SourceOfFundsPackageRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueTransferDetail {
    pub source: String,
    pub transfer: VenueTransferRecord,
    pub connection: Option<VenueConnectionRecord>,
    pub account: Option<VenueAccountRecord>,
    pub beneficiary_profile: Option<BeneficiaryProfileRecord>,
    pub source_of_funds_packages: Vec<SourceOfFundsPackageRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueConnectorReadinessRequirement {
    pub code: String,
    pub satisfied: bool,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LighterConnectorReadiness {
    pub source: String,
    pub connector_key: String,
    pub subject_type: String,
    pub subject_id: String,
    pub status: String,
    pub connection_id: Option<String>,
    pub account_id: Option<String>,
    pub public_pool_mode: Option<String>,
    pub proof_anchor_mode: Option<String>,
    pub operator_linkage_status: Option<String>,
    pub institutional_evidence_status: Option<String>,
    pub requirements: Vec<VenueConnectorReadinessRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CexConnectorReadiness {
    pub source: String,
    pub connector_key: String,
    pub subject_type: String,
    pub subject_id: String,
    pub status: String,
    pub connection_id: Option<String>,
    pub account_id: Option<String>,
    pub api_key_mode: Option<String>,
    pub subaccount_mode: Option<String>,
    pub withdrawal_allowlist_status: Option<String>,
    pub custody_boundary_mode: Option<String>,
    pub requirements: Vec<VenueConnectorReadinessRequirement>,
}

#[derive(Clone)]
pub struct VenueTrustService {
    repository: Option<Arc<dyn VenueTrustRepository>>,
}

impl VenueTrustService {
    pub fn new() -> Self {
        Self { repository: None }
    }

    pub fn with_pool(pool: PgPool) -> Self {
        Self {
            repository: Some(Arc::new(PgVenueTrustRepository::new(pool))),
        }
    }

    pub fn with_repository(repository: Arc<dyn VenueTrustRepository>) -> Self {
        Self {
            repository: Some(repository),
        }
    }

    pub async fn ensure_wallet_attestation(
        &self,
        request: &EnsureWalletAttestationRequest,
    ) -> Result<()> {
        self.repository()?.ensure_wallet_attestation(request).await
    }

    pub async fn get_wallet_attestation(
        &self,
        tenant_id: &str,
        attestation_id: uuid::Uuid,
    ) -> Result<Option<WalletAttestationRecord>> {
        self.repository()?
            .get_wallet_attestation(tenant_id, attestation_id)
            .await
    }

    pub async fn list_wallet_attestations(
        &self,
        filter: &WalletAttestationFilter,
    ) -> Result<Vec<WalletAttestationRecord>> {
        self.repository()?.list_wallet_attestations(filter).await
    }

    pub async fn upsert_connection(&self, request: &UpsertVenueConnectionRequest) -> Result<()> {
        self.repository()?.upsert_connection(request).await
    }

    pub async fn upsert_account(&self, request: &UpsertVenueAccountRequest) -> Result<()> {
        self.repository()?.upsert_account(request).await
    }

    pub async fn upsert_beneficiary_profile(
        &self,
        request: &UpsertBeneficiaryProfileRequest,
    ) -> Result<()> {
        self.repository()?.upsert_beneficiary_profile(request).await
    }

    pub async fn upsert_transfer(&self, request: &UpsertVenueTransferRequest) -> Result<()> {
        self.repository()?.upsert_transfer(request).await
    }

    pub async fn upsert_source_of_funds_package(
        &self,
        request: &UpsertSourceOfFundsPackageRequest,
    ) -> Result<()> {
        self.repository()?
            .upsert_source_of_funds_package(request)
            .await
    }

    pub async fn transition_connection_status(
        &self,
        tenant_id: &str,
        connection_id: &str,
        next_status: &str,
        metadata_patch: serde_json::Value,
    ) -> Result<()> {
        let repository = self.repository()?;
        let record = repository
            .get_connection(tenant_id, connection_id)
            .await?
            .ok_or_else(|| {
                ramp_common::Error::NotFound(format!(
                    "Venue connection not found: {}",
                    connection_id
                ))
            })?;

        ensure_transition_allowed(
            "venue_connection",
            &record.status,
            next_status,
            allowed_connection_status_transition,
        )?;

        repository
            .upsert_connection(&UpsertVenueConnectionRequest {
                connection_id: record.connection_id,
                tenant_id: record.tenant_id,
                subject_type: record.subject_type,
                subject_id: record.subject_id,
                user_id: record.user_id,
                venue_key: record.venue_key,
                connection_mode: record.connection_mode,
                status: next_status.to_string(),
                metadata: merge_metadata(record.metadata, metadata_patch),
                last_verified_at: Some(Utc::now()),
            })
            .await
    }

    pub async fn transition_wallet_attestation_status(
        &self,
        tenant_id: &str,
        attestation_id: uuid::Uuid,
        next_status: &str,
        metadata_patch: serde_json::Value,
    ) -> Result<()> {
        let repository = self.repository()?;
        let record = repository
            .get_wallet_attestation(tenant_id, attestation_id)
            .await?
            .ok_or_else(|| {
                ramp_common::Error::NotFound(format!(
                    "Wallet attestation not found: {}",
                    attestation_id
                ))
            })?;

        ensure_transition_allowed(
            "wallet_attestation",
            &record.attestation_status,
            next_status,
            allowed_wallet_attestation_status_transition,
        )?;

        repository
            .ensure_wallet_attestation(&EnsureWalletAttestationRequest {
                attestation_id: record.attestation_id,
                tenant_id: record.tenant_id,
                user_id: record.user_id,
                wallet_address: record.wallet_address,
                chain_id: record.chain_id,
                attestation_status: next_status.to_string(),
                proof_kind: record.proof_kind,
                proof_artifact_uri: record.proof_artifact_uri,
                risk_state: record.risk_state,
                metadata: merge_metadata(record.metadata, metadata_patch),
            })
            .await
    }

    pub async fn transition_beneficiary_verification_status(
        &self,
        tenant_id: &str,
        beneficiary_profile_id: &str,
        next_status: &str,
        metadata_patch: serde_json::Value,
    ) -> Result<()> {
        let repository = self.repository()?;
        let record = repository
            .get_beneficiary_profile(tenant_id, beneficiary_profile_id)
            .await?
            .ok_or_else(|| {
                ramp_common::Error::NotFound(format!(
                    "Beneficiary profile not found: {}",
                    beneficiary_profile_id
                ))
            })?;

        ensure_transition_allowed(
            "beneficiary_profile",
            &record.verification_status,
            next_status,
            allowed_beneficiary_status_transition,
        )?;

        repository
            .upsert_beneficiary_profile(&UpsertBeneficiaryProfileRequest {
                beneficiary_profile_id: record.beneficiary_profile_id,
                tenant_id: record.tenant_id,
                subject_type: record.subject_type,
                subject_id: record.subject_id,
                user_id: record.user_id,
                destination_type: record.destination_type,
                destination_ref: record.destination_ref,
                display_name: record.display_name,
                asset_symbol: record.asset_symbol,
                network: record.network,
                verification_status: next_status.to_string(),
                cooldown_ends_at: record.cooldown_ends_at,
                metadata: merge_metadata(record.metadata, metadata_patch),
                last_verified_at: Some(Utc::now()),
            })
            .await
    }

    pub async fn transition_transfer_status(
        &self,
        tenant_id: &str,
        transfer_id: &str,
        next_status: &str,
        metadata_patch: serde_json::Value,
    ) -> Result<()> {
        let repository = self.repository()?;
        let record = repository
            .get_transfer(tenant_id, transfer_id)
            .await?
            .ok_or_else(|| {
                ramp_common::Error::NotFound(format!("Venue transfer not found: {}", transfer_id))
            })?;

        ensure_transition_allowed(
            "venue_transfer",
            &record.status,
            next_status,
            allowed_transfer_status_transition,
        )?;

        let now = Utc::now();
        repository
            .upsert_transfer(&UpsertVenueTransferRequest {
                transfer_id: record.transfer_id,
                tenant_id: record.tenant_id,
                user_id: record.user_id,
                beneficiary_profile_id: record.beneficiary_profile_id,
                wallet_attestation_id: record.wallet_attestation_id,
                venue_connection_id: record.venue_connection_id,
                venue_account_id: record.venue_account_id,
                transfer_direction: record.transfer_direction,
                asset_symbol: record.asset_symbol,
                network: record.network,
                amount: record.amount,
                origin_intent_id: record.origin_intent_id,
                rfq_id: record.rfq_id,
                status: next_status.to_string(),
                wallet_tx_hash: record.wallet_tx_hash,
                venue_credit_ref: record.venue_credit_ref,
                failure_code: record.failure_code,
                metadata: merge_metadata(record.metadata, metadata_patch),
                submitted_at: if next_status == "submitted" {
                    Some(record.submitted_at.unwrap_or(now))
                } else {
                    record.submitted_at
                },
                completed_at: if next_status == "completed" {
                    Some(now)
                } else {
                    record.completed_at
                },
            })
            .await
    }

    pub async fn transition_source_of_funds_review_status(
        &self,
        tenant_id: &str,
        package_id: &str,
        next_status: &str,
        metadata_patch: serde_json::Value,
    ) -> Result<()> {
        let repository = self.repository()?;
        let record = repository
            .get_source_of_funds_package(tenant_id, package_id)
            .await?
            .ok_or_else(|| {
                ramp_common::Error::NotFound(format!(
                    "Source-of-funds package not found: {}",
                    package_id
                ))
            })?;

        ensure_transition_allowed(
            "source_of_funds_package",
            &record.review_status,
            next_status,
            allowed_source_of_funds_status_transition,
        )?;

        repository
            .upsert_source_of_funds_package(&UpsertSourceOfFundsPackageRequest {
                package_id: record.package_id,
                tenant_id: record.tenant_id,
                subject_type: record.subject_type,
                subject_id: record.subject_id,
                wallet_attestation_id: record.wallet_attestation_id,
                venue_account_id: record.venue_account_id,
                venue_transfer_id: record.venue_transfer_id,
                review_status: next_status.to_string(),
                package_uri: record.package_uri,
                metadata: merge_metadata(record.metadata, metadata_patch),
                reviewed_at: if matches!(next_status, "approved" | "rejected") {
                    Some(Utc::now())
                } else {
                    record.reviewed_at
                },
            })
            .await
    }

    pub async fn get_subject_snapshot(
        &self,
        tenant_id: &str,
        subject_type: &str,
        subject_id: &str,
    ) -> Result<VenueSubjectSnapshot> {
        let Some(repository) = &self.repository else {
            return Ok(VenueSubjectSnapshot {
                source: "fallback".to_string(),
                subject_type: subject_type.to_string(),
                subject_id: subject_id.to_string(),
                connections: Vec::new(),
                accounts: Vec::new(),
                beneficiary_profiles: Vec::new(),
                wallet_attestations: Vec::new(),
                source_of_funds_packages: Vec::new(),
            });
        };

        let connections = repository
            .list_connections(&VenueConnectionFilter {
                tenant_id: tenant_id.to_string(),
                subject_type: Some(subject_type.to_string()),
                subject_id: Some(subject_id.to_string()),
                venue_key: None,
                status: None,
            })
            .await?;

        let mut accounts = Vec::new();
        for connection in &connections {
            let mut batch = repository
                .list_accounts(&VenueAccountFilter {
                    tenant_id: tenant_id.to_string(),
                    venue_connection_id: Some(connection.connection_id.clone()),
                    venue_key: None,
                    status: None,
                    wallet_address: None,
                })
                .await?;
            accounts.append(&mut batch);
        }

        let beneficiary_profiles = repository
            .list_beneficiary_profiles(&BeneficiaryProfileFilter {
                tenant_id: tenant_id.to_string(),
                subject_type: Some(subject_type.to_string()),
                subject_id: Some(subject_id.to_string()),
                destination_type: None,
                verification_status: None,
            })
            .await?;

        let wallet_attestations = if subject_type == "user" {
            repository
                .list_wallet_attestations(&WalletAttestationFilter {
                    tenant_id: tenant_id.to_string(),
                    user_id: Some(subject_id.to_string()),
                    wallet_address: None,
                    chain_id: None,
                    attestation_status: None,
                })
                .await?
        } else {
            Vec::new()
        };

        let source_of_funds_packages = repository
            .list_source_of_funds_packages(&SourceOfFundsPackageFilter {
                tenant_id: tenant_id.to_string(),
                subject_type: Some(subject_type.to_string()),
                subject_id: Some(subject_id.to_string()),
                venue_transfer_id: None,
                venue_account_id: None,
                review_status: None,
            })
            .await?;

        let has_registry_data = !connections.is_empty()
            || !accounts.is_empty()
            || !beneficiary_profiles.is_empty()
            || !wallet_attestations.is_empty()
            || !source_of_funds_packages.is_empty();

        Ok(VenueSubjectSnapshot {
            source: if has_registry_data {
                "registry".to_string()
            } else {
                "fallback".to_string()
            },
            subject_type: subject_type.to_string(),
            subject_id: subject_id.to_string(),
            connections,
            accounts,
            beneficiary_profiles,
            wallet_attestations,
            source_of_funds_packages,
        })
    }

    pub async fn get_lighter_connector_readiness(
        &self,
        tenant_id: &str,
        subject_type: &str,
        subject_id: &str,
    ) -> Result<LighterConnectorReadiness> {
        let snapshot = self
            .get_subject_snapshot(tenant_id, subject_type, subject_id)
            .await?;

        let connection = snapshot
            .connections
            .iter()
            .find(|record| record.venue_key == "lighter")
            .cloned();
        let account = connection.as_ref().and_then(|selected_connection| {
            snapshot
                .accounts
                .iter()
                .find(|record| {
                    record.venue_key == "lighter"
                        && record.venue_connection_id == selected_connection.connection_id
                })
                .cloned()
        });

        let public_pool_mode = connection
            .as_ref()
            .and_then(|record| metadata_string(&record.metadata, "public_pool_mode"));
        let proof_anchor_mode = account
            .as_ref()
            .and_then(|record| metadata_string(&record.metadata, "proof_anchor_mode"));
        let operator_linkage_status = connection
            .as_ref()
            .and_then(|record| metadata_string(&record.metadata, "operator_linkage_status"));
        let institutional_evidence_status = connection
            .as_ref()
            .and_then(|record| metadata_string(&record.metadata, "institutional_evidence_status"))
            .or_else(|| {
                account.as_ref().and_then(|record| {
                    metadata_string(&record.metadata, "institutional_evidence_status")
                })
            });

        let requirements = vec![
            VenueConnectorReadinessRequirement {
                code: "lighter_public_pool_mode".to_string(),
                satisfied: readiness_mode_configured(public_pool_mode.as_deref()),
                source: "venue_connection.metadata.public_pool_mode".to_string(),
                message: "Lighter public-pool mode must be configured for operator use."
                    .to_string(),
            },
            VenueConnectorReadinessRequirement {
                code: "lighter_proof_anchor_mode".to_string(),
                satisfied: readiness_mode_configured(proof_anchor_mode.as_deref()),
                source: "venue_account.metadata.proof_anchor_mode".to_string(),
                message:
                    "Lighter proof-anchor mode must be configured for exit and evidence flows."
                        .to_string(),
            },
            VenueConnectorReadinessRequirement {
                code: "lighter_operator_linkage".to_string(),
                satisfied: connection
                    .as_ref()
                    .is_some_and(|record| record.status == "active")
                    && matches!(
                        operator_linkage_status.as_deref(),
                        Some("verified" | "linked" | "institutional_linked")
                    ),
                source: "venue_connection.status+metadata.operator_linkage_status".to_string(),
                message: "Lighter must be operator-linked with an active connection before use."
                    .to_string(),
            },
            VenueConnectorReadinessRequirement {
                code: "lighter_institutional_evidence".to_string(),
                satisfied: matches!(
                    institutional_evidence_status.as_deref(),
                    Some("approved" | "ready" | "complete")
                ),
                source: "venue_connection.metadata.institutional_evidence_status".to_string(),
                message: "Lighter requires institutional evidence to be approved or marked ready."
                    .to_string(),
            },
        ];

        Ok(LighterConnectorReadiness {
            source: snapshot.source,
            connector_key: "lighter".to_string(),
            subject_type: subject_type.to_string(),
            subject_id: subject_id.to_string(),
            status: if requirements.iter().all(|requirement| requirement.satisfied) {
                "ready".to_string()
            } else {
                "attention_required".to_string()
            },
            connection_id: connection
                .as_ref()
                .map(|record| record.connection_id.clone()),
            account_id: account.as_ref().map(|record| record.account_id.clone()),
            public_pool_mode,
            proof_anchor_mode,
            operator_linkage_status,
            institutional_evidence_status,
            requirements,
        })
    }

    pub async fn get_cex_connector_readiness(
        &self,
        tenant_id: &str,
        subject_type: &str,
        subject_id: &str,
        connector_key: &str,
    ) -> Result<CexConnectorReadiness> {
        let snapshot = self
            .get_subject_snapshot(tenant_id, subject_type, subject_id)
            .await?;

        let connection = snapshot
            .connections
            .iter()
            .find(|record| record.venue_key == connector_key)
            .cloned();
        let account = connection.as_ref().and_then(|selected_connection| {
            snapshot
                .accounts
                .iter()
                .find(|record| {
                    record.venue_key == connector_key
                        && record.venue_connection_id == selected_connection.connection_id
                })
                .cloned()
        });

        let api_key_mode = connection
            .as_ref()
            .and_then(|record| metadata_string(&record.metadata, "api_key_mode"));
        let subaccount_mode = account
            .as_ref()
            .and_then(|record| metadata_string(&record.metadata, "subaccount_mode"));
        let withdrawal_allowlist_status = connection
            .as_ref()
            .and_then(|record| metadata_string(&record.metadata, "withdrawal_allowlist_status"));
        let custody_boundary_mode = connection
            .as_ref()
            .and_then(|record| metadata_string(&record.metadata, "custody_boundary_mode"));

        let requirements = vec![
            VenueConnectorReadinessRequirement {
                code: "cex_api_key_mode".to_string(),
                satisfied: readiness_mode_configured(api_key_mode.as_deref()),
                source: "venue_connection.metadata.api_key_mode".to_string(),
                message: "CEX connector requires an API key mode to be configured.".to_string(),
            },
            VenueConnectorReadinessRequirement {
                code: "cex_subaccount_mode".to_string(),
                satisfied: readiness_mode_configured(subaccount_mode.as_deref()),
                source: "venue_account.metadata.subaccount_mode".to_string(),
                message: "CEX connector requires a subaccount mode for operator segregation."
                    .to_string(),
            },
            VenueConnectorReadinessRequirement {
                code: "cex_withdrawal_allowlist".to_string(),
                satisfied: matches!(
                    withdrawal_allowlist_status.as_deref(),
                    Some("verified" | "enabled" | "approved" | "ready")
                ),
                source: "venue_connection.metadata.withdrawal_allowlist_status".to_string(),
                message: "CEX connector requires a verified withdrawal allowlist before use."
                    .to_string(),
            },
            VenueConnectorReadinessRequirement {
                code: "cex_custody_boundary".to_string(),
                satisfied: readiness_mode_configured(custody_boundary_mode.as_deref()),
                source: "venue_connection.metadata.custody_boundary_mode".to_string(),
                message:
                    "CEX connector requires an explicit custody boundary mode for operator use."
                        .to_string(),
            },
        ];

        Ok(CexConnectorReadiness {
            source: snapshot.source,
            connector_key: connector_key.to_string(),
            subject_type: subject_type.to_string(),
            subject_id: subject_id.to_string(),
            status: if requirements.iter().all(|requirement| requirement.satisfied) {
                "ready".to_string()
            } else {
                "attention_required".to_string()
            },
            connection_id: connection
                .as_ref()
                .map(|record| record.connection_id.clone()),
            account_id: account.as_ref().map(|record| record.account_id.clone()),
            api_key_mode,
            subaccount_mode,
            withdrawal_allowlist_status,
            custody_boundary_mode,
            requirements,
        })
    }

    pub async fn get_transfer_detail(
        &self,
        tenant_id: &str,
        transfer_id: &str,
    ) -> Result<Option<VenueTransferDetail>> {
        let Some(repository) = &self.repository else {
            return Ok(None);
        };

        let Some(transfer) = repository.get_transfer(tenant_id, transfer_id).await? else {
            return Ok(None);
        };

        let connection = repository
            .get_connection(tenant_id, &transfer.venue_connection_id)
            .await?;
        let account = repository
            .get_account(tenant_id, &transfer.venue_account_id)
            .await?;
        let beneficiary_profile = match transfer.beneficiary_profile_id.as_deref() {
            Some(profile_id) => {
                repository
                    .get_beneficiary_profile(tenant_id, profile_id)
                    .await?
            }
            None => None,
        };
        let source_of_funds_packages = repository
            .list_source_of_funds_packages(&SourceOfFundsPackageFilter {
                tenant_id: tenant_id.to_string(),
                subject_type: None,
                subject_id: None,
                venue_transfer_id: Some(transfer.transfer_id.clone()),
                venue_account_id: Some(transfer.venue_account_id.clone()),
                review_status: None,
            })
            .await?;

        Ok(Some(VenueTransferDetail {
            source: "registry".to_string(),
            transfer,
            connection,
            account,
            beneficiary_profile,
            source_of_funds_packages,
        }))
    }

    fn repository(&self) -> Result<&Arc<dyn VenueTrustRepository>> {
        self.repository.as_ref().ok_or_else(|| {
            ramp_common::Error::Internal("Venue trust repository is not configured".to_string())
        })
    }
}

impl Default for VenueTrustService {
    fn default() -> Self {
        Self::new()
    }
}

fn metadata_string(metadata: &serde_json::Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn readiness_mode_configured(value: Option<&str>) -> bool {
    matches!(value, Some(mode) if !mode.trim().is_empty() && mode != "disabled")
}

fn ensure_transition_allowed(
    entity: &str,
    current_status: &str,
    next_status: &str,
    transition_guard: fn(&str, &str) -> bool,
) -> Result<()> {
    if current_status == next_status {
        return Ok(());
    }

    if transition_guard(current_status, next_status) {
        return Ok(());
    }

    Err(ramp_common::Error::InvalidStateTransition {
        from: format!("{}:{}", entity, current_status),
        to: format!("{}:{}", entity, next_status),
    })
}

fn allowed_connection_status_transition(current_status: &str, next_status: &str) -> bool {
    matches!(
        (current_status, next_status),
        ("pending", "active")
            | ("pending", "rejected")
            | ("active", "suspended")
            | ("active", "revoked")
            | ("suspended", "active")
            | ("suspended", "revoked")
    )
}

fn allowed_wallet_attestation_status_transition(current_status: &str, next_status: &str) -> bool {
    matches!(
        (current_status, next_status),
        ("pending", "verified")
            | ("pending", "rejected")
            | ("pending", "flagged")
            | ("verified", "flagged")
            | ("verified", "rejected")
    )
}

fn allowed_beneficiary_status_transition(current_status: &str, next_status: &str) -> bool {
    matches!(
        (current_status, next_status),
        ("pending", "verified")
            | ("pending", "rejected")
            | ("verified", "suspended")
            | ("verified", "revoked")
            | ("suspended", "verified")
            | ("suspended", "revoked")
    )
}

fn allowed_transfer_status_transition(current_status: &str, next_status: &str) -> bool {
    matches!(
        (current_status, next_status),
        ("draft", "submitted")
            | ("draft", "cancelled")
            | ("submitted", "completed")
            | ("submitted", "failed")
            | ("submitted", "cancelled")
    )
}

fn allowed_source_of_funds_status_transition(current_status: &str, next_status: &str) -> bool {
    matches!(
        (current_status, next_status),
        ("draft", "in_review") | ("in_review", "approved") | ("in_review", "rejected")
    )
}

fn merge_metadata(current: serde_json::Value, patch: serde_json::Value) -> serde_json::Value {
    match (current, patch) {
        (serde_json::Value::Object(mut current_map), serde_json::Value::Object(patch_map)) => {
            current_map.extend(patch_map);
            serde_json::Value::Object(current_map)
        }
        (_, patch) => patch,
    }
}
