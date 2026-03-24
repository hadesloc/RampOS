use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use rust_decimal::Decimal;
use uuid::Uuid;

use ramp_common::Result;
use ramp_core::repository::{
    BeneficiaryProfileFilter, BeneficiaryProfileRecord, EnsureWalletAttestationRequest,
    WalletAttestationFilter, WalletAttestationRecord,
    SourceOfFundsPackageFilter, SourceOfFundsPackageRecord, UpsertBeneficiaryProfileRequest,
    UpsertSourceOfFundsPackageRequest, UpsertVenueAccountRequest, UpsertVenueConnectionRequest,
    UpsertVenueTransferRequest, VenueAccountFilter, VenueAccountRecord, VenueConnectionFilter,
    VenueConnectionRecord, VenueTransferFilter, VenueTransferRecord, VenueTrustRepository,
};
use ramp_core::service::VenueTrustService;

#[derive(Default)]
struct MockVenueTrustRepository {
    attestations: Mutex<Vec<WalletAttestationRecord>>,
    connections: Mutex<Vec<VenueConnectionRecord>>,
    accounts: Mutex<Vec<VenueAccountRecord>>,
    beneficiaries: Mutex<Vec<BeneficiaryProfileRecord>>,
    transfers: Mutex<Vec<VenueTransferRecord>>,
    packages: Mutex<Vec<SourceOfFundsPackageRecord>>,
}

