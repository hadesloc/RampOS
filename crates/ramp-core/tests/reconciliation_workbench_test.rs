use chrono::{Duration, Utc};
use ramp_core::service::{
    OnChainTransaction, ReconciliationAgeBucket, ReconciliationMatchConfidence,
    ReconciliationOwnerLane, ReconciliationRootCause, ReconciliationService, SettlementRecord,
};

#[test]
fn reconciliation_workbench_queue_surfaces_owner_root_cause_and_match_hints() {
    let service = ReconciliationService::new();
    let now = Utc::now();

    let on_chain_txs = vec![OnChainTransaction {
        tx_hash: "0xqueue".to_string(),
        from: "0xsource".to_string(),
        to: "0xdestination".to_string(),
        amount: 100.0,
        currency: "USDT".to_string(),
        timestamp: now - Duration::minutes(46),
        confirmed: true,
    }];
    let settlements = vec![SettlementRecord {
        id: "stl_candidate_001".to_string(),
        tx_hash: None,
        amount: 100.005,
        currency: "USDT".to_string(),
        status: "PROCESSING".to_string(),
        created_at: now - Duration::minutes(55),
        updated_at: now - Duration::minutes(47),
    }];

    let report = service.reconcile(&on_chain_txs, &settlements);
    let queue = service.build_break_queue(&report, &settlements);

    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].owner_lane, ReconciliationOwnerLane::SettlementOperations);
    assert_eq!(queue[0].root_cause, ReconciliationRootCause::OffchainRecordingGap);
    assert_eq!(queue[0].age_bucket, ReconciliationAgeBucket::Fresh);
    assert_eq!(queue[0].suggested_matches.len(), 1);
    assert_eq!(queue[0].suggested_matches[0].settlement_id, "stl_candidate_001");
    assert_eq!(
        queue[0].suggested_matches[0].confidence,
        ReconciliationMatchConfidence::High
    );
}
