//! Provider Routing Service
//!
//! Policy-based compliance provider routing by corridor, entity type, risk tier,
//! amount, asset, and partner. Delivers FR-016: provider-routing plus institutional
//! compliance productization on current seams.

use ramp_common::Result;
use ramp_compliance::provider_routing::ProviderRoutingPolicy;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::str::FromStr;

/// A single routing policy rule that determines which compliance provider to use.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRoutingRule {
    pub rule_id: String,
    pub priority: u32,
    pub corridor_code: Option<String>,
    pub entity_type: Option<String>,
    pub risk_tier: Option<String>,
    pub amount_min: Option<String>,
    pub amount_max: Option<String>,
    pub asset: Option<String>,
    pub partner_id: Option<String>,
    pub provider_key: String,
    pub provider_class: String,
    pub enabled: bool,
    pub metadata: serde_json::Value,
}

/// The result of evaluating routing rules for a given context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRoutingDecision {
    pub selected_provider_key: String,
    pub selected_provider_class: String,
    pub matched_rule_id: String,
    pub match_priority: u32,
    pub evaluation_context: ProviderRoutingContext,
    pub fallback_used: bool,
    pub explanation: String,
    pub provenance: serde_json::Value,
}

/// Context used when evaluating provider routing rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRoutingContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_family: Option<String>,
    pub corridor_code: Option<String>,
    pub entity_type: Option<String>,
    pub risk_tier: Option<String>,
    pub amount: Option<String>,
    pub asset: Option<String>,
    pub partner_id: Option<String>,
    pub tenant_id: Option<String>,
}

/// Institutional compliance record visible on operator surfaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstitutionalComplianceRecord {
    pub record_id: String,
    pub entity_id: String,
    pub entity_type: String,
    pub trust_level: String,
    pub ownership_verified: bool,
    pub rescreening_due_at: Option<String>,
    pub travel_rule_status: Option<String>,
    pub review_state: String,
    pub evidence: serde_json::Value,
    pub metadata: serde_json::Value,
}

/// Snapshot of provider routing state for operator inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRoutingSnapshot {
    pub action_mode: String,
    pub source: String,
    pub rules: Vec<ProviderRoutingRule>,
    pub institutional_records: Vec<InstitutionalComplianceRecord>,
    pub provenance: serde_json::Value,
}

/// Provider routing service — evaluates policy rules and tracks institutional compliance.
#[derive(Clone, Default)]
pub struct ProviderRoutingService {
    rules: Vec<ProviderRoutingRule>,
    institutional_records: Vec<InstitutionalComplianceRecord>,
}

