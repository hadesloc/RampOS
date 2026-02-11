//! E2E integration tests for the Settlement/Reconciliation feature (F13).
//!
//! These tests build an in-memory settlement repository and service layer
//! on top of the existing `SettlementService` from ramp-core to exercise:
//!
//! 1. Settlement creation and lifecycle (pending -> processing -> completed)
//! 2. Settlement status transitions (valid and invalid)
//! 3. Settlement batching - multiple intents grouped into one batch
//! 4. Settlement calculation accuracy (amounts, fees, net amounts)
//! 5. Multi-tenant settlement isolation
//! 6. Settlement reconciliation matching
//! 7. Failed settlement handling and error states
//!
//! The approach follows the same pattern as `webhook_delivery_e2e_test.rs`:
//! in-memory repositories with synchronous access via `Arc<Mutex<...>>`.

use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use ramp_core::service::settlement::{Settlement, SettlementService, SettlementStatus};

// ============================================================================
// Extended Settlement Types (for E2E testing of batching, recon, tenancy)
// ============================================================================

/// A settlement batch groups multiple off-ramp intents into one payout.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SettlementBatch {
    id: String,
    tenant_id: String,
    /// Individual settlement IDs included in this batch.
    settlement_ids: Vec<String>,
    /// Total gross amount (sum of all intent amounts).
    total_gross_amount: Decimal,
    /// Total fees deducted.
    total_fees: Decimal,
    /// Net amount to settle.
    total_net_amount: Decimal,
    /// Batch status.
    status: SettlementStatus,
    /// Bank reference for the batch payout.
    bank_reference: Option<String>,
    /// Error message if the batch failed.
    error_message: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

/// A reconciliation record that pairs a settlement with an external bank
/// confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReconciliationRecord {
    id: String,
    settlement_id: String,
    tenant_id: String,
    /// Amount that the bank confirmed.
    bank_confirmed_amount: Decimal,
    /// Amount we expected.
    expected_amount: Decimal,
    /// Whether the two match (within tolerance).
    matched: bool,
    /// Discrepancy amount (bank - expected).
    discrepancy: Decimal,
    created_at: chrono::DateTime<Utc>,
}

// ============================================================================
// In-Memory Settlement Repository
// ============================================================================

#[derive(Clone, Default)]
struct InMemorySettlementRepo {
    settlements: Arc<Mutex<HashMap<String, Settlement>>>,
    batches: Arc<Mutex<HashMap<String, SettlementBatch>>>,
    recon_records: Arc<Mutex<HashMap<String, ReconciliationRecord>>>,
}