#[async_trait]
impl VenueTrustRepository for MockVenueTrustRepository {
    async fn ensure_wallet_attestation(
        &self,
        request: &EnsureWalletAttestationRequest,
    ) -> Result<()> {
        let mut items = self.attestations.lock().expect("attestations lock");
        items.retain(|item| item.attestation_id != request.attestation_id);
        items.push(WalletAttestationRecord {
            attestation_id: request.attestation_id,
            tenant_id: request.tenant_id.clone(),
            user_id: request.user_id.clone(),
            wallet_address: request.wallet_address.clone(),
            chain_id: request.chain_id.clone(),
            attestation_status: request.attestation_status.clone(),
            proof_kind: request.proof_kind.clone(),
            proof_artifact_uri: request.proof_artifact_uri.clone(),
            risk_state: request.risk_state.clone(),
            metadata: request.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        Ok(())
    }

    async fn get_wallet_attestation(
        &self,
        tenant_id: &str,
        attestation_id: Uuid,
    ) -> Result<Option<WalletAttestationRecord>> {
        let items = self.attestations.lock().expect("attestations lock");
        Ok(items
            .iter()
            .find(|item| item.tenant_id == tenant_id && item.attestation_id == attestation_id)
            .cloned())
    }

    async fn list_wallet_attestations(
        &self,
        filter: &WalletAttestationFilter,
    ) -> Result<Vec<WalletAttestationRecord>> {
        let items = self.attestations.lock().expect("attestations lock");
        Ok(items
            .iter()
            .filter(|item| {
                item.tenant_id == filter.tenant_id
                    && filter
                        .user_id
                        .as_deref()
                        .is_none_or(|value| item.user_id == value)
                    && filter
                        .wallet_address
                        .as_deref()
                        .is_none_or(|value| item.wallet_address == value)
                    && filter
                        .chain_id
                        .as_deref()
                        .is_none_or(|value| item.chain_id == value)
                    && filter
                        .attestation_status
                        .as_deref()
                        .is_none_or(|value| item.attestation_status == value)
            })
            .cloned()
            .collect())
    }

    async fn upsert_connection(&self, request: &UpsertVenueConnectionRequest) -> Result<()> {
        let mut items = self.connections.lock().expect("connections lock");
        items.retain(|item| item.connection_id != request.connection_id);
        items.push(VenueConnectionRecord {
            connection_id: request.connection_id.clone(),
            tenant_id: request.tenant_id.clone(),
            subject_type: request.subject_type.clone(),
            subject_id: request.subject_id.clone(),
            user_id: request.user_id.clone(),
            venue_key: request.venue_key.clone(),
            connection_mode: request.connection_mode.clone(),
            status: request.status.clone(),
            metadata: request.metadata.clone(),
            last_verified_at: request.last_verified_at,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        Ok(())
    }

    async fn get_connection(
        &self,
        tenant_id: &str,
        connection_id: &str,
    ) -> Result<Option<VenueConnectionRecord>> {
        let items = self.connections.lock().expect("connections lock");
        Ok(items
            .iter()
            .find(|item| item.tenant_id == tenant_id && item.connection_id == connection_id)
            .cloned())
    }

    async fn list_connections(
        &self,
        filter: &VenueConnectionFilter,
    ) -> Result<Vec<VenueConnectionRecord>> {
        let items = self.connections.lock().expect("connections lock");
        Ok(items
            .iter()
            .filter(|item| {
                item.tenant_id == filter.tenant_id
                    && filter
                        .subject_type
                        .as_deref()
                        .is_none_or(|value| item.subject_type == value)
                    && filter
                        .subject_id
                        .as_deref()
                        .is_none_or(|value| item.subject_id == value)
                    && filter
                        .venue_key
                        .as_deref()
                        .is_none_or(|value| item.venue_key == value)
                    && filter
                        .status
                        .as_deref()
                        .is_none_or(|value| item.status == value)
            })
            .cloned()
            .collect())
    }

    async fn upsert_account(&self, request: &UpsertVenueAccountRequest) -> Result<()> {
        let mut items = self.accounts.lock().expect("accounts lock");
        items.retain(|item| item.account_id != request.account_id);
        items.push(VenueAccountRecord {
            account_id: request.account_id.clone(),
            tenant_id: request.tenant_id.clone(),
            venue_connection_id: request.venue_connection_id.clone(),
            venue_key: request.venue_key.clone(),
            account_label: request.account_label.clone(),
            account_ref: request.account_ref.clone(),
            wallet_address: request.wallet_address.clone(),
            subaccount_ref: request.subaccount_ref.clone(),
            api_scope_summary: request.api_scope_summary.clone(),
            status: request.status.clone(),
            metadata: request.metadata.clone(),
            last_verified_at: request.last_verified_at,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        Ok(())
    }

    async fn get_account(&self, tenant_id: &str, account_id: &str) -> Result<Option<VenueAccountRecord>> {
        let items = self.accounts.lock().expect("accounts lock");
        Ok(items
            .iter()
            .find(|item| item.tenant_id == tenant_id && item.account_id == account_id)
            .cloned())
    }

    async fn list_accounts(&self, filter: &VenueAccountFilter) -> Result<Vec<VenueAccountRecord>> {
        let items = self.accounts.lock().expect("accounts lock");
        Ok(items
            .iter()
            .filter(|item| {
                item.tenant_id == filter.tenant_id
                    && filter
                        .venue_connection_id
                        .as_deref()
                        .is_none_or(|value| item.venue_connection_id == value)
                    && filter
                        .venue_key
                        .as_deref()
                        .is_none_or(|value| item.venue_key == value)
                    && filter
                        .status
                        .as_deref()
                        .is_none_or(|value| item.status == value)
                    && filter
                        .wallet_address
                        .as_deref()
                        .is_none_or(|value| item.wallet_address.as_deref() == Some(value))
            })
            .cloned()
            .collect())
    }

    async fn upsert_beneficiary_profile(
        &self,
        request: &UpsertBeneficiaryProfileRequest,
    ) -> Result<()> {
        let mut items = self.beneficiaries.lock().expect("beneficiaries lock");
        items.retain(|item| item.beneficiary_profile_id != request.beneficiary_profile_id);
        items.push(BeneficiaryProfileRecord {
            beneficiary_profile_id: request.beneficiary_profile_id.clone(),
            tenant_id: request.tenant_id.clone(),
            subject_type: request.subject_type.clone(),
            subject_id: request.subject_id.clone(),
            user_id: request.user_id.clone(),
            destination_type: request.destination_type.clone(),
            destination_ref: request.destination_ref.clone(),
            display_name: request.display_name.clone(),
            asset_symbol: request.asset_symbol.clone(),
            network: request.network.clone(),
            verification_status: request.verification_status.clone(),
            cooldown_ends_at: request.cooldown_ends_at,
            metadata: request.metadata.clone(),
            last_verified_at: request.last_verified_at,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        Ok(())
    }

    async fn get_beneficiary_profile(
        &self,
        tenant_id: &str,
        beneficiary_profile_id: &str,
    ) -> Result<Option<BeneficiaryProfileRecord>> {
        let items = self.beneficiaries.lock().expect("beneficiaries lock");
        Ok(items
            .iter()
            .find(|item| {
                item.tenant_id == tenant_id
                    && item.beneficiary_profile_id == beneficiary_profile_id
            })
            .cloned())
    }

    async fn list_beneficiary_profiles(
        &self,
        filter: &BeneficiaryProfileFilter,
    ) -> Result<Vec<BeneficiaryProfileRecord>> {
        let items = self.beneficiaries.lock().expect("beneficiaries lock");
        Ok(items
            .iter()
            .filter(|item| {
                item.tenant_id == filter.tenant_id
                    && filter
                        .subject_type
                        .as_deref()
                        .is_none_or(|value| item.subject_type == value)
                    && filter
                        .subject_id
                        .as_deref()
                        .is_none_or(|value| item.subject_id == value)
                    && filter
                        .destination_type
                        .as_deref()
                        .is_none_or(|value| item.destination_type == value)
                    && filter
                        .verification_status
                        .as_deref()
                        .is_none_or(|value| item.verification_status == value)
            })
            .cloned()
            .collect())
    }

    async fn upsert_transfer(&self, request: &UpsertVenueTransferRequest) -> Result<()> {
        let mut items = self.transfers.lock().expect("transfers lock");
        items.retain(|item| item.transfer_id != request.transfer_id);
        items.push(VenueTransferRecord {
            transfer_id: request.transfer_id.clone(),
            tenant_id: request.tenant_id.clone(),
            user_id: request.user_id.clone(),
            beneficiary_profile_id: request.beneficiary_profile_id.clone(),
            wallet_attestation_id: request.wallet_attestation_id,
            venue_connection_id: request.venue_connection_id.clone(),
            venue_account_id: request.venue_account_id.clone(),
            transfer_direction: request.transfer_direction.clone(),
            asset_symbol: request.asset_symbol.clone(),
            network: request.network.clone(),
            amount: request.amount,
            origin_intent_id: request.origin_intent_id.clone(),
            rfq_id: request.rfq_id.clone(),
            status: request.status.clone(),
            wallet_tx_hash: request.wallet_tx_hash.clone(),
            venue_credit_ref: request.venue_credit_ref.clone(),
            failure_code: request.failure_code.clone(),
            metadata: request.metadata.clone(),
            submitted_at: request.submitted_at,
            completed_at: request.completed_at,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        Ok(())
    }

    async fn get_transfer(&self, tenant_id: &str, transfer_id: &str) -> Result<Option<VenueTransferRecord>> {
        let items = self.transfers.lock().expect("transfers lock");
        Ok(items
            .iter()
            .find(|item| item.tenant_id == tenant_id && item.transfer_id == transfer_id)
            .cloned())
    }

    async fn list_transfers(&self, filter: &VenueTransferFilter) -> Result<Vec<VenueTransferRecord>> {
        let items = self.transfers.lock().expect("transfers lock");
        Ok(items
            .iter()
            .filter(|item| {
                item.tenant_id == filter.tenant_id
                    && filter
                        .user_id
                        .as_deref()
                        .is_none_or(|value| item.user_id == value)
                    && filter
                        .venue_connection_id
                        .as_deref()
                        .is_none_or(|value| item.venue_connection_id == value)
                    && filter
                        .venue_account_id
                        .as_deref()
                        .is_none_or(|value| item.venue_account_id == value)
                    && filter
                        .status
                        .as_deref()
                        .is_none_or(|value| item.status == value)
            })
            .cloned()
            .collect())
    }

    async fn upsert_source_of_funds_package(
        &self,
        request: &UpsertSourceOfFundsPackageRequest,
    ) -> Result<()> {
        let mut items = self.packages.lock().expect("packages lock");
        items.retain(|item| item.package_id != request.package_id);
        items.push(SourceOfFundsPackageRecord {
            package_id: request.package_id.clone(),
            tenant_id: request.tenant_id.clone(),
            subject_type: request.subject_type.clone(),
            subject_id: request.subject_id.clone(),
            wallet_attestation_id: request.wallet_attestation_id,
            venue_account_id: request.venue_account_id.clone(),
            venue_transfer_id: request.venue_transfer_id.clone(),
            review_status: request.review_status.clone(),
            package_uri: request.package_uri.clone(),
            metadata: request.metadata.clone(),
            reviewed_at: request.reviewed_at,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        Ok(())
    }

    async fn get_source_of_funds_package(
        &self,
        tenant_id: &str,
        package_id: &str,
    ) -> Result<Option<SourceOfFundsPackageRecord>> {
        let items = self.packages.lock().expect("packages lock");
        Ok(items
            .iter()
            .find(|item| item.tenant_id == tenant_id && item.package_id == package_id)
            .cloned())
    }

    async fn list_source_of_funds_packages(
        &self,
        filter: &SourceOfFundsPackageFilter,
    ) -> Result<Vec<SourceOfFundsPackageRecord>> {
        let items = self.packages.lock().expect("packages lock");
        Ok(items
            .iter()
            .filter(|item| {
                item.tenant_id == filter.tenant_id
                    && filter
                        .subject_type
                        .as_deref()
                        .is_none_or(|value| item.subject_type == value)
                    && filter
                        .subject_id
                        .as_deref()
                        .is_none_or(|value| item.subject_id == value)
                    && filter
                        .venue_transfer_id
                        .as_deref()
                        .is_none_or(|value| item.venue_transfer_id.as_deref() == Some(value))
                    && filter
                        .venue_account_id
                        .as_deref()
                        .is_none_or(|value| item.venue_account_id.as_deref() == Some(value))
                    && filter
                        .review_status
                        .as_deref()
                        .is_none_or(|value| item.review_status == value)
            })
            .cloned()
            .collect())
    }
}

#[tokio::test]
async fn service_returns_fallback_subject_snapshot_without_repository() {
    let service = VenueTrustService::new();

    let snapshot = service
        .get_subject_snapshot("tenant-a", "user", "user-a")
        .await
        .expect("snapshot should load");

    assert_eq!(snapshot.source, "fallback");
    assert!(snapshot.connections.is_empty());
    assert!(snapshot.accounts.is_empty());
    assert!(snapshot.beneficiary_profiles.is_empty());
    assert!(snapshot.wallet_attestations.is_empty());
    assert!(snapshot.source_of_funds_packages.is_empty());
}

#[tokio::test]
async fn service_builds_subject_and_transfer_views_from_repository() {
    let repository = Arc::new(MockVenueTrustRepository::default());
    let service = VenueTrustService::with_repository(repository);
    let attestation_id = Uuid::parse_str("20000000-0000-0000-0000-000000000023")
        .expect("attestation uuid");

    service
        .ensure_wallet_attestation(&EnsureWalletAttestationRequest {
            attestation_id,
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            wallet_address: "0xabc".to_string(),
            chain_id: "ethereum".to_string(),
            attestation_status: "verified".to_string(),
            proof_kind: "signature".to_string(),
            proof_artifact_uri: Some("s3://proofs/subject-view.json".to_string()),
            risk_state: Some("clear".to_string()),
            metadata: serde_json::json!({ "scope": "venue_funding" }),
        })
        .await
        .expect("attestation should persist");

    service
        .upsert_connection(&UpsertVenueConnectionRequest {
            connection_id: "connection-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            user_id: Some("user-a".to_string()),
            venue_key: "hyperliquid".to_string(),
            connection_mode: "wallet_linked".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
            last_verified_at: None,
        })
        .await
        .expect("connection should persist");

    service
        .upsert_account(&UpsertVenueAccountRequest {
            account_id: "account-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            venue_connection_id: "connection-1".to_string(),
            venue_key: "hyperliquid".to_string(),
            account_label: Some("Main".to_string()),
            account_ref: Some("acct-1".to_string()),
            wallet_address: Some("0xabc".to_string()),
            subaccount_ref: None,
            api_scope_summary: serde_json::json!({ "permissions": ["read"] }),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
            last_verified_at: None,
        })
        .await
        .expect("account should persist");

    service
        .upsert_beneficiary_profile(&UpsertBeneficiaryProfileRequest {
            beneficiary_profile_id: "beneficiary-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            user_id: Some("user-a".to_string()),
            destination_type: "exchange_wallet".to_string(),
            destination_ref: "0xbeneficiary".to_string(),
            display_name: Some("Beneficiary".to_string()),
            asset_symbol: Some("USDC".to_string()),
            network: Some("ethereum".to_string()),
            verification_status: "verified".to_string(),
            cooldown_ends_at: None,
            metadata: serde_json::json!({}),
            last_verified_at: None,
        })
        .await
        .expect("beneficiary should persist");

    service
        .upsert_transfer(&UpsertVenueTransferRequest {
            transfer_id: "transfer-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            beneficiary_profile_id: Some("beneficiary-1".to_string()),
            wallet_attestation_id: attestation_id,
            venue_connection_id: "connection-1".to_string(),
            venue_account_id: "account-1".to_string(),
            transfer_direction: "wallet_to_venue".to_string(),
            asset_symbol: "USDC".to_string(),
            network: "ethereum".to_string(),
            amount: Decimal::new(25_000, 2),
            origin_intent_id: None,
            rfq_id: None,
            status: "submitted".to_string(),
            wallet_tx_hash: Some("0xhash".to_string()),
            venue_credit_ref: None,
            failure_code: None,
            metadata: serde_json::json!({}),
            submitted_at: None,
            completed_at: None,
        })
        .await
        .expect("transfer should persist");

    service
        .upsert_source_of_funds_package(&UpsertSourceOfFundsPackageRequest {
            package_id: "package-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            wallet_attestation_id: Some(attestation_id),
            venue_account_id: Some("account-1".to_string()),
            venue_transfer_id: Some("transfer-1".to_string()),
            review_status: "pending".to_string(),
            package_uri: Some("s3://pkg".to_string()),
            metadata: serde_json::json!({}),
            reviewed_at: None,
        })
        .await
        .expect("package should persist");

    let subject_snapshot = service
        .get_subject_snapshot("tenant-a", "user", "user-a")
        .await
        .expect("subject snapshot should load");
    assert_eq!(subject_snapshot.source, "registry");
    assert_eq!(subject_snapshot.connections.len(), 1);
    assert_eq!(subject_snapshot.accounts.len(), 1);
    assert_eq!(subject_snapshot.beneficiary_profiles.len(), 1);
    assert_eq!(subject_snapshot.wallet_attestations.len(), 1);
    assert_eq!(
        subject_snapshot.wallet_attestations[0].proof_artifact_uri.as_deref(),
        Some("s3://proofs/subject-view.json")
    );
    assert_eq!(subject_snapshot.source_of_funds_packages.len(), 1);

    let transfer_detail = service
        .get_transfer_detail("tenant-a", "transfer-1")
        .await
        .expect("transfer detail should load")
        .expect("transfer detail should exist");
    assert_eq!(transfer_detail.transfer.transfer_id, "transfer-1");
    assert_eq!(
        transfer_detail
            .connection
            .expect("connection should exist")
            .venue_key,
        "hyperliquid"
    );
    assert_eq!(
        transfer_detail
            .beneficiary_profile
            .expect("beneficiary should exist")
            .destination_ref,
        "0xbeneficiary"
    );
    assert_eq!(transfer_detail.source_of_funds_packages.len(), 1);
}

#[tokio::test]
async fn service_exposes_explicit_venue_transitions_and_attestation_setup() {
    let repository = Arc::new(MockVenueTrustRepository::default());
    let service = VenueTrustService::with_repository(repository.clone());
    let attestation_id = Uuid::parse_str("30000000-0000-0000-0000-000000000023")
        .expect("attestation uuid");

    service
        .ensure_wallet_attestation(&EnsureWalletAttestationRequest {
            attestation_id,
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            wallet_address: "0xabc".to_string(),
            chain_id: "ethereum".to_string(),
            attestation_status: "verified".to_string(),
            proof_kind: "signature".to_string(),
            proof_artifact_uri: Some("s3://proofs/attestation.json".to_string()),
            risk_state: Some("clear".to_string()),
            metadata: serde_json::json!({ "scope": "venue_funding" }),
        })
        .await
        .expect("attestation should persist");

    service
        .transition_wallet_attestation_status(
            "tenant-a",
            attestation_id,
            "flagged",
            serde_json::json!({ "failureReason": "chain_mismatch", "reviewReason": "manual_review" }),
        )
        .await
        .expect("attestation flag should succeed");

    let flagged_attestation = repository
        .get_wallet_attestation("tenant-a", attestation_id)
        .await
        .expect("attestation lookup should succeed")
        .expect("attestation should exist");
    assert_eq!(flagged_attestation.attestation_status, "flagged");
    assert_eq!(flagged_attestation.metadata["failureReason"], "chain_mismatch");
    assert_eq!(flagged_attestation.metadata["reviewReason"], "manual_review");

    let err = service
        .transition_wallet_attestation_status(
            "tenant-a",
            attestation_id,
            "pending",
            serde_json::json!({ "reviewReason": "rollback" }),
        )
        .await
        .expect_err("flagged attestation should reject reopening");
    assert!(matches!(err, ramp_common::Error::InvalidStateTransition { .. }));

    service
        .upsert_connection(&UpsertVenueConnectionRequest {
            connection_id: "connection-transition".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            user_id: Some("user-a".to_string()),
            venue_key: "hyperliquid".to_string(),
            connection_mode: "wallet_linked".to_string(),
            status: "pending".to_string(),
            metadata: serde_json::json!({}),
            last_verified_at: None,
        })
        .await
        .expect("connection should persist");

    service
        .upsert_beneficiary_profile(&UpsertBeneficiaryProfileRequest {
            beneficiary_profile_id: "beneficiary-transition".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            user_id: Some("user-a".to_string()),
            destination_type: "exchange_wallet".to_string(),
            destination_ref: "0xbeneficiary".to_string(),
            display_name: Some("Beneficiary".to_string()),
            asset_symbol: Some("USDC".to_string()),
            network: Some("ethereum".to_string()),
            verification_status: "pending".to_string(),
            cooldown_ends_at: None,
            metadata: serde_json::json!({}),
            last_verified_at: None,
        })
        .await
        .expect("beneficiary should persist");

    service
        .upsert_account(&UpsertVenueAccountRequest {
            account_id: "account-transition".to_string(),
            tenant_id: "tenant-a".to_string(),
            venue_connection_id: "connection-transition".to_string(),
            venue_key: "hyperliquid".to_string(),
            account_label: Some("Main".to_string()),
            account_ref: Some("acct-transition".to_string()),
            wallet_address: Some("0xabc".to_string()),
            subaccount_ref: None,
            api_scope_summary: serde_json::json!({ "permissions": ["read"] }),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
            last_verified_at: None,
        })
        .await
        .expect("account should persist");

    service
        .upsert_transfer(&UpsertVenueTransferRequest {
            transfer_id: "transfer-transition".to_string(),
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            beneficiary_profile_id: Some("beneficiary-transition".to_string()),
            wallet_attestation_id: attestation_id,
            venue_connection_id: "connection-transition".to_string(),
            venue_account_id: "account-transition".to_string(),
            transfer_direction: "wallet_to_venue".to_string(),
            asset_symbol: "USDC".to_string(),
            network: "ethereum".to_string(),
            amount: Decimal::new(25_000, 2),
            origin_intent_id: None,
            rfq_id: None,
            status: "draft".to_string(),
            wallet_tx_hash: None,
            venue_credit_ref: None,
            failure_code: None,
            metadata: serde_json::json!({}),
            submitted_at: None,
            completed_at: None,
        })
        .await
        .expect("transfer should persist");

    service
        .upsert_source_of_funds_package(&UpsertSourceOfFundsPackageRequest {
            package_id: "package-transition".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            wallet_attestation_id: Some(attestation_id),
            venue_account_id: Some("account-transition".to_string()),
            venue_transfer_id: Some("transfer-transition".to_string()),
            review_status: "draft".to_string(),
            package_uri: Some("s3://pkg-transition".to_string()),
            metadata: serde_json::json!({}),
            reviewed_at: None,
        })
        .await
        .expect("package should persist");

    service
        .transition_connection_status(
            "tenant-a",
            "connection-transition",
            "active",
            serde_json::json!({ "review": "approved" }),
        )
        .await
        .expect("connection activation should succeed");
    service
        .transition_beneficiary_verification_status(
            "tenant-a",
            "beneficiary-transition",
            "verified",
            serde_json::json!({ "review": "approved" }),
        )
        .await
        .expect("beneficiary verification should succeed");
    service
        .transition_transfer_status(
            "tenant-a",
            "transfer-transition",
            "submitted",
            serde_json::json!({ "tx": "sent" }),
        )
        .await
        .expect("transfer submission should succeed");
    service
        .transition_source_of_funds_review_status(
            "tenant-a",
            "package-transition",
            "in_review",
            serde_json::json!({ "queue": "ops" }),
        )
        .await
        .expect("package review should succeed");

    let connection = repository
        .get_connection("tenant-a", "connection-transition")
        .await
        .expect("connection lookup should succeed")
        .expect("connection should exist");
    assert_eq!(connection.status, "active");
    assert_eq!(connection.metadata["review"], "approved");

    let beneficiary = repository
        .get_beneficiary_profile("tenant-a", "beneficiary-transition")
        .await
        .expect("beneficiary lookup should succeed")
        .expect("beneficiary should exist");
    assert_eq!(beneficiary.verification_status, "verified");
    assert_eq!(beneficiary.metadata["review"], "approved");

    let transfer = repository
        .get_transfer("tenant-a", "transfer-transition")
        .await
        .expect("transfer lookup should succeed")
        .expect("transfer should exist");
    assert_eq!(transfer.status, "submitted");
    assert_eq!(transfer.metadata["tx"], "sent");

    let package = repository
        .get_source_of_funds_package("tenant-a", "package-transition")
        .await
        .expect("package lookup should succeed")
        .expect("package should exist");
    assert_eq!(package.review_status, "in_review");
    assert_eq!(package.metadata["queue"], "ops");

    let err = service
        .transition_connection_status(
            "tenant-a",
            "connection-transition",
            "pending",
            serde_json::json!({ "review": "rollback" }),
        )
        .await
        .expect_err("connection should reject backward transition");
    assert!(matches!(err, ramp_common::Error::InvalidStateTransition { .. }));

    let err = service
        .transition_beneficiary_verification_status(
            "tenant-a",
            "beneficiary-transition",
            "pending",
            serde_json::json!({ "review": "rollback" }),
        )
        .await
        .expect_err("beneficiary should reject backward transition");
    assert!(matches!(err, ramp_common::Error::InvalidStateTransition { .. }));

    service
        .transition_transfer_status(
            "tenant-a",
            "transfer-transition",
            "completed",
            serde_json::json!({ "credit": "confirmed" }),
        )
        .await
        .expect("transfer completion should succeed");
    let completed_transfer = repository
        .get_transfer("tenant-a", "transfer-transition")
        .await
        .expect("transfer lookup should succeed")
        .expect("transfer should exist");
    assert_eq!(completed_transfer.status, "completed");
    assert_eq!(completed_transfer.metadata["credit"], "confirmed");

    let err = service
        .transition_transfer_status(
            "tenant-a",
            "transfer-transition",
            "submitted",
            serde_json::json!({ "credit": "rollback" }),
        )
        .await
        .expect_err("completed transfer should reject reopening");
    assert!(matches!(err, ramp_common::Error::InvalidStateTransition { .. }));

    service
        .transition_source_of_funds_review_status(
            "tenant-a",
            "package-transition",
            "approved",
            serde_json::json!({ "decision": "clear" }),
        )
        .await
        .expect("package approval should succeed");
    let approved_package = repository
        .get_source_of_funds_package("tenant-a", "package-transition")
        .await
        .expect("package lookup should succeed")
        .expect("package should exist");
    assert_eq!(approved_package.review_status, "approved");
    assert_eq!(approved_package.metadata["decision"], "clear");

    let err = service
        .transition_source_of_funds_review_status(
            "tenant-a",
            "package-transition",
            "in_review",
            serde_json::json!({ "decision": "rollback" }),
        )
        .await
        .expect_err("approved package should reject reopening");
    assert!(matches!(err, ramp_common::Error::InvalidStateTransition { .. }));
}
