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
use ramp_core::service::venue_trust_reporting::VenueTrustReportingService;
use rust_decimal_macros::dec;
use uuid::Uuid;

#[derive(Default)]
struct MockVenueTrustRepository {
    connections: Mutex<HashMap<String, VenueConnectionRecord>>,
    accounts: Mutex<HashMap<String, VenueAccountRecord>>,
    beneficiaries: Mutex<HashMap<String, BeneficiaryProfileRecord>>,
    attestations: Mutex<HashMap<Uuid, WalletAttestationRecord>>,
    transfers: Mutex<HashMap<String, VenueTransferRecord>>,
    source_of_funds_packages: Mutex<HashMap<String, SourceOfFundsPackageRecord>>,
}

#[async_trait]
impl VenueTrustRepository for MockVenueTrustRepository {
    async fn ensure_wallet_attestation(
        &self,
        _request: &EnsureWalletAttestationRequest,
    ) -> Result<()> {
        panic!("ensure_wallet_attestation should not be called in reporting tests")
    }

    async fn get_wallet_attestation(
        &self,
        tenant_id: &str,
        attestation_id: Uuid,
    ) -> Result<Option<WalletAttestationRecord>> {
        Ok(self
            .attestations
            .lock()
            .unwrap()
            .get(&attestation_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_wallet_attestations(
        &self,
        filter: &WalletAttestationFilter,
    ) -> Result<Vec<WalletAttestationRecord>> {
        Ok(self
            .attestations
            .lock()
            .unwrap()
            .values()
            .filter(|record| record.tenant_id == filter.tenant_id)
            .filter(|record| filter.user_id.as_ref().map(|id| &record.user_id == id).unwrap_or(true))
            .cloned()
            .collect())
    }

    async fn upsert_connection(&self, _request: &UpsertVenueConnectionRequest) -> Result<()> {
        panic!("upsert_connection should not be called in reporting tests")
    }

    async fn get_connection(
        &self,
        tenant_id: &str,
        connection_id: &str,
    ) -> Result<Option<VenueConnectionRecord>> {
        Ok(self
            .connections
            .lock()
            .unwrap()
            .get(connection_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_connections(
        &self,
        filter: &VenueConnectionFilter,
    ) -> Result<Vec<VenueConnectionRecord>> {
        Ok(self
            .connections
            .lock()
            .unwrap()
            .values()
            .filter(|record| record.tenant_id == filter.tenant_id)
            .filter(|record| {
                filter
                    .subject_type
                    .as_ref()
                    .map(|value| &record.subject_type == value)
                    .unwrap_or(true)
            })
            .filter(|record| {
                filter
                    .subject_id
                    .as_ref()
                    .map(|value| &record.subject_id == value)
                    .unwrap_or(true)
            })
            .cloned()
            .collect())
    }

    async fn upsert_account(&self, _request: &UpsertVenueAccountRequest) -> Result<()> {
        panic!("upsert_account should not be called in reporting tests")
    }

    async fn get_account(&self, tenant_id: &str, account_id: &str) -> Result<Option<VenueAccountRecord>> {
        Ok(self
            .accounts
            .lock()
            .unwrap()
            .get(account_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_accounts(&self, filter: &VenueAccountFilter) -> Result<Vec<VenueAccountRecord>> {
        Ok(self
            .accounts
            .lock()
            .unwrap()
            .values()
            .filter(|record| record.tenant_id == filter.tenant_id)
            .filter(|record| {
                filter
                    .venue_connection_id
                    .as_ref()
                    .map(|value| &record.venue_connection_id == value)
                    .unwrap_or(true)
            })
            .cloned()
            .collect())
    }

    async fn upsert_beneficiary_profile(
        &self,
        _request: &UpsertBeneficiaryProfileRequest,
    ) -> Result<()> {
        panic!("upsert_beneficiary_profile should not be called in reporting tests")
    }

    async fn get_beneficiary_profile(
        &self,
        tenant_id: &str,
        beneficiary_profile_id: &str,
    ) -> Result<Option<BeneficiaryProfileRecord>> {
        Ok(self
            .beneficiaries
            .lock()
            .unwrap()
            .get(beneficiary_profile_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_beneficiary_profiles(
        &self,
        filter: &BeneficiaryProfileFilter,
    ) -> Result<Vec<BeneficiaryProfileRecord>> {
        Ok(self
            .beneficiaries
            .lock()
            .unwrap()
            .values()
            .filter(|record| record.tenant_id == filter.tenant_id)
            .filter(|record| {
                filter
                    .subject_type
                    .as_ref()
                    .map(|value| &record.subject_type == value)
                    .unwrap_or(true)
            })
            .filter(|record| {
                filter
                    .subject_id
                    .as_ref()
                    .map(|value| &record.subject_id == value)
                    .unwrap_or(true)
            })
            .cloned()
            .collect())
    }

    async fn upsert_transfer(&self, _request: &UpsertVenueTransferRequest) -> Result<()> {
        panic!("upsert_transfer should not be called in reporting tests")
    }

    async fn get_transfer(
        &self,
        tenant_id: &str,
        transfer_id: &str,
    ) -> Result<Option<VenueTransferRecord>> {
        Ok(self
            .transfers
            .lock()
            .unwrap()
            .get(transfer_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_transfers(&self, filter: &VenueTransferFilter) -> Result<Vec<VenueTransferRecord>> {
        Ok(self
            .transfers
            .lock()
            .unwrap()
            .values()
            .filter(|record| record.tenant_id == filter.tenant_id)
            .filter(|record| filter.user_id.as_ref().map(|id| &record.user_id == id).unwrap_or(true))
            .filter(|record| {
                filter
                    .venue_connection_id
                    .as_ref()
                    .map(|value| &record.venue_connection_id == value)
                    .unwrap_or(true)
            })
            .filter(|record| {
                filter
                    .venue_account_id
                    .as_ref()
                    .map(|value| &record.venue_account_id == value)
                    .unwrap_or(true)
            })
            .filter(|record| filter.status.as_ref().map(|value| &record.status == value).unwrap_or(true))
            .cloned()
            .collect())
    }

    async fn upsert_source_of_funds_package(
        &self,
        _request: &UpsertSourceOfFundsPackageRequest,
    ) -> Result<()> {
        panic!("upsert_source_of_funds_package should not be called in reporting tests")
    }

    async fn get_source_of_funds_package(
        &self,
        tenant_id: &str,
        package_id: &str,
    ) -> Result<Option<SourceOfFundsPackageRecord>> {
        Ok(self
            .source_of_funds_packages
            .lock()
            .unwrap()
            .get(package_id)
            .filter(|record| record.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_source_of_funds_packages(
        &self,
        filter: &SourceOfFundsPackageFilter,
    ) -> Result<Vec<SourceOfFundsPackageRecord>> {
        Ok(self
            .source_of_funds_packages
            .lock()
            .unwrap()
            .values()
            .filter(|record| record.tenant_id == filter.tenant_id)
            .filter(|record| {
                filter
                    .subject_type
                    .as_ref()
                    .map(|value| &record.subject_type == value)
                    .unwrap_or(true)
            })
            .filter(|record| {
                filter
                    .subject_id
                    .as_ref()
                    .map(|value| &record.subject_id == value)
                    .unwrap_or(true)
            })
            .cloned()
            .collect())
    }
}

fn build_service(repo: Arc<MockVenueTrustRepository>) -> VenueTrustReportingService {
    VenueTrustReportingService::new(repo)
}

fn seed_subject(repo: &Arc<MockVenueTrustRepository>) {
    let attestation_id = Uuid::parse_str("53000000-0000-0000-0000-000000000033").unwrap();

    repo.connections.lock().unwrap().insert(
        "conn-report-1".to_string(),
        VenueConnectionRecord {
            connection_id: "conn-report-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            user_id: Some("user-a".to_string()),
            venue_key: "hyperliquid".to_string(),
            connection_mode: "wallet_linked".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({"connector": "read_only"}),
            last_verified_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    repo.accounts.lock().unwrap().insert(
        "acct-report-1".to_string(),
        VenueAccountRecord {
            account_id: "acct-report-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            venue_connection_id: "conn-report-1".to_string(),
            venue_key: "hyperliquid".to_string(),
            account_label: Some("Main".to_string()),
            account_ref: Some("hl-main".to_string()),
            wallet_address: Some("0xwallet".to_string()),
            subaccount_ref: Some("sub-1".to_string()),
            api_scope_summary: serde_json::json!({"permissions": ["read"]}),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
            last_verified_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    repo.beneficiaries.lock().unwrap().insert(
        "benef-report-1".to_string(),
        BeneficiaryProfileRecord {
            beneficiary_profile_id: "benef-report-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            user_id: Some("user-a".to_string()),
            destination_type: "bank_account".to_string(),
            destination_ref: "VCB-123".to_string(),
            display_name: Some("Primary".to_string()),
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
            proof_artifact_uri: Some("s3://proofs/trust-report-wallet.json".to_string()),
            risk_state: Some("clear".to_string()),
            metadata: serde_json::json!({"scope": "venue_trust_report"}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    repo.transfers.lock().unwrap().insert(
        "transfer-in-1".to_string(),
        VenueTransferRecord {
            transfer_id: "transfer-in-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            beneficiary_profile_id: None,
            wallet_attestation_id: attestation_id,
            venue_connection_id: "conn-report-1".to_string(),
            venue_account_id: "acct-report-1".to_string(),
            transfer_direction: "wallet_to_venue".to_string(),
            asset_symbol: "USDT".to_string(),
            network: "ethereum".to_string(),
            amount: dec!(125),
            origin_intent_id: Some("intent-report-1".to_string()),
            rfq_id: None,
            status: "submitted".to_string(),
            wallet_tx_hash: Some("0xwalletfunding".to_string()),
            venue_credit_ref: None,
            failure_code: None,
            metadata: serde_json::json!({"channel": "portal"}),
            submitted_at: Some(Utc::now()),
            completed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    repo.transfers.lock().unwrap().insert(
        "transfer-out-1".to_string(),
        VenueTransferRecord {
            transfer_id: "transfer-out-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            user_id: "user-a".to_string(),
            beneficiary_profile_id: Some("benef-report-1".to_string()),
            wallet_attestation_id: attestation_id,
            venue_connection_id: "conn-report-1".to_string(),
            venue_account_id: "acct-report-1".to_string(),
            transfer_direction: "venue_to_wallet".to_string(),
            asset_symbol: "USDT".to_string(),
            network: "ethereum".to_string(),
            amount: dec!(75),
            origin_intent_id: None,
            rfq_id: Some("rfq-1".to_string()),
            status: "submitted".to_string(),
            wallet_tx_hash: Some("0xwalletcashout".to_string()),
            venue_credit_ref: None,
            failure_code: None,
            metadata: serde_json::json!({"offrampReference": "venue_cashout_transfer-out-1"}),
            submitted_at: Some(Utc::now()),
            completed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    repo.source_of_funds_packages.lock().unwrap().insert(
        "sof-report-1".to_string(),
        SourceOfFundsPackageRecord {
            package_id: "sof-report-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            wallet_attestation_id: Some(attestation_id),
            venue_account_id: Some("acct-report-1".to_string()),
            venue_transfer_id: Some("transfer-in-1".to_string()),
            review_status: "approved".to_string(),
            package_uri: Some("s3://sof/report-1.json".to_string()),
            metadata: serde_json::json!({}),
            reviewed_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
}

#[tokio::test]
async fn venue_trust_report_builds_cash_in_and_cash_out_summary() {
    let repo = Arc::new(MockVenueTrustRepository::default());
    seed_subject(&repo);
    let service = build_service(repo);

    let report = service
        .build_subject_trust_report("tenant-a", "user", "user-a")
        .await
        .expect("report should build");

    assert_eq!(report.subject_type, "user");
    assert_eq!(report.subject_id, "user-a");
    assert_eq!(report.cash_in.transfer_direction, "wallet_to_venue");
    assert_eq!(report.cash_in.transfer_count, 1);
    assert_eq!(report.cash_in.total_amount, dec!(125));
    assert_eq!(report.cash_out.transfer_direction, "venue_to_wallet");
    assert_eq!(report.cash_out.transfer_count, 1);
    assert_eq!(report.cash_out.total_amount, dec!(75));
    assert!(report
        .evidence_references
        .iter()
        .any(|entry| entry.kind == "wallet_attestation"));
    assert!(report
        .evidence_references
        .iter()
        .any(|entry| entry.kind == "source_of_funds_package"));
    assert!(report
        .evidence_references
        .iter()
        .any(|entry| entry.kind == "beneficiary_profile"));
}

#[tokio::test]
async fn venue_trust_report_exports_evidence_json_contract() {
    let repo = Arc::new(MockVenueTrustRepository::default());
    seed_subject(&repo);
    let service = build_service(repo);

    let report = service
        .build_subject_trust_report("tenant-a", "user", "user-a")
        .await
        .expect("report should build");

    let artifact = service
        .export_evidence_json(&report)
        .expect("evidence export should build");

    assert_eq!(artifact.media_type, "application/json");
    assert!(artifact.file_name.starts_with("venue_trust_evidence_user_user-a"));

    let exported: serde_json::Value =
        serde_json::from_slice(&artifact.contents).expect("evidence export JSON should decode");
    assert_eq!(exported["subjectType"], "user");
    assert_eq!(exported["subjectId"], "user-a");
    assert_eq!(exported["cashIn"]["transferCount"], 1);
    assert_eq!(exported["cashOut"]["transferCount"], 1);
    assert_eq!(exported["evidenceReferences"][0]["kind"], "connection");
}
