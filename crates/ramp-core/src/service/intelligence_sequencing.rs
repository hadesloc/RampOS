//! Intelligence Sequencing Service
//!
//! Sequences intelligence work (Risk Lab evolution, incident intelligence,
//! SLA intelligence, recommendation systems) after runtime truth so intelligence
//! remains bounded, auditable, and operator-supporting.

use ramp_common::Result;
use serde::{Deserialize, Serialize};

/// An intelligence work package with explicit sequencing and guardrails.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceWorkPackage {
    pub package_id: String,
    pub domain: String,
    pub label: String,
    pub description: String,
    /// Sequencing phase — intelligence work should be sequenced after runtime truth.
    pub phase: IntelligencePhase,
    /// Work packages that must be completed first.
    pub depends_on: Vec<String>,
    /// Whether operator controls are enforced for this package.
    pub operator_controlled: bool,
    /// Whether outputs are auditable.
    pub auditable: bool,
    /// Whether the work package is active.
    pub active: bool,
    pub metadata: serde_json::Value,
}

/// Sequencing phase for intelligence work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntelligencePhase {
    /// Can start only after runtime truth baseline is established.
    PostRuntimeTruth,
    /// Can start only after partner and compliance governance is ready.
    PostGovernance,
    /// Can start only after commercial readiness baseline.
    PostCommercialReadiness,
}

/// Guardrail applied to intelligence features.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceGuardrail {
    pub guardrail_id: String,
    pub label: String,
    pub rule: String,
    pub enforced: bool,
    /// Whether this guardrail's conditions are currently satisfied.
    /// An enforced guardrail that is NOT satisfied blocks the package.
    pub satisfied: bool,
    pub applies_to: Vec<String>,
}

/// Snapshot of intelligence sequencing state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceSequencingSnapshot {
    pub action_mode: String,
    pub source: String,
    pub work_packages: Vec<IntelligenceWorkPackage>,
    pub guardrails: Vec<IntelligenceGuardrail>,
    pub active_count: usize,
    pub blocked_count: usize,
}

/// Service for managing intelligence sequencing.
#[derive(Clone, Default)]
pub struct IntelligenceSequencingService {
    packages: Vec<IntelligenceWorkPackage>,
    guardrails: Vec<IntelligenceGuardrail>,
}

impl IntelligenceSequencingService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_packages(
        packages: Vec<IntelligenceWorkPackage>,
        guardrails: Vec<IntelligenceGuardrail>,
    ) -> Self {
        Self {
            packages,
            guardrails,
        }
    }

    /// Get a snapshot of all sequencing state for operator inspection.
    pub fn snapshot(&self) -> IntelligenceSequencingSnapshot {
        let active_count = self.packages.iter().filter(|p| p.active).count();
        let blocked_count = self.packages.len() - active_count;
        IntelligenceSequencingSnapshot {
            action_mode: "intelligence_sequencing".to_string(),
            source: if self.packages.is_empty() {
                "fallback"
            } else {
                "registry"
            }
            .to_string(),
            work_packages: self.packages.clone(),
            guardrails: self.guardrails.clone(),
            active_count,
            blocked_count,
        }
    }

    /// Check if a work package's dependencies are all satisfied.
    pub fn check_dependencies(
        &self,
        package_id: &str,
        completed_packages: &[String],
    ) -> Result<DependencyCheckResult> {
        let package = self
            .packages
            .iter()
            .find(|p| p.package_id == package_id)
            .ok_or_else(|| {
                ramp_common::Error::NotFound(format!(
                    "Intelligence work package '{}' not found",
                    package_id
                ))
            })?;

        let missing: Vec<String> = package
            .depends_on
            .iter()
            .filter(|dep| !completed_packages.contains(dep))
            .cloned()
            .collect();

        let guardrail_violations: Vec<String> = self
            .guardrails
            .iter()
            .filter(|g| {
                g.enforced
                    && !g.satisfied
                    && g.applies_to.contains(&package_id.to_string())
            })
            .map(|g| format!("{}: {}", g.guardrail_id, g.rule))
            .collect();

        Ok(DependencyCheckResult {
            package_id: package_id.to_string(),
            can_start: missing.is_empty() && guardrail_violations.is_empty(),
            missing_dependencies: missing,
            guardrail_violations,
            operator_controlled: package.operator_controlled,
        })
    }

    /// List work packages by domain (e.g. "risk_lab", "incident", "sla", "recommendation").
    pub fn list_by_domain(&self, domain: &str) -> Vec<&IntelligenceWorkPackage> {
        self.packages
            .iter()
            .filter(|p| p.domain.eq_ignore_ascii_case(domain))
            .collect()
    }

    /// Verify that no intelligence work bypasses operator controls.
    pub fn verify_operator_control_enforcement(&self) -> Vec<String> {
        self.packages
            .iter()
            .filter(|p| p.active && !p.operator_controlled)
            .map(|p| {
                format!(
                    "Package '{}' ({}) is active but NOT operator-controlled",
                    p.package_id, p.label
                )
            })
            .collect()
    }
}

