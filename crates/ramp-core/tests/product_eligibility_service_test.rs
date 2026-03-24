use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use ramp_common::Result;
use ramp_core::repository::{
    BeneficiaryProfileFilter, BeneficiaryProfileRecord, CorridorPackRecord, CorridorPackRepository,
    EnsureWalletAttestationRequest, PaymentMethodCapabilityRecord,
    PaymentMethodCapabilityRepository, SourceOfFundsPackageFilter, SourceOfFundsPackageRecord,
    UpsertBeneficiaryProfileRequest, UpsertCorridorComplianceHookRequest,
    UpsertCorridorCutoffPolicyRequest, UpsertCorridorEligibilityRuleRequest,
    UpsertCorridorEndpointRequest, UpsertCorridorFeeProfileRequest, UpsertCorridorPackRequest,
    UpsertCorridorRolloutScopeRequest, UpsertPaymentMethodCapabilityRequest,
    UpsertSourceOfFundsPackageRequest, UpsertVenueAccountRequest, UpsertVenueConnectionRequest,
    UpsertVenueTransferRequest, VenueAccountFilter, VenueAccountRecord, VenueConnectionFilter,
    VenueConnectionRecord, VenueTransferFilter, VenueTransferRecord, VenueTrustRepository,
    WalletAttestationFilter, WalletAttestationRecord,
};
use ramp_core::service::{
    CommercialExtensionRecord, CommercialReadinessService, CorridorPackService,
    PaymentMethodCapabilityService, ProductEligibilityDecisionState, ProductEligibilityRequest,
    ProductEligibilityService, SourceOfFundsPackageAction, VenueTrustService,
};
use uuid::Uuid;

#[derive(Default)]
struct MockVenueTrustRepository {
    connections: Mutex<Vec<VenueConnectionRecord>>,
    accounts: Mutex<Vec<VenueAccountRecord>>,
    packages: Mutex<Vec<SourceOfFundsPackageRecord>>,
}

#[async_trait]
impl VenueTrustRepository for MockVenueTrustRepository {
    async fn ensure_wallet_attestation(&self, _: &EnsureWalletAttestationRequest) -> Result<()> {
        Ok(())
    }

    async fn get_wallet_attestation(
        &self,
        _: &str,
        _: Uuid,
    ) -> Result<Option<WalletAttestationRecord>> {
        Ok(None)
    }

    async fn list_wallet_attestations(
        &self,
        _: &WalletAttestationFilter,
    ) -> Result<Vec<WalletAttestationRecord>> {
        Ok(Vec::new())
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
        Ok(self
            .connections
            .lock()
            .expect("connections lock")
            .iter()
            .find(|item| item.tenant_id == tenant_id && item.connection_id == connection_id)
            .cloned())
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

    async fn get_account(
        &self,
        tenant_id: &str,
        account_id: &str,
    ) -> Result<Option<VenueAccountRecord>> {
        Ok(self
            .accounts
            .lock()
            .expect("accounts lock")
            .iter()
            .find(|item| item.tenant_id == tenant_id && item.account_id == account_id)
            .cloned())
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
            })
            .cloned()
            .collect())
    }

    async fn upsert_beneficiary_profile(&self, _: &UpsertBeneficiaryProfileRequest) -> Result<()> {
        Ok(())
    }

    async fn get_beneficiary_profile(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<BeneficiaryProfileRecord>> {
        Ok(None)
    }

    async fn list_beneficiary_profiles(
        &self,
        _: &BeneficiaryProfileFilter,
    ) -> Result<Vec<BeneficiaryProfileRecord>> {
        Ok(Vec::new())
    }

    async fn upsert_transfer(&self, _: &UpsertVenueTransferRequest) -> Result<()> {
        Ok(())
    }

    async fn get_transfer(&self, _: &str, _: &str) -> Result<Option<VenueTransferRecord>> {
        Ok(None)
    }

    async fn list_transfers(&self, _: &VenueTransferFilter) -> Result<Vec<VenueTransferRecord>> {
        Ok(Vec::new())
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
        Ok(self
            .packages
            .lock()
            .expect("packages lock")
            .iter()
            .find(|item| item.tenant_id == tenant_id && item.package_id == package_id)
            .cloned())
    }

    async fn list_source_of_funds_packages(
        &self,
        filter: &SourceOfFundsPackageFilter,
    ) -> Result<Vec<SourceOfFundsPackageRecord>> {
        Ok(self
            .packages
            .lock()
            .expect("packages lock")
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
                        .venue_account_id
                        .as_deref()
                        .is_none_or(|value| item.venue_account_id.as_deref() == Some(value))
            })
            .cloned()
            .collect())
    }
}

