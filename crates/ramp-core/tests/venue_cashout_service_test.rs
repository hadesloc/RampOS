use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use ramp_common::{types::TenantId, Result};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::{
    BeneficiaryProfileFilter, BeneficiaryProfileRecord, EnsureWalletAttestationRequest,
    LpReliabilitySnapshotRow, RfqBidRow, RfqRepository, RfqRequestRow,
    SourceOfFundsPackageFilter, SourceOfFundsPackageRecord, UpsertBeneficiaryProfileRequest,
    UpsertSourceOfFundsPackageRequest, UpsertVenueAccountRequest, UpsertVenueConnectionRequest,
    UpsertVenueTransferRequest, VenueAccountFilter, VenueAccountRecord, VenueConnectionFilter,
    VenueConnectionRecord, VenueTransferFilter, VenueTransferRecord, VenueTrustRepository,
    WalletAttestationFilter, WalletAttestationRecord,
};
use ramp_core::service::rfq::RfqService;
use ramp_core::service::venue_cashout::{
    ConfirmVenueCashoutReceiptRequest, PrepareHyperliquidCashoutRequest, VenueCashoutService,
};
use rust_decimal_macros::dec;
use uuid::Uuid;

#[derive(Default)]
struct MockVenueTrustRepository {
    attestations: Mutex<HashMap<Uuid, WalletAttestationRecord>>,
    connections: Mutex<HashMap<String, VenueConnectionRecord>>,
    accounts: Mutex<HashMap<String, VenueAccountRecord>>,
    beneficiaries: Mutex<HashMap<String, BeneficiaryProfileRecord>>,
    transfers: Mutex<HashMap<String, VenueTransferRecord>>,
}

#[async_trait]
impl VenueTrustRepository for MockVenueTrustRepository {
    async fn ensure_wallet_attestation(
        &self,
        _request: &EnsureWalletAttestationRequest,
    ) -> Result<()> {
        panic!("ensure_wallet_attestation should not be called in venue cashout tests")
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
        panic!("list_wallet_attestations should not be called in venue cashout tests")
    }

    async fn upsert_connection(&self, _request: &UpsertVenueConnectionRequest) -> Result<()> {
        panic!("upsert_connection should not be called in venue cashout tests")
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
        panic!("list_connections should not be called in venue cashout tests")
    }

    async fn upsert_account(&self, _request: &UpsertVenueAccountRequest) -> Result<()> {
        panic!("upsert_account should not be called in venue cashout tests")
    }

