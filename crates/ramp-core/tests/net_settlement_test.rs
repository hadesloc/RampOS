use chrono::Utc;

use ramp_core::service::net_settlement::NetSettlementService;
use ramp_core::service::settlement::{Settlement, SettlementStatus};

#[test]
fn net_settlement_workbench_active_scenario_is_approval_gated() {
    let snapshot = NetSettlementService::new().build_workbench(None);

    assert_eq!(snapshot.action_mode, "approval_gated");
    assert_eq!(snapshot.approval_mode, "manual_approval");
    assert!(!snapshot.proposals.is_empty());
    assert!(snapshot
        .proposals
        .iter()
        .all(|proposal| proposal.approval_required));
}

#[test]
fn net_settlement_can_build_single_bilateral_proposal_from_settlements() {
    let now = Utc::now();
    let settlements = vec![
        Settlement {
            id: "stl_w10_001".to_string(),
            offramp_intent_id: "tenant_demo_lp_alpha_ofr_001".to_string(),
            status: SettlementStatus::Processing,
            bank_reference: Some("RAMP-W10-001".to_string()),
            error_message: None,
            created_at: now,
            updated_at: now,
        },
        Settlement {
            id: "stl_w10_002".to_string(),
            offramp_intent_id: "tenant_demo_lp_alpha_ofr_002".to_string(),
            status: SettlementStatus::Completed,
            bank_reference: Some("RAMP-W10-002".to_string()),
            error_message: None,
            created_at: now,
            updated_at: now,
        },
    ];

    let proposals = NetSettlementService::new()
        .build_bilateral_proposals("tenant_demo", "lp_alpha", "USDT", &settlements);

    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].counterparty_id, "lp_alpha");
    assert_eq!(proposals[0].status, "pending_approval");
    assert_eq!(proposals[0].settlement_ids.len(), 2);
}