#[derive(Default)]
struct MockCorridorRepository {
    packs: Mutex<Vec<CorridorPackRecord>>,
}

#[async_trait]
impl CorridorPackRepository for MockCorridorRepository {
    async fn upsert_corridor_pack(&self, request: &UpsertCorridorPackRequest) -> Result<()> {
        let mut items = self.packs.lock().expect("packs lock");
        items.retain(|item| item.corridor_pack_id != request.corridor_pack_id);
        items.push(CorridorPackRecord {
            corridor_pack_id: request.corridor_pack_id.clone(),
            tenant_id: request.tenant_id.clone(),
            corridor_code: request.corridor_code.clone(),
            source_market: request.source_market.clone(),
            destination_market: request.destination_market.clone(),
            source_currency: request.source_currency.clone(),
            destination_currency: request.destination_currency.clone(),
            settlement_direction: request.settlement_direction.clone(),
            fee_model: request.fee_model.clone(),
            lifecycle_state: request.lifecycle_state.clone(),
            rollout_state: request.rollout_state.clone(),
            eligibility_state: request.eligibility_state.clone(),
            metadata: request.metadata.clone(),
            endpoints: Vec::new(),
            fee_profiles: Vec::new(),
            cutoff_policies: Vec::new(),
            compliance_hooks: Vec::new(),
            rollout_scopes: Vec::new(),
            eligibility_rules: Vec::new(),
        });
        Ok(())
    }

    async fn upsert_endpoint(&self, _: &UpsertCorridorEndpointRequest) -> Result<()> {
        Ok(())
    }
    async fn upsert_fee_profile(&self, _: &UpsertCorridorFeeProfileRequest) -> Result<()> {
        Ok(())
    }
    async fn upsert_cutoff_policy(&self, _: &UpsertCorridorCutoffPolicyRequest) -> Result<()> {
        Ok(())
    }
    async fn upsert_compliance_hook(&self, _: &UpsertCorridorComplianceHookRequest) -> Result<()> {
        Ok(())
    }
    async fn upsert_rollout_scope(&self, _: &UpsertCorridorRolloutScopeRequest) -> Result<()> {
        Ok(())
    }
    async fn upsert_eligibility_rule(
        &self,
        _: &UpsertCorridorEligibilityRuleRequest,
    ) -> Result<()> {
        Ok(())
    }

    async fn list_corridor_packs(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<Vec<CorridorPackRecord>> {
        Ok(self
            .packs
            .lock()
            .expect("packs lock")
            .iter()
            .filter(|item| tenant_id.is_none_or(|tenant| item.tenant_id.as_deref() == Some(tenant)))
            .cloned()
            .collect())
    }

    async fn get_corridor_pack(
        &self,
        tenant_id: Option<&str>,
        corridor_code: &str,
    ) -> Result<Option<CorridorPackRecord>> {
        Ok(self
            .packs
            .lock()
            .expect("packs lock")
            .iter()
            .find(|item| {
                item.corridor_code == corridor_code
                    && tenant_id.is_none_or(|tenant| item.tenant_id.as_deref() == Some(tenant))
            })
            .cloned())
    }
}

#[derive(Default)]
struct MockPaymentMethodCapabilityRepository {
    capabilities: Mutex<Vec<PaymentMethodCapabilityRecord>>,
}

#[async_trait]
impl PaymentMethodCapabilityRepository for MockPaymentMethodCapabilityRepository {
    async fn upsert_payment_method_capability(
        &self,
        request: &UpsertPaymentMethodCapabilityRequest,
    ) -> Result<()> {
        let mut items = self.capabilities.lock().expect("capabilities lock");
        items.retain(|item| {
            item.payment_method_capability_id != request.payment_method_capability_id
        });
        items.push(PaymentMethodCapabilityRecord {
            payment_method_capability_id: request.payment_method_capability_id.clone(),
            corridor_pack_id: request.corridor_pack_id.clone(),
            partner_capability_id: request.partner_capability_id.clone(),
            method_family: request.method_family.clone(),
            funding_source: request.funding_source.clone(),
            settlement_direction: request.settlement_direction.clone(),
            presentment_model: request.presentment_model.clone(),
            card_funding_enabled: request.card_funding_enabled,
            policy_flags: request.policy_flags.clone(),
            metadata: request.metadata.clone(),
        });
        Ok(())
    }