impl ProviderRoutingService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rules(rules: Vec<ProviderRoutingRule>) -> Self {
        Self {
            rules,
            institutional_records: Vec::new(),
        }
    }

    /// Add an institutional compliance record.
    pub fn add_institutional_record(&mut self, record: InstitutionalComplianceRecord) {
        self.institutional_records.push(record);
    }

    /// Evaluate routing rules against a context and return the best match.
    pub fn evaluate(&self, context: &ProviderRoutingContext) -> Result<ProviderRoutingDecision> {
        let mut candidates: Vec<&ProviderRoutingRule> = self
            .rules
            .iter()
            .filter(|rule| rule.enabled && matches_context(rule, context))
            .collect();

        candidates.sort_by_key(|rule| rule.priority);

        if let Some(best) = candidates.first() {
            Ok(ProviderRoutingDecision {
                selected_provider_key: best.provider_key.clone(),
                selected_provider_class: best.provider_class.clone(),
                matched_rule_id: best.rule_id.clone(),
                match_priority: best.priority,
                evaluation_context: context.clone(),
                fallback_used: false,
                explanation: format!(
                    "Matched rule '{}' (priority {}) for provider '{}'",
                    best.rule_id, best.priority, best.provider_key
                ),
                provenance: derive_rule_provenance(best),
            })
        } else {
            // Fallback: return default provider
            Ok(ProviderRoutingDecision {
                selected_provider_key: "default_provider".to_string(),
                selected_provider_class: "kyc_aml".to_string(),
                matched_rule_id: "fallback".to_string(),
                match_priority: u32::MAX,
                evaluation_context: context.clone(),
                fallback_used: true,
                explanation: "No matching rule found; using default fallback provider".to_string(),
                provenance: serde_json::json!({
                    "mode": "fallback",
                    "sourceClass": "bounded_fallback",
                    "reason": "no_matching_rule"
                }),
            })
        }
    }

    pub fn from_policies(policies: Vec<ProviderRoutingPolicy>) -> Self {
        Self {
            rules: policies.into_iter().map(rule_from_policy).collect(),
            institutional_records: Vec::new(),
        }
    }

    pub fn decision_from_policy(
        policy: &ProviderRoutingPolicy,
        context: &ProviderRoutingContext,
    ) -> ProviderRoutingDecision {
        let selected_provider_key = primary_provider_key(policy);
        ProviderRoutingDecision {
            selected_provider_class: policy.provider_family.as_str().to_string(),
            matched_rule_id: policy.policy_id.clone(),
            match_priority: 0,
            evaluation_context: context.clone(),
            fallback_used: false,
            explanation: format!(
                "Matched authoritative policy '{}' for provider '{}'",
                policy.policy_id, selected_provider_key
            ),
            provenance: serde_json::json!({
                "policyId": policy.policy_id,
                "policyName": policy.policy_name,
                "tenantId": policy.tenant_id,
                "lifecycleState": policy.lifecycle_state,
                "providerFamily": policy.provider_family.as_str(),
                "sourceClass": "persisted_registry",
                "source": "provider_routing_policies",
                "fallbackOrder": policy.fallback_order,
                "providerWeights": policy.provider_weights,
                "scorecard": policy.scorecard,
                "metadata": policy.metadata,
            }),
            selected_provider_key,
        }
    }

    /// Get snapshot of all routing state for operator inspection.
    pub fn snapshot(&self) -> ProviderRoutingSnapshot {
        let source = if self.rules.is_empty() {
            "fallback"
        } else {
            "registry"
        }
        .to_string();
        ProviderRoutingSnapshot {
            action_mode: "policy_routed".to_string(),
            source: source.clone(),
            rules: self.rules.clone(),
            institutional_records: self.institutional_records.clone(),
            provenance: derive_snapshot_provenance(&self.rules, &source),
        }
    }

    /// List institutional compliance records, optionally filtered by entity type.
    pub fn list_institutional_records(
        &self,
        entity_type: Option<&str>,
    ) -> Vec<&InstitutionalComplianceRecord> {
        self.institutional_records
            .iter()
            .filter(|rec| entity_type.map_or(true, |et| rec.entity_type.eq_ignore_ascii_case(et)))
            .collect()
    }
}

fn primary_provider_key(policy: &ProviderRoutingPolicy) -> String {
    if let Some(first) = policy.fallback_order.first() {
        return first.clone();
    }

    policy
        .provider_weights
        .as_object()
        .and_then(|weights| {
            weights
                .iter()
                .filter_map(|(provider, weight)| weight.as_i64().map(|value| (provider, value)))
                .max_by_key(|(_, value)| *value)
                .map(|(provider, _)| provider.to_string())
        })
        .unwrap_or_else(|| format!("{}_unassigned", policy.provider_family.as_str()))
}

fn rule_from_policy(policy: ProviderRoutingPolicy) -> ProviderRoutingRule {
    ProviderRoutingRule {
        rule_id: policy.policy_id.clone(),
        priority: 0,
        corridor_code: policy.corridor_code.clone(),
        entity_type: policy.entity_type.clone(),
        risk_tier: policy.risk_tier.clone(),
        amount_min: policy.amount_min.map(|value| value.normalize().to_string()),
        amount_max: policy.amount_max.map(|value| value.normalize().to_string()),
        asset: policy.asset_code.clone(),
        partner_id: policy.partner_key.clone(),
        provider_key: primary_provider_key(&policy),
        provider_class: policy.provider_family.as_str().to_string(),
        enabled: policy.lifecycle_state.eq_ignore_ascii_case("active"),
        metadata: serde_json::json!({
            "policyId": policy.policy_id,
            "policyName": policy.policy_name,
            "tenantId": policy.tenant_id,
            "providerFamily": policy.provider_family.as_str(),
            "lifecycleState": policy.lifecycle_state,
            "sourceClass": "persisted_registry",
            "source": "provider_routing_policies",
            "fallbackOrder": policy.fallback_order,
            "providerWeights": policy.provider_weights,
            "scorecard": policy.scorecard,
            "metadata": policy.metadata,
        }),
    }
}