    async fn get_account(&self, tenant_id: &str, account_id: &str) -> Result<Option<VenueAccountRecord>> {
        Ok(self
            .accounts
            .lock()
            .expect("accounts lock")
            .get(account_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_accounts(&self, _filter: &VenueAccountFilter) -> Result<Vec<VenueAccountRecord>> {
        panic!("list_accounts should not be called in venue cashout tests")
    }

    async fn upsert_beneficiary_profile(
        &self,
        _request: &UpsertBeneficiaryProfileRequest,
    ) -> Result<()> {
        panic!("upsert_beneficiary_profile should not be called in venue cashout tests")
    }

    async fn get_beneficiary_profile(
        &self,
        tenant_id: &str,
        beneficiary_profile_id: &str,
    ) -> Result<Option<BeneficiaryProfileRecord>> {
        Ok(self
            .beneficiaries
            .lock()
            .expect("beneficiaries lock")
            .get(beneficiary_profile_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_beneficiary_profiles(
        &self,
        _filter: &BeneficiaryProfileFilter,
    ) -> Result<Vec<BeneficiaryProfileRecord>> {
        panic!("list_beneficiary_profiles should not be called in venue cashout tests")
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

    async fn get_transfer(&self, tenant_id: &str, transfer_id: &str) -> Result<Option<VenueTransferRecord>> {
        Ok(self
            .transfers
            .lock()
            .expect("transfers lock")
            .get(transfer_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_transfers(&self, _filter: &VenueTransferFilter) -> Result<Vec<VenueTransferRecord>> {
        panic!("list_transfers should not be called in venue cashout tests")
    }

    async fn upsert_source_of_funds_package(
        &self,
        _request: &UpsertSourceOfFundsPackageRequest,
    ) -> Result<()> {
        panic!("upsert_source_of_funds_package should not be called in venue cashout tests")
    }

    async fn get_source_of_funds_package(
        &self,
        _tenant_id: &str,
        _package_id: &str,
    ) -> Result<Option<SourceOfFundsPackageRecord>> {
        panic!("get_source_of_funds_package should not be called in venue cashout tests")
    }

    async fn list_source_of_funds_packages(
        &self,
        _filter: &SourceOfFundsPackageFilter,
    ) -> Result<Vec<SourceOfFundsPackageRecord>> {
        panic!("list_source_of_funds_packages should not be called in venue cashout tests")
    }
}

#[derive(Default)]
struct TestRfqRepository {
    requests: tokio::sync::RwLock<Vec<RfqRequestRow>>,
}

#[async_trait]
impl RfqRepository for TestRfqRepository {
    async fn create_request(&self, req: &RfqRequestRow) -> Result<()> {
        self.requests.write().await.push(req.clone());
        Ok(())
    }

    async fn get_request(&self, _tenant_id: &TenantId, id: &str) -> Result<Option<RfqRequestRow>> {
        Ok(self
            .requests
            .read()
            .await
            .iter()
            .find(|request| request.id == id)
            .cloned())
    }

    async fn list_open_requests(
        &self,
        _tenant_id: &TenantId,
        _direction: Option<&str>,
        _limit: i64,
        _offset: i64,
    ) -> Result<Vec<RfqRequestRow>> {
        panic!("list_open_requests should not be called in venue cashout tests")
    }

    async fn update_request(&self, req: &RfqRequestRow) -> Result<()> {
        let mut requests = self.requests.write().await;
        if let Some(existing) = requests.iter_mut().find(|request| request.id == req.id) {
            *existing = req.clone();
        }
        Ok(())
    }

    async fn create_bid(&self, _bid: &RfqBidRow) -> Result<()> {
        panic!("create_bid should not be called in venue cashout tests")
    }

    async fn list_bids_for_request(
        &self,
        _tenant_id: &TenantId,
        _rfq_id: &str,
    ) -> Result<Vec<RfqBidRow>> {
        panic!("list_bids_for_request should not be called in venue cashout tests")
    }

    async fn get_best_bid(
        &self,
        _tenant_id: &TenantId,
        _rfq_id: &str,
        _direction: &str,
    ) -> Result<Option<RfqBidRow>> {
        panic!("get_best_bid should not be called in venue cashout tests")
    }

    async fn update_bid_state(
        &self,
        _tenant_id: &TenantId,
        _bid_id: &str,
        _state: &str,
    ) -> Result<()> {
        panic!("update_bid_state should not be called in venue cashout tests")
    }

    async fn upsert_reliability_snapshot(&self, _snapshot: &LpReliabilitySnapshotRow) -> Result<()> {
        panic!("upsert_reliability_snapshot should not be called in venue cashout tests")
    }

    async fn list_reliability_snapshots(
        &self,
        _tenant_id: &TenantId,
        _lp_id: &str,
        _direction: Option<&str>,
        _limit: i64,
    ) -> Result<Vec<LpReliabilitySnapshotRow>> {
        panic!("list_reliability_snapshots should not be called in venue cashout tests")
    }

    async fn get_latest_reliability_snapshot(
        &self,
        _tenant_id: &TenantId,
        _lp_id: &str,
        _direction: &str,
        _window_kind: &str,
    ) -> Result<Option<LpReliabilitySnapshotRow>> {
        panic!("get_latest_reliability_snapshot should not be called in venue cashout tests")
    }
}

fn build_service(
    venue_repo: Arc<MockVenueTrustRepository>,
    rfq_repo: Arc<TestRfqRepository>,
) -> VenueCashoutService {
    VenueCashoutService::new(
        venue_repo,
        RfqService::new(rfq_repo, Arc::new(InMemoryEventPublisher::new())),
    )
}

fn seed_hyperliquid_subject(repo: &Arc<MockVenueTrustRepository>) -> Uuid {
    let attestation_id = Uuid::parse_str("51000000-0000-0000-0000-000000000029").unwrap();
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
            proof_artifact_uri: Some("s3://proofs/hyperliquid-wallet.json".to_string()),
            risk_state: Some("clear".to_string()),
            metadata: serde_json::json!({ "scope": "venue_cashout" }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    repo.connections.lock().unwrap().insert(
        "conn-hl-1".to_string(),
        VenueConnectionRecord {
            connection_id: "conn-hl-1".to_string(),
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
        "acct-hl-1".to_string(),
        VenueAccountRecord {
            account_id: "acct-hl-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            venue_connection_id: "conn-hl-1".to_string(),
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
    repo.beneficiaries.lock().unwrap().insert(
        "beneficiary-1".to_string(),
        BeneficiaryProfileRecord {
            beneficiary_profile_id: "beneficiary-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            user_id: Some("user-a".to_string()),
            destination_type: "bank_account".to_string(),
            destination_ref: "vcb:0123456789".to_string(),
            display_name: Some("Primary payout".to_string()),
            asset_symbol: Some("USDT".to_string()),
            network: Some("ethereum".to_string()),
            verification_status: "verified".to_string(),
            cooldown_ends_at: None,
            metadata: serde_json::json!({}),
            last_verified_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    attestation_id
}

#[tokio::test]
async fn prepare_hyperliquid_cashout_creates_draft_transfer() {
    let venue_repo = Arc::new(MockVenueTrustRepository::default());
    let rfq_repo = Arc::new(TestRfqRepository::default());
    let service = build_service(venue_repo.clone(), rfq_repo);
    let attestation_id = seed_hyperliquid_subject(&venue_repo);

    let prepared = service
        .prepare_hyperliquid_cashout(&PrepareHyperliquidCashoutRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            venue_connection_id: "conn-hl-1".to_string(),
            venue_account_id: "acct-hl-1".to_string(),
            beneficiary_profile_id: "beneficiary-1".to_string(),
            wallet_attestation_id: attestation_id,
            asset_symbol: "USDT".to_string(),
            network: "ethereum".to_string(),
            amount: dec!(125),
        })
        .await
        .expect("prepare should succeed");

    assert_eq!(prepared.venue_key, "hyperliquid");
    assert_eq!(prepared.status, "draft");
    assert_eq!(prepared.transfer_direction, "venue_to_wallet");
    assert!(prepared.rfq_id.is_none());

    let stored = venue_repo
        .get_transfer("tenant-a", &prepared.transfer_id)
        .await
        .unwrap()
        .expect("transfer should persist");
    assert_eq!(stored.status, "draft");
    assert_eq!(stored.transfer_direction, "venue_to_wallet");
    assert_eq!(stored.amount, dec!(125));
}

#[tokio::test]
async fn confirm_wallet_receipt_backfills_rfq_and_marks_transfer_submitted() {
    let venue_repo = Arc::new(MockVenueTrustRepository::default());
    let rfq_repo = Arc::new(TestRfqRepository::default());
    let service = build_service(venue_repo.clone(), rfq_repo.clone());
    let attestation_id = seed_hyperliquid_subject(&venue_repo);

    let prepared = service
        .prepare_hyperliquid_cashout(&PrepareHyperliquidCashoutRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            venue_connection_id: "conn-hl-1".to_string(),
            venue_account_id: "acct-hl-1".to_string(),
            beneficiary_profile_id: "beneficiary-1".to_string(),
            wallet_attestation_id: attestation_id,
            asset_symbol: "USDT".to_string(),
            network: "ethereum".to_string(),
            amount: dec!(42),
        })
        .await
        .expect("prepare should succeed");

    let confirmed = service
        .confirm_wallet_receipt(&ConfirmVenueCashoutReceiptRequest {
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            transfer_id: prepared.transfer_id.clone(),
            wallet_tx_hash: "0xwalletreceipt".to_string(),
            ttl_minutes: Some(7),
        })
        .await
        .expect("confirm should succeed");

    assert_eq!(confirmed.transfer_id, prepared.transfer_id);
    assert_eq!(confirmed.status, "submitted");
    assert!(confirmed.rfq_id.starts_with("rfq_"));
    assert_eq!(confirmed.wallet_tx_hash, "0xwalletreceipt");

    let stored_transfer = venue_repo
        .get_transfer("tenant-a", &prepared.transfer_id)
        .await
        .unwrap()
        .expect("transfer should persist");
    assert_eq!(stored_transfer.status, "submitted");
    assert_eq!(stored_transfer.wallet_tx_hash.as_deref(), Some("0xwalletreceipt"));
    assert_eq!(stored_transfer.rfq_id.as_deref(), Some(confirmed.rfq_id.as_str()));

    let rfq = rfq_repo
        .get_request(&TenantId("tenant-a".to_string()), &confirmed.rfq_id)
        .await
        .unwrap()
        .expect("rfq should exist");
    assert_eq!(rfq.direction, "OFFRAMP");
    assert_eq!(rfq.crypto_asset, "USDT");
    assert_eq!(rfq.crypto_amount, dec!(42));
    assert_eq!(rfq.offramp_id.as_deref(), Some(confirmed.offramp_reference.as_str()));
}