impl InMemorySettlementRepo {
    fn new() -> Self {
        Self {
            settlements: Arc::new(Mutex::new(HashMap::new())),
            batches: Arc::new(Mutex::new(HashMap::new())),
            recon_records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // -- Settlement CRUD --

    fn insert_settlement(&self, s: Settlement) {
        self.settlements.lock().unwrap().insert(s.id.clone(), s);
    }

    fn get_settlement(&self, id: &str) -> Option<Settlement> {
        self.settlements.lock().unwrap().get(id).cloned()
    }

    fn update_status(
        &self,
        id: &str,
        new_status: SettlementStatus,
        error_msg: Option<String>,
    ) -> Result<Settlement, String> {
        let mut map = self.settlements.lock().unwrap();
        match map.get_mut(id) {
            Some(s) => {
                // Enforce valid transitions
                if !is_valid_transition(&s.status, &new_status) {
                    return Err(format!(
                        "Invalid transition from {:?} to {:?}",
                        s.status, new_status
                    ));
                }
                s.status = new_status;
                s.error_message = error_msg;
                s.updated_at = Utc::now();
                Ok(s.clone())
            }
            None => Err(format!("Settlement {} not found", id)),
        }
    }

    fn list_by_tenant(&self, tenant_id: &str) -> Vec<Settlement> {
        self.settlements
            .lock()
            .unwrap()
            .values()
            .filter(|s| {
                // We encode tenant_id into the offramp_intent_id prefix for testing
                s.offramp_intent_id.starts_with(tenant_id)
            })
            .cloned()
            .collect()
    }

    fn count_by_status(&self, status: &SettlementStatus) -> usize {
        self.settlements
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.status == *status)
            .count()
    }

    // -- Batch CRUD --

    fn insert_batch(&self, b: SettlementBatch) {
        self.batches.lock().unwrap().insert(b.id.clone(), b);
    }

    fn get_batch(&self, id: &str) -> Option<SettlementBatch> {
        self.batches.lock().unwrap().get(id).cloned()
    }

    fn update_batch_status(
        &self,
        id: &str,
        new_status: SettlementStatus,
        error_msg: Option<String>,
    ) -> Result<SettlementBatch, String> {
        let mut map = self.batches.lock().unwrap();
        match map.get_mut(id) {
            Some(b) => {
                if !is_valid_transition(&b.status, &new_status) {
                    return Err(format!(
                        "Invalid batch transition from {:?} to {:?}",
                        b.status, new_status
                    ));
                }
                b.status = new_status;
                b.error_message = error_msg;
                b.updated_at = Utc::now();
                Ok(b.clone())
            }
            None => Err(format!("Batch {} not found", id)),
        }
    }

    fn list_batches_by_tenant(&self, tenant_id: &str) -> Vec<SettlementBatch> {
        self.batches
            .lock()
            .unwrap()
            .values()
            .filter(|b| b.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    // -- Reconciliation CRUD --

    fn insert_recon_record(&self, r: ReconciliationRecord) {
        self.recon_records
            .lock()
            .unwrap()
            .insert(r.id.clone(), r);
    }

    #[allow(dead_code)]
    fn get_recon_record(&self, id: &str) -> Option<ReconciliationRecord> {
        self.recon_records.lock().unwrap().get(id).cloned()
    }

    fn list_recon_by_settlement(&self, settlement_id: &str) -> Vec<ReconciliationRecord> {
        self.recon_records
            .lock()
            .unwrap()
            .values()
            .filter(|r| r.settlement_id == settlement_id)
            .cloned()
            .collect()
    }

    fn list_unmatched_recon(&self) -> Vec<ReconciliationRecord> {
        self.recon_records
            .lock()
            .unwrap()
            .values()
            .filter(|r| !r.matched)
            .cloned()
            .collect()
    }
}

// ============================================================================
// Transition Guard
// ============================================================================

/// Defines the valid state transitions for a settlement.
///
///   Pending -> Processing
///   Processing -> Completed
///   Processing -> Failed
///   Failed -> Pending     (retry)
///   Pending -> Failed     (pre-processing rejection)
fn is_valid_transition(from: &SettlementStatus, to: &SettlementStatus) -> bool {
    matches!(
        (from, to),
        (SettlementStatus::Pending, SettlementStatus::Processing)
            | (SettlementStatus::Processing, SettlementStatus::Completed)
            | (SettlementStatus::Processing, SettlementStatus::Failed)
            | (SettlementStatus::Failed, SettlementStatus::Pending)
            | (SettlementStatus::Pending, SettlementStatus::Failed)
    )
}

// ============================================================================
// Helper: trigger + persist a settlement through the core service
// ============================================================================

fn trigger_and_persist(
    svc: &SettlementService,
    repo: &InMemorySettlementRepo,
    offramp_intent_id: &str,
) -> Settlement {
    let mut s = svc.trigger_settlement(offramp_intent_id).unwrap();
    // The core service creates with Processing; we move to Pending first for
    // our richer lifecycle (core stub skips Pending).
    s.status = SettlementStatus::Pending;
    repo.insert_settlement(s.clone());
    s
}

// ============================================================================
// Helper: create a batch from settlements
// ============================================================================

fn create_batch(
    repo: &InMemorySettlementRepo,
    tenant_id: &str,
    settlement_ids: Vec<String>,
    amounts: Vec<(Decimal, Decimal)>, // (gross, fee) per settlement
) -> SettlementBatch {
    let total_gross: Decimal = amounts.iter().map(|(g, _)| g).sum();
    let total_fees: Decimal = amounts.iter().map(|(_, f)| f).sum();
    let total_net = total_gross - total_fees;

    let batch = SettlementBatch {
        id: format!("batch_{}", Uuid::now_v7()),
        tenant_id: tenant_id.to_string(),
        settlement_ids,
        total_gross_amount: total_gross,
        total_fees,
        total_net_amount: total_net,
        status: SettlementStatus::Pending,
        bank_reference: None,
        error_message: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    repo.insert_batch(batch.clone());
    batch
}

// ============================================================================
// 1. Settlement Creation and Full Lifecycle
// ============================================================================

#[test]
fn test_settlement_creation_and_lifecycle() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    // Step 1: Trigger a settlement (comes back as Pending after our helper).
    let s = trigger_and_persist(&svc, &repo, "tenantA_ofr_001");
    assert!(s.id.starts_with("stl_"));
    assert_eq!(s.status, SettlementStatus::Pending);
    assert_eq!(s.offramp_intent_id, "tenantA_ofr_001");
    assert!(s.bank_reference.is_some());
    assert!(s.error_message.is_none());

    // Step 2: Transition Pending -> Processing.
    let s2 = repo
        .update_status(&s.id, SettlementStatus::Processing, None)
        .expect("Pending -> Processing should succeed");
    assert_eq!(s2.status, SettlementStatus::Processing);

    // Step 3: Transition Processing -> Completed.
    let s3 = repo
        .update_status(&s.id, SettlementStatus::Completed, None)
        .expect("Processing -> Completed should succeed");
    assert_eq!(s3.status, SettlementStatus::Completed);
    assert!(s3.updated_at >= s2.updated_at);

    // Verify from repo.
    let stored = repo.get_settlement(&s.id).unwrap();
    assert_eq!(stored.status, SettlementStatus::Completed);
}

// ============================================================================
// 2. Valid and Invalid Status Transitions
// ============================================================================

#[test]
fn test_settlement_valid_transitions() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    // Pending -> Processing  (valid)
    let s1 = trigger_and_persist(&svc, &repo, "tenantA_ofr_trans_1");
    assert!(repo
        .update_status(&s1.id, SettlementStatus::Processing, None)
        .is_ok());

    // Processing -> Completed  (valid)
    assert!(repo
        .update_status(&s1.id, SettlementStatus::Completed, None)
        .is_ok());

    // Pending -> Failed  (valid: pre-processing rejection)
    let s2 = trigger_and_persist(&svc, &repo, "tenantA_ofr_trans_2");
    assert!(repo
        .update_status(
            &s2.id,
            SettlementStatus::Failed,
            Some("Bank account invalid".to_string()),
        )
        .is_ok());

    // Failed -> Pending  (valid: retry)
    assert!(repo
        .update_status(&s2.id, SettlementStatus::Pending, None)
        .is_ok());

    // Processing -> Failed  (valid: bank rejection mid-processing)
    let s3 = trigger_and_persist(&svc, &repo, "tenantA_ofr_trans_3");
    repo.update_status(&s3.id, SettlementStatus::Processing, None)
        .unwrap();
    assert!(repo
        .update_status(
            &s3.id,
            SettlementStatus::Failed,
            Some("Bank timeout".to_string()),
        )
        .is_ok());
}

#[test]
fn test_settlement_invalid_transitions() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    // Pending -> Completed  (INVALID: must go through Processing)
    let s1 = trigger_and_persist(&svc, &repo, "tenantA_ofr_inv_1");
    let err = repo
        .update_status(&s1.id, SettlementStatus::Completed, None)
        .unwrap_err();
    assert!(
        err.contains("Invalid transition"),
        "Expected invalid transition error, got: {}",
        err
    );

    // Completed -> Processing  (INVALID: Completed is terminal for forward)
    let s2 = trigger_and_persist(&svc, &repo, "tenantA_ofr_inv_2");
    repo.update_status(&s2.id, SettlementStatus::Processing, None)
        .unwrap();
    repo.update_status(&s2.id, SettlementStatus::Completed, None)
        .unwrap();
    let err2 = repo
        .update_status(&s2.id, SettlementStatus::Processing, None)
        .unwrap_err();
    assert!(err2.contains("Invalid transition"));

    // Completed -> Pending  (INVALID)
    let err3 = repo
        .update_status(&s2.id, SettlementStatus::Pending, None)
        .unwrap_err();
    assert!(err3.contains("Invalid transition"));

    // Completed -> Failed  (INVALID)
    let err4 = repo
        .update_status(&s2.id, SettlementStatus::Failed, None)
        .unwrap_err();
    assert!(err4.contains("Invalid transition"));

    // Failed -> Processing  (INVALID: must go back to Pending first)
    let s3 = trigger_and_persist(&svc, &repo, "tenantA_ofr_inv_3");
    repo.update_status(&s3.id, SettlementStatus::Failed, Some("error".to_string()))
        .unwrap();
    let err5 = repo
        .update_status(&s3.id, SettlementStatus::Processing, None)
        .unwrap_err();
    assert!(err5.contains("Invalid transition"));

    // Failed -> Completed  (INVALID)
    let err6 = repo
        .update_status(&s3.id, SettlementStatus::Completed, None)
        .unwrap_err();
    assert!(err6.contains("Invalid transition"));
}

// ============================================================================
// 3. Settlement Batching
// ============================================================================

#[test]
fn test_settlement_batching_multiple_intents() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();
    let tenant = "tenantBatch";

