use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use ramp_common::Result;
use ramp_core::repository::{
    BeneficiaryProfileFilter, BeneficiaryProfileRecord, EnsureWalletAttestationRequest,
    SourceOfFundsPackageFilter, SourceOfFundsPackageRecord, UpsertBeneficiaryProfileRequest,
    UpsertSourceOfFundsPackageRequest, UpsertVenueAccountRequest, UpsertVenueConnectionRequest,
    UpsertVenueTransferRequest, VenueAccountFilter, VenueAccountRecord, VenueConnectionFilter,
    VenueConnectionRecord, VenueTransferFilter, VenueTransferRecord, VenueTrustRepository,
    WalletAttestationFilter, WalletAttestationRecord,
};
use ramp_core::service::venue_funding::{
    PrepareVenueFundingTransferRequest, SubmitVenueFundingTransferRequest,
    VenueFundingService,
};
use rust_decimal_macros::dec;
use uuid::Uuid;

#[derive(Default)]
struct MockVenueTrustRepository {
    attestations: Mutex<HashMap<Uuid, WalletAttestationRecord>>,
    connections: Mutex<HashMap<String, VenueConnectionRecord>>,
    accounts: Mutex<HashMap<String, VenueAccountRecord>>,
    transfers: Mutex<HashMap<String, VenueTransferRecord>>,
}

#[async_trait]
impl VenueTrustRepository for MockVenueTrustRepository {
    async fn ensure_wallet_attestation(
        &self,
        _request: &EnsureWalletAttestationRequest,
    ) -> Result<()> {
        panic!("ensure_wallet_attestation should not be called in venue funding tests")
    }