    async fn list_payment_method_capabilities(
        &self,
        corridor_pack_id: Option<&str>,
        _: Option<&str>,
    ) -> Result<Vec<PaymentMethodCapabilityRecord>> {
        Ok(self
            .capabilities
            .lock()
            .expect("capabilities lock")
            .iter()
            .filter(|item| {
                corridor_pack_id.is_none_or(|corridor| item.corridor_pack_id == corridor)
            })
            .cloned()
            .collect())
    }
}

fn build_service() -> ProductEligibilityService {
    ProductEligibilityService::new(
        VenueTrustService::with_repository(Arc::new(MockVenueTrustRepository::default())),
        CorridorPackService::with_repository(Arc::new(MockCorridorRepository::default())),
        PaymentMethodCapabilityService::with_repository(Arc::new(
            MockPaymentMethodCapabilityRepository::default(),
        )),
    )
}

fn build_service_with_commercial_readiness() -> ProductEligibilityService {
    ProductEligibilityService::with_commercial_readiness(
        VenueTrustService::with_repository(Arc::new(MockVenueTrustRepository::default())),
        CorridorPackService::with_repository(Arc::new(MockCorridorRepository::default())),
        PaymentMethodCapabilityService::with_repository(Arc::new(
            MockPaymentMethodCapabilityRepository::default(),
        )),
        CommercialReadinessService::with_extensions(vec![CommercialExtensionRecord {
            extension_id: "ext-review".to_string(),
            extension_kind: "stablecoin_account".to_string(),
            label: "Review Gate".to_string(),
            description: "Requires an approval reference before enablement".to_string(),
            enabled: false,
            approval_reference: None,
            required_partner_capabilities: Vec::new(),
            required_corridor_packs: vec!["SG_USDT_FUNDING".to_string()],
            required_compliance_checks: Vec::new(),
            metadata: serde_json::json!({"sourceClass":"governed_registry"}),
        }]),
    )
}

fn request() -> ProductEligibilityRequest {
    ProductEligibilityRequest {
        tenant_id: "tenant-a".to_string(),
        subject_type: "user".to_string(),
        subject_id: "user-a".to_string(),
        jurisdiction: "SG".to_string(),
        user_tier: Some("tier_2".to_string()),
        kyb_state: Some("approved".to_string()),
        wallet_attestation_state: "verified".to_string(),
        venue_key: "hyperliquid".to_string(),
        action: "funding".to_string(),
        asset: "USDT".to_string(),
        network: "solana".to_string(),
        payment_method_family: Some("bank_transfer".to_string()),
        funding_source: Some("bank_account".to_string()),
        commercial_extension_id: None,
    }
}

async fn seed_allow_lane(service: &ProductEligibilityService, source_of_funds_required: bool) {
    seed_allow_lane_with_policy(service, source_of_funds_required, false).await;
}

async fn seed_allow_lane_with_policy(
    service: &ProductEligibilityService,
    source_of_funds_required: bool,
    source_of_funds_auto_submit: bool,
) {
    service
        .venue_trust_service()
        .upsert_connection(&UpsertVenueConnectionRequest {
            connection_id: "connection-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            user_id: Some("user-a".to_string()),
            venue_key: "hyperliquid".to_string(),
            connection_mode: "oauth".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
            last_verified_at: Some(Utc::now()),
        })
        .await
        .expect("connection seed");
    service
        .venue_trust_service()
        .upsert_account(&UpsertVenueAccountRequest {
            account_id: "account-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            venue_connection_id: "connection-1".to_string(),
            venue_key: "hyperliquid".to_string(),
            account_label: Some("Primary".to_string()),
            account_ref: None,
            wallet_address: Some("0xabc".to_string()),
            subaccount_ref: None,
            api_scope_summary: serde_json::json!({"canTransfer": true}),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
            last_verified_at: Some(Utc::now()),
        })
        .await
        .expect("account seed");
    service
        .corridor_pack_service()
        .upsert_corridor_pack_bundle(&ramp_core::service::UpsertCorridorPackBundle {
            corridor_pack: UpsertCorridorPackRequest {
                corridor_pack_id: "corridor-1".to_string(),
                tenant_id: Some("tenant-a".to_string()),
                corridor_code: "SG_USDT_FUNDING".to_string(),
                source_market: "GLOBAL".to_string(),
                destination_market: "SG".to_string(),
                source_currency: "USDT".to_string(),
                destination_currency: "USDT".to_string(),
                settlement_direction: "funding".to_string(),
                fee_model: "shared".to_string(),
                lifecycle_state: "active".to_string(),
                rollout_state: "approved".to_string(),
                eligibility_state: "active".to_string(),
                metadata: serde_json::json!({"venueKey": "hyperliquid", "asset": "USDT", "network": "solana"}),
            },
            endpoints: vec![],
            fee_profiles: vec![],
            cutoff_policies: vec![],
            compliance_hooks: vec![],
            rollout_scopes: vec![],
            eligibility_rules: vec![],
        })
        .await
        .expect("corridor seed");
    service
        .payment_method_capability_service()
        .upsert_capability(&UpsertPaymentMethodCapabilityRequest {
            payment_method_capability_id: "pmc-1".to_string(),
            corridor_pack_id: "corridor-1".to_string(),
            partner_capability_id: None,
            method_family: "bank_transfer".to_string(),
            funding_source: Some("bank_account".to_string()),
            settlement_direction: "funding".to_string(),
            presentment_model: Some("hosted".to_string()),
            card_funding_enabled: false,
            policy_flags: serde_json::json!({
                "sourceOfFundsRequired": source_of_funds_required,
                "sourceOfFundsAutoSubmit": source_of_funds_auto_submit
            }),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("capability seed");
}

#[tokio::test]
async fn allow_fast_lane_when_all_inventory_gates_match() {
    let service = build_service();
    seed_allow_lane(&service, false).await;

    let decision = service.evaluate(&request()).await.expect("decision");

    assert_eq!(decision.decision, ProductEligibilityDecisionState::Allow);
    assert_eq!(
        decision.source_of_funds.action,
        SourceOfFundsPackageAction::None
    );
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.code == "venue_ready"));
}

#[tokio::test]
async fn review_lane_reuses_existing_source_of_funds_package() {
    let service = build_service();
    seed_allow_lane(&service, true).await;
    service
        .venue_trust_service()
        .upsert_source_of_funds_package(&UpsertSourceOfFundsPackageRequest {
            package_id: "sof-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-a".to_string(),
            wallet_attestation_id: None,
            venue_account_id: Some("account-1".to_string()),
            venue_transfer_id: None,
            review_status: "in_review".to_string(),
            package_uri: Some("s3://sof-1".to_string()),
            metadata: serde_json::json!({}),
            reviewed_at: None,
        })
        .await
        .expect("sof seed");

    let decision = service.evaluate(&request()).await.expect("decision");

    assert_eq!(decision.decision, ProductEligibilityDecisionState::Review);
    assert_eq!(
        decision.source_of_funds.action,
        SourceOfFundsPackageAction::Reuse
    );
    assert_eq!(
        decision.source_of_funds.package_id.as_deref(),
        Some("sof-1")
    );
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.code == "source_of_funds_in_review"));
}

