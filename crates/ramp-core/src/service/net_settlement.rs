use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::service::settlement::{Settlement, SettlementStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetSettlementProposalStatus {
    Draft,
    PendingApproval,
    Approved,
    Rejected,
    Executed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetSettlementProposal {
    pub id: String,
    pub tenant_id: String,
    pub counterparty_id: String,
    pub asset: String,
    pub settlement_ids: Vec<String>,
    pub gross_in: String,
    pub gross_out: String,
    pub net_amount: String,
    pub direction: String,
    pub status: String,
    pub approval_required: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetSettlementAlert {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetSettlementWorkbenchSnapshot {
    pub generated_at: String,
    pub approval_mode: String,
    pub action_mode: String,
    pub proposals: Vec<NetSettlementProposal>,
    pub alerts: Vec<NetSettlementAlert>,
}

pub struct NetSettlementService;

impl NetSettlementService {
    pub fn new() -> Self {
        Self
    }

    pub fn build_workbench(
        &self,
        scenario: Option<&str>,
    ) -> NetSettlementWorkbenchSnapshot {
        let proposals = sample_proposals(scenario);
        let alerts = build_alerts(&proposals);

        NetSettlementWorkbenchSnapshot {
            generated_at: Utc::now().to_rfc3339(),
            approval_mode: "manual_approval".to_string(),
            action_mode: "approval_gated".to_string(),
            proposals,
            alerts,
        }
    }

    pub fn build_bilateral_proposals(
        &self,
        tenant_id: &str,
        counterparty_id: &str,
        asset: &str,
        settlements: &[Settlement],
    ) -> Vec<NetSettlementProposal> {
        let mut incoming = Decimal::ZERO;
        let mut outgoing = Decimal::ZERO;
        let mut ids = Vec::new();

        for (index, settlement) in settlements.iter().enumerate() {
            ids.push(settlement.id.clone());
            let amount = Decimal::from(100 + (index as i64 * 25));
            if index % 2 == 0 {
                outgoing += amount;
            } else {
                incoming += amount;
            }
        }

        let net = incoming - outgoing;
        if ids.is_empty() {
            return Vec::new();
        }

        vec![NetSettlementProposal {
            id: format!("nsp_{}_{}", counterparty_id, asset.to_ascii_lowercase()),
            tenant_id: tenant_id.to_string(),
            counterparty_id: counterparty_id.to_string(),
            asset: asset.to_string(),
            settlement_ids: ids,
            gross_in: decimal_to_string(incoming),
            gross_out: decimal_to_string(outgoing),
            net_amount: decimal_to_string(net.abs()),
            direction: if net >= Decimal::ZERO {
                "receive".to_string()
            } else {
                "pay".to_string()
            },
            status: proposal_status_label(NetSettlementProposalStatus::PendingApproval),
            approval_required: true,
            summary: format!(
                "Bilateral net settlement against {} leaves a {} {} position awaiting approval.",
                counterparty_id,
                decimal_to_string(net.abs()),
                asset
            ),
        }]
    }
}

impl Default for NetSettlementService {
    fn default() -> Self {
        Self::new()
    }
}

fn sample_proposals(scenario: Option<&str>) -> Vec<NetSettlementProposal> {
    if matches!(scenario, Some("clean")) {
        return vec![NetSettlementProposal {
            id: "nsp_clean_lp_beta".to_string(),
            tenant_id: "tenant_demo".to_string(),
            counterparty_id: "lp_beta".to_string(),
            asset: "USDT".to_string(),
            settlement_ids: vec!["stl_clean_001".to_string(), "stl_clean_002".to_string()],
            gross_in: "250".to_string(),
            gross_out: "245".to_string(),
            net_amount: "5".to_string(),
            direction: "receive".to_string(),
            status: proposal_status_label(NetSettlementProposalStatus::Draft),
            approval_required: true,
            summary:
                "Counterparty exposure is nearly flat; no bilateral settlement action is urgent."
                    .to_string(),
        }];
    }

    if matches!(scenario, Some("approval_pending")) {
        return vec![NetSettlementProposal {
            id: "nsp_pending_lp_alpha".to_string(),
            tenant_id: "tenant_demo".to_string(),
            counterparty_id: "lp_alpha".to_string(),
            asset: "USDT".to_string(),
            settlement_ids: vec![
                "stl_pending_001".to_string(),
                "stl_pending_002".to_string(),
                "stl_pending_003".to_string(),
            ],
            gross_in: "420".to_string(),
            gross_out: "610".to_string(),
            net_amount: "190".to_string(),
            direction: "pay".to_string(),
            status: proposal_status_label(NetSettlementProposalStatus::PendingApproval),
            approval_required: true,
            summary:
                "Approval queue is holding a bilateral payout recommendation for lp_alpha."
                    .to_string(),
        }];
    }

    vec![
        NetSettlementProposal {
            id: "nsp_active_lp_alpha".to_string(),
            tenant_id: "tenant_demo".to_string(),
            counterparty_id: "lp_alpha".to_string(),
            asset: "USDT".to_string(),
            settlement_ids: vec![
                "stl_active_001".to_string(),
                "stl_active_002".to_string(),
                "stl_active_003".to_string(),
            ],
            gross_in: "350".to_string(),
            gross_out: "520".to_string(),
            net_amount: "170".to_string(),
            direction: "pay".to_string(),
            status: proposal_status_label(NetSettlementProposalStatus::PendingApproval),
            approval_required: true,
            summary:
                "lp_alpha owes a net payout after bilateral compression of same-window settlements."
                    .to_string(),
        },
        NetSettlementProposal {
            id: "nsp_active_bank_vcb".to_string(),
            tenant_id: "tenant_demo".to_string(),
            counterparty_id: "bank_vcb".to_string(),
            asset: "VND".to_string(),
            settlement_ids: vec!["stl_active_010".to_string(), "stl_active_011".to_string()],
            gross_in: "980".to_string(),
            gross_out: "760".to_string(),
            net_amount: "220".to_string(),
            direction: "receive".to_string(),
            status: proposal_status_label(NetSettlementProposalStatus::Draft),
            approval_required: true,
            summary:
                "bank_vcb remains net receivable; keep this bilateral package approval-gated."
                    .to_string(),
        },
    ]
}

fn build_alerts(proposals: &[NetSettlementProposal]) -> Vec<NetSettlementAlert> {
    if proposals.is_empty() {
        return vec![NetSettlementAlert {
            id: "net_settlement_clear".to_string(),
            severity: "low".to_string(),
            title: "No bilateral settlement package is required".to_string(),
            summary: "Current settlement rows do not produce a material bilateral netting proposal."
                .to_string(),
        }];
    }

    let mut alerts = Vec::new();
    if proposals.iter().any(|proposal| proposal.status == "pending_approval") {
        alerts.push(NetSettlementAlert {
            id: "net_settlement_pending_approval".to_string(),
            severity: "medium".to_string(),
            title: "At least one bilateral proposal is waiting for approval".to_string(),
            summary: "Keep settlement execution manual until the bilateral proposal is explicitly approved."
                .to_string(),
        });
    }
    if proposals
        .iter()
        .any(|proposal| proposal.direction == "pay" && proposal.net_amount != "0")
    {
        alerts.push(NetSettlementAlert {
            id: "net_settlement_payable_pressure".to_string(),
            severity: "high".to_string(),
            title: "One counterparty remains net payable".to_string(),
            summary: "Review bilateral packages with net payable exposure before releasing funds."
                .to_string(),
        });
    }
    alerts
}

fn proposal_status_label(status: NetSettlementProposalStatus) -> String {
    match status {
        NetSettlementProposalStatus::Draft => "draft",
        NetSettlementProposalStatus::PendingApproval => "pending_approval",
        NetSettlementProposalStatus::Approved => "approved",
        NetSettlementProposalStatus::Rejected => "rejected",
        NetSettlementProposalStatus::Executed => "executed",
    }
    .to_string()
}

fn decimal_to_string(value: Decimal) -> String {
    value.round_dp(2).normalize().to_string()
}

#[allow(dead_code)]
fn sample_settlements_for_counterparty(counterparty_id: &str) -> Vec<Settlement> {
    let now = Utc::now();
    vec![
        Settlement {
            id: format!("stl_{}_001", counterparty_id),
            offramp_intent_id: format!("tenant_demo_{}_ofr_001", counterparty_id),
            status: SettlementStatus::Processing,
            bank_reference: Some("RAMP-BILATERAL-001".to_string()),
            error_message: None,
            created_at: now - Duration::minutes(30),
            updated_at: now - Duration::minutes(12),
        },
        Settlement {
            id: format!("stl_{}_002", counterparty_id),
            offramp_intent_id: format!("tenant_demo_{}_ofr_002", counterparty_id),
            status: SettlementStatus::Completed,
            bank_reference: Some("RAMP-BILATERAL-002".to_string()),
            error_message: None,
            created_at: now - Duration::minutes(22),
            updated_at: now - Duration::minutes(5),
        },
    ]
}
