use ramp_core::repository::{
    CorridorPackRepository, PgCorridorPackRepository, UpsertCorridorComplianceHookRequest,
    UpsertCorridorCutoffPolicyRequest, UpsertCorridorEligibilityRuleRequest,
    UpsertCorridorEndpointRequest, UpsertCorridorFeeProfileRequest,
    UpsertCorridorPackRequest, UpsertCorridorRolloutScopeRequest,
};
use sqlx::PgPool;

#[tokio::test]
async fn corridor_pack_repository_persists_graph() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };

    let pool = PgPool::connect(&database_url)
        .await
        .expect("database connection should succeed");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should succeed");

    let repository = PgCorridorPackRepository::new(pool);
    repository
        .upsert_corridor_pack(&UpsertCorridorPackRequest {
            corridor_pack_id: "corridor_vn_sg".to_string(),
            tenant_id: Some("tenant_corridor_repo".to_string()),
            corridor_code: "VN_SG_PAYOUT".to_string(),
            source_market: "VN".to_string(),
            destination_market: "SG".to_string(),
            source_currency: "VND".to_string(),
            destination_currency: "SGD".to_string(),
            settlement_direction: "payout".to_string(),
            fee_model: "shared".to_string(),
            lifecycle_state: "pilot".to_string(),
            rollout_state: "approved".to_string(),
            eligibility_state: "active".to_string(),
            metadata: serde_json::json!({ "phase": "m2" }),
        })
        .await
        .expect("corridor pack should persist");

    repository
        .upsert_endpoint(&UpsertCorridorEndpointRequest {
            endpoint_id: "endpoint_source_vn".to_string(),
            corridor_pack_id: "corridor_vn_sg".to_string(),
            endpoint_role: "source".to_string(),
            partner_id: Some("partner_vcb_vn".to_string()),
            provider_key: None,
            adapter_key: Some("vietqr".to_string()),
            entity_type: "individual".to_string(),
            rail: "vietqr".to_string(),
            method_family: Some("bank_transfer".to_string()),
            settlement_mode: Some("instant".to_string()),
            instrument_family: Some("bank_transfer".to_string()),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("source endpoint should persist");
    repository
        .upsert_endpoint(&UpsertCorridorEndpointRequest {
            endpoint_id: "endpoint_destination_sg".to_string(),
            corridor_pack_id: "corridor_vn_sg".to_string(),
            endpoint_role: "destination".to_string(),
            partner_id: Some("partner_scb_sg".to_string()),
            provider_key: None,
            adapter_key: Some("fps".to_string()),
            entity_type: "business".to_string(),
            rail: "fps".to_string(),
            method_family: Some("push_transfer".to_string()),
            settlement_mode: Some("same_day".to_string()),
            instrument_family: Some("bank_transfer".to_string()),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("destination endpoint should persist");
    repository
        .upsert_fee_profile(&UpsertCorridorFeeProfileRequest {
            fee_profile_id: "fee_vn_sg".to_string(),
            corridor_pack_id: "corridor_vn_sg".to_string(),
            fee_currency: "SGD".to_string(),
            base_fee: Some("3.50".to_string()),
            fx_spread_bps: Some(20),
            liquidity_cost_bps: Some(10),
            surcharge_bps: None,
            metadata: serde_json::json!({}),
        })
        .await
        .expect("fee profile should persist");
    repository
        .upsert_cutoff_policy(&UpsertCorridorCutoffPolicyRequest {
            cutoff_policy_id: "cutoff_vn_sg".to_string(),
            corridor_pack_id: "corridor_vn_sg".to_string(),
            timezone: "Asia/Singapore".to_string(),
            cutoff_windows: serde_json::json!([{"day":"weekday","cutoff":"17:00"}]),
            holiday_calendar: Some("SG".to_string()),
            retry_rule: Some("next_business_day".to_string()),
            exception_policy: Some("operator_approved".to_string()),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("cutoff policy should persist");
    repository
        .upsert_compliance_hook(&UpsertCorridorComplianceHookRequest {
            compliance_hook_id: "hook_vn_sg".to_string(),
            corridor_pack_id: "corridor_vn_sg".to_string(),
            hook_kind: "sanctions".to_string(),
            provider_key: Some("provider_chainalysis".to_string()),
            required: true,
            config: serde_json::json!({ "screening": "pre_settlement" }),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("compliance hook should persist");
    repository
        .upsert_rollout_scope(&UpsertCorridorRolloutScopeRequest {
            rollout_scope_id: "rollout_vn_sg".to_string(),
            corridor_pack_id: "corridor_vn_sg".to_string(),
            tenant_id: Some("tenant_corridor_repo".to_string()),
            environment: "production".to_string(),
            geography: Some("SG".to_string()),
            method_family: Some("push_transfer".to_string()),
            rollout_state: "approved".to_string(),
            approval_reference: Some("approval_corridor_vn_sg".to_string()),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("rollout scope should persist");
    repository
        .upsert_eligibility_rule(&UpsertCorridorEligibilityRuleRequest {
            eligibility_rule_id: "eligibility_vn_sg".to_string(),
            corridor_pack_id: "corridor_vn_sg".to_string(),
            partner_id: Some("partner_scb_sg".to_string()),
            entity_type: Some("business".to_string()),
            method_family: Some("push_transfer".to_string()),
            amount_bounds: serde_json::json!({"min":"100.00","max":"5000.00"}),
            compliance_requirements: serde_json::json!({ "travelRule": true }),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("eligibility rule should persist");

    let rows = repository
        .list_corridor_packs(Some("tenant_corridor_repo"))
        .await
        .expect("corridor list should load");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].corridor_code, "VN_SG_PAYOUT");
    assert_eq!(rows[0].endpoints.len(), 2);
    assert_eq!(rows[0].compliance_hooks[0].hook_kind, "sanctions");

    let detail = repository
        .get_corridor_pack(Some("tenant_corridor_repo"), "VN_SG_PAYOUT")
        .await
        .expect("corridor detail should load")
        .expect("corridor should exist");
    assert_eq!(detail.source_currency, "VND");
    assert_eq!(detail.destination_currency, "SGD");
    assert_eq!(detail.rollout_scopes[0].rollout_state, "approved");
}