    // Create 5 individual settlements for the same tenant.
    let mut settlement_ids = Vec::new();
    let mut amounts = Vec::new();
    for i in 0..5 {
        let intent_id = format!("{}_ofr_{}", tenant, i);
        let s = trigger_and_persist(&svc, &repo, &intent_id);
        settlement_ids.push(s.id.clone());
        // Simulated: gross = 10M VND, fee = 200K VND each.
        amounts.push((dec!(10_000_000), dec!(200_000)));
    }

    // Create a batch from all 5 settlements.
    let batch = create_batch(&repo, tenant, settlement_ids.clone(), amounts);

    assert!(batch.id.starts_with("batch_"));
    assert_eq!(batch.settlement_ids.len(), 5);
    assert_eq!(batch.total_gross_amount, dec!(50_000_000));
    assert_eq!(batch.total_fees, dec!(1_000_000));
    assert_eq!(batch.total_net_amount, dec!(49_000_000));
    assert_eq!(batch.status, SettlementStatus::Pending);

    // Verify the batch is persisted.
    let stored = repo.get_batch(&batch.id).unwrap();
    assert_eq!(stored.settlement_ids.len(), 5);
    assert_eq!(stored.tenant_id, tenant);
}

#[test]
fn test_settlement_batch_lifecycle() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();
    let tenant = "tenantBatchLC";

    let s1 = trigger_and_persist(&svc, &repo, &format!("{}_ofr_1", tenant));
    let s2 = trigger_and_persist(&svc, &repo, &format!("{}_ofr_2", tenant));

    let batch = create_batch(
        &repo,
        tenant,
        vec![s1.id.clone(), s2.id.clone()],
        vec![(dec!(5_000_000), dec!(100_000)), (dec!(8_000_000), dec!(160_000))],
    );

    // Batch: Pending -> Processing
    let b2 = repo
        .update_batch_status(&batch.id, SettlementStatus::Processing, None)
        .unwrap();
    assert_eq!(b2.status, SettlementStatus::Processing);

    // Also move individual settlements to Processing.
    repo.update_status(&s1.id, SettlementStatus::Processing, None)
        .unwrap();
    repo.update_status(&s2.id, SettlementStatus::Processing, None)
        .unwrap();

    // Batch: Processing -> Completed
    let b3 = repo
        .update_batch_status(&batch.id, SettlementStatus::Completed, None)
        .unwrap();
    assert_eq!(b3.status, SettlementStatus::Completed);

    // Mark individual settlements as completed too.
    repo.update_status(&s1.id, SettlementStatus::Completed, None)
        .unwrap();
    repo.update_status(&s2.id, SettlementStatus::Completed, None)
        .unwrap();

    // Verify final state.
    assert_eq!(repo.count_by_status(&SettlementStatus::Completed), 2);
    assert_eq!(repo.count_by_status(&SettlementStatus::Pending), 0);
}

#[test]
fn test_settlement_batch_with_single_intent() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();
    let tenant = "tenantSingle";

    let s = trigger_and_persist(&svc, &repo, &format!("{}_ofr_solo", tenant));
    let batch = create_batch(
        &repo,
        tenant,
        vec![s.id.clone()],
        vec![(dec!(25_000_000), dec!(500_000))],
    );

    assert_eq!(batch.settlement_ids.len(), 1);
    assert_eq!(batch.total_gross_amount, dec!(25_000_000));
    assert_eq!(batch.total_net_amount, dec!(24_500_000));
}

// ============================================================================
// 4. Settlement Calculation Accuracy
// ============================================================================