#[tokio::test]
async fn deny_when_requested_funding_source_is_not_supported() {
    let service = build_service();
    seed_allow_lane(&service, false).await;
    let mut req = request();
    req.funding_source = Some("card".to_string());

    let decision = service.evaluate(&req).await.expect("decision");

    assert_eq!(decision.decision, ProductEligibilityDecisionState::Deny);
    assert_eq!(
        decision.source_of_funds.action,
        SourceOfFundsPackageAction::None
    );
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.code == "payment_method_not_supported"));
}

#[tokio::test]
async fn review_when_commercial_readiness_gate_is_not_enabled() {
    let service = build_service_with_commercial_readiness();
    seed_allow_lane(&service, false).await;
    let mut req = request();
    req.commercial_extension_id = Some("ext-review".to_string());

    let decision = service.evaluate(&req).await.expect("decision");

    assert_eq!(decision.decision, ProductEligibilityDecisionState::Review);
    assert_eq!(
        decision.source_of_funds.action,
        SourceOfFundsPackageAction::None
    );
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.code == "commercial_readiness_review"));
}

#[tokio::test]
async fn review_creates_draft_source_of_funds_when_required_and_missing() {
    let service = build_service();
    seed_allow_lane_with_policy(&service, true, false).await;

    let decision = service.evaluate(&request()).await.expect("decision");

    assert_eq!(decision.decision, ProductEligibilityDecisionState::Review);
    assert_eq!(
        decision.source_of_funds.action,
        SourceOfFundsPackageAction::CreateDraft
    );
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.code == "source_of_funds_required"));
}
