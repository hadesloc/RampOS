//! Commercial Readiness Service
//!
//! Bounded control-plane extensions for stablecoin-account operations,
//! card-adjacent distribution, and related commercialization patterns.
//! All extensions are disabled by default and policy-bounded (FR-018).

use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::repository::{
    ApprovalReferenceRecord, CorridorPackRecord, CorridorPackRepository, PartnerRegistryRecord,
    PartnerRegistryRepository, PgCorridorPackRepository, PgPartnerRegistryRepository,
};

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
    pub provenance: serde_json::Value,
}

/// Service managing commercial readiness extensions.
#[derive(Clone)]
pub struct CommercialReadinessService {
    extensions: Vec<CommercialExtensionRecord>,
}

impl CommercialReadinessService {
    pub fn new() -> Self {
        Self::with_extensions(fallback_extensions())
    }

    pub fn with_extensions(extensions: Vec<CommercialExtensionRecord>) -> Self {
        Self { extensions }
    }

    pub async fn from_pool(pool: PgPool) -> Result<Self> {
        Self::from_pool_for_tenant(pool, None).await
    }

    pub async fn from_pool_for_tenant(pool: PgPool, tenant_id: Option<&str>) -> Result<Self> {
        let partner_repo = PgPartnerRegistryRepository::new(pool.clone());
        let corridor_repo = PgCorridorPackRepository::new(pool.clone());
        let partners = partner_repo.list_registry_records(tenant_id).await?;
        let corridors = corridor_repo.list_corridor_packs(tenant_id).await?;
        let approvals = load_approval_references(&pool, tenant_id).await?;

        if partners.is_empty() && corridors.is_empty() && approvals.is_empty() {
            return Ok(Self::new());
        }

        Ok(Self::with_extensions(build_governed_extensions(
            &partners, &corridors, &approvals,
        )))
    }

