use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use ramp_common::Result;
use ramp_core::repository::{
    BeneficiaryProfileFilter, BeneficiaryProfileRecord, EnsureWalletAttestationRequest,
    SourceOfFundsPackageFilter, SourceOfFundsPackageRecord, UpsertBeneficiaryProfileRequest,
    UpsertSourceOfFundsPackageRequest, UpsertVenueAccountRequest, UpsertVenueConnectionRequest,
    UpsertVenueTransferRequest, VenueAccountFilter, VenueAccountRecord, VenueConnectionFilter,
    VenueConnectionRecord, VenueTransferFilter, VenueTransferRecord, VenueTrustRepository,
    WalletAttestationFilter, WalletAttestationRecord,
};
use ramp_core::service::VenueTrustService;

#[derive(Default)]
struct MockVenueTrustRepository {
    connections: Mutex<Vec<VenueConnectionRecord>>,
    accounts: Mutex<Vec<VenueAccountRecord>>,
}

#[async_trait]
impl VenueTrustRepository for MockVenueTrustRepository {
    async fn ensure_wallet_attestation(
        &self,
        _request: &EnsureWalletAttestationRequest,
    ) -> Result<()> {
        panic!("ensure_wallet_attestation should not be called")
    }

    async fn get_wallet_attestation(
        &self,
        _tenant_id: &str,
        _attestation_id: Uuid,
    ) -> Result<Option<WalletAttestationRecord>> {
        panic!("get_wallet_attestation should not be called")
    }

    async fn list_wallet_attestations(
        &self,
        _filter: &WalletAttestationFilter,
    ) -> Result<Vec<WalletAttestationRecord>> {
        Ok(Vec::new())
    }

    async fn upsert_connection(&self, _request: &UpsertVenueConnectionRequest) -> Result<()> {
        panic!("upsert_connection should not be called")
    }

    async fn get_connection(
        &self,
        _tenant_id: &str,
        _connection_id: &str,
    ) -> Result<Option<VenueConnectionRecord>> {
        panic!("get_connection should not be called")
    }

