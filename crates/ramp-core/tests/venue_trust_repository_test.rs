use rust_decimal::Decimal;
use uuid::Uuid;

use ramp_core::repository::{
    BeneficiaryProfileFilter, EnsureWalletAttestationRequest, PgVenueTrustRepository,
    SourceOfFundsPackageFilter, UpsertBeneficiaryProfileRequest, UpsertSourceOfFundsPackageRequest,
    UpsertVenueAccountRequest, UpsertVenueConnectionRequest, UpsertVenueTransferRequest,
    VenueAccountFilter, VenueConnectionFilter, VenueTransferFilter, VenueTrustRepository,
    WalletAttestationFilter,
};

#[tokio::test]
async fn venue_trust_repository_persists_and_reads_subject_graph() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("database connection should succeed");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should succeed");

    let attestation_id =
        Uuid::parse_str("10000000-0000-0000-0000-000000000023").expect("attestation uuid");
    let repository = PgVenueTrustRepository::new(pool.clone());
    repository
        .ensure_wallet_attestation(&EnsureWalletAttestationRequest {
            attestation_id,
            tenant_id: "tenant_a_123".to_string(),
            user_id: "user_a_1".to_string(),
            wallet_address: "0x1234".to_string(),
            chain_id: "ethereum".to_string(),
            attestation_status: "verified".to_string(),
            proof_kind: "signature".to_string(),
            proof_artifact_uri: Some("s3://proofs/tuw023".to_string()),
            risk_state: Some("clear".to_string()),
            metadata: serde_json::json!({ "scope": "venue_funding" }),
        })
        .await
        .expect("wallet attestation should persist");

    let attestation = repository
        .get_wallet_attestation("tenant_a_123", attestation_id)
        .await
        .expect("wallet attestation lookup should succeed")
        .expect("wallet attestation should exist");
    assert_eq!(attestation.attestation_status, "verified");
    assert_eq!(attestation.metadata["scope"], "venue_funding");

    let attestations = repository
        .list_wallet_attestations(&WalletAttestationFilter {
            tenant_id: "tenant_a_123".to_string(),
            user_id: Some("user_a_1".to_string()),
            wallet_address: Some("0x1234".to_string()),
            chain_id: Some("ethereum".to_string()),
            attestation_status: Some("verified".to_string()),
        })
        .await
        .expect("wallet attestations should load");
    assert_eq!(attestations.len(), 1);
    assert_eq!(attestations[0].proof_kind, "signature");

    repository
        .upsert_connection(&UpsertVenueConnectionRequest {
            connection_id: "venue_connection_tuw023".to_string(),
            tenant_id: "tenant_a_123".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user_a_1".to_string(),
            user_id: Some("user_a_1".to_string()),
            venue_key: "hyperliquid".to_string(),
            connection_mode: "wallet_linked".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({ "sync": "read" }),
            last_verified_at: None,
        })
        .await
        .expect("connection should persist");

    repository
        .upsert_account(&UpsertVenueAccountRequest {
            account_id: "venue_account_tuw023".to_string(),
            tenant_id: "tenant_a_123".to_string(),
            venue_connection_id: "venue_connection_tuw023".to_string(),
            venue_key: "hyperliquid".to_string(),
            account_label: Some("Primary".to_string()),
            account_ref: Some("acct-001".to_string()),
            wallet_address: Some("0x1234".to_string()),
            subaccount_ref: None,
            api_scope_summary: serde_json::json!({ "permissions": ["read", "trade"] }),
            status: "active".to_string(),
            metadata: serde_json::json!({ "tier": "pro" }),
            last_verified_at: None,
        })
        .await
        .expect("account should persist");

    repository
        .upsert_beneficiary_profile(&UpsertBeneficiaryProfileRequest {
            beneficiary_profile_id: "beneficiary_tuw023".to_string(),
            tenant_id: "tenant_a_123".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user_a_1".to_string(),
            user_id: Some("user_a_1".to_string()),
            destination_type: "exchange_wallet".to_string(),
            destination_ref: "0xbeneficiary".to_string(),
            display_name: Some("HL Main".to_string()),
            asset_symbol: Some("USDC".to_string()),
            network: Some("ethereum".to_string()),
            verification_status: "verified".to_string(),
            cooldown_ends_at: None,
            metadata: serde_json::json!({ "travelRule": "clear" }),
            last_verified_at: None,
        })
        .await
        .expect("beneficiary should persist");

    repository
        .upsert_transfer(&UpsertVenueTransferRequest {
            transfer_id: "venue_transfer_tuw023".to_string(),
            tenant_id: "tenant_a_123".to_string(),
            user_id: "user_a_1".to_string(),
            beneficiary_profile_id: Some("beneficiary_tuw023".to_string()),
            wallet_attestation_id: attestation_id,
            venue_connection_id: "venue_connection_tuw023".to_string(),
            venue_account_id: "venue_account_tuw023".to_string(),
            transfer_direction: "wallet_to_venue".to_string(),
            asset_symbol: "USDC".to_string(),
            network: "ethereum".to_string(),
            amount: Decimal::new(125_000, 2),
            origin_intent_id: None,
            rfq_id: None,
            status: "submitted".to_string(),
            wallet_tx_hash: Some("0xtransferhash".to_string()),
            venue_credit_ref: None,
            failure_code: None,
            metadata: serde_json::json!({ "channel": "portal" }),
            submitted_at: None,
            completed_at: None,
        })
        .await
        .expect("transfer should persist");

    repository
        .upsert_source_of_funds_package(&UpsertSourceOfFundsPackageRequest {
            package_id: "sof_package_tuw023".to_string(),
            tenant_id: "tenant_a_123".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user_a_1".to_string(),
            wallet_attestation_id: Some(attestation_id),
            venue_account_id: Some("venue_account_tuw023".to_string()),
            venue_transfer_id: Some("venue_transfer_tuw023".to_string()),
            review_status: "in_review".to_string(),
            package_uri: Some("s3://packages/tuw023.json".to_string()),
            metadata: serde_json::json!({ "reviewer": "ops" }),
            reviewed_at: None,
        })
        .await
        .expect("source-of-funds package should persist");

    let connections = repository
        .list_connections(&VenueConnectionFilter {
            tenant_id: "tenant_a_123".to_string(),
            subject_type: Some("user".to_string()),
            subject_id: Some("user_a_1".to_string()),
            venue_key: None,
            status: Some("active".to_string()),
        })
        .await
        .expect("connections should load");
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].venue_key, "hyperliquid");

    let accounts = repository
        .list_accounts(&VenueAccountFilter {
            tenant_id: "tenant_a_123".to_string(),
            venue_connection_id: Some("venue_connection_tuw023".to_string()),
            venue_key: None,
            status: Some("active".to_string()),
            wallet_address: None,
        })
        .await
        .expect("accounts should load");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account_ref.as_deref(), Some("acct-001"));

    let beneficiaries = repository
        .list_beneficiary_profiles(&BeneficiaryProfileFilter {
            tenant_id: "tenant_a_123".to_string(),
            subject_type: Some("user".to_string()),
            subject_id: Some("user_a_1".to_string()),
            destination_type: None,
            verification_status: Some("verified".to_string()),
        })
        .await
        .expect("beneficiaries should load");
    assert_eq!(beneficiaries.len(), 1);
    assert_eq!(beneficiaries[0].destination_ref, "0xbeneficiary");

    let transfer = repository
        .get_transfer("tenant_a_123", "venue_transfer_tuw023")
        .await
        .expect("transfer lookup should succeed")
        .expect("transfer should exist");
    assert_eq!(transfer.status, "submitted");
    assert_eq!(transfer.amount, Decimal::new(125_000, 2));

    let transfers = repository
        .list_transfers(&VenueTransferFilter {
            tenant_id: "tenant_a_123".to_string(),
            user_id: Some("user_a_1".to_string()),
            venue_connection_id: Some("venue_connection_tuw023".to_string()),
            venue_account_id: None,
            status: None,
        })
        .await
        .expect("transfers should load");
    assert_eq!(transfers.len(), 1);
    assert_eq!(
        transfers[0].wallet_tx_hash.as_deref(),
        Some("0xtransferhash")
    );

    let packages = repository
        .list_source_of_funds_packages(&SourceOfFundsPackageFilter {
            tenant_id: "tenant_a_123".to_string(),
            subject_type: Some("user".to_string()),
            subject_id: Some("user_a_1".to_string()),
            venue_transfer_id: Some("venue_transfer_tuw023".to_string()),
            venue_account_id: None,
            review_status: None,
        })
        .await
        .expect("packages should load");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].review_status, "in_review");
}