/// Result of checking whether a work package can start.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyCheckResult {
    pub package_id: String,
    pub can_start: bool,
    pub missing_dependencies: Vec<String>,
    pub guardrail_violations: Vec<String>,
    pub operator_controlled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_packages() -> Vec<IntelligenceWorkPackage> {
        vec![
            IntelligenceWorkPackage {
                package_id: "risk_lab_v2".to_string(),
                domain: "risk_lab".to_string(),
                label: "Risk Lab Evolution".to_string(),
                description: "Evolve Risk Lab with replay-based learning".to_string(),
                phase: IntelligencePhase::PostRuntimeTruth,
                depends_on: vec!["runtime_truth_baseline".to_string()],
                operator_controlled: true,
                auditable: true,
                active: false,
                metadata: serde_json::json!({}),
            },
            IntelligenceWorkPackage {
                package_id: "incident_intelligence".to_string(),
                domain: "incident".to_string(),
                label: "Incident Intelligence".to_string(),
                description: "Pattern detection from incident timelines".to_string(),
                phase: IntelligencePhase::PostGovernance,
                depends_on: vec![
                    "runtime_truth_baseline".to_string(),
                    "partner_governance".to_string(),
                ],
                operator_controlled: true,
                auditable: true,
                active: false,
                metadata: serde_json::json!({}),
            },
            IntelligenceWorkPackage {
                package_id: "sla_intelligence".to_string(),
                domain: "sla".to_string(),
                label: "SLA Intelligence".to_string(),
                description: "Predictive SLA alerting".to_string(),
                phase: IntelligencePhase::PostRuntimeTruth,
                depends_on: vec!["runtime_truth_baseline".to_string()],
                operator_controlled: true,
                auditable: true,
                active: true,
                metadata: serde_json::json!({}),
            },
        ]
    }

    fn sample_guardrails() -> Vec<IntelligenceGuardrail> {
        vec![IntelligenceGuardrail {
            guardrail_id: "no_autonomous_actions".to_string(),
            label: "No Autonomous Actions".to_string(),
            rule: "Intelligence outputs must stay recommendation-only".to_string(),
            enforced: true,
            satisfied: false, // Not yet satisfied — blocks applicable packages
            applies_to: vec![
                "risk_lab_v2".to_string(),
                "incident_intelligence".to_string(),
            ],
        }]
    }

    #[test]
    fn snapshot_reports_active_and_blocked() {
        let service =
            IntelligenceSequencingService::with_packages(sample_packages(), sample_guardrails());
        let snap = service.snapshot();
        assert_eq!(snap.active_count, 1);
        assert_eq!(snap.blocked_count, 2);
        assert_eq!(snap.guardrails.len(), 1);
    }

    #[test]
    fn check_dependencies_with_all_met() {
        let service =
            IntelligenceSequencingService::with_packages(sample_packages(), Vec::new());
        let result = service
            .check_dependencies(
                "risk_lab_v2",
                &["runtime_truth_baseline".to_string()],
            )
            .expect("check should succeed");

        assert!(result.can_start);
        assert!(result.missing_dependencies.is_empty());
    }

    #[test]
    fn check_dependencies_with_missing() {
        let service =
            IntelligenceSequencingService::with_packages(sample_packages(), Vec::new());
        let result = service
            .check_dependencies("incident_intelligence", &["runtime_truth_baseline".to_string()])
            .expect("check should succeed");

        assert!(!result.can_start);
        assert_eq!(result.missing_dependencies, vec!["partner_governance"]);
    }

    #[test]
    fn check_dependencies_with_guardrail_violation() {
        let service =
            IntelligenceSequencingService::with_packages(sample_packages(), sample_guardrails());
        let result = service
            .check_dependencies(
                "risk_lab_v2",
                &["runtime_truth_baseline".to_string()],
            )
            .expect("check should succeed");

        // Guardrail applies and is enforced
        assert!(!result.can_start);
        assert!(!result.guardrail_violations.is_empty());
    }

    #[test]
    fn list_by_domain_filters() {
        let service =
            IntelligenceSequencingService::with_packages(sample_packages(), Vec::new());
        assert_eq!(service.list_by_domain("risk_lab").len(), 1);
        assert_eq!(service.list_by_domain("incident").len(), 1);
        assert_eq!(service.list_by_domain("sla").len(), 1);
        assert!(service.list_by_domain("unknown").is_empty());
    }

    #[test]
    fn verify_operator_control_all_controlled() {
        let service =
            IntelligenceSequencingService::with_packages(sample_packages(), Vec::new());
        let violations = service.verify_operator_control_enforcement();
        assert!(violations.is_empty());
    }
}