#[test]
fn test_settlement_calculation_accuracy_basic() {
    let repo = InMemorySettlementRepo::new();

    // Test exact fee subtraction.
    let gross = dec!(50_000_000);
    let fee = dec!(500_000); // 1%
    let net = gross - fee;

    let batch = create_batch(
        &repo,
        "tenantCalc",
        vec!["stl_calc_1".to_string()],
        vec![(gross, fee)],
    );

    assert_eq!(batch.total_gross_amount, dec!(50_000_000));
    assert_eq!(batch.total_fees, dec!(500_000));
    assert_eq!(batch.total_net_amount, dec!(49_500_000));
    assert_eq!(batch.total_net_amount, net);
}

#[test]
fn test_settlement_calculation_accuracy_multiple() {
    let repo = InMemorySettlementRepo::new();

    // 3 settlements with varying amounts and fee rates.
    let items = vec![
        (dec!(10_000_000), dec!(200_000)),   // 2% fee
        (dec!(50_000_000), dec!(500_000)),   // 1% fee
        (dec!(100_000_000), dec!(750_000)),  // 0.75% fee
    ];

    let ids: Vec<String> = (0..3).map(|i| format!("stl_multi_{}", i)).collect();
    let batch = create_batch(&repo, "tenantMultiCalc", ids, items.clone());

    let expected_gross: Decimal = items.iter().map(|(g, _)| g).sum();
    let expected_fees: Decimal = items.iter().map(|(_, f)| f).sum();

    assert_eq!(batch.total_gross_amount, expected_gross);
    assert_eq!(batch.total_fees, expected_fees);
    assert_eq!(
        batch.total_net_amount,
        expected_gross - expected_fees
    );
    // Specific values:
    assert_eq!(batch.total_gross_amount, dec!(160_000_000));
    assert_eq!(batch.total_fees, dec!(1_450_000));
    assert_eq!(batch.total_net_amount, dec!(158_550_000));
}

#[test]
fn test_settlement_calculation_zero_fee() {
    let repo = InMemorySettlementRepo::new();

    // Zero fee scenario (promotional / waived fees).
    let batch = create_batch(
        &repo,
        "tenantZeroFee",
        vec!["stl_zero_1".to_string()],
        vec![(dec!(1_000_000), dec!(0))],
    );

    assert_eq!(batch.total_fees, dec!(0));
    assert_eq!(batch.total_net_amount, dec!(1_000_000));
    assert_eq!(batch.total_net_amount, batch.total_gross_amount);
}

#[test]
fn test_settlement_calculation_large_amounts() {
    let repo = InMemorySettlementRepo::new();

    // Whale settlement: 10 billion VND.
    let gross = dec!(10_000_000_000);
    let fee = dec!(50_000_000); // 0.5%
    let batch = create_batch(
        &repo,
        "tenantWhale",
        vec!["stl_whale_1".to_string()],
        vec![(gross, fee)],
    );

    assert_eq!(batch.total_gross_amount, dec!(10_000_000_000));
    assert_eq!(batch.total_fees, dec!(50_000_000));
    assert_eq!(batch.total_net_amount, dec!(9_950_000_000));
}

#[test]
fn test_settlement_calculation_small_amounts() {
    let repo = InMemorySettlementRepo::new();

    // Micro settlement.
    let gross = dec!(50_000); // 50K VND
    let fee = dec!(1_000);    // 2% fee
    let batch = create_batch(
        &repo,
        "tenantMicro",
        vec!["stl_micro_1".to_string()],
        vec![(gross, fee)],
    );

    assert_eq!(batch.total_net_amount, dec!(49_000));
}

// ============================================================================
// 5. Multi-Tenant Settlement Isolation
// ============================================================================

#[test]
fn test_multi_tenant_settlement_isolation() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    // Tenant A gets 3 settlements.
    for i in 0..3 {
        trigger_and_persist(&svc, &repo, &format!("tenantA_ofr_{}", i));
    }

    // Tenant B gets 2 settlements.
    for i in 0..2 {
        trigger_and_persist(&svc, &repo, &format!("tenantB_ofr_{}", i));
    }

    // Tenant C gets 1 settlement.
    trigger_and_persist(&svc, &repo, "tenantC_ofr_0");

    // Verify isolation.
    let a_settlements = repo.list_by_tenant("tenantA");
    assert_eq!(a_settlements.len(), 3, "Tenant A should have 3 settlements");

    let b_settlements = repo.list_by_tenant("tenantB");
    assert_eq!(b_settlements.len(), 2, "Tenant B should have 2 settlements");

    let c_settlements = repo.list_by_tenant("tenantC");
    assert_eq!(c_settlements.len(), 1, "Tenant C should have 1 settlement");

    // Tenant D has none.
    let d_settlements = repo.list_by_tenant("tenantD");
    assert_eq!(d_settlements.len(), 0, "Tenant D should have 0 settlements");
}

#[test]
fn test_multi_tenant_batch_isolation() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    // Create settlements and batches for two tenants.
    let s_a = trigger_and_persist(&svc, &repo, "tenantAlpha_ofr_1");
    let _batch_a = create_batch(
        &repo,
        "tenantAlpha",
        vec![s_a.id.clone()],
        vec![(dec!(20_000_000), dec!(400_000))],
    );

    let s_b = trigger_and_persist(&svc, &repo, "tenantBeta_ofr_1");
    let _batch_b = create_batch(
        &repo,
        "tenantBeta",
        vec![s_b.id.clone()],
        vec![(dec!(30_000_000), dec!(600_000))],
    );

    // Each tenant should only see their own batches.
    let alpha_batches = repo.list_batches_by_tenant("tenantAlpha");
    assert_eq!(alpha_batches.len(), 1);
    assert_eq!(alpha_batches[0].total_gross_amount, dec!(20_000_000));

    let beta_batches = repo.list_batches_by_tenant("tenantBeta");
    assert_eq!(beta_batches.len(), 1);
    assert_eq!(beta_batches[0].total_gross_amount, dec!(30_000_000));

    // Non-existent tenant.
    let gamma_batches = repo.list_batches_by_tenant("tenantGamma");
    assert!(gamma_batches.is_empty());
}