    async fn list_connections(
        &self,
        filter: &VenueConnectionFilter,
    ) -> Result<Vec<VenueConnectionRecord>> {
        Ok(self
            .connections
            .lock()
            .expect("connections lock")
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

    async fn upsert_account(&self, _request: &UpsertVenueAccountRequest) -> Result<()> {
        panic!("upsert_account should not be called")
    }

    async fn get_account(
        &self,
        _tenant_id: &str,
        _account_id: &str,
    ) -> Result<Option<VenueAccountRecord>> {
        panic!("get_account should not be called")
    }

    async fn list_accounts(&self, filter: &VenueAccountFilter) -> Result<Vec<VenueAccountRecord>> {
        Ok(self
            .accounts
            .lock()
            .expect("accounts lock")
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
        _request: &UpsertBeneficiaryProfileRequest,
    ) -> Result<()> {
        panic!("upsert_beneficiary_profile should not be called")
    }

    async fn get_beneficiary_profile(
        &self,
        _tenant_id: &str,
        _beneficiary_profile_id: &str,
    ) -> Result<Option<BeneficiaryProfileRecord>> {
        panic!("get_beneficiary_profile should not be called")
    }

    async fn list_beneficiary_profiles(
        &self,
        _filter: &BeneficiaryProfileFilter,
    ) -> Result<Vec<BeneficiaryProfileRecord>> {
        Ok(Vec::new())
    }

    async fn upsert_transfer(&self, _request: &UpsertVenueTransferRequest) -> Result<()> {
        panic!("upsert_transfer should not be called")
    }

    async fn get_transfer(
        &self,
        _tenant_id: &str,
        _transfer_id: &str,
    ) -> Result<Option<VenueTransferRecord>> {
        panic!("get_transfer should not be called")
    }

    async fn list_transfers(
        &self,
        _filter: &VenueTransferFilter,
    ) -> Result<Vec<VenueTransferRecord>> {
        Ok(Vec::new())
    }

    async fn upsert_source_of_funds_package(
        &self,
        _request: &UpsertSourceOfFundsPackageRequest,
    ) -> Result<()> {
        panic!("upsert_source_of_funds_package should not be called")
    }

    async fn get_source_of_funds_package(
        &self,
        _tenant_id: &str,
        _package_id: &str,
    ) -> Result<Option<SourceOfFundsPackageRecord>> {
        panic!("get_source_of_funds_package should not be called")
    }

    async fn list_source_of_funds_packages(
        &self,
        _filter: &SourceOfFundsPackageFilter,
    ) -> Result<Vec<SourceOfFundsPackageRecord>> {
        Ok(Vec::new())
    }
}

fn build_service(repo: Arc<MockVenueTrustRepository>) -> VenueTrustService {
    VenueTrustService::with_repository(repo)
}

#[tokio::test]
async fn lighter_connector_readiness_reports_ready_when_operator_evidence_is_present() {
    let repo = Arc::new(MockVenueTrustRepository::default());
    repo.connections.lock().unwrap().push(VenueConnectionRecord {
        connection_id: "lighter-conn-1".to_string(),
        tenant_id: "tenant-a".to_string(),
        subject_type: "user".to_string(),
        subject_id: "user-a".to_string(),
        user_id: Some("user-a".to_string()),
        venue_key: "lighter".to_string(),
        connection_mode: "operator_linked".to_string(),
        status: "active".to_string(),
        metadata: serde_json::json!({
            "public_pool_mode": "enabled",
            "operator_linkage_status": "verified",
            "institutional_evidence_status": "approved"
        }),
        last_verified_at: Some(Utc::now()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });
    repo.accounts.lock().unwrap().push(VenueAccountRecord {
        account_id: "lighter-acct-1".to_string(),
        tenant_id: "tenant-a".to_string(),
        venue_connection_id: "lighter-conn-1".to_string(),
        venue_key: "lighter".to_string(),
        account_label: Some("Lighter Ops".to_string()),
        account_ref: Some("lighter-main".to_string()),
        wallet_address: None,
        subaccount_ref: Some("ops".to_string()),
        api_scope_summary: serde_json::json!({ "permissions": ["read"] }),
        status: "active".to_string(),
        metadata: serde_json::json!({
            "proof_anchor_mode": "operator_exit_proof"
        }),
        last_verified_at: Some(Utc::now()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    let readiness = build_service(repo)
        .get_lighter_connector_readiness("tenant-a", "user", "user-a")
        .await
        .expect("lighter readiness should load");

    assert_eq!(readiness.connector_key, "lighter");
    assert_eq!(readiness.status, "ready");
    assert_eq!(readiness.public_pool_mode.as_deref(), Some("enabled"));
    assert_eq!(
        readiness.proof_anchor_mode.as_deref(),
        Some("operator_exit_proof")
    );
    assert_eq!(
        readiness.operator_linkage_status.as_deref(),
        Some("verified")
    );
    assert!(readiness
        .requirements
        .iter()
        .all(|requirement| requirement.satisfied));
}

#[tokio::test]
async fn lighter_connector_readiness_reports_attention_when_operator_evidence_is_missing() {
    let repo = Arc::new(MockVenueTrustRepository::default());
    repo.connections.lock().unwrap().push(VenueConnectionRecord {
        connection_id: "lighter-conn-2".to_string(),
        tenant_id: "tenant-a".to_string(),
        subject_type: "user".to_string(),
        subject_id: "user-a".to_string(),
        user_id: Some("user-a".to_string()),
        venue_key: "lighter".to_string(),
        connection_mode: "wallet_linked".to_string(),
        status: "pending_review".to_string(),
        metadata: serde_json::json!({}),
        last_verified_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    let readiness = build_service(repo)
        .get_lighter_connector_readiness("tenant-a", "user", "user-a")
        .await
        .expect("lighter readiness should load");

    assert_eq!(readiness.status, "attention_required");
    assert_eq!(
        readiness
            .requirements
            .iter()
            .filter(|requirement| !requirement.satisfied)
            .count(),
        4
    );
    assert!(readiness.requirements.iter().any(|requirement| {
        requirement.code == "lighter_operator_linkage"
            && requirement.message.contains("operator-linked")
    }));
}
