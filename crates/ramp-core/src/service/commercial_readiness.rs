//! Commercial Readiness Service
//!
//! Bounded control-plane extensions for stablecoin-account operations,
//! card-adjacent distribution, and related commercialization patterns.
//! All extensions are disabled by default and policy-bounded (FR-018).

use ramp_common::Result;
use serde::{Deserialize, Serialize};

/// A commercial extension record — represents a bounded, policy-gated capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercialExtensionRecord {
    pub extension_id: String,
    pub extension_kind: String,
    pub label: String,
    pub description: String,
    /// Extensions are disabled by default until partner + policy + compliance records allow it.
    pub enabled: bool,
    /// Must have an approval reference to activate.
    pub approval_reference: Option<String>,
    /// Required partner capabilities before enablement.
    pub required_partner_capabilities: Vec<String>,
    /// Required corridor packs before enablement.
    pub required_corridor_packs: Vec<String>,
    /// Required compliance checks before enablement.
    pub required_compliance_checks: Vec<String>,
    pub metadata: serde_json::Value,
}

/// Snapshot of commercial readiness state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercialReadinessSnapshot {
    pub action_mode: String,
    pub source: String,
    pub extensions: Vec<CommercialExtensionRecord>,
    pub enabled_count: usize,
    pub disabled_count: usize,
}

/// Service managing commercial readiness extensions.
#[derive(Clone, Default)]
pub struct CommercialReadinessService {
    extensions: Vec<CommercialExtensionRecord>,
}

impl CommercialReadinessService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_extensions(extensions: Vec<CommercialExtensionRecord>) -> Self {
        Self { extensions }
    }

    /// Return a snapshot of all registered extensions for operator inspection.
    pub fn snapshot(&self) -> CommercialReadinessSnapshot {
        let enabled_count = self.extensions.iter().filter(|e| e.enabled).count();
        let disabled_count = self.extensions.len() - enabled_count;
        CommercialReadinessSnapshot {
            action_mode: "commercial_readiness".to_string(),
            source: if self.extensions.is_empty() {
                "fallback"
            } else {
                "registry"
            }
            .to_string(),
            extensions: self.extensions.clone(),
            enabled_count,
            disabled_count,
        }
    }

    /// Check if a specific extension can be enabled based on its preconditions.
    pub fn check_enablement_conditions(
        &self,
        extension_id: &str,
        available_partner_capabilities: &[String],
        available_corridor_packs: &[String],
        compliance_checks_passed: &[String],
    ) -> Result<EnablementCheckResult> {
        let extension = self
            .extensions
            .iter()
            .find(|e| e.extension_id == extension_id)
            .ok_or_else(|| {
                ramp_common::Error::NotFound(format!("Extension '{}' not found", extension_id))
            })?;

        let missing_capabilities: Vec<String> = extension
            .required_partner_capabilities
            .iter()
            .filter(|cap| !available_partner_capabilities.contains(cap))
            .cloned()
            .collect();

        let missing_corridors: Vec<String> = extension
            .required_corridor_packs
            .iter()
            .filter(|cp| !available_corridor_packs.contains(cp))
            .cloned()
            .collect();

        let missing_compliance: Vec<String> = extension
            .required_compliance_checks
            .iter()
            .filter(|cc| !compliance_checks_passed.contains(cc))
            .cloned()
            .collect();

        let can_enable =
            missing_capabilities.is_empty()
                && missing_corridors.is_empty()
                && missing_compliance.is_empty()
                && extension.approval_reference.is_some();

        Ok(EnablementCheckResult {
            extension_id: extension_id.to_string(),
            can_enable,
            has_approval: extension.approval_reference.is_some(),
            missing_capabilities,
            missing_corridors,
            missing_compliance,
        })
    }

    /// List extensions filtered by kind (e.g. "stablecoin_account", "card_adjacent").
    pub fn list_by_kind(&self, kind: &str) -> Vec<&CommercialExtensionRecord> {
        self.extensions
            .iter()
            .filter(|e| e.extension_kind.eq_ignore_ascii_case(kind))
            .collect()
    }
}

