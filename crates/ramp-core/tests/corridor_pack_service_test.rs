use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use ramp_common::Result;
use ramp_core::repository::{
    CorridorComplianceHookRecord, CorridorCutoffPolicyRecord, CorridorEligibilityRuleRecord,
    CorridorEndpointRecord, CorridorFeeProfileRecord, CorridorPackRecord, CorridorPackRepository,
    CorridorRolloutScopeRecord, UpsertCorridorComplianceHookRequest,
    UpsertCorridorCutoffPolicyRequest, UpsertCorridorEligibilityRuleRequest,
    UpsertCorridorEndpointRequest, UpsertCorridorFeeProfileRequest, UpsertCorridorPackRequest,
    UpsertCorridorRolloutScopeRequest,
};
use ramp_core::service::{CorridorPackService, UpsertCorridorPackBundle};

#[derive(Default)]
struct MockCorridorRepository {
    records: Mutex<Vec<CorridorPackRecord>>,
}

#[async_trait]
impl CorridorPackRepository for MockCorridorRepository {
    async fn upsert_corridor_pack(&self, request: &UpsertCorridorPackRequest) -> Result<()> {
        let mut records = self.records.lock().expect("records lock");
        records.retain(|record| record.corridor_pack_id != request.corridor_pack_id);
        records.push(CorridorPackRecord {
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

    async fn upsert_endpoint(&self, request: &UpsertCorridorEndpointRequest) -> Result<()> {
        let mut records = self.records.lock().expect("records lock");
        if let Some(record) = records
            .iter_mut()
            .find(|record| record.corridor_pack_id == request.corridor_pack_id)
        {
            record.endpoints.push(CorridorEndpointRecord {
                endpoint_id: request.endpoint_id.clone(),
                endpoint_role: request.endpoint_role.clone(),
                partner_id: request.partner_id.clone(),
                provider_key: request.provider_key.clone(),
                adapter_key: request.adapter_key.clone(),
                entity_type: request.entity_type.clone(),
                rail: request.rail.clone(),
                method_family: request.method_family.clone(),
                settlement_mode: request.settlement_mode.clone(),
                instrument_family: request.instrument_family.clone(),
                metadata: request.metadata.clone(),
            });
        }
        Ok(())
    }

    async fn upsert_fee_profile(&self, request: &UpsertCorridorFeeProfileRequest) -> Result<()> {
        let mut records = self.records.lock().expect("records lock");
        if let Some(record) = records
            .iter_mut()
            .find(|record| record.corridor_pack_id == request.corridor_pack_id)
        {
            record.fee_profiles.push(CorridorFeeProfileRecord {
                fee_profile_id: request.fee_profile_id.clone(),
                fee_currency: request.fee_currency.clone(),
                base_fee: request.base_fee.clone(),
                fx_spread_bps: request.fx_spread_bps,
                liquidity_cost_bps: request.liquidity_cost_bps,
                surcharge_bps: request.surcharge_bps,
                metadata: request.metadata.clone(),
            });
        }
        Ok(())
    }

    async fn upsert_cutoff_policy(&self, request: &UpsertCorridorCutoffPolicyRequest) -> Result<()> {
        let mut records = self.records.lock().expect("records lock");
        if let Some(record) = records
            .iter_mut()
            .find(|record| record.corridor_pack_id == request.corridor_pack_id)
        {
            record.cutoff_policies.push(CorridorCutoffPolicyRecord {
                cutoff_policy_id: request.cutoff_policy_id.clone(),
                timezone: request.timezone.clone(),
                cutoff_windows: request.cutoff_windows.clone(),
                holiday_calendar: request.holiday_calendar.clone(),
                retry_rule: request.retry_rule.clone(),
                exception_policy: request.exception_policy.clone(),
                metadata: request.metadata.clone(),
            });
        }
        Ok(())
    }

    async fn upsert_compliance_hook(&self, request: &UpsertCorridorComplianceHookRequest) -> Result<()> {
        let mut records = self.records.lock().expect("records lock");
        if let Some(record) = records
            .iter_mut()
            .find(|record| record.corridor_pack_id == request.corridor_pack_id)
        {
            record.compliance_hooks.push(CorridorComplianceHookRecord {
                compliance_hook_id: request.compliance_hook_id.clone(),
                hook_kind: request.hook_kind.clone(),
                provider_key: request.provider_key.clone(),
                required: request.required,
                config: request.config.clone(),
                metadata: request.metadata.clone(),
            });
        }
        Ok(())
    }

    async fn upsert_rollout_scope(&self, request: &UpsertCorridorRolloutScopeRequest) -> Result<()> {
        let mut records = self.records.lock().expect("records lock");
        if let Some(record) = records
            .iter_mut()
            .find(|record| record.corridor_pack_id == request.corridor_pack_id)
        {
            record.rollout_scopes.push(CorridorRolloutScopeRecord {
                rollout_scope_id: request.rollout_scope_id.clone(),
                tenant_id: request.tenant_id.clone(),
                environment: request.environment.clone(),
                geography: request.geography.clone(),
                method_family: request.method_family.clone(),
                rollout_state: request.rollout_state.clone(),
                approval_reference: request.approval_reference.clone(),
                metadata: request.metadata.clone(),
            });
        }
        Ok(())
    }

    async fn upsert_eligibility_rule(
        &self,
        request: &UpsertCorridorEligibilityRuleRequest,
    ) -> Result<()> {
        let mut records = self.records.lock().expect("records lock");
        if let Some(record) = records
            .iter_mut()
            .find(|record| record.corridor_pack_id == request.corridor_pack_id)
        {
            record.eligibility_rules.push(CorridorEligibilityRuleRecord {
                eligibility_rule_id: request.eligibility_rule_id.clone(),
                partner_id: request.partner_id.clone(),
                entity_type: request.entity_type.clone(),
                method_family: request.method_family.clone(),
                amount_bounds: request.amount_bounds.clone(),
                compliance_requirements: request.compliance_requirements.clone(),
                metadata: request.metadata.clone(),
            });
        }
        Ok(())
    }

    async fn list_corridor_packs(&self, tenant_id: Option<&str>) -> Result<Vec<CorridorPackRecord>> {
        let records = self.records.lock().expect("records lock");
        Ok(records
            .iter()
            .filter(|record| match tenant_id {
                Some(tenant_id) => {
                    record.tenant_id.as_deref() == Some(tenant_id) || record.tenant_id.is_none()
                }
                None => true,
            })
            .cloned()
            .collect())
    }

    async fn get_corridor_pack(
        &self,
        tenant_id: Option<&str>,
        corridor_code: &str,
    ) -> Result<Option<CorridorPackRecord>> {
        let records = self.records.lock().expect("records lock");
        Ok(records
            .iter()
            .find(|record| {
                record.corridor_code == corridor_code
                    && match tenant_id {
                        Some(tenant_id) => {
                            record.tenant_id.as_deref() == Some(tenant_id)
                                || record.tenant_id.is_none()
                        }
                        None => true,
                    }
            })
            .cloned())
    }
}

#[tokio::test]
async fn service_returns_empty_fallback_without_repository() {
    let service = CorridorPackService::new();
    let snapshot = service
        .list_corridor_packs(Some("tenant-a"))
        .await
        .expect("snapshot should load");

    assert_eq!(snapshot.source, "fallback");
    assert!(snapshot.corridor_packs.is_empty());
}

#[tokio::test]
async fn service_upsert_and_get_corridor_use_registry_path() {
    let service = CorridorPackService::with_repository(Arc::new(MockCorridorRepository::default()));

    let corridor = service
        .upsert_corridor_pack_bundle(&UpsertCorridorPackBundle {
            corridor_pack: UpsertCorridorPackRequest {
                corridor_pack_id: "corridor_vn_hk".to_string(),
                tenant_id: Some("tenant-a".to_string()),
                corridor_code: "VN_HK_PAYOUT".to_string(),
                source_market: "VN".to_string(),
                destination_market: "HK".to_string(),
                source_currency: "VND".to_string(),
                destination_currency: "HKD".to_string(),
                settlement_direction: "PAYOUT".to_string(),
                fee_model: "shared".to_string(),
                lifecycle_state: "pilot".to_string(),
                rollout_state: "approved".to_string(),
                eligibility_state: "active".to_string(),
                metadata: serde_json::json!({"pilot": true}),
            },
            endpoints: vec![UpsertCorridorEndpointRequest {
                endpoint_id: "endpoint_destination_hk".to_string(),
                corridor_pack_id: "corridor_vn_hk".to_string(),
                endpoint_role: "DESTINATION".to_string(),
                partner_id: Some("bank_hk_partner".to_string()),
                provider_key: None,
                adapter_key: Some("fps".to_string()),
                entity_type: "individual".to_string(),
                rail: "fps".to_string(),
                method_family: Some("push_transfer".to_string()),
                settlement_mode: Some("just_in_time".to_string()),
                instrument_family: Some("account".to_string()),
                metadata: serde_json::json!({}),
            }],
            fee_profiles: vec![UpsertCorridorFeeProfileRequest {
                fee_profile_id: "fee_profile_hk".to_string(),
                corridor_pack_id: "corridor_vn_hk".to_string(),
                fee_currency: "HKD".to_string(),
                base_fee: Some("25.00".to_string()),
                fx_spread_bps: Some(18),
                liquidity_cost_bps: Some(5),
                surcharge_bps: None,
                metadata: serde_json::json!({}),
            }],
            cutoff_policies: vec![UpsertCorridorCutoffPolicyRequest {
                cutoff_policy_id: "cutoff_hk".to_string(),
                corridor_pack_id: "corridor_vn_hk".to_string(),
                timezone: "Asia/Hong_Kong".to_string(),
                cutoff_windows: serde_json::json!([{"day":"weekday","cutoff":"17:00"}]),
                holiday_calendar: Some("HK".to_string()),
                retry_rule: Some("next_business_day".to_string()),
                exception_policy: Some("operator_approved".to_string()),
                metadata: serde_json::json!({}),
            }],
            compliance_hooks: vec![UpsertCorridorComplianceHookRequest {
                compliance_hook_id: "hook_travel_rule".to_string(),
                corridor_pack_id: "corridor_vn_hk".to_string(),
                hook_kind: "travel_rule".to_string(),
                provider_key: Some("travel_rule_provider".to_string()),
                required: true,
                config: serde_json::json!({"threshold":"1000"}),
                metadata: serde_json::json!({}),
            }],
            rollout_scopes: vec![UpsertCorridorRolloutScopeRequest {
                rollout_scope_id: "rollout_hk".to_string(),
                corridor_pack_id: "corridor_vn_hk".to_string(),
                tenant_id: Some("tenant-a".to_string()),
                environment: "production".to_string(),
                geography: Some("HK".to_string()),
                method_family: Some("push_transfer".to_string()),
                rollout_state: "approved".to_string(),
                approval_reference: Some("approval_hk".to_string()),
                metadata: serde_json::json!({}),
            }],
            eligibility_rules: vec![UpsertCorridorEligibilityRuleRequest {
                eligibility_rule_id: "eligibility_hk".to_string(),
                corridor_pack_id: "corridor_vn_hk".to_string(),
                partner_id: Some("bank_hk_partner".to_string()),
                entity_type: Some("individual".to_string()),
                method_family: Some("push_transfer".to_string()),
                amount_bounds: serde_json::json!({"min":"100","max":"10000"}),
                compliance_requirements: serde_json::json!({"travelRule":true}),
                metadata: serde_json::json!({}),
            }],
        })
        .await
        .expect("upsert should succeed")
        .expect("corridor should exist");

    assert_eq!(corridor.destination_currency, "HKD");
    assert_eq!(corridor.endpoints.len(), 1);
    assert_eq!(corridor.fee_profiles.len(), 1);

    let loaded = service
        .get_corridor_pack(Some("tenant-a"), "VN_HK_PAYOUT")
        .await
        .expect("load should succeed")
        .expect("corridor should exist");

    assert_eq!(loaded.corridor_code, "VN_HK_PAYOUT");
    assert_eq!(loaded.compliance_hooks[0].hook_kind, "travel_rule");
}