#[test]
fn test_settlement_status_changes_dont_leak_across_tenants() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    let s_a = trigger_and_persist(&svc, &repo, "tenantIsoA_ofr_1");
    let s_b = trigger_and_persist(&svc, &repo, "tenantIsoB_ofr_1");

    // Move A's settlement to Processing.
    repo.update_status(&s_a.id, SettlementStatus::Processing, None)
        .unwrap();

    // B's settlement should still be Pending.
    let b_stored = repo.get_settlement(&s_b.id).unwrap();
    assert_eq!(
        b_stored.status,
        SettlementStatus::Pending,
        "Tenant B settlement should remain Pending"
    );

    // Move A to Completed.
    repo.update_status(&s_a.id, SettlementStatus::Completed, None)
        .unwrap();

    // B should still be Pending.
    let b_stored2 = repo.get_settlement(&s_b.id).unwrap();
    assert_eq!(b_stored2.status, SettlementStatus::Pending);
}

// ============================================================================
// 6. Settlement Reconciliation Matching
// ============================================================================

fn create_recon_record(
    repo: &InMemorySettlementRepo,
    settlement_id: &str,
    tenant_id: &str,
    expected: Decimal,
    bank_confirmed: Decimal,
    tolerance: Decimal,
) -> ReconciliationRecord {
    let discrepancy = bank_confirmed - expected;
    let matched = discrepancy.abs() <= tolerance;

    let record = ReconciliationRecord {
        id: format!("recon_{}", Uuid::now_v7()),
        settlement_id: settlement_id.to_string(),
        tenant_id: tenant_id.to_string(),
        bank_confirmed_amount: bank_confirmed,
        expected_amount: expected,
        matched,
        discrepancy,
        created_at: Utc::now(),
    };

    repo.insert_recon_record(record.clone());
    record
}

#[test]
fn test_reconciliation_exact_match() {
    let repo = InMemorySettlementRepo::new();
    let tolerance = dec!(0); // exact match required

    let rec = create_recon_record(
        &repo,
        "stl_recon_1",
        "tenantRecon",
        dec!(49_500_000), // expected
        dec!(49_500_000), // bank confirmed
        tolerance,
    );

    assert!(rec.matched, "Exact match should be flagged as matched");
    assert_eq!(rec.discrepancy, dec!(0));
}

#[test]
fn test_reconciliation_within_tolerance() {
    let repo = InMemorySettlementRepo::new();
    let tolerance = dec!(100); // 100 VND tolerance (rounding)

    let rec = create_recon_record(
        &repo,
        "stl_recon_tol",
        "tenantRecon",
        dec!(49_500_000),
        dec!(49_500_050), // 50 VND over = within tolerance
        tolerance,
    );

    assert!(rec.matched, "50 VND over should match within 100 VND tolerance");
    assert_eq!(rec.discrepancy, dec!(50));
}

#[test]
fn test_reconciliation_outside_tolerance() {
    let repo = InMemorySettlementRepo::new();
    let tolerance = dec!(100);

    let rec = create_recon_record(
        &repo,
        "stl_recon_out",
        "tenantRecon",
        dec!(49_500_000),
        dec!(49_499_800), // 200 VND short = outside tolerance
        tolerance,
    );

    assert!(!rec.matched, "200 VND discrepancy should NOT match within 100 VND tolerance");
    assert_eq!(rec.discrepancy, dec!(-200));
}

#[test]
fn test_reconciliation_negative_discrepancy() {
    let repo = InMemorySettlementRepo::new();
    let tolerance = dec!(1000);

    // Bank confirmed LESS than expected.
    let rec = create_recon_record(
        &repo,
        "stl_recon_neg",
        "tenantRecon",
        dec!(10_000_000),
        dec!(9_998_500), // 1500 VND short = outside 1000 tolerance
        tolerance,
    );

    assert!(!rec.matched);
    assert_eq!(rec.discrepancy, dec!(-1500));
}

#[test]
fn test_reconciliation_listing_unmatched() {
    let repo = InMemorySettlementRepo::new();
    let tolerance = dec!(100);

    // Create 3 matched, 2 unmatched records.
    for i in 0..3 {
        create_recon_record(
            &repo,
            &format!("stl_matched_{}", i),
            "tenantList",
            dec!(1_000_000),
            dec!(1_000_000),
            tolerance,
        );
    }
    for i in 0..2 {
        create_recon_record(
            &repo,
            &format!("stl_unmatched_{}", i),
            "tenantList",
            dec!(1_000_000),
            dec!(999_000), // 1000 VND short
            tolerance,
        );
    }

    let unmatched = repo.list_unmatched_recon();
    assert_eq!(unmatched.len(), 2, "Should have 2 unmatched records");
    for r in &unmatched {
        assert!(!r.matched);
    }
}