fn derive_rule_provenance(rule: &ProviderRoutingRule) -> serde_json::Value {
    let mut provenance = match rule.metadata.clone() {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    provenance
        .entry("policyId".to_string())
        .or_insert_with(|| serde_json::Value::String(rule.rule_id.clone()));
    provenance
        .entry("sourceClass".to_string())
        .or_insert_with(|| serde_json::Value::String("registry_rules".to_string()));
    provenance
        .entry("selectedProviderKey".to_string())
        .or_insert_with(|| serde_json::Value::String(rule.provider_key.clone()));
    provenance
        .entry("selectedProviderClass".to_string())
        .or_insert_with(|| serde_json::Value::String(rule.provider_class.clone()));
    serde_json::Value::Object(provenance)
}

fn derive_snapshot_provenance(rules: &[ProviderRoutingRule], source: &str) -> serde_json::Value {
    let source_class = rules
        .iter()
        .filter_map(|rule| {
            rule.metadata
                .get("sourceClass")
                .and_then(|value| value.as_str())
        })
        .next()
        .unwrap_or_else(|| {
            if source == "registry" {
                "registry_rules"
            } else {
                "bounded_fallback"
            }
        });
    let provider_families: BTreeSet<String> = rules
        .iter()
        .map(|rule| rule.provider_class.clone())
        .collect();

    serde_json::json!({
        "sourceClass": source_class,
        "policyCount": rules.len(),
        "providerFamilies": provider_families.into_iter().collect::<Vec<_>>(),
    })
}

fn matches_context(rule: &ProviderRoutingRule, context: &ProviderRoutingContext) -> bool {
    rule.corridor_code.as_ref().map_or(true, |code| {
        context.corridor_code.as_deref() == Some(code.as_str())
    }) && rule.entity_type.as_ref().map_or(true, |et| {
        context.entity_type.as_deref() == Some(et.as_str())
    }) && rule
        .risk_tier
        .as_ref()
        .map_or(true, |rt| context.risk_tier.as_deref() == Some(rt.as_str()))
        && rule
            .asset
            .as_ref()
            .map_or(true, |a| context.asset.as_deref() == Some(a.as_str()))
        && rule.partner_id.as_ref().map_or(true, |pid| {
            context.partner_id.as_deref() == Some(pid.as_str())
        })
        && matches_amount_bounds(rule, context)
}

/// Check if context amount falls within rule's amount_min/amount_max bounds.
/// If the context has no amount, amount-bounded rules do not match.
/// If the amounts cannot be parsed as Decimal, the bound is ignored.
fn matches_amount_bounds(rule: &ProviderRoutingRule, context: &ProviderRoutingContext) -> bool {
    let context_amount = match context
        .amount
        .as_deref()
        .and_then(|a| Decimal::from_str(a).ok())
    {
        Some(amt) => amt,
        None => {
            // No context amount: match only if rule has no amount bounds
            return rule.amount_min.is_none() && rule.amount_max.is_none();
        }
    };

    let min_ok = rule
        .amount_min
        .as_deref()
        .and_then(|m| Decimal::from_str(m).ok())
        .map_or(true, |min| context_amount >= min);

    let max_ok = rule
        .amount_max
        .as_deref()
        .and_then(|m| Decimal::from_str(m).ok())
        .map_or(true, |max| context_amount <= max);

    min_ok && max_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rules() -> Vec<ProviderRoutingRule> {
        vec![
            ProviderRoutingRule {
                rule_id: "rule_vn_kyc".to_string(),
                priority: 1,
                corridor_code: Some("USDT_VN_OFFRAMP".to_string()),
                entity_type: Some("individual".to_string()),
                risk_tier: None,
                amount_min: None,
                amount_max: None,
                asset: None,
                partner_id: None,
                provider_key: "onfido".to_string(),
                provider_class: "kyc".to_string(),
                enabled: true,
                metadata: serde_json::json!({}),
            },
            ProviderRoutingRule {
                rule_id: "rule_global_kyt".to_string(),
                priority: 10,
                corridor_code: None,
                entity_type: None,
                risk_tier: None,
                amount_min: None,
                amount_max: None,
                asset: None,
                partner_id: None,
                provider_key: "chainalysis".to_string(),
                provider_class: "kyt".to_string(),
                enabled: true,
                metadata: serde_json::json!({}),
            },
        ]
    }

    #[test]
    fn evaluate_matches_specific_corridor_rule() {
        let service = ProviderRoutingService::with_rules(sample_rules());
        let decision = service
            .evaluate(&ProviderRoutingContext {
                provider_family: None,
                corridor_code: Some("USDT_VN_OFFRAMP".to_string()),
                entity_type: Some("individual".to_string()),
                risk_tier: None,
                amount: None,
                asset: None,
                partner_id: None,
                tenant_id: None,
            })
            .expect("should evaluate");

        assert_eq!(decision.selected_provider_key, "onfido");
        assert_eq!(decision.matched_rule_id, "rule_vn_kyc");
        assert!(!decision.fallback_used);
    }

    #[test]
    fn evaluate_falls_back_to_global_rule() {
        let service = ProviderRoutingService::with_rules(sample_rules());
        let decision = service
            .evaluate(&ProviderRoutingContext {
                provider_family: None,
                corridor_code: Some("USDT_HK_OFFRAMP".to_string()),
                entity_type: Some("institution".to_string()),
                risk_tier: None,
                amount: None,
                asset: None,
                partner_id: None,
                tenant_id: None,
            })
            .expect("should evaluate");

        assert_eq!(decision.selected_provider_key, "chainalysis");
        assert_eq!(decision.matched_rule_id, "rule_global_kyt");
    }

    #[test]
    fn evaluate_uses_fallback_when_no_rules() {
        let service = ProviderRoutingService::new();
        let decision = service
            .evaluate(&ProviderRoutingContext {
                provider_family: None,
                corridor_code: None,
                entity_type: None,
                risk_tier: None,
                amount: None,
                asset: None,
                partner_id: None,
                tenant_id: None,
            })
            .expect("should evaluate");

        assert!(decision.fallback_used);
        assert_eq!(decision.selected_provider_key, "default_provider");
    }

    #[test]
    fn snapshot_shows_registry_source_with_rules() {
        let service = ProviderRoutingService::with_rules(sample_rules());
        let snap = service.snapshot();
        assert_eq!(snap.source, "registry");
        assert_eq!(snap.rules.len(), 2);
    }

    #[test]
    fn snapshot_shows_fallback_without_rules() {
        let service = ProviderRoutingService::new();
        let snap = service.snapshot();
        assert_eq!(snap.source, "fallback");
    }

    #[test]
    fn institutional_records_filter_by_entity_type() {
        let mut service = ProviderRoutingService::new();
        service.add_institutional_record(InstitutionalComplianceRecord {
            record_id: "rec_1".to_string(),
            entity_id: "entity_a".to_string(),
            entity_type: "individual".to_string(),
            trust_level: "verified".to_string(),
            ownership_verified: true,
            rescreening_due_at: None,
            travel_rule_status: Some("compliant".to_string()),
            review_state: "approved".to_string(),
            evidence: serde_json::json!({}),
            metadata: serde_json::json!({}),
        });
        service.add_institutional_record(InstitutionalComplianceRecord {
            record_id: "rec_2".to_string(),
            entity_id: "entity_b".to_string(),
            entity_type: "institution".to_string(),
            trust_level: "pending".to_string(),
            ownership_verified: false,
            rescreening_due_at: None,
            travel_rule_status: None,
            review_state: "pending".to_string(),
            evidence: serde_json::json!({}),
            metadata: serde_json::json!({}),
        });

        let individuals = service.list_institutional_records(Some("individual"));
        assert_eq!(individuals.len(), 1);
        assert_eq!(individuals[0].entity_id, "entity_a");

        let all = service.list_institutional_records(None);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn evaluate_respects_amount_bounds() {
        let rules = vec![ProviderRoutingRule {
            rule_id: "rule_high_value".to_string(),
            priority: 1,
            corridor_code: None,
            entity_type: None,
            risk_tier: None,
            amount_min: Some("10000".to_string()),
            amount_max: Some("100000".to_string()),
            asset: None,
            partner_id: None,
            provider_key: "enhanced_kyc".to_string(),
            provider_class: "kyc".to_string(),
            enabled: true,
            metadata: serde_json::json!({}),
        }];
        let service = ProviderRoutingService::with_rules(rules);

        // Within bounds: should match
        let in_bounds = service
            .evaluate(&ProviderRoutingContext {
                provider_family: None,
                corridor_code: None,
                entity_type: None,
                risk_tier: None,
                amount: Some("50000".to_string()),
                asset: None,
                partner_id: None,
                tenant_id: None,
            })
            .expect("should evaluate");
        assert_eq!(in_bounds.selected_provider_key, "enhanced_kyc");
        assert!(!in_bounds.fallback_used);

        // Below min: should NOT match, fallback
        let below = service
            .evaluate(&ProviderRoutingContext {
                provider_family: None,
                corridor_code: None,
                entity_type: None,
                risk_tier: None,
                amount: Some("5000".to_string()),
                asset: None,
                partner_id: None,
                tenant_id: None,
            })
            .expect("should evaluate");
        assert!(below.fallback_used);

        // Above max: should NOT match, fallback
        let above = service
            .evaluate(&ProviderRoutingContext {
                provider_family: None,
                corridor_code: None,
                entity_type: None,
                risk_tier: None,
                amount: Some("200000".to_string()),
                asset: None,
                partner_id: None,
                tenant_id: None,
            })
            .expect("should evaluate");
        assert!(above.fallback_used);
    }

    #[test]
    fn amount_bounded_rule_skipped_when_no_context_amount() {
        let rules = vec![ProviderRoutingRule {
            rule_id: "rule_bounded".to_string(),
            priority: 1,
            corridor_code: None,
            entity_type: None,
            risk_tier: None,
            amount_min: Some("100".to_string()),
            amount_max: None,
            asset: None,
            partner_id: None,
            provider_key: "bounded_provider".to_string(),
            provider_class: "kyc".to_string(),
            enabled: true,
            metadata: serde_json::json!({}),
        }];
        let service = ProviderRoutingService::with_rules(rules);

        // No amount in context: amount-bounded rule should NOT match
        let decision = service
            .evaluate(&ProviderRoutingContext {
                provider_family: None,
                corridor_code: None,
                entity_type: None,
                risk_tier: None,
                amount: None,
                asset: None,
                partner_id: None,
                tenant_id: None,
            })
            .expect("should evaluate");
        assert!(decision.fallback_used);
    }

    #[test]
    fn evaluate_exposes_matched_rule_provenance() {
        let service = ProviderRoutingService::with_rules(vec![ProviderRoutingRule {
            rule_id: "provider_policy_vn_sg".to_string(),
            priority: 1,
            corridor_code: Some("VN_SG_PAYOUT".to_string()),
            entity_type: Some("business".to_string()),
            risk_tier: Some("high".to_string()),
            amount_min: Some("100".to_string()),
            amount_max: Some("5000".to_string()),
            asset: Some("USDT".to_string()),
            partner_id: Some("partner_scb".to_string()),
            provider_key: "notabene".to_string(),
            provider_class: "travel_rule".to_string(),
            enabled: true,
            metadata: serde_json::json!({
                "policyId": "provider_policy_vn_sg",
                "tenantId": "tenant_provider_policy",
                "sourceClass": "persisted_registry",
                "lifecycleState": "active",
            }),
        }]);

        let decision = service
            .evaluate(&ProviderRoutingContext {
                provider_family: None,
                corridor_code: Some("VN_SG_PAYOUT".to_string()),
                entity_type: Some("business".to_string()),
                risk_tier: Some("high".to_string()),
                amount: Some("1000".to_string()),
                asset: Some("USDT".to_string()),
                partner_id: Some("partner_scb".to_string()),
                tenant_id: Some("tenant_provider_policy".to_string()),
            })
            .expect("should evaluate");
        let payload = serde_json::to_value(decision).expect("decision should serialize");

        assert_eq!(
            payload["provenance"]["policyId"], "provider_policy_vn_sg",
            "matched decision must expose authoritative persisted provenance"
        );
        assert_eq!(payload["provenance"]["sourceClass"], "persisted_registry");
    }

    #[test]
    fn snapshot_exposes_top_level_provenance_for_registry_rules() {
        let service = ProviderRoutingService::with_rules(vec![ProviderRoutingRule {
            rule_id: "provider_policy_vn_sg".to_string(),
            priority: 1,
            corridor_code: Some("VN_SG_PAYOUT".to_string()),
            entity_type: Some("business".to_string()),
            risk_tier: Some("high".to_string()),
            amount_min: Some("100".to_string()),
            amount_max: Some("5000".to_string()),
            asset: Some("USDT".to_string()),
            partner_id: Some("partner_scb".to_string()),
            provider_key: "notabene".to_string(),
            provider_class: "travel_rule".to_string(),
            enabled: true,
            metadata: serde_json::json!({
                "policyId": "provider_policy_vn_sg",
                "tenantId": "tenant_provider_policy",
                "sourceClass": "persisted_registry",
                "providerFamily": "travel_rule",
            }),
        }]);

        let payload = serde_json::to_value(service.snapshot()).expect("snapshot should serialize");

        assert_eq!(payload["provenance"]["sourceClass"], "persisted_registry");
        assert_eq!(payload["provenance"]["policyCount"], 1);
    }
}
