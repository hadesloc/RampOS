use rust_decimal_macros::dec;

use ramp_compliance::{
    AmountThreshold, AssetScope, CounterpartyScope, TravelRuleAction, TravelRuleCounterparty,
    TravelRuleDirection, TravelRuleDirectionScope, TravelRuleEvaluationRequest,
    TravelRulePolicy, TravelRulePolicyEngine, VaspInteroperabilityStatus, VaspReviewStatus,
};

fn sample_policy() -> TravelRulePolicy {
    TravelRulePolicy {
        policy_code: "fatf-default".to_string(),
        display_name: "FATF default".to_string(),
        jurisdiction_code: Some("SG".to_string()),
        direction_scope: TravelRuleDirectionScope::Outbound,
        asset_scope: AssetScope {
            asset_symbols: vec!["USDC".to_string()],
            asset_networks: vec!["SOL".to_string()],
        },
        threshold: Some(AmountThreshold::new(dec!(1000), "USD")),
        counterparty_scope: CounterpartyScope {
            require_approved_vasp: true,
            require_interoperable: true,
            require_directional_support: true,
            ..CounterpartyScope::default()
        },
        default_transport_profile: Some("trp-bridge".to_string()),
        default_action: TravelRuleAction::DiscloseBeforeSettlement,
        policy_version: "v1".to_string(),
        is_active: true,
        metadata: serde_json::json!({}),
    }
}

fn sample_request() -> TravelRuleEvaluationRequest {
    TravelRuleEvaluationRequest {
        direction: TravelRuleDirection::Outbound,
        jurisdiction_code: Some("SG".to_string()),
        asset_symbol: "USDC".to_string(),
        asset_network: Some("SOL".to_string()),
        amount: dec!(2500),
        amount_currency: "USD".to_string(),
        transport_profile: None,
        originator_vasp: None,
        beneficiary_vasp: Some(TravelRuleCounterparty {
            vasp_code: Some("vasp-1".to_string()),
            jurisdiction_code: Some("SG".to_string()),
            travel_rule_profile: Some("trp-bridge".to_string()),
            transport_profile: Some("trp-bridge".to_string()),
            review_status: Some(VaspReviewStatus::Approved),
            interoperability_status: Some(VaspInteroperabilityStatus::Ready),
            supports_inbound: Some(true),
            supports_outbound: Some(true),
        }),
    }
}

#[test]
fn policy_engine_triggers_disclosure_for_matching_outbound_counterparty() {
    let result = TravelRulePolicyEngine::evaluate(&[sample_policy()], &sample_request());

    assert_eq!(result.matched_policy_code.as_deref(), Some("fatf-default"));
    assert_eq!(result.action, TravelRuleAction::DiscloseBeforeSettlement);
    assert!(result.disclosure_required);
    assert_eq!(result.selected_transport_profile.as_deref(), Some("trp-bridge"));
}

#[test]
fn policy_engine_requires_review_when_counterparty_scope_does_not_match() {
    let mut request = sample_request();
    request.beneficiary_vasp = Some(TravelRuleCounterparty {
        interoperability_status: Some(VaspInteroperabilityStatus::Unknown),
        supports_inbound: Some(false),
        ..request.beneficiary_vasp.unwrap()
    });

    let result = TravelRulePolicyEngine::evaluate(&[sample_policy()], &request);

    assert_eq!(result.action, TravelRuleAction::ReviewRequired);
    assert!(!result.disclosure_required);
}