#[test]
fn test_reconciliation_per_settlement() {
    let repo = InMemorySettlementRepo::new();
    let tolerance = dec!(0);

    // Multiple recon records for the same settlement (e.g. partial payments).
    create_recon_record(
        &repo,
        "stl_multi_recon",
        "tenantMultiRecon",
        dec!(5_000_000),
        dec!(5_000_000),
        tolerance,
    );
    create_recon_record(
        &repo,
        "stl_multi_recon",
        "tenantMultiRecon",
        dec!(5_000_000),
        dec!(4_999_000), // discrepancy on second partial
        tolerance,
    );

    let recs = repo.list_recon_by_settlement("stl_multi_recon");
    assert_eq!(recs.len(), 2);

    let matched_count = recs.iter().filter(|r| r.matched).count();
    let unmatched_count = recs.iter().filter(|r| !r.matched).count();
    assert_eq!(matched_count, 1);
    assert_eq!(unmatched_count, 1);
}

// ============================================================================
// 7. Failed Settlement Handling and Error States
// ============================================================================

#[test]
fn test_settlement_failure_with_error_message() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    let s = trigger_and_persist(&svc, &repo, "tenantFail_ofr_1");

    // Move to Processing.
    repo.update_status(&s.id, SettlementStatus::Processing, None)
        .unwrap();

    // Fail with a descriptive error.
    let error_msg = "NAPAS timeout: bank did not respond within 30s".to_string();
    let failed = repo
        .update_status(&s.id, SettlementStatus::Failed, Some(error_msg.clone()))
        .unwrap();

    assert_eq!(failed.status, SettlementStatus::Failed);
    assert_eq!(failed.error_message, Some(error_msg));
}

#[test]
fn test_settlement_failure_pre_processing() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    let s = trigger_and_persist(&svc, &repo, "tenantPreFail_ofr_1");

    // Fail directly from Pending (e.g. validation / compliance block).
    let failed = repo
        .update_status(
            &s.id,
            SettlementStatus::Failed,
            Some("AML screening rejected".to_string()),
        )
        .unwrap();

    assert_eq!(failed.status, SettlementStatus::Failed);
    assert_eq!(
        failed.error_message.as_deref(),
        Some("AML screening rejected")
    );
}

#[test]
fn test_settlement_retry_after_failure() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    let s = trigger_and_persist(&svc, &repo, "tenantRetry_ofr_1");

    // Pending -> Failed.
    repo.update_status(
        &s.id,
        SettlementStatus::Failed,
        Some("Temporary network error".to_string()),
    )
    .unwrap();

    // Failed -> Pending (retry).
    let retried = repo
        .update_status(&s.id, SettlementStatus::Pending, None)
        .unwrap();
    assert_eq!(retried.status, SettlementStatus::Pending);
    // Error message should be cleared on retry (or kept for audit -- here
    // we pass None which sets it to None).
    assert!(retried.error_message.is_none());

    // Now proceed through normal lifecycle: Pending -> Processing -> Completed.
    repo.update_status(&s.id, SettlementStatus::Processing, None)
        .unwrap();
    let completed = repo
        .update_status(&s.id, SettlementStatus::Completed, None)
        .unwrap();
    assert_eq!(completed.status, SettlementStatus::Completed);
}

#[test]
fn test_settlement_batch_failure_propagation() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();
    let tenant = "tenantBatchFail";

    let s1 = trigger_and_persist(&svc, &repo, &format!("{}_ofr_1", tenant));
    let s2 = trigger_and_persist(&svc, &repo, &format!("{}_ofr_2", tenant));

    let batch = create_batch(
        &repo,
        tenant,
        vec![s1.id.clone(), s2.id.clone()],
        vec![(dec!(10_000_000), dec!(200_000)), (dec!(15_000_000), dec!(300_000))],
    );

    // Batch Pending -> Processing.
    repo.update_batch_status(&batch.id, SettlementStatus::Processing, None)
        .unwrap();

    // Batch Processing -> Failed (e.g. bank gateway down).
    let error = "Bank gateway returned HTTP 503".to_string();
    let failed_batch = repo
        .update_batch_status(&batch.id, SettlementStatus::Failed, Some(error.clone()))
        .unwrap();

    assert_eq!(failed_batch.status, SettlementStatus::Failed);
    assert_eq!(failed_batch.error_message, Some(error));
    assert_eq!(failed_batch.settlement_ids.len(), 2);

    // After batch failure, individual settlements should be marked Failed too.
    for sid in &batch.settlement_ids {
        repo.update_status(sid, SettlementStatus::Failed, Some("Batch failed".to_string()))
            .unwrap();
    }
    assert_eq!(repo.count_by_status(&SettlementStatus::Failed), 2);
}

#[test]
fn test_settlement_error_message_preserved_on_failure() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    let s = trigger_and_persist(&svc, &repo, "tenantErrMsg_ofr_1");
    repo.update_status(&s.id, SettlementStatus::Processing, None)
        .unwrap();

    let detailed_error = "NAPAS error code=E042: insufficient funds in pool account, reference=RAMP-ABC12345".to_string();
    repo.update_status(
        &s.id,
        SettlementStatus::Failed,
        Some(detailed_error.clone()),
    )
    .unwrap();

    let stored = repo.get_settlement(&s.id).unwrap();
    assert_eq!(stored.error_message.as_deref(), Some(detailed_error.as_str()));
}

// ============================================================================
// Additional: Core SettlementService Smoke Tests
// ============================================================================

#[test]
fn test_core_service_trigger_settlement_produces_unique_ids() {
    let svc = SettlementService::new();
    let mut ids = std::collections::HashSet::new();

    for i in 0..100 {
        let s = svc
            .trigger_settlement(&format!("ofr_unique_{}", i))
            .unwrap();
        assert!(
            ids.insert(s.id.clone()),
            "Settlement ID {} should be unique",
            s.id
        );
    }
    assert_eq!(ids.len(), 100);
}

