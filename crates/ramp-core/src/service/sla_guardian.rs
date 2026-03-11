use serde::{Deserialize, Serialize};

use crate::service::{
    IncidentActionMode, IncidentRecommendationPriority, IncidentSignalSnapshot, IncidentTimeline,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SlaGuardianOwnerLane {
    Webhooks,
    SettlementOps,
    ReconciliationOps,
    RiskOps,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SlaGuardianRiskLevel {
    Low,
    Elevated,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SlaGuardianAlert {
    pub code: String,
    pub owner_lane: SlaGuardianOwnerLane,
    pub priority: IncidentRecommendationPriority,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SlaGuardianSnapshot {
    pub action_mode: IncidentActionMode,
    pub predicted_breach_risk: SlaGuardianRiskLevel,
    pub alert_count: usize,
    pub owner_lanes: Vec<SlaGuardianOwnerLane>,
    pub alerts: Vec<SlaGuardianAlert>,
}

pub struct SlaGuardianService;

impl SlaGuardianService {
    pub fn new() -> Self {
        Self
    }

    pub fn summarize(
        &self,
        timeline: &IncidentTimeline,
        signals: IncidentSignalSnapshot,
    ) -> SlaGuardianSnapshot {
        let mut owner_lanes = Vec::new();
        let alerts: Vec<_> = timeline
            .recommendations
            .iter()
            .map(|recommendation| {
                let owner_lane = owner_lane_for_code(&recommendation.code);
                if !owner_lanes.contains(&owner_lane) {
                    owner_lanes.push(owner_lane.clone());
                }
                SlaGuardianAlert {
                    code: recommendation.code.clone(),
                    owner_lane,
                    priority: recommendation.priority.clone(),
                    summary: recommendation.summary.clone(),
                }
            })
            .collect();

        SlaGuardianSnapshot {
            action_mode: timeline.action_mode.clone(),
            predicted_breach_risk: derive_risk_level(&alerts, &signals),
            alert_count: alerts.len(),
            owner_lanes,
            alerts,
        }
    }
}

impl Default for SlaGuardianService {
    fn default() -> Self {
        Self::new()
    }
}

fn owner_lane_for_code(code: &str) -> SlaGuardianOwnerLane {
    match code {
        "review_webhook_delivery" => SlaGuardianOwnerLane::Webhooks,
        "review_settlement_state" => SlaGuardianOwnerLane::SettlementOps,
        "inspect_reconciliation_mismatch" => SlaGuardianOwnerLane::ReconciliationOps,
        "keep_risk_review_in_loop" => SlaGuardianOwnerLane::RiskOps,
        _ => SlaGuardianOwnerLane::RiskOps,
    }
}

fn derive_risk_level(
    alerts: &[SlaGuardianAlert],
    signals: &IncidentSignalSnapshot,
) -> SlaGuardianRiskLevel {
    let has_immediate = alerts
        .iter()
        .any(|alert| alert.priority == IncidentRecommendationPriority::Immediate);

    if signals.critical_fraud_signals > 0
        || alerts
            .iter()
            .any(|alert| alert.owner_lane == SlaGuardianOwnerLane::RiskOps)
    {
        SlaGuardianRiskLevel::Critical
    } else if has_immediate
        || signals.failed_settlements > 0
        || signals.failed_webhooks > 0
    {
        SlaGuardianRiskLevel::High
    } else if !alerts.is_empty() || signals.processing_settlements > 0 {
        SlaGuardianRiskLevel::Elevated
    } else {
        SlaGuardianRiskLevel::Low
    }
}
