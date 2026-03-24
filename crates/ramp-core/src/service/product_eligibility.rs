use ramp_common::Result;
use serde::{Deserialize, Serialize};

use super::{
    CommercialReadinessService, CorridorPackService, PaymentMethodCapabilityService,
    VenueTrustService,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProductEligibilityDecisionState {
    Allow,
    Review,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceOfFundsPackageAction {
    None,
    Reuse,
    CreateDraft,
    CreateInReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductEligibilityRequest {
    pub tenant_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub jurisdiction: String,
    pub user_tier: Option<String>,
    pub kyb_state: Option<String>,
    pub wallet_attestation_state: String,
    pub venue_key: String,
    pub action: String,
    pub asset: String,
    pub network: String,
    pub payment_method_family: Option<String>,
    pub funding_source: Option<String>,
    pub commercial_extension_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductEligibilityReason {
    pub code: String,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductEligibilitySourceOfFundsDecision {
    pub action: SourceOfFundsPackageAction,
    pub package_id: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductEligibilityDecision {
    pub decision: ProductEligibilityDecisionState,
    pub source: String,
    pub reasons: Vec<ProductEligibilityReason>,
    pub source_of_funds: ProductEligibilitySourceOfFundsDecision,
}

#[derive(Clone)]
pub struct ProductEligibilityService {
    venue_trust_service: VenueTrustService,
    corridor_pack_service: CorridorPackService,
    payment_method_capability_service: PaymentMethodCapabilityService,
    commercial_readiness_service: Option<CommercialReadinessService>,
}

impl ProductEligibilityService {
    pub fn new(
        venue_trust_service: VenueTrustService,
        corridor_pack_service: CorridorPackService,
        payment_method_capability_service: PaymentMethodCapabilityService,
    ) -> Self {
        Self {
            venue_trust_service,
            corridor_pack_service,
            payment_method_capability_service,
            commercial_readiness_service: None,
        }
    }

    pub fn with_commercial_readiness(
        venue_trust_service: VenueTrustService,
        corridor_pack_service: CorridorPackService,
        payment_method_capability_service: PaymentMethodCapabilityService,
        commercial_readiness_service: CommercialReadinessService,
    ) -> Self {
        Self {
            venue_trust_service,
            corridor_pack_service,
            payment_method_capability_service,
            commercial_readiness_service: Some(commercial_readiness_service),
        }
    }

    pub fn venue_trust_service(&self) -> &VenueTrustService {
        &self.venue_trust_service
    }

    pub fn corridor_pack_service(&self) -> &CorridorPackService {
        &self.corridor_pack_service
    }

    pub fn payment_method_capability_service(&self) -> &PaymentMethodCapabilityService {
        &self.payment_method_capability_service
    }

    pub async fn evaluate(
        &self,
        request: &ProductEligibilityRequest,
    ) -> Result<ProductEligibilityDecision> {
        let subject_snapshot = self
            .venue_trust_service
            .get_subject_snapshot(
                &request.tenant_id,
                &request.subject_type,
                &request.subject_id,
            )
            .await?;

        let mut reasons = Vec::new();
        let Some(connection) = subject_snapshot.connections.iter().find(|connection| {
            connection
                .venue_key
                .eq_ignore_ascii_case(&request.venue_key)
                && is_ready_state(&connection.status)
        }) else {
            reasons.push(reason(
                "venue_not_ready",
                "venue_trust",
                "No active venue connection matched the request",
            ));
            return Ok(decision(
                ProductEligibilityDecisionState::Deny,
                reasons,
                sof_decision(SourceOfFundsPackageAction::None, None, "venue_trust"),
            ));
        };

        let Some(account) = subject_snapshot.accounts.iter().find(|account| {
            account.venue_connection_id == connection.connection_id
                && is_ready_state(&account.status)
        }) else {
            reasons.push(reason(
                "venue_account_not_ready",
                "venue_trust",
                "No active venue account matched the venue connection",
            ));
            return Ok(decision(
                ProductEligibilityDecisionState::Deny,
                reasons,
                sof_decision(SourceOfFundsPackageAction::None, None, "venue_trust"),
            ));
        };
        reasons.push(reason(
            "venue_ready",
            "venue_trust",
            "Venue connection and account are active",
        ));

        if !is_ready_state(&request.wallet_attestation_state) {
            reasons.push(reason(
                "wallet_attestation_not_ready",
                "caller",
                "Wallet attestation state does not allow fast-lane funding",
            ));
            return Ok(decision(
                ProductEligibilityDecisionState::Deny,
                reasons,
                sof_decision(SourceOfFundsPackageAction::None, None, "caller"),
            ));
        }

        let corridor_snapshot = self
            .corridor_pack_service
            .list_corridor_packs(Some(&request.tenant_id))
            .await?;
        let Some(corridor) = corridor_snapshot.corridor_packs.iter().find(|corridor| {
            corridor
                .destination_market
                .eq_ignore_ascii_case(&request.jurisdiction)
                && corridor
                    .settlement_direction
                    .eq_ignore_ascii_case(&request.action)
                && corridor
                    .metadata
                    .get("venueKey")
                    .and_then(serde_json::Value::as_str)
                    .is_none_or(|value| value.eq_ignore_ascii_case(&request.venue_key))
                && corridor
                    .metadata
                    .get("asset")
                    .and_then(serde_json::Value::as_str)
                    .is_none_or(|value| value.eq_ignore_ascii_case(&request.asset))
                && corridor
                    .metadata
                    .get("network")
                    .and_then(serde_json::Value::as_str)
                    .is_none_or(|value| value.eq_ignore_ascii_case(&request.network))
                && is_corridor_eligible(corridor.lifecycle_state.as_str())
                && is_corridor_eligible(corridor.rollout_state.as_str())
                && is_corridor_eligible(corridor.eligibility_state.as_str())
        }) else {
            reasons.push(reason(
                "corridor_not_eligible",
                "corridor_pack",
                "No corridor inventory matched the request",
            ));
            return Ok(decision(
                ProductEligibilityDecisionState::Deny,
                reasons,
                sof_decision(SourceOfFundsPackageAction::None, None, "corridor_pack"),
            ));
        };
        reasons.push(reason(
            "corridor_inventory_match",
            "corridor_pack",
            "Corridor inventory matched the requested venue funding lane",
        ));

        let capabilities = self
            .payment_method_capability_service
            .list_capabilities(Some(&corridor.corridor_pack_id), None)
            .await?;
        let Some(capability) = capabilities.capabilities.iter().find(|capability| {
            capability
                .settlement_direction
                .eq_ignore_ascii_case(&request.action)
                && request
                    .payment_method_family
                    .as_deref()
                    .is_none_or(|value| capability.method_family.eq_ignore_ascii_case(value))
                && request.funding_source.as_deref().is_none_or(|value| {
                    capability
                        .funding_source
                        .as_deref()
                        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(value))
                })
        }) else {
            reasons.push(reason(
                "payment_method_not_supported",
                "payment_method_capability",
                "No payment method capability matched the requested funding method",
            ));
            return Ok(decision(
                ProductEligibilityDecisionState::Deny,
                reasons,
                sof_decision(
                    SourceOfFundsPackageAction::None,
                    None,
                    "payment_method_capability",
                ),
            ));
        };
        reasons.push(reason(
            "payment_method_supported",
            "payment_method_capability",
            "Payment method capability inventory matched the request",
        ));

        if let Some(extension_id) = request.commercial_extension_id.as_deref() {
            let Some(readiness_service) = &self.commercial_readiness_service else {
                reasons.push(reason(
                    "commercial_readiness_unavailable",
                    "commercial_readiness",
                    "Commercial extension gating was requested but no readiness service was injected",
                ));
                return Ok(decision(
                    ProductEligibilityDecisionState::Review,
                    reasons,
                    sof_decision(
                        SourceOfFundsPackageAction::None,
                        None,
                        "commercial_readiness",
                    ),
                ));
            };
            let readiness = readiness_service.check_enablement_conditions(
                extension_id,
                &Vec::new(),
                &vec![corridor.corridor_code.clone()],
                &Vec::new(),
            )?;
            if !readiness.can_enable {
                reasons.push(reason(
                    "commercial_readiness_review",
                    "commercial_readiness",
                    "Commercial extension prerequisites are not fully enabled",
                ));
                return Ok(decision(
                    ProductEligibilityDecisionState::Review,
                    reasons,
                    sof_decision(
                        SourceOfFundsPackageAction::None,
                        None,
                        "commercial_readiness",
                    ),
                ));
            }
            reasons.push(reason(
                "commercial_readiness_enabled",
                "commercial_readiness",
                "Commercial readiness gate passed",
            ));
        }

        if requires_source_of_funds(capability.policy_flags.get("sourceOfFundsRequired")) {
            let package = subject_snapshot
                .source_of_funds_packages
                .iter()
                .find(|package| {
                    package
                        .venue_account_id
                        .as_deref()
                        .is_none_or(|value| value == account.account_id)
                });
            if let Some(package) = package {
                let status = package.review_status.to_ascii_lowercase();
                if status == "approved" {
                    reasons.push(reason(
                        "source_of_funds_approved",
                        "venue_trust",
                        "Existing source-of-funds package is approved",
                    ));
                    return Ok(decision(
                        ProductEligibilityDecisionState::Allow,
                        reasons,
                        sof_decision(
                            SourceOfFundsPackageAction::Reuse,
                            Some(package.package_id.clone()),
                            "venue_trust",
                        ),
                    ));
                }
                reasons.push(reason(
                    "source_of_funds_in_review",
                    "venue_trust",
                    "Existing source-of-funds package still requires review",
                ));
                return Ok(decision(
                    ProductEligibilityDecisionState::Review,
                    reasons,
                    sof_decision(
                        SourceOfFundsPackageAction::Reuse,
                        Some(package.package_id.clone()),
                        "venue_trust",
                    ),
                ));
            }

            let action = if capability
                .policy_flags
                .get("sourceOfFundsAutoSubmit")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                SourceOfFundsPackageAction::CreateInReview
            } else {
                SourceOfFundsPackageAction::CreateDraft
            };
            reasons.push(reason(
                "source_of_funds_required",
                "payment_method_capability",
                "Funding lane requires a source-of-funds package before approval",
            ));
            return Ok(decision(
                ProductEligibilityDecisionState::Review,
                reasons,
                sof_decision(action, None, "product_eligibility"),
            ));
        }

        Ok(decision(
            ProductEligibilityDecisionState::Allow,
            reasons,
            sof_decision(
                SourceOfFundsPackageAction::None,
                None,
                "product_eligibility",
            ),
        ))
    }
}

fn reason(code: &str, source: &str, message: &str) -> ProductEligibilityReason {
    ProductEligibilityReason {
        code: code.to_string(),
        source: source.to_string(),
        message: message.to_string(),
    }
}

fn sof_decision(
    action: SourceOfFundsPackageAction,
    package_id: Option<String>,
    source: &str,
) -> ProductEligibilitySourceOfFundsDecision {
    ProductEligibilitySourceOfFundsDecision {
        action,
        package_id,
        source: source.to_string(),
    }
}

fn decision(
    state: ProductEligibilityDecisionState,
    reasons: Vec<ProductEligibilityReason>,
    source_of_funds: ProductEligibilitySourceOfFundsDecision,
) -> ProductEligibilityDecision {
    ProductEligibilityDecision {
        decision: state,
        source: "product_eligibility".to_string(),
        reasons,
        source_of_funds,
    }
}

fn is_ready_state(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "active" | "approved" | "verified" | "connected"
    )
}

fn is_corridor_eligible(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "active" | "approved" | "pilot"
    )
}

fn requires_source_of_funds(value: Option<&serde_json::Value>) -> bool {
    value.and_then(serde_json::Value::as_bool).unwrap_or(false)
}