#[test]
fn test_core_service_trigger_settlement_fields() {
    let svc = SettlementService::new();
    let s = svc.trigger_settlement("ofr_field_check").unwrap();

    assert!(s.id.starts_with("stl_"), "ID should start with stl_");
    assert_eq!(s.offramp_intent_id, "ofr_field_check");
    assert_eq!(
        s.status,
        SettlementStatus::Processing,
        "Core service starts in Processing"
    );
    assert!(s.bank_reference.is_some());
    assert!(
        s.bank_reference.as_ref().unwrap().starts_with("RAMP-"),
        "Bank reference should start with RAMP-"
    );
    assert!(s.error_message.is_none());
    assert!(s.created_at <= Utc::now());
    assert!(s.updated_at <= Utc::now());
}

#[test]
fn test_core_service_check_status_not_found() {
    let svc = SettlementService::new();
    let result = svc.check_settlement_status("stl_does_not_exist");
    assert!(result.is_err(), "Should return NotFound for unknown ID");
}

// ============================================================================
// Async: Concurrent Settlement Processing
// ============================================================================

#[tokio::test]
async fn test_concurrent_settlement_creation() {
    let svc = Arc::new(SettlementService::new());
    let repo = Arc::new(InMemorySettlementRepo::new());

    let handles: Vec<_> = (0..20)
        .map(|i| {
            let svc = svc.clone();
            let repo = repo.clone();
            tokio::spawn(async move {
                let intent_id = format!("tenantConc_ofr_{}", i);
                let mut s = svc.trigger_settlement(&intent_id).unwrap();
                s.status = SettlementStatus::Pending;
                repo.insert_settlement(s.clone());
                s.id
            })
        })
        .collect();

    let mut ids = Vec::new();
    for h in handles {
        ids.push(h.await.unwrap());
    }

    // All 20 should exist and have unique IDs.
    let unique: std::collections::HashSet<String> = ids.iter().cloned().collect();
    assert_eq!(unique.len(), 20, "All 20 settlement IDs must be unique");

    // All should be in Pending state.
    assert_eq!(repo.count_by_status(&SettlementStatus::Pending), 20);
}

#[tokio::test]
async fn test_concurrent_status_transitions() {
    let svc = Arc::new(SettlementService::new());
    let repo = Arc::new(InMemorySettlementRepo::new());

    // Create 10 settlements.
    let mut ids = Vec::new();
    for i in 0..10 {
        let s = trigger_and_persist(&svc, &repo, &format!("tenantConcTrans_ofr_{}", i));
        ids.push(s.id);
    }

    // Move all to Processing concurrently.
    let handles: Vec<_> = ids
        .iter()
        .cloned()
        .map(|id| {
            let repo = repo.clone();
            tokio::spawn(async move {
                repo.update_status(&id, SettlementStatus::Processing, None)
            })
        })
        .collect();

    for h in handles {
        let result = h.await.unwrap();
        assert!(result.is_ok(), "Concurrent Processing transition should succeed");
    }

    assert_eq!(repo.count_by_status(&SettlementStatus::Processing), 10);
    assert_eq!(repo.count_by_status(&SettlementStatus::Pending), 0);

    // Move all to Completed concurrently.
    let handles2: Vec<_> = ids
        .iter()
        .cloned()
        .map(|id| {
            let repo = repo.clone();
            tokio::spawn(async move {
                repo.update_status(&id, SettlementStatus::Completed, None)
            })
        })
        .collect();

    for h in handles2 {
        assert!(h.await.unwrap().is_ok());
    }

    assert_eq!(repo.count_by_status(&SettlementStatus::Completed), 10);
}

// ============================================================================
// Full Lifecycle: End-to-End settlement with batching + recon
// ============================================================================

#[test]
fn test_full_settlement_lifecycle_with_recon() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();
    let tenant = "tenantFullLC";

    // Phase 1: Create 3 settlements.
    let s1 = trigger_and_persist(&svc, &repo, &format!("{}_ofr_1", tenant));
    let s2 = trigger_and_persist(&svc, &repo, &format!("{}_ofr_2", tenant));
    let s3 = trigger_and_persist(&svc, &repo, &format!("{}_ofr_3", tenant));
    assert_eq!(repo.count_by_status(&SettlementStatus::Pending), 3);

    // Phase 2: Batch them.
    let batch = create_batch(
        &repo,
        tenant,
        vec![s1.id.clone(), s2.id.clone(), s3.id.clone()],
        vec![
            (dec!(10_000_000), dec!(200_000)),
            (dec!(20_000_000), dec!(400_000)),
            (dec!(30_000_000), dec!(600_000)),
        ],
    );
    assert_eq!(batch.total_gross_amount, dec!(60_000_000));
    assert_eq!(batch.total_fees, dec!(1_200_000));
    assert_eq!(batch.total_net_amount, dec!(58_800_000));

    // Phase 3: Process the batch.
    repo.update_batch_status(&batch.id, SettlementStatus::Processing, None)
        .unwrap();
    for sid in &[&s1.id, &s2.id, &s3.id] {
        repo.update_status(sid, SettlementStatus::Processing, None)
            .unwrap();
    }
    assert_eq!(repo.count_by_status(&SettlementStatus::Processing), 3);

    // Phase 4: Complete the batch.
    repo.update_batch_status(&batch.id, SettlementStatus::Completed, None)
        .unwrap();
    for sid in &[&s1.id, &s2.id, &s3.id] {
        repo.update_status(sid, SettlementStatus::Completed, None)
            .unwrap();
    }
    assert_eq!(repo.count_by_status(&SettlementStatus::Completed), 3);

    // Phase 5: Reconciliation -- bank confirms 58,800,000 VND (exact match).
    let tolerance = dec!(100);
    let recon = create_recon_record(
        &repo,
        &batch.id,
        tenant,
        dec!(58_800_000),
        dec!(58_800_000),
        tolerance,
    );
    assert!(recon.matched);
    assert_eq!(recon.discrepancy, dec!(0));

    // Verify no unmatched records.
    let unmatched = repo.list_unmatched_recon();
    assert!(unmatched.is_empty(), "No unmatched recon records expected");

    // Verify final batch state.
    let final_batch = repo.get_batch(&batch.id).unwrap();
    assert_eq!(final_batch.status, SettlementStatus::Completed);
}