    async fn get_wallet_attestation(
        &self,
        tenant_id: &str,
        attestation_id: Uuid,
    ) -> Result<Option<WalletAttestationRecord>> {
        Ok(self
            .attestations
            .lock()
            .expect("attestations lock")
            .get(&attestation_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_wallet_attestations(
        &self,
        _filter: &WalletAttestationFilter,
    ) -> Result<Vec<WalletAttestationRecord>> {
        panic!("list_wallet_attestations should not be called in venue funding tests")
    }

    async fn upsert_connection(&self, _request: &UpsertVenueConnectionRequest) -> Result<()> {
        panic!("upsert_connection should not be called in venue funding tests")
    }

    async fn get_connection(
        &self,
        tenant_id: &str,
        connection_id: &str,
    ) -> Result<Option<VenueConnectionRecord>> {
        Ok(self
            .connections
            .lock()
            .expect("connections lock")
            .get(connection_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_connections(
        &self,
        _filter: &VenueConnectionFilter,
    ) -> Result<Vec<VenueConnectionRecord>> {
        panic!("list_connections should not be called in venue funding tests")
    }

    async fn upsert_account(&self, _request: &UpsertVenueAccountRequest) -> Result<()> {
        panic!("upsert_account should not be called in venue funding tests")
    }

    async fn get_account(
        &self,
        tenant_id: &str,
        account_id: &str,
    ) -> Result<Option<VenueAccountRecord>> {
        Ok(self
            .accounts
            .lock()
            .expect("accounts lock")
            .get(account_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_accounts(&self, _filter: &VenueAccountFilter) -> Result<Vec<VenueAccountRecord>> {
        panic!("list_accounts should not be called in venue funding tests")
    }

    async fn upsert_beneficiary_profile(
        &self,
        _request: &UpsertBeneficiaryProfileRequest,
    ) -> Result<()> {
        panic!("upsert_beneficiary_profile should not be called in venue funding tests")
    }

    async fn get_beneficiary_profile(
        &self,
        _tenant_id: &str,
        _beneficiary_profile_id: &str,
    ) -> Result<Option<BeneficiaryProfileRecord>> {
        panic!("get_beneficiary_profile should not be called in venue funding tests")
    }

    async fn list_beneficiary_profiles(
        &self,
        _filter: &BeneficiaryProfileFilter,
    ) -> Result<Vec<BeneficiaryProfileRecord>> {
        panic!("list_beneficiary_profiles should not be called in venue funding tests")
    }

    async fn upsert_transfer(&self, request: &UpsertVenueTransferRequest) -> Result<()> {
        self.transfers.lock().expect("transfers lock").insert(
            request.transfer_id.clone(),
            VenueTransferRecord {
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
            },
        );
        Ok(())
    }

    async fn get_transfer(
        &self,
        tenant_id: &str,
        transfer_id: &str,
    ) -> Result<Option<VenueTransferRecord>> {
        Ok(self
            .transfers
            .lock()
            .expect("transfers lock")
            .get(transfer_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_transfers(&self, _filter: &VenueTransferFilter) -> Result<Vec<VenueTransferRecord>> {
        panic!("list_transfers should not be called in venue funding tests")
    }

    async fn upsert_source_of_funds_package(
        &self,
        _request: &UpsertSourceOfFundsPackageRequest,
    ) -> Result<()> {
        panic!("upsert_source_of_funds_package should not be called in venue funding tests")
    }

    async fn get_source_of_funds_package(
        &self,
        _tenant_id: &str,
        _package_id: &str,
    ) -> Result<Option<SourceOfFundsPackageRecord>> {
        panic!("get_source_of_funds_package should not be called in venue funding tests")
    }

    async fn list_source_of_funds_packages(
        &self,
        _filter: &SourceOfFundsPackageFilter,
    ) -> Result<Vec<SourceOfFundsPackageRecord>> {
        panic!("list_source_of_funds_packages should not be called in venue funding tests")
    }
}

fn build_service(repo: Arc<MockVenueTrustRepository>) -> VenueFundingService {
    VenueFundingService::new(repo)
}

fn seed_subject(repo: &Arc<MockVenueTrustRepository>) -> Uuid {
    let attestation_id = Uuid::parse_str("52000000-0000-0000-0000-000000000028").unwrap();
    repo.attestations.lock().unwrap().insert(
        attestation_id,
        WalletAttestationRecord {
            attestation_id,
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            wallet_address: "0xwallet".to_string(),
            chain_id: "ethereum".to_string(),
            attestation_status: "verified".to_string(),
            proof_kind: "signature".to_string(),
            proof_artifact_uri: Some("s3://proofs/venue-funding-wallet.json".to_string()),
            risk_state: Some("clear".to_string()),
            metadata: serde_json::json!({ "scope": "venue_funding" }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    repo.connections.lock().unwrap().insert(
        "conn-vf-1".to_string(),
        VenueConnectionRecord {
            connection_id: "conn-vf-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            user_id: Some("user-a".to_string()),
            venue_key: "hyperliquid".to_string(),
            connection_mode: "wallet_linked".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({ "connector": "read_only" }),
            last_verified_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    repo.accounts.lock().unwrap().insert(
        "acct-vf-1".to_string(),
        VenueAccountRecord {
            account_id: "acct-vf-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            venue_connection_id: "conn-vf-1".to_string(),
            venue_key: "hyperliquid".to_string(),
            account_label: Some("Main".to_string()),
            account_ref: Some("hl-main".to_string()),
            wallet_address: Some("0xwallet".to_string()),
            subaccount_ref: Some("sub-1".to_string()),
            api_scope_summary: serde_json::json!({ "permissions": ["read"] }),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
            last_verified_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    attestation_id
}

#[tokio::test]
async fn prepare_wallet_to_venue_transfer_creates_draft_transfer() {
    let repo = Arc::new(MockVenueTrustRepository::default());
    let service = build_service(repo.clone());
    let attestation_id = seed_subject(&repo);

    let prepared = service
        .prepare_wallet_to_venue_transfer(&PrepareVenueFundingTransferRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            venue_connection_id: "conn-vf-1".to_string(),
            venue_account_id: "acct-vf-1".to_string(),
            wallet_attestation_id: attestation_id,
            asset_symbol: "USDT".to_string(),
            network: "ethereum".to_string(),
            amount: dec!(150),
            origin_intent_id: Some("intent_123".to_string()),
        })
        .await
        .expect("prepare should succeed");

    assert_eq!(prepared.venue_key, "hyperliquid");
    assert_eq!(prepared.status, "draft");
    assert_eq!(prepared.transfer_direction, "wallet_to_venue");
    assert_eq!(prepared.origin_intent_id.as_deref(), Some("intent_123"));

    let stored = repo
        .get_transfer("tenant-a", &prepared.transfer_id)
        .await
        .unwrap()
        .expect("transfer should persist");
    assert_eq!(stored.status, "draft");
    assert_eq!(stored.transfer_direction, "wallet_to_venue");
    assert_eq!(stored.amount, dec!(150));
    assert_eq!(stored.origin_intent_id.as_deref(), Some("intent_123"));
}

#[tokio::test]
async fn submit_wallet_to_venue_transfer_attaches_wallet_reference() {
    let repo = Arc::new(MockVenueTrustRepository::default());
    let service = build_service(repo.clone());
    let attestation_id = seed_subject(&repo);

    let prepared = service
        .prepare_wallet_to_venue_transfer(&PrepareVenueFundingTransferRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            venue_connection_id: "conn-vf-1".to_string(),
            venue_account_id: "acct-vf-1".to_string(),
            wallet_attestation_id: attestation_id,
            asset_symbol: "USDT".to_string(),
            network: "ethereum".to_string(),
            amount: dec!(75),
            origin_intent_id: None,
        })
        .await
        .expect("prepare should succeed");

    let submitted = service
        .submit_wallet_to_venue_transfer(&SubmitVenueFundingTransferRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            transfer_id: prepared.transfer_id.clone(),
            wallet_transfer_reference: "0xwalletfunding".to_string(),
        })
        .await
        .expect("submit should succeed");

    assert_eq!(submitted.transfer_id, prepared.transfer_id);
    assert_eq!(submitted.status, "submitted");
    assert_eq!(submitted.wallet_transfer_reference, "0xwalletfunding");

    let stored = repo
        .get_transfer("tenant-a", &prepared.transfer_id)
        .await
        .unwrap()
        .expect("transfer should persist");
    assert_eq!(stored.status, "submitted");
    assert_eq!(stored.wallet_tx_hash.as_deref(), Some("0xwalletfunding"));
    assert_eq!(
        stored.metadata["walletTransferReference"].as_str(),
        Some("0xwalletfunding")
    );
    assert!(stored.submitted_at.is_some());
}

#[tokio::test]
async fn get_wallet_to_venue_transfer_returns_persisted_state() {
    let repo = Arc::new(MockVenueTrustRepository::default());
    let service = build_service(repo.clone());
    let attestation_id = seed_subject(&repo);

    let prepared = service
        .prepare_wallet_to_venue_transfer(&PrepareVenueFundingTransferRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            venue_connection_id: "conn-vf-1".to_string(),
            venue_account_id: "acct-vf-1".to_string(),
            wallet_attestation_id: attestation_id,
            asset_symbol: "USDT".to_string(),
            network: "ethereum".to_string(),
            amount: dec!(90),
            origin_intent_id: Some("intent_456".to_string()),
        })
        .await
        .expect("prepare should succeed");

    service
        .submit_wallet_to_venue_transfer(&SubmitVenueFundingTransferRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            transfer_id: prepared.transfer_id.clone(),
            wallet_transfer_reference: "0xwalletfunding2".to_string(),
        })
        .await
        .expect("submit should succeed");

    let persisted = service
        .get_wallet_to_venue_transfer("tenant-a", "user-a", &prepared.transfer_id)
        .await
        .expect("status lookup should succeed")
        .expect("transfer should exist");

    assert_eq!(persisted.transfer_id, prepared.transfer_id);
    assert_eq!(persisted.venue_key, "hyperliquid");
    assert_eq!(persisted.transfer_direction, "wallet_to_venue");
    assert_eq!(persisted.status, "submitted");
    assert_eq!(persisted.amount, dec!(90));
    assert_eq!(persisted.origin_intent_id.as_deref(), Some("intent_456"));
    assert_eq!(
        persisted.wallet_transfer_reference.as_deref(),
        Some("0xwalletfunding2")
    );
}

#[tokio::test]
async fn prepare_wallet_to_venue_transfer_rejects_non_hyperliquid_connection() {
    let repo = Arc::new(MockVenueTrustRepository::default());
    let service = build_service(repo.clone());
    let attestation_id = seed_subject(&repo);

    let existing = repo
        .connections
        .lock()
        .unwrap()
        .get("conn-vf-1")
        .cloned()
        .expect("seeded connection should exist");
    repo.connections.lock().unwrap().insert(
        "conn-vf-1".to_string(),
        VenueConnectionRecord {
            venue_key: "kraken".to_string(),
            ..existing
        },
    );

    let error = service
        .prepare_wallet_to_venue_transfer(&PrepareVenueFundingTransferRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            venue_connection_id: "conn-vf-1".to_string(),
            venue_account_id: "acct-vf-1".to_string(),
            wallet_attestation_id: attestation_id,
            asset_symbol: "USDT".to_string(),
            network: "ethereum".to_string(),
            amount: dec!(25),
            origin_intent_id: None,
        })
        .await
        .expect_err("non-hyperliquid connection should be rejected");

    assert!(
        error
            .to_string()
            .contains("venue funding currently supports only hyperliquid"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn prepare_wallet_to_venue_transfer_rejects_non_wallet_linked_mode() {
    let repo = Arc::new(MockVenueTrustRepository::default());
    let service = build_service(repo.clone());
    let attestation_id = seed_subject(&repo);

    let existing = repo
        .connections
        .lock()
        .unwrap()
        .get("conn-vf-1")
        .cloned()
        .expect("seeded connection should exist");
    repo.connections.lock().unwrap().insert(
        "conn-vf-1".to_string(),
        VenueConnectionRecord {
            connection_mode: "oauth".to_string(),
            ..existing
        },
    );

    let error = service
        .prepare_wallet_to_venue_transfer(&PrepareVenueFundingTransferRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            venue_connection_id: "conn-vf-1".to_string(),
            venue_account_id: "acct-vf-1".to_string(),
            wallet_attestation_id: attestation_id,
            asset_symbol: "USDT".to_string(),
            network: "ethereum".to_string(),
            amount: dec!(25),
            origin_intent_id: None,
        })
        .await
        .expect_err("non-wallet-linked mode should be rejected");

    assert!(
        error
            .to_string()
            .contains("hyperliquid funding requires wallet_linked connection mode"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn prepare_wallet_to_venue_transfer_rejects_non_usdt_asset_for_hyperliquid() {
    let repo = Arc::new(MockVenueTrustRepository::default());
    let service = build_service(repo.clone());
    let attestation_id = seed_subject(&repo);

    let error = service
        .prepare_wallet_to_venue_transfer(&PrepareVenueFundingTransferRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            venue_connection_id: "conn-vf-1".to_string(),
            venue_account_id: "acct-vf-1".to_string(),
            wallet_attestation_id: attestation_id,
            asset_symbol: "USDC".to_string(),
            network: "ethereum".to_string(),
            amount: dec!(25),
            origin_intent_id: None,
        })
        .await
        .expect_err("non-USDT asset should be rejected");

    assert!(
        error
            .to_string()
            .contains("hyperliquid funding currently supports only USDT"),
        "unexpected error: {error}"
    );
}