    /// Return a snapshot of all registered extensions for operator inspection.
    pub fn snapshot(&self) -> CommercialReadinessSnapshot {
        let enabled_count = self.extensions.iter().filter(|e| e.enabled).count();
        let disabled_count = self.extensions.len() - enabled_count;
        let source = if !self.extensions.is_empty()
            && self.extensions.iter().all(|extension| {
                extension.metadata["sourceClass"]
                    .as_str()
                    .map(|value| value == "bounded_fallback")
                    .unwrap_or(false)
            }) {
            "fallback"
        } else {
            "registry"
        }
        .to_string();
        CommercialReadinessSnapshot {
            action_mode: "commercial_readiness".to_string(),
            source: source.clone(),
            extensions: self.extensions.clone(),
            enabled_count,
            disabled_count,
            provenance: serde_json::json!({
                "sourceClass": if source == "registry" {
                    "governed_registry"
                } else {
                    "bounded_fallback"
                },
                "extensionCount": self.extensions.len(),
                "enabledCount": enabled_count,
            }),
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

        let can_enable = missing_capabilities.is_empty()
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
            provenance: extension.metadata.clone(),
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

impl Default for CommercialReadinessService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, FromRow)]
struct ApprovalReferenceRow {
    id: String,
    tenant_id: Option<String>,
    action_class: String,
    status: String,
    metadata: serde_json::Value,
}

async fn load_approval_references(
    pool: &PgPool,
    tenant_id: Option<&str>,
) -> Result<Vec<ApprovalReferenceRecord>> {
    let rows = if let Some(tenant_id) = tenant_id {
        sqlx::query_as::<_, ApprovalReferenceRow>(
            r#"
            SELECT id, tenant_id, action_class, status, metadata
            FROM partner_approval_references
            WHERE status = 'approved'
              AND (tenant_id = $1 OR tenant_id IS NULL)
            ORDER BY created_at DESC, id ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?
    } else {
        sqlx::query_as::<_, ApprovalReferenceRow>(
            r#"
            SELECT id, tenant_id, action_class, status, metadata
            FROM partner_approval_references
            WHERE status = 'approved'
            ORDER BY created_at DESC, id ASC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?
    };

    Ok(rows
        .into_iter()
        .map(|row| ApprovalReferenceRecord {
            approval_reference_id: row.id,
            tenant_id: row.tenant_id,
            action_class: row.action_class,
            status: row.status,
            metadata: row.metadata,
        })
        .collect())
}

fn build_governed_extensions(
    partners: &[PartnerRegistryRecord],
    corridors: &[CorridorPackRecord],
    approvals: &[ApprovalReferenceRecord],
) -> Vec<CommercialExtensionRecord> {
    let has_custody_capability = partners.iter().any(|partner| {
        partner.lifecycle_state.eq_ignore_ascii_case("active")
            && partner.approval_status.eq_ignore_ascii_case("approved")
            && partner.capabilities.iter().any(|capability| {
                capability.capability_family.eq_ignore_ascii_case("custody")
                    && capability.approval_status.eq_ignore_ascii_case("approved")
            })
    });
    let has_card_issuing_capability = partners.iter().any(|partner| {
        partner.lifecycle_state.eq_ignore_ascii_case("active")
            && partner.approval_status.eq_ignore_ascii_case("approved")
            && partner.capabilities.iter().any(|capability| {
                capability
                    .capability_family
                    .eq_ignore_ascii_case("card_issuing")
                    && capability.approval_status.eq_ignore_ascii_case("approved")
            })
    });
    let stablecoin_corridor = corridors.iter().find(|corridor| {
        corridor.corridor_code == "USDT_VN_SETTLEMENT"
            && corridor.lifecycle_state.eq_ignore_ascii_case("active")
            && corridor.rollout_state.eq_ignore_ascii_case("active")
    });
    let stablecoin_approval = approvals
        .iter()
        .find(|approval| {
            approval
                .action_class
                .eq_ignore_ascii_case("commercial_readiness")
        })
        .map(|approval| approval.approval_reference_id.clone());

    let stablecoin_enabled = has_custody_capability
        && stablecoin_corridor.is_some()
        && stablecoin_approval.is_some()
        && stablecoin_corridor.is_some_and(|corridor| {
            corridor
                .compliance_hooks
                .iter()
                .any(|hook| hook.required && hook.hook_kind.eq_ignore_ascii_case("kyc_verified"))
        });

    vec![
        CommercialExtensionRecord {
            extension_id: "stablecoin_account_ops".to_string(),
            extension_kind: "stablecoin_account".to_string(),
            label: "Stablecoin Account Operations".to_string(),
            description: "Enables stablecoin balance and transfer ops".to_string(),
            enabled: stablecoin_enabled,
            approval_reference: stablecoin_approval.clone(),
            required_partner_capabilities: vec!["custody".to_string()],
            required_corridor_packs: vec!["USDT_VN_SETTLEMENT".to_string()],
            required_compliance_checks: vec!["kyc_verified".to_string()],
            metadata: serde_json::json!({
                "sourceClass": "governed_registry",
                "requiredCapabilityPresent": has_custody_capability,
                "requiredCorridorPresent": stablecoin_corridor.is_some(),
                "requiredApprovalPresent": stablecoin_approval.is_some(),
            }),
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
            metadata: serde_json::json!({
                "sourceClass": "governed_registry",
                "requiredCapabilityPresent": has_card_issuing_capability,
                "requiredApprovalPresent": false,
            }),
        },
    ]
}

fn fallback_extensions() -> Vec<CommercialExtensionRecord> {
    vec![
        CommercialExtensionRecord {
            extension_id: "stablecoin_account_ops".to_string(),
            extension_kind: "stablecoin_account".to_string(),
            label: "Stablecoin Account Operations".to_string(),
            description: "Enables stablecoin balance and transfer ops".to_string(),
            enabled: false,
            approval_reference: Some("bounded_fallback_commercial_readiness".to_string()),
            required_partner_capabilities: vec!["custody".to_string()],
            required_corridor_packs: vec!["USDT_VN_SETTLEMENT".to_string()],
            required_compliance_checks: vec!["kyc_verified".to_string()],
            metadata: serde_json::json!({
                "sourceClass": "bounded_fallback",
                "reason": "no_governed_records",
            }),
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
            metadata: serde_json::json!({
                "sourceClass": "bounded_fallback",
                "reason": "no_governed_records",
            }),
        },
    ]
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
    pub provenance: serde_json::Value,
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
            .check_enablement_conditions("card_payout", &["card_issuing".to_string()], &[], &[])
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

    #[test]
    fn fallback_snapshot_exposes_bounded_catalog_and_provenance() {
        let service = CommercialReadinessService::new();
        let snap = service.snapshot();

        assert_eq!(snap.source, "fallback");
        assert!(
            !snap.extensions.is_empty(),
            "fallback mode must still expose the bounded commercial-readiness catalog"
        );
        assert_eq!(snap.provenance["sourceClass"], "bounded_fallback");
    }

    #[test]
    fn fallback_enablement_check_surfaces_missing_prerequisites() {
        let service = CommercialReadinessService::new();
        let result = service
            .check_enablement_conditions("stablecoin_account_ops", &[], &[], &[])
            .expect("fallback catalog should still support bounded readiness checks");

        assert!(!result.can_enable);
        assert!(result.has_approval);
        assert_eq!(result.missing_capabilities, vec!["custody"]);
        assert_eq!(result.missing_corridors, vec!["USDT_VN_SETTLEMENT"]);
        assert_eq!(result.missing_compliance, vec!["kyc_verified"]);
        assert_eq!(result.provenance["sourceClass"], "bounded_fallback");
    }
}