#[test]
fn test_full_lifecycle_with_failure_and_retry() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();
    let tenant = "tenantRetryLC";

    // Create a settlement.
    let s = trigger_and_persist(&svc, &repo, &format!("{}_ofr_retry", tenant));

    // Phase 1: Try to process -- fails.
    repo.update_status(&s.id, SettlementStatus::Processing, None)
        .unwrap();
    repo.update_status(
        &s.id,
        SettlementStatus::Failed,
        Some("Bank connection reset".to_string()),
    )
    .unwrap();

    let failed = repo.get_settlement(&s.id).unwrap();
    assert_eq!(failed.status, SettlementStatus::Failed);

    // Phase 2: Retry.
    repo.update_status(&s.id, SettlementStatus::Pending, None)
        .unwrap();
    let retried = repo.get_settlement(&s.id).unwrap();
    assert_eq!(retried.status, SettlementStatus::Pending);

    // Phase 3: Second attempt succeeds.
    repo.update_status(&s.id, SettlementStatus::Processing, None)
        .unwrap();
    repo.update_status(&s.id, SettlementStatus::Completed, None)
        .unwrap();

    let completed = repo.get_settlement(&s.id).unwrap();
    assert_eq!(completed.status, SettlementStatus::Completed);

    // Phase 4: Reconciliation shows slight discrepancy but within tolerance.
    let tolerance = dec!(500);
    let recon = create_recon_record(
        &repo,
        &s.id,
        tenant,
        dec!(10_000_000),
        dec!(9_999_700), // 300 VND short, within 500 VND tolerance
        tolerance,
    );
    assert!(recon.matched);
    assert_eq!(recon.discrepancy, dec!(-300));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_settlement_not_found() {
    let repo = InMemorySettlementRepo::new();

    assert!(repo.get_settlement("nonexistent").is_none());
    let result = repo.update_status("nonexistent", SettlementStatus::Processing, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_batch_not_found() {
    let repo = InMemorySettlementRepo::new();

    assert!(repo.get_batch("nonexistent").is_none());
    let result = repo.update_batch_status("nonexistent", SettlementStatus::Processing, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_settlement_timestamp_ordering() {
    let svc = SettlementService::new();
    let repo = InMemorySettlementRepo::new();

    let s = trigger_and_persist(&svc, &repo, "tenantTime_ofr_1");
    let created = s.created_at;

    // Small delay to ensure timestamps differ.
    std::thread::sleep(std::time::Duration::from_millis(10));

    repo.update_status(&s.id, SettlementStatus::Processing, None)
        .unwrap();
    let after_processing = repo.get_settlement(&s.id).unwrap();
    assert!(
        after_processing.updated_at >= created,
        "updated_at should be >= created_at after Processing"
    );

    std::thread::sleep(std::time::Duration::from_millis(10));

    repo.update_status(&s.id, SettlementStatus::Completed, None)
        .unwrap();
    let after_completed = repo.get_settlement(&s.id).unwrap();
    assert!(
        after_completed.updated_at >= after_processing.updated_at,
        "updated_at should increase after Completed"
    );
}

#[test]
fn test_empty_batch() {
    let repo = InMemorySettlementRepo::new();

    // A batch with zero settlements is technically valid (edge case).
    let batch = create_batch(&repo, "tenantEmpty", vec![], vec![]);

    assert_eq!(batch.settlement_ids.len(), 0);
    assert_eq!(batch.total_gross_amount, dec!(0));
    assert_eq!(batch.total_fees, dec!(0));
    assert_eq!(batch.total_net_amount, dec!(0));
}

#[test]
fn test_settlement_serialization_roundtrip() {
    let svc = SettlementService::new();
    let s = svc.trigger_settlement("ofr_ser_roundtrip").unwrap();

    // Serialize to JSON and back.
    let json = serde_json::to_string(&s).unwrap();
    let deserialized: Settlement = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, s.id);
    assert_eq!(deserialized.offramp_intent_id, s.offramp_intent_id);
    assert_eq!(deserialized.status, s.status);
    assert_eq!(deserialized.bank_reference, s.bank_reference);
    assert_eq!(deserialized.error_message, s.error_message);
}

#[test]
fn test_settlement_status_serialization() {
    // Verify serde rename_all = "SCREAMING_SNAKE_CASE".
    let pending_json = serde_json::to_string(&SettlementStatus::Pending).unwrap();
    assert_eq!(pending_json, "\"PENDING\"");

    let processing_json = serde_json::to_string(&SettlementStatus::Processing).unwrap();
    assert_eq!(processing_json, "\"PROCESSING\"");

    let completed_json = serde_json::to_string(&SettlementStatus::Completed).unwrap();
    assert_eq!(completed_json, "\"COMPLETED\"");

    let failed_json = serde_json::to_string(&SettlementStatus::Failed).unwrap();
    assert_eq!(failed_json, "\"FAILED\"");

    // Deserialize back.
    let p: SettlementStatus = serde_json::from_str("\"PENDING\"").unwrap();
    assert_eq!(p, SettlementStatus::Pending);

    let c: SettlementStatus = serde_json::from_str("\"COMPLETED\"").unwrap();
    assert_eq!(c, SettlementStatus::Completed);
}
