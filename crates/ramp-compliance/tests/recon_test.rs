use chrono::{Duration, Utc};
use ramp_compliance::reconciliation::{
    DiscrepancyType, RailsTransaction, RamposTransaction, ReconBatch, ReconConfig, ReconEngine,
};
use rust_decimal::Decimal;

#[test]
fn test_reconciliation_logic() {
    let config = ReconConfig {
        amount_tolerance: Decimal::from(100),
        timestamp_tolerance: Duration::minutes(10),
        auto_resolve_minor: false,
    };
    let engine = ReconEngine::new(config);

    let mut batch = ReconBatch::new(
        "tenant_1",
        "mock_bank",
        Utc::now() - Duration::days(1),
        Utc::now(),
    );

    let now = Utc::now();

    let rampos_txs = vec![
        // Perfect match
        RamposTransaction {
            intent_id: "tx1".to_string(),
            reference_code: "REF1".to_string(),
            amount: Decimal::from(1000),
            status: "COMPLETED".to_string(),
            bank_tx_id: Some("bank1".to_string()),
            created_at: now,
            settled_at: Some(now),
        },
        // Amount mismatch
        RamposTransaction {
            intent_id: "tx2".to_string(),
            reference_code: "REF2".to_string(),
            amount: Decimal::from(2000),
            status: "COMPLETED".to_string(),
            bank_tx_id: Some("bank2".to_string()),
            created_at: now,
            settled_at: Some(now),
        },
        // Missing in rails
        RamposTransaction {
            intent_id: "tx3".to_string(),
            reference_code: "REF3".to_string(),
            amount: Decimal::from(3000),
            status: "COMPLETED".to_string(),
            bank_tx_id: None,
            created_at: now,
            settled_at: Some(now),
        },
    ];

    let rails_txs = vec![
        // Perfect match
        RailsTransaction {
            tx_id: "bank1".to_string(),
            reference_code: Some("REF1".to_string()),
            amount: Decimal::from(1000),
            status: "SUCCESS".to_string(),
            timestamp: now,
        },
        // Amount mismatch
        RailsTransaction {
            tx_id: "bank2".to_string(),
            reference_code: Some("REF2".to_string()),
            amount: Decimal::from(2500), // 500 diff > 100 tolerance
            status: "SUCCESS".to_string(),
            timestamp: now,
        },
        // Missing in RampOS
        RailsTransaction {
            tx_id: "bank4".to_string(),
            reference_code: Some("REF4".to_string()),
            amount: Decimal::from(4000),
            status: "SUCCESS".to_string(),
            timestamp: now,
        },
    ];

    engine.reconcile(&mut batch, rampos_txs, rails_txs);

    assert_eq!(batch.matched_count, 2); // tx1 and tx2 matched
    assert_eq!(batch.discrepancy_count, 3); // tx2 amount mismatch, tx3 missing in rails, bank4 missing in rampos

    // Verify amount mismatch
    let amount_mismatch = batch
        .discrepancies
        .iter()
        .find(|d| d.discrepancy_type == DiscrepancyType::AmountMismatch)
        .unwrap();
    assert_eq!(amount_mismatch.intent_id.as_deref(), Some("tx2"));

    // Verify missing in rails
    let missing_in_rails = batch
        .discrepancies
        .iter()
        .find(|d| d.discrepancy_type == DiscrepancyType::MissingInRails)
        .unwrap();
    assert_eq!(missing_in_rails.intent_id.as_deref(), Some("tx3"));

    // Verify missing in rampos
    let missing_in_rampos = batch
        .discrepancies
        .iter()
        .find(|d| d.discrepancy_type == DiscrepancyType::MissingInRampos)
        .unwrap();
    assert_eq!(missing_in_rampos.rails_tx_id.as_deref(), Some("bank4"));
}

#[test]
fn test_csv_ingestion() {
    let engine = ReconEngine::new(ReconConfig::default());
    let csv_data = "tx_id,reference_code,amount,status,timestamp\n\
                    bank1,REF1,1000,SUCCESS,2023-01-01T12:00:00Z\n\
                    bank2,REF2,2000,PENDING,2023-01-01T12:05:00Z";

    let txs = engine.ingest_csv(csv_data).unwrap();
    assert_eq!(txs.len(), 2);
    assert_eq!(txs[0].tx_id, "bank1");
    assert_eq!(txs[0].amount, Decimal::from(1000));
}
