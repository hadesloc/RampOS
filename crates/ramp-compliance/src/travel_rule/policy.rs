use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::travel_rule::{
    TravelRuleAction, TravelRuleDirection, TravelRuleDirectionScope, TravelRuleRequirement,
    VaspInteroperabilityStatus, VaspReviewStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmountThreshold {
    pub amount: Decimal,
    pub currency: String,
}

impl AmountThreshold {
    pub fn new(amount: Decimal, currency: impl Into<String>) -> Self {
        Self {
            amount,
            currency: currency.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetScope {
    pub asset_symbols: Vec<String>,
    pub asset_networks: Vec<String>,
}

impl AssetScope {
    fn matches(&self, request: &TravelRuleEvaluationRequest) -> ScopeCheck {
        if !self.asset_symbols.is_empty()
            && !contains_ignore_case(&self.asset_symbols, &request.asset_symbol)
        {
            return ScopeCheck::no_match();
        }

        if self.asset_networks.is_empty() {
            return ScopeCheck::matched();
        }

        match request.asset_network.as_deref() {
            Some(network) if contains_ignore_case(&self.asset_networks, network) => {
                ScopeCheck::matched()
            }
            Some(_) => ScopeCheck::no_match(),
            None => ScopeCheck::missing(TravelRuleRequirement::AssetNetwork),
        }
    }

    fn specificity_score(&self) -> u8 {
        u8::from(!self.asset_symbols.is_empty()) + u8::from(!self.asset_networks.is_empty())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyScope {
    pub vasp_codes: Vec<String>,
    pub jurisdictions: Vec<String>,
    pub travel_rule_profiles: Vec<String>,
    pub transport_profiles: Vec<String>,
    pub require_approved_vasp: bool,
    pub require_interoperable: bool,
    pub require_directional_support: bool,
}

impl CounterpartyScope {
    fn is_empty(&self) -> bool {
        self.vasp_codes.is_empty()
            && self.jurisdictions.is_empty()
            && self.travel_rule_profiles.is_empty()
            && self.transport_profiles.is_empty()
            && !self.require_approved_vasp
            && !self.require_interoperable
            && !self.require_directional_support
    }

    fn specificity_score(&self) -> u8 {
        u8::from(!self.vasp_codes.is_empty())
            + u8::from(!self.jurisdictions.is_empty())
            + u8::from(!self.travel_rule_profiles.is_empty())
            + u8::from(!self.transport_profiles.is_empty())
            + u8::from(self.require_approved_vasp)
            + u8::from(self.require_interoperable)
            + u8::from(self.require_directional_support)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleCounterparty {
    pub vasp_code: Option<String>,
    pub jurisdiction_code: Option<String>,
    pub travel_rule_profile: Option<String>,
    pub transport_profile: Option<String>,
    pub review_status: Option<VaspReviewStatus>,
    pub interoperability_status: Option<VaspInteroperabilityStatus>,
    pub supports_inbound: Option<bool>,
    pub supports_outbound: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRulePolicy {
    pub policy_code: String,
    pub display_name: String,
    pub jurisdiction_code: Option<String>,
    pub direction_scope: TravelRuleDirectionScope,
    pub asset_scope: AssetScope,
    pub threshold: Option<AmountThreshold>,
    pub counterparty_scope: CounterpartyScope,
    pub default_transport_profile: Option<String>,
    pub default_action: TravelRuleAction,
    pub policy_version: String,
    pub is_active: bool,
    pub metadata: Value,
}

impl TravelRulePolicy {
    fn specificity_score(&self) -> u8 {
        u8::from(self.jurisdiction_code.is_some())
            + u8::from(!matches!(
                self.direction_scope,
                TravelRuleDirectionScope::Both
            ))
            + self.asset_scope.specificity_score()
            + u8::from(self.threshold.is_some())
            + self.counterparty_scope.specificity_score()
            + u8::from(self.default_transport_profile.is_some())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleEvaluationRequest {
    pub direction: TravelRuleDirection,
    pub jurisdiction_code: Option<String>,
    pub asset_symbol: String,
    pub asset_network: Option<String>,
    pub amount: Decimal,
    pub amount_currency: String,
    pub transport_profile: Option<String>,
    pub originator_vasp: Option<TravelRuleCounterparty>,
    pub beneficiary_vasp: Option<TravelRuleCounterparty>,
}

impl TravelRuleEvaluationRequest {
    pub fn counterparty(&self) -> Option<&TravelRuleCounterparty> {
        match self.direction {
            TravelRuleDirection::Outbound => self.beneficiary_vasp.as_ref(),
            TravelRuleDirection::Inbound => self.originator_vasp.as_ref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleEvaluationResult {
    pub matched_policy_code: Option<String>,
    pub matched_policy_version: Option<String>,
    pub action: TravelRuleAction,
    pub disclosure_required: bool,
    pub selected_transport_profile: Option<String>,
    pub unmet_requirements: Vec<TravelRuleRequirement>,
}

impl TravelRuleEvaluationResult {
    pub fn is_match(&self) -> bool {
        self.matched_policy_code.is_some()
    }

    pub fn requires_review(&self) -> bool {
        self.action == TravelRuleAction::ReviewRequired
    }
}

pub struct TravelRulePolicyEngine;

impl TravelRulePolicyEngine {
    pub fn evaluate(
        policies: &[TravelRulePolicy],
        request: &TravelRuleEvaluationRequest,
    ) -> TravelRuleEvaluationResult {
        let mut best_match: Option<PolicyCandidate> = None;

        for policy in policies {
            if let Some(candidate) = evaluate_policy(policy, request) {
                let replace = match best_match.as_ref() {
                    None => true,
                    Some(current) => {
                        candidate.specificity_score > current.specificity_score
                            || (candidate.specificity_score == current.specificity_score
                                && candidate.is_complete
                                && !current.is_complete)
                    }
                };

                if replace {
                    best_match = Some(candidate);
                }
            }
        }

        if let Some(candidate) = best_match {
            return TravelRuleEvaluationResult {
                matched_policy_code: Some(candidate.policy_code),
                matched_policy_version: Some(candidate.policy_version),
                action: if candidate.is_complete {
                    candidate.policy_action
                } else {
                    TravelRuleAction::ReviewRequired
                },
                disclosure_required: candidate.policy_action.requires_disclosure(),
                selected_transport_profile: candidate.selected_transport_profile,
                unmet_requirements: candidate.unmet_requirements,
            };
        }

        TravelRuleEvaluationResult {
            matched_policy_code: None,
            matched_policy_version: None,
            action: TravelRuleAction::ReviewRequired,
            disclosure_required: false,
            selected_transport_profile: None,
            unmet_requirements: vec![TravelRuleRequirement::Counterparty],
        }
    }
}

#[derive(Debug, Clone)]
struct PolicyCandidate {
    policy_code: String,
    policy_version: String,
    policy_action: TravelRuleAction,
    selected_transport_profile: Option<String>,
    unmet_requirements: Vec<TravelRuleRequirement>,
    specificity_score: u8,
    is_complete: bool,
}

#[derive(Debug, Clone, Default)]
struct ScopeCheck {
    is_match: bool,
    missing_requirements: Vec<TravelRuleRequirement>,
}

impl ScopeCheck {
    fn matched() -> Self {
        Self {
            is_match: true,
            missing_requirements: Vec::new(),
        }
    }

    fn missing(requirement: TravelRuleRequirement) -> Self {
        Self {
            is_match: true,
            missing_requirements: vec![requirement],
        }
    }

    fn no_match() -> Self {
        Self {
            is_match: false,
            missing_requirements: Vec::new(),
        }
    }
}

fn evaluate_policy(
    policy: &TravelRulePolicy,
    request: &TravelRuleEvaluationRequest,
) -> Option<PolicyCandidate> {
    if !policy.is_active || !policy.direction_scope.matches(request.direction) {
        return None;
    }

    let mut unmet_requirements = Vec::new();

    if let Some(policy_jurisdiction) = policy.jurisdiction_code.as_deref() {
        match request.jurisdiction_code.as_deref() {
            Some(request_jurisdiction)
                if equals_ignore_case(policy_jurisdiction, request_jurisdiction) => {}
            Some(_) => return None,
            None => unmet_requirements.push(TravelRuleRequirement::Jurisdiction),
        }
    }

    let asset_check = policy.asset_scope.matches(request);
    if !asset_check.is_match {
        return None;
    }
    append_requirements(&mut unmet_requirements, asset_check.missing_requirements);

    if let Some(threshold) = &policy.threshold {
        if !equals_ignore_case(&threshold.currency, &request.amount_currency)
            || request.amount < threshold.amount
        {
            return None;
        }
    }

    let counterparty = request.counterparty();
    let counterparty_check =
        evaluate_counterparty_scope(&policy.counterparty_scope, counterparty, request.direction);
    if !counterparty_check.is_match {
        return None;
    }
    append_requirements(
        &mut unmet_requirements,
        counterparty_check.missing_requirements,
    );

    let selected_transport_profile = request
        .transport_profile
        .clone()
        .or_else(|| counterparty.and_then(|value| value.transport_profile.clone()))
        .or_else(|| policy.default_transport_profile.clone());

    if !policy.counterparty_scope.transport_profiles.is_empty() {
        match selected_transport_profile.as_deref() {
            Some(profile)
                if contains_ignore_case(&policy.counterparty_scope.transport_profiles, profile) => {
            }
            Some(_) => return None,
            None => unmet_requirements.push(TravelRuleRequirement::TransportProfile),
        }
    }

    if policy.default_action.requires_disclosure() {
        if counterparty.is_none() {
            unmet_requirements.push(TravelRuleRequirement::Counterparty);
        }

        if selected_transport_profile.is_none() {
            unmet_requirements.push(TravelRuleRequirement::TransportProfile);
        }
    }

    dedup_requirements(&mut unmet_requirements);

    Some(PolicyCandidate {
        policy_code: policy.policy_code.clone(),
        policy_version: policy.policy_version.clone(),
        policy_action: policy.default_action,
        selected_transport_profile,
        unmet_requirements: unmet_requirements.clone(),
        specificity_score: policy.specificity_score(),
        is_complete: unmet_requirements.is_empty(),
    })
}

fn evaluate_counterparty_scope(
    scope: &CounterpartyScope,
    counterparty: Option<&TravelRuleCounterparty>,
    direction: TravelRuleDirection,
) -> ScopeCheck {
    if scope.is_empty() {
        return ScopeCheck::matched();
    }

    let Some(counterparty) = counterparty else {
        return ScopeCheck::missing(TravelRuleRequirement::Counterparty);
    };

    let mut missing_requirements = Vec::new();

    if !scope.vasp_codes.is_empty() {
        match counterparty.vasp_code.as_deref() {
            Some(code) if contains_ignore_case(&scope.vasp_codes, code) => {}
            Some(_) => return ScopeCheck::no_match(),
            None => missing_requirements.push(TravelRuleRequirement::CounterpartyVaspCode),
        }
    }

    if !scope.jurisdictions.is_empty() {
        match counterparty.jurisdiction_code.as_deref() {
            Some(code) if contains_ignore_case(&scope.jurisdictions, code) => {}
            Some(_) => return ScopeCheck::no_match(),
            None => missing_requirements.push(TravelRuleRequirement::CounterpartyJurisdiction),
        }
    }

    if !scope.travel_rule_profiles.is_empty() {
        match counterparty.travel_rule_profile.as_deref() {
            Some(profile) if contains_ignore_case(&scope.travel_rule_profiles, profile) => {}
            Some(_) => return ScopeCheck::no_match(),
            None => missing_requirements.push(TravelRuleRequirement::CounterpartyTravelRuleProfile),
        }
    }

    if scope.require_approved_vasp {
        match counterparty.review_status {
            Some(VaspReviewStatus::Approved) => {}
            Some(VaspReviewStatus::Rejected | VaspReviewStatus::Suspended) => {
                return ScopeCheck::no_match();
            }
            Some(VaspReviewStatus::Pending) | None => {
                missing_requirements.push(TravelRuleRequirement::ApprovedCounterparty);
            }
        }
    }

    if scope.require_interoperable {
        match counterparty.interoperability_status {
            Some(VaspInteroperabilityStatus::Ready) | Some(VaspInteroperabilityStatus::Limited) => {
            }
            Some(VaspInteroperabilityStatus::Disabled)
            | Some(VaspInteroperabilityStatus::Degraded) => {
                return ScopeCheck::no_match();
            }
            Some(VaspInteroperabilityStatus::Unknown) | None => {
                missing_requirements.push(TravelRuleRequirement::InteroperableCounterparty);
            }
        }
    }

    if scope.require_directional_support {
        let supported = match direction {
            TravelRuleDirection::Outbound => counterparty.supports_inbound,
            TravelRuleDirection::Inbound => counterparty.supports_outbound,
        };

        match supported {
            Some(true) => {}
            Some(false) => return ScopeCheck::no_match(),
            None => missing_requirements.push(TravelRuleRequirement::DirectionalSupport),
        }
    }

    ScopeCheck {
        is_match: true,
        missing_requirements,
    }
}

fn append_requirements(
    target: &mut Vec<TravelRuleRequirement>,
    requirements: Vec<TravelRuleRequirement>,
) {
    target.extend(requirements);
    dedup_requirements(target);
}

fn dedup_requirements(requirements: &mut Vec<TravelRuleRequirement>) {
    requirements.sort_unstable_by_key(requirement_rank);
    requirements.dedup();
}

fn requirement_rank(requirement: &TravelRuleRequirement) -> u8 {
    match requirement {
        TravelRuleRequirement::Jurisdiction => 0,
        TravelRuleRequirement::AssetNetwork => 1,
        TravelRuleRequirement::Counterparty => 2,
        TravelRuleRequirement::CounterpartyJurisdiction => 3,
        TravelRuleRequirement::CounterpartyVaspCode => 4,
        TravelRuleRequirement::CounterpartyTravelRuleProfile => 5,
        TravelRuleRequirement::TransportProfile => 6,
        TravelRuleRequirement::ApprovedCounterparty => 7,
        TravelRuleRequirement::InteroperableCounterparty => 8,
        TravelRuleRequirement::DirectionalSupport => 9,
    }
}

fn contains_ignore_case(values: &[String], candidate: &str) -> bool {
    values
        .iter()
        .any(|value| equals_ignore_case(value, candidate))
}

fn equals_ignore_case(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;
    use crate::travel_rule::{
        TravelRuleAction, TravelRuleDirection, TravelRuleDirectionScope, TravelRuleRequirement,
        VaspInteroperabilityStatus, VaspReviewStatus,
    };

    #[test]
    fn evaluates_outbound_threshold_match() {
        let policy = sample_policy();

        let result = TravelRulePolicyEngine::evaluate(&[policy], &sample_request());

        assert_eq!(result.action, TravelRuleAction::DiscloseBeforeSettlement);
        assert_eq!(
            result.matched_policy_code.as_deref(),
            Some("global-outbound")
        );
        assert_eq!(
            result.selected_transport_profile.as_deref(),
            Some("trp-bridge")
        );
        assert!(result.disclosure_required);
    }

    #[test]
    fn ignores_policy_when_direction_or_amount_do_not_match() {
        let policy = sample_policy();
        let mut request = sample_request();
        request.direction = TravelRuleDirection::Inbound;

        let result = TravelRulePolicyEngine::evaluate(&[policy.clone()], &request);
        assert_eq!(result.action, TravelRuleAction::ReviewRequired);
        assert!(!result.is_match());

        let mut low_amount_request = sample_request();
        low_amount_request.amount = dec!(999.99);

        let result = TravelRulePolicyEngine::evaluate(&[policy], &low_amount_request);
        assert_eq!(result.action, TravelRuleAction::ReviewRequired);
        assert!(!result.is_match());
    }

    #[test]
    fn review_required_when_counterparty_or_transport_is_missing() {
        let policy = TravelRulePolicy {
            counterparty_scope: CounterpartyScope {
                require_approved_vasp: true,
                require_interoperable: true,
                transport_profiles: vec!["trp-bridge".to_string()],
                ..CounterpartyScope::default()
            },
            default_transport_profile: None,
            ..sample_policy()
        };

        let mut request = sample_request();
        request.beneficiary_vasp = None;
        request.transport_profile = None;

        let result = TravelRulePolicyEngine::evaluate(&[policy], &request);

        assert_eq!(result.action, TravelRuleAction::ReviewRequired);
        assert_eq!(
            result.matched_policy_code.as_deref(),
            Some("global-outbound")
        );
        assert!(result.requires_review());
        assert!(result
            .unmet_requirements
            .contains(&TravelRuleRequirement::Counterparty));
        assert!(result
            .unmet_requirements
            .contains(&TravelRuleRequirement::TransportProfile));
    }

    #[test]
    fn policy_priority_prefers_most_specific_match() {
        let generic_policy = TravelRulePolicy {
            policy_code: "generic".to_string(),
            display_name: "Generic".to_string(),
            jurisdiction_code: None,
            direction_scope: TravelRuleDirectionScope::Both,
            asset_scope: AssetScope::default(),
            threshold: Some(AmountThreshold::new(dec!(1000), "USD")),
            counterparty_scope: CounterpartyScope::default(),
            default_transport_profile: Some("fallback".to_string()),
            default_action: TravelRuleAction::Allow,
            policy_version: "v1".to_string(),
            is_active: true,
            metadata: serde_json::json!({}),
        };
        let specific_policy = sample_policy();

        let result =
            TravelRulePolicyEngine::evaluate(&[generic_policy, specific_policy], &sample_request());

        assert_eq!(
            result.matched_policy_code.as_deref(),
            Some("global-outbound")
        );
        assert_eq!(result.action, TravelRuleAction::DiscloseBeforeSettlement);
    }

    #[test]
    fn review_required_when_no_candidate_policy_survives() {
        let policy = sample_policy();
        let mut request = sample_request();
        request.beneficiary_vasp = Some(TravelRuleCounterparty {
            interoperability_status: Some(VaspInteroperabilityStatus::Disabled),
            ..request
                .beneficiary_vasp
                .clone()
                .expect("sample request should include beneficiary")
        });

        let result = TravelRulePolicyEngine::evaluate(&[policy], &request);

        assert_eq!(result.action, TravelRuleAction::ReviewRequired);
        assert!(!result.is_match());
        assert!(result.requires_review());
        assert!(!result.disclosure_required);
        assert!(result.unmet_requirements.is_empty());
    }

    fn sample_policy() -> TravelRulePolicy {
        TravelRulePolicy {
            policy_code: "global-outbound".to_string(),
            display_name: "Global outbound disclosure".to_string(),
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
}