/// Result of checking enablement conditions for a commercial extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnablementCheckResult {
    pub extension_id: String,
    pub can_enable: bool,
    pub has_approval: bool,
    pub missing_capabilities: Vec<String>,
    pub missing_corridors: Vec<String>,
    pub missing_compliance: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_extensions() -> Vec<CommercialExtensionRecord> {
        vec![
            CommercialExtensionRecord {
                extension_id: "stablecoin_account_ops".to_string(),
                extension_kind: "stablecoin_account".to_string(),
                label: "Stablecoin Account Operations".to_string(),
                description: "Enables stablecoin balance and transfer ops".to_string(),
                enabled: false,
                approval_reference: Some("approval_stablecoin_001".to_string()),
                required_partner_capabilities: vec!["custody".to_string()],
                required_corridor_packs: vec!["USDT_VN_SETTLEMENT".to_string()],
                required_compliance_checks: vec!["kyc_verified".to_string()],
                metadata: serde_json::json!({}),
            },
            CommercialExtensionRecord {
                extension_id: "card_payout".to_string(),
                extension_kind: "card_adjacent".to_string(),
                label: "Card-Adjacent Payout".to_string(),
                description: "Enables card-funded distribution lanes".to_string(),
                enabled: false,
                approval_reference: None,
                required_partner_capabilities: vec!["card_issuing".to_string()],
                required_corridor_packs: vec![],
                required_compliance_checks: vec![],
                metadata: serde_json::json!({}),
            },
        ]
    }

    #[test]
    fn snapshot_shows_disabled_extensions() {
        let service = CommercialReadinessService::with_extensions(sample_extensions());
        let snap = service.snapshot();
        assert_eq!(snap.extensions.len(), 2);
        assert_eq!(snap.enabled_count, 0);
        assert_eq!(snap.disabled_count, 2);
        assert_eq!(snap.source, "registry");
    }

    #[test]
    fn enablement_check_all_conditions_met() {
        let service = CommercialReadinessService::with_extensions(sample_extensions());
        let result = service
            .check_enablement_conditions(
                "stablecoin_account_ops",
                &["custody".to_string()],
                &["USDT_VN_SETTLEMENT".to_string()],
                &["kyc_verified".to_string()],
            )
            .expect("check should succeed");

        assert!(result.can_enable);
        assert!(result.has_approval);
        assert!(result.missing_capabilities.is_empty());
    }

    #[test]
    fn enablement_check_missing_capability() {
        let service = CommercialReadinessService::with_extensions(sample_extensions());
        let result = service
            .check_enablement_conditions(
                "stablecoin_account_ops",
                &[],
                &["USDT_VN_SETTLEMENT".to_string()],
                &["kyc_verified".to_string()],
            )
            .expect("check should succeed");

        assert!(!result.can_enable);
        assert_eq!(result.missing_capabilities, vec!["custody"]);
    }

    #[test]
    fn enablement_check_no_approval_cannot_enable() {
        let service = CommercialReadinessService::with_extensions(sample_extensions());
        let result = service
            .check_enablement_conditions(
                "card_payout",
                &["card_issuing".to_string()],
                &[],
                &[],
            )
            .expect("check should succeed");

        assert!(!result.can_enable);
        assert!(!result.has_approval);
    }

    #[test]
    fn list_by_kind_filters_correctly() {
        let service = CommercialReadinessService::with_extensions(sample_extensions());
        let stablecoin = service.list_by_kind("stablecoin_account");
        assert_eq!(stablecoin.len(), 1);
        let card = service.list_by_kind("card_adjacent");
        assert_eq!(card.len(), 1);
        let unknown = service.list_by_kind("unknown");
        assert!(unknown.is_empty());
    }
}
