use chrono::{Duration, Utc};
use ramp_core::service::reconciliation_export::ReconciliationExportService;
use ramp_core::service::settlement::{Settlement, SettlementStatus};
use ramp_core::service::{OnChainTransaction, ReconciliationService, SettlementRecord};

#[test]
fn reconciliation_export_service_builds_snapshot_and_evidence_exports() {
    let service = ReconciliationService::new();
    let now = Utc::now();

    let on_chain_txs = vec![OnChainTransaction {
        tx_hash: "0xstatus".to_string(),
        from: "0xsource".to_string(),
        to: "0xdestination".to_string(),
        amount: 250.0,
        currency: "USDT".to_string(),
        timestamp: now - Duration::minutes(19),
        confirmed: false,
    }];
    let settlement_records = vec![SettlementRecord {
        id: "stl_status_001".to_string(),
        tx_hash: Some("0xstatus".to_string()),
        amount: 250.0,
        currency: "USDT".to_string(),
        status: "COMPLETED".to_string(),
        created_at: now - Duration::minutes(30),
        updated_at: now - Duration::minutes(18),
    }];
    let settlements = vec![Settlement {
        id: "stl_status_001".to_string(),
        offramp_intent_id: "ofr_status_001".to_string(),
        status: SettlementStatus::Completed,
        bank_reference: Some("RAMP-STATUS".to_string()),
        error_message: None,
        created_at: now - Duration::minutes(30),
        updated_at: now - Duration::minutes(18),
    }];

    let snapshot =
        ReconciliationExportService::build_snapshot(&service, &on_chain_txs, &settlement_records);
    assert_eq!(snapshot.report.total_discrepancies, 1);
    assert_eq!(snapshot.queue.len(), 1);

    let csv = ReconciliationExportService::export_queue_csv(&snapshot);
    assert!(csv.contains("discrepancy_id,report_id"));

    let discrepancy_id = snapshot.queue[0].discrepancy_id.clone();
    let export_service = ReconciliationExportService::new();
    let artifact = export_service
        .export_evidence_json(&snapshot.report, &settlements, &discrepancy_id)
        .expect("evidence export should build");
    let exported: serde_json::Value =
        serde_json::from_slice(&artifact.contents).expect("evidence JSON should decode");

    assert_eq!(artifact.content_type, "application/json");
    assert!(artifact.file_name.contains(&discrepancy_id));
    assert_eq!(
        exported["evidencePack"]["settlementIds"][0],
        "stl_status_001"
    );
    assert_eq!(
        exported["evidencePack"]["queueItem"]["settlementId"],
        "stl_status_001"
    );
}

#[test]
fn reconciliation_snapshot_export_makes_provenance_classification_explicit() {
    let service = ReconciliationService::new();
    let now = Utc::now();

    let on_chain_txs = vec![OnChainTransaction {
        tx_hash: "0xruntime".to_string(),
        from: "0xsource".to_string(),
        to: "0xdestination".to_string(),
        amount: 125.0,
        currency: "USDT".to_string(),
        timestamp: now - Duration::minutes(11),
        confirmed: true,
    }];
    let settlement_records = vec![SettlementRecord {
        id: "stl_runtime_001".to_string(),
        tx_hash: Some("0xruntime".to_string()),
        amount: 125.0,
        currency: "USDT".to_string(),
        status: "COMPLETED".to_string(),
        created_at: now - Duration::minutes(15),
        updated_at: now - Duration::minutes(10),
    }];
    let settlements = vec![Settlement {
        id: "stl_runtime_001".to_string(),
        offramp_intent_id: "ofr_runtime_001".to_string(),
        status: SettlementStatus::Completed,
        bank_reference: Some("RAMP-RUNTIME".to_string()),
        error_message: None,
        created_at: now - Duration::minutes(15),
        updated_at: now - Duration::minutes(10),
    }];

    let runtime_snapshot =
        ReconciliationExportService::build_snapshot(&service, &on_chain_txs, &settlement_records);
    let runtime_json = ReconciliationExportService::export_snapshot_json(&runtime_snapshot);
    assert_eq!(runtime_json["provenance"]["sourceKind"], "runtime_inputs");
    assert_eq!(runtime_json["provenance"]["sourceClass"], "evidence_backed");

    let sample_snapshot = ReconciliationExportService::build_snapshot_with_source_kind(
        &service,
        &on_chain_txs,
        &settlement_records,
        "sample_fallback",
    );
    let sample_json = ReconciliationExportService::export_snapshot_json(&sample_snapshot);
    assert_eq!(sample_json["provenance"]["sourceKind"], "sample_fallback");
    assert_eq!(sample_json["provenance"]["sourceClass"], "bounded_fallback");
    assert!(sample_json["provenance"]["freshnessWarning"]
        .as_str()
        .expect("sample fallback warning")
        .contains("sample fixture data"));

    let workbench = ReconciliationExportService::new().build_workbench(&settlements);
    let workbench_artifact = ReconciliationExportService::new()
        .export_workbench_json(&workbench)
        .expect("workbench export should build");
    let workbench_json: serde_json::Value =
        serde_json::from_slice(&workbench_artifact.contents).expect("workbench JSON should decode");
    assert_eq!(workbench_json["provenance"]["sourceClass"], "evidence_backed");
    assert!(workbench_json["provenance"]["freshnessWarning"]
        .as_str()
        .expect("workbench warning")
        .contains("No on-chain transaction data was provided"));
}
