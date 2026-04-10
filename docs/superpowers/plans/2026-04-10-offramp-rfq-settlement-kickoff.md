# OFFRAMP RFQ Match → Settlement Kickoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn a linked OFFRAMP RFQ into an executable payout path by creating a settlement record at match time and propagating settlement outcomes back into the linked off-ramp intent.

**Architecture:** Keep price selection inside `RfqService`, keep settlement state transitions inside `SettlementService`, and add one thin `LinkedOfframpExecutionService` that coordinates the handoff using the existing `rfq_requests.offramp_id` seam. Persist the execution link in both `settlements` and `offramp_intents` so portal/admin handlers can expose the chain directly without inventing a second read model. Reuse existing `intent.status_changed` and `SettlementService::ingest_reliability_outcome(...)` instead of adding a new event subsystem. Before wiring handlers, extend the existing `OfframpIntentRow`, `SettlementRow`, `Settlement`, and `TriggerSettlementRequest` types so the new fields compile cleanly through repository/service boundaries.

**Tech Stack:** Rust 2021, Axum, sqlx/PostgreSQL, rust_decimal, serde/serde_json, tokio, testcontainers

---

## File Structure

**Create**
- `migrations/064_offramp_rfq_settlement_linkage.sql` — adds settlement linkage columns, tenant scoping, and idempotency indexes.
- `crates/ramp-core/src/service/linked_offramp_execution.rs` — coordinates RFQ finalize → settlement kickoff and settlement outcome → off-ramp terminal state.

**Modify**
- `crates/ramp-core/src/repository/offramp.rs` — persist linked RFQ / winning LP / matched rate / settlement ID on off-ramp intents.
- `crates/ramp-core/src/repository/settlement.rs` — persist tenant / RFQ / LP / final-rate metadata and add `get_by_rfq_id(&TenantId, &str)` lookup.
- `crates/ramp-core/src/service/settlement.rs` — add richer trigger request and idempotent outcome application helper.
- `crates/ramp-core/src/service/mod.rs` — export the new coordinator service.
- `crates/ramp-api/src/handlers/portal/rfq.rs` — call the coordinator instead of raw `RfqService::finalize_rfq(...)`.
- `crates/ramp-api/src/handlers/admin/rfq.rs` — same coordinator for manual finalize.
- `crates/ramp-api/src/handlers/admin/settlement.rs` — add admin settlement-outcome endpoint.
- `crates/ramp-api/src/handlers/portal/offramp.rs` — expose settlement linkage fields on status responses.
- `crates/ramp-api/src/handlers/admin/offramp.rs` — expose settlement linkage fields on admin responses.
- `crates/ramp-api/src/router.rs` — wire the new admin settlement status route.
- `crates/ramp-api/tests/e2e_offramp_test.rs` — add linked RFQ kickoff/outcome/status HTTP+DB tests.
- `crates/ramp-api/tests/e2e_payout_test.rs` — fix stale `BANK_REJECTED` expectation to `REVERSED`.

**Intentionally not touched**
- `crates/ramp-api/src/handlers/admin/offramp.rs` approval state machine logic beyond response fields. This slice should not redesign the manual approval path.
- `crates/ramp-api/src/main.rs` / `AppState`. The existing `db_pool` + `event_publisher` are enough to construct the coordinator inside handlers.
- `crates/ramp-api/src/handlers/admin/settlement.rs` workbench/export behavior. Leave the workbench as-is and add one focused outcome route.

---

### Task 1: Write the failing linked-offramp kickoff test

**Files:**
- Modify: `crates/ramp-api/tests/e2e_offramp_test.rs`
- Test: `crates/ramp-api/tests/e2e_offramp_test.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_portal_accept_linked_offramp_rfq_creates_settlement() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }

    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;
    let tenant_id = TenantId("00000000-0000-0000-0000-000000000001".to_string());
    let user_id = "00000000-0000-0000-0000-000000000002";
    let now = Utc::now();

    PgOfframpIntentRepository::new(pool.clone())
        .create_intent(&OfframpIntentRow {
            id: "ofr_linked_kickoff_1".to_string(),
            tenant_id: tenant_id.0.clone(),
            user_id: user_id.to_string(),
            chain_id: Some(137),
            crypto_asset: "USDT".to_string(),
            crypto_amount: dec!(100),
            exchange_rate: dec!(25000),
            locked_rate_id: Some("lock_kickoff_1".to_string()),
            fees: json!({"total": "10000"}),
            net_vnd_amount: dec!(2490000),
            gross_vnd_amount: dec!(2500000),
            bank_account: json!({
                "bankCode": "VCB",
                "accountNumber": "1234567890",
                "accountName": "Nguyen Van A"
            }),
            deposit_address: Some("0x1111111111111111111111111111111111111111".to_string()),
            tx_hash: Some("0xkickoff001".to_string()),
            bank_reference: None,
            state: "CRYPTO_RECEIVED".to_string(),
            state_history: json!([]),
            created_at: now,
            updated_at: now,
            quote_expires_at: now + chrono::Duration::minutes(10),
            linked_rfq_id: None,
            winning_lp_id: None,
            matched_rate: None,
            settlement_id: None,
        })
        .await
        .unwrap();

    let rfq_repo = Arc::new(PgRfqRepository::new(pool.clone()));
    let event_publisher = Arc::new(InMemoryEventPublisher::new());
    let rfq_service = RfqService::new(rfq_repo.clone(), event_publisher.clone());

    let rfq = rfq_service
        .create_rfq(CreateRfqRequest {
            tenant_id: tenant_id.clone(),
            user_id: user_id.to_string(),
            direction: "OFFRAMP".to_string(),
            offramp_id: Some("ofr_linked_kickoff_1".to_string()),
            crypto_asset: "USDT".to_string(),
            crypto_amount: dec!(100),
            vnd_amount: None,
            ttl_minutes: 5,
        })
        .await
        .unwrap();

    rfq_service
        .submit_bid(SubmitBidRequest {
            tenant_id: tenant_id.clone(),
            rfq_id: rfq.id.clone(),
            lp_id: "lp_alpha_kickoff".to_string(),
            lp_name: Some("Alpha LP".to_string()),
            exchange_rate: dec!(25000),
            vnd_amount: dec!(2500000),
            valid_minutes: 5,
        })
        .await
        .unwrap();

    rfq_service
        .submit_bid(SubmitBidRequest {
            tenant_id: tenant_id.clone(),
            rfq_id: rfq.id.clone(),
            lp_id: "lp_beta_kickoff".to_string(),
            lp_name: Some("Beta LP".to_string()),
            exchange_rate: dec!(25500),
            vnd_amount: dec!(2550000),
            valid_minutes: 5,
        })
        .await
        .unwrap();

    let request = Request::builder()
        .uri(format!("/v1/portal/rfq/{}/accept", rfq.id))
        .method("POST")
        .header("Authorization", format!("Bearer {}", jwt))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let settlement = PgSettlementRepository::new(pool.clone())
        .get_by_rfq_id(&tenant_id, &rfq.id)
        .await
        .unwrap()
        .expect("linked settlement should exist");
    assert_eq!(settlement.winning_lp_id.as_deref(), Some("lp_beta_kickoff"));
    assert_eq!(settlement.final_rate, Some(dec!(25500)));

    let intent = PgOfframpIntentRepository::new(pool.clone())
        .get_intent(&tenant_id, "ofr_linked_kickoff_1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(intent.state, "VND_TRANSFERRING");
    assert_eq!(intent.linked_rfq_id.as_deref(), Some(rfq.id.as_str()));
    assert_eq!(intent.winning_lp_id.as_deref(), Some("lp_beta_kickoff"));
    assert_eq!(intent.matched_rate, Some(dec!(25500)));
    assert_eq!(intent.settlement_id.as_deref(), Some(settlement.id.as_str()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
cargo test -p ramp-api --test e2e_offramp_test test_portal_accept_linked_offramp_rfq_creates_settlement -- --nocapture
```

Expected: FAIL with compile/runtime errors like `unknown field linked_rfq_id`, `no method named get_by_rfq_id`, or missing linked settlement because the kickoff path is not wired yet.

- [ ] **Step 3: Commit the failing test**

```bash
git add crates/ramp-api/tests/e2e_offramp_test.rs
git commit -m "test: capture missing linked offramp settlement kickoff"
```

---

### Task 2: Add linkage schema and repository persistence

**Files:**
- Create: `migrations/064_offramp_rfq_settlement_linkage.sql`
- Modify: `crates/ramp-core/src/repository/offramp.rs`
- Modify: `crates/ramp-core/src/repository/settlement.rs`
- Test: `crates/ramp-api/tests/e2e_offramp_test.rs`

- [ ] **Step 1: Write the migration and row-model changes**

Pre-spawn note: enabling RLS on `settlements` is not just a migration task. Audit every existing settlement repository method and callsite for tenant-context setup before turning the policy on, or existing reads/writes can start failing outside the new RFQ lookup path.

```sql
-- migrations/064_offramp_rfq_settlement_linkage.sql
ALTER TABLE settlements ADD COLUMN tenant_id TEXT;
UPDATE settlements s
SET tenant_id = o.tenant_id
FROM offramp_intents o
WHERE o.id = s.offramp_intent_id
  AND s.tenant_id IS NULL;
ALTER TABLE settlements ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE settlements
    ADD CONSTRAINT settlements_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES tenants(id);
ALTER TABLE settlements ADD COLUMN rfq_id TEXT REFERENCES rfq_requests(id);
ALTER TABLE settlements ADD COLUMN winning_lp_id TEXT;
ALTER TABLE settlements ADD COLUMN final_rate NUMERIC;
CREATE UNIQUE INDEX IF NOT EXISTS idx_settlements_rfq_id_unique
    ON settlements(rfq_id) WHERE rfq_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_settlements_tenant_offramp
    ON settlements(tenant_id, offramp_intent_id, created_at DESC);
ALTER TABLE settlements ENABLE ROW LEVEL SECURITY;
CREATE POLICY settlements_tenant_isolation ON settlements
    USING (tenant_id = current_setting('app.current_tenant', true));

ALTER TABLE offramp_intents ADD COLUMN linked_rfq_id TEXT REFERENCES rfq_requests(id);
ALTER TABLE offramp_intents ADD COLUMN winning_lp_id TEXT;
ALTER TABLE offramp_intents ADD COLUMN matched_rate NUMERIC;
ALTER TABLE offramp_intents ADD COLUMN settlement_id TEXT REFERENCES settlements(id);
CREATE INDEX IF NOT EXISTS idx_offramp_intents_linked_rfq
    ON offramp_intents(linked_rfq_id) WHERE linked_rfq_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_offramp_intents_settlement_id
    ON offramp_intents(settlement_id) WHERE settlement_id IS NOT NULL;
```

```rust
// crates/ramp-core/src/repository/offramp.rs
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OfframpIntentRow {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub chain_id: Option<i64>,
    pub crypto_asset: String,
    pub crypto_amount: Decimal,
    pub exchange_rate: Decimal,
    pub locked_rate_id: Option<String>,
    pub fees: serde_json::Value,
    pub net_vnd_amount: Decimal,
    pub gross_vnd_amount: Decimal,
    pub bank_account: serde_json::Value,
    pub deposit_address: Option<String>,
    pub tx_hash: Option<String>,
    pub bank_reference: Option<String>,
    pub state: String,
    pub state_history: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub quote_expires_at: DateTime<Utc>,
    pub linked_rfq_id: Option<String>,
    pub winning_lp_id: Option<String>,
    pub matched_rate: Option<Decimal>,
    pub settlement_id: Option<String>,
}
```

```rust
// crates/ramp-core/src/repository/settlement.rs
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SettlementRow {
    pub id: String,
    pub tenant_id: String,
    pub offramp_intent_id: String,
    pub rfq_id: Option<String>,
    pub winning_lp_id: Option<String>,
    pub final_rate: Option<Decimal>,
    pub status: String,
    pub bank_reference: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait SettlementRepository: Send + Sync {
    async fn create(&self, row: &SettlementRow) -> Result<()>;
    async fn get_by_id(&self, id: &str) -> Result<Option<SettlementRow>>;
    async fn get_by_rfq_id(&self, tenant_id: &TenantId, rfq_id: &str) -> Result<Option<SettlementRow>>;
    async fn list_by_offramp_in_tenant(
        &self,
        tenant_id: &TenantId,
        offramp_intent_id: &str,
    ) -> Result<Vec<SettlementRow>>;
    async fn update_status(
        &self,
        id: &str,
        new_status: &str,
        error_message: Option<&str>,
    ) -> Result<()>;
}
```

- [ ] **Step 2: Update SQL queries to persist the new columns**

```rust
// crates/ramp-core/src/repository/offramp.rs (create_intent + update_intent)
sqlx::query(
    r#"
    INSERT INTO offramp_intents (
        id, tenant_id, user_id, chain_id, crypto_asset, crypto_amount, exchange_rate,
        locked_rate_id, fees, net_vnd_amount, gross_vnd_amount, bank_account,
        deposit_address, tx_hash, bank_reference, state, state_history,
        created_at, updated_at, quote_expires_at,
        linked_rfq_id, winning_lp_id, matched_rate, settlement_id
    ) VALUES (
        $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
        $13, $14, $15, $16, $17, $18, $19, $20,
        $21, $22, $23, $24
    )
    "#,
)
.bind(&intent.id)
.bind(&intent.tenant_id)
.bind(&intent.user_id)
.bind(intent.chain_id)
.bind(&intent.crypto_asset)
.bind(intent.crypto_amount)
.bind(intent.exchange_rate)
.bind(&intent.locked_rate_id)
.bind(&intent.fees)
.bind(intent.net_vnd_amount)
.bind(intent.gross_vnd_amount)
.bind(&intent.bank_account)
.bind(&intent.deposit_address)
.bind(&intent.tx_hash)
.bind(&intent.bank_reference)
.bind(&intent.state)
.bind(&intent.state_history)
.bind(intent.created_at)
.bind(intent.updated_at)
.bind(intent.quote_expires_at)
.bind(&intent.linked_rfq_id)
.bind(&intent.winning_lp_id)
.bind(intent.matched_rate)
.bind(&intent.settlement_id);
```

```rust
// crates/ramp-core/src/repository/settlement.rs (create + get_by_rfq_id)
sqlx::query(
    r#"
    INSERT INTO settlements (
        id, tenant_id, offramp_intent_id, rfq_id, winning_lp_id, final_rate,
        status, bank_reference, error_message, created_at, updated_at
    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    "#,
)
.bind(&row.id)
.bind(&row.tenant_id)
.bind(&row.offramp_intent_id)
.bind(&row.rfq_id)
.bind(&row.winning_lp_id)
.bind(row.final_rate)
.bind(&row.status)
.bind(&row.bank_reference)
.bind(&row.error_message)
.bind(row.created_at)
.bind(row.updated_at);

async fn get_by_rfq_id(&self, tenant_id: &TenantId, rfq_id: &str) -> Result<Option<SettlementRow>> {
    let mut tx = self.pool.begin().await.map_err(|e| Error::Database(e.to_string()))?;
    set_rls_context(&mut tx, tenant_id).await.map_err(|e| Error::Database(e.to_string()))?;

    let row = sqlx::query_as::<_, SettlementRow>(
        r#"
        SELECT * FROM settlements
        WHERE tenant_id = $1 AND rfq_id = $2
        LIMIT 1
        "#,
    )
    .bind(&tenant_id.0)
    .bind(rfq_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;

    tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
    Ok(row)
}
```

- [ ] **Step 3: Run the failing kickoff test again**

Run:
```bash
cargo test -p ramp-api --test e2e_offramp_test test_portal_accept_linked_offramp_rfq_creates_settlement -- --nocapture
```

Expected: still FAIL, but not necessarily only for business-logic reasons. At this point type propagation gaps, RLS/context issues, or service-integration wiring can still fail before the kickoff assertion passes.

- [ ] **Step 4: Commit the persistence layer**

```bash
git add migrations/064_offramp_rfq_settlement_linkage.sql crates/ramp-core/src/repository/offramp.rs crates/ramp-core/src/repository/settlement.rs
git commit -m "feat: persist linked offramp settlement metadata"
```

---

### Task 3: Implement the RFQ finalize → settlement kickoff coordinator

**Files:**
- Create: `crates/ramp-core/src/service/linked_offramp_execution.rs`
- Modify: `crates/ramp-core/src/service/settlement.rs`
- Modify: `crates/ramp-core/src/service/mod.rs`
- Modify: `crates/ramp-api/src/handlers/portal/rfq.rs`
- Modify: `crates/ramp-api/src/handlers/admin/rfq.rs`
- Test: `crates/ramp-api/tests/e2e_offramp_test.rs`

- [ ] **Step 1: Add the new coordinator service**

```rust
// crates/ramp-core/src/service/linked_offramp_execution.rs
use chrono::Utc;
use std::sync::Arc;

use ramp_common::{types::{IntentId, TenantId}, Error, Result};
use serde_json::json;

use crate::event::EventPublisher;
use crate::repository::{
    OfframpIntentRepository, OfframpIntentRow, RfqRepository, SettlementRepository,
};
use crate::service::rfq::{FinalizeResult, RfqService};
use crate::service::settlement::{Settlement, SettlementService, SettlementStatus, TriggerSettlementRequest};

pub struct LinkedOfframpExecutionResult {
    pub rfq: crate::repository::RfqRequestRow,
    pub winning_bid: crate::repository::RfqBidRow,
    pub settlement: Settlement,
    pub offramp: OfframpIntentRow,
}

pub struct LinkedOfframpExecutionService {
    rfq_service: RfqService,
    rfq_repo: Arc<dyn RfqRepository>,
    offramp_repo: Arc<dyn OfframpIntentRepository>,
    settlement_repo: Arc<dyn SettlementRepository>,
    settlement_service: SettlementService,
    event_publisher: Arc<dyn EventPublisher>,
}

impl LinkedOfframpExecutionService {
    pub fn new(
        rfq_service: RfqService,
        rfq_repo: Arc<dyn RfqRepository>,
        offramp_repo: Arc<dyn OfframpIntentRepository>,
        settlement_repo: Arc<dyn SettlementRepository>,
        settlement_service: SettlementService,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            rfq_service,
            rfq_repo,
            offramp_repo,
            settlement_repo,
            settlement_service,
            event_publisher,
        }
    }

    pub async fn finalize_linked_offramp_rfq(
        &self,
        tenant_id: &TenantId,
        rfq_id: &str,
    ) -> Result<LinkedOfframpExecutionResult> {
        let existing_rfq = self
            .rfq_repo
            .get_request(tenant_id, rfq_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("RFQ {} not found", rfq_id)))?;

        if existing_rfq.direction != "OFFRAMP" {
            return Err(Error::Validation("Only OFFRAMP RFQs can trigger linked settlement".to_string()));
        }

        let finalize_result = if existing_rfq.state == "OPEN" {
            self.rfq_service.finalize_rfq(tenant_id, rfq_id).await?
        } else if existing_rfq.state == "MATCHED" {
            let winning_bid = self
                .rfq_repo
                .list_bids_for_request(tenant_id, rfq_id)
                .await?
                .into_iter()
                .find(|bid| bid.state == "ACCEPTED")
                .ok_or_else(|| Error::Conflict(format!("RFQ {} is matched but has no accepted bid", rfq_id)))?;

            FinalizeResult {
                rfq: existing_rfq.clone(),
                winning_bid,
            }
        } else {
            return Err(Error::Conflict(format!(
                "RFQ {} cannot kick off settlement from state {}",
                rfq_id, existing_rfq.state
            )));
        };

        let offramp_id = finalize_result
            .rfq
            .offramp_id
            .clone()
            .ok_or_else(|| Error::Validation("Matched OFFRAMP RFQ is missing offramp_id".to_string()))?;

        let mut intent = self
            .offramp_repo
            .get_intent(tenant_id, &offramp_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Linked off-ramp {} not found", offramp_id)))?;

        let settlement = if let Some(existing) = self.settlement_repo.get_by_rfq_id(tenant_id, rfq_id).await? {
            Settlement::from_row(existing)?
        } else {
            self.settlement_service
                .trigger_settlement_async(TriggerSettlementRequest {
                    tenant_id: tenant_id.clone(),
                    offramp_intent_id: intent.id.clone(),
                    rfq_id: Some(finalize_result.rfq.id.clone()),
                    winning_lp_id: Some(finalize_result.winning_bid.lp_id.clone()),
                    final_rate: Some(finalize_result.winning_bid.exchange_rate),
                    bank_reference: None,
                })
                .await?
        };

        if intent.state == "CRYPTO_RECEIVED" {
            intent.state_history = append_state_transition(
                intent.state_history.clone(),
                "CRYPTO_RECEIVED",
                "VND_TRANSFERRING",
                Some("Settlement kickoff created bank transfer work item"),
            );
            intent.state = "VND_TRANSFERRING".to_string();
            intent.bank_reference = settlement.bank_reference.clone();
            intent.updated_at = Utc::now();
        }

        // Note: introducing an intermediate `CONVERTING` state is optional/new.
        // Do not add it unless the team intentionally wants to expand the state machine
        // and update all downstream handlers/tests/docs accordingly.

        intent.linked_rfq_id = Some(finalize_result.rfq.id.clone());
        intent.winning_lp_id = Some(finalize_result.winning_bid.lp_id.clone());
        intent.matched_rate = Some(finalize_result.winning_bid.exchange_rate);
        intent.settlement_id = Some(settlement.id.clone());
        self.offramp_repo.update_intent(&intent).await?;

        let _ = self
            .event_publisher
            .publish_intent_status_changed(&IntentId::new(&intent.id), tenant_id, &intent.state)
            .await;

        Ok(LinkedOfframpExecutionResult {
            rfq: finalize_result.rfq,
            winning_bid: finalize_result.winning_bid,
            settlement,
            offramp: intent,
        })
    }
}

fn append_state_transition(
    mut history: serde_json::Value,
    from: &str,
    to: &str,
    reason: Option<&str>,
) -> serde_json::Value {
    let Some(arr) = history.as_array_mut() else {
        return json!([{ "from": from, "to": to, "timestamp": Utc::now().to_rfc3339(), "reason": reason }]);
    };

    arr.push(json!({
        "from": from,
        "to": to,
        "timestamp": Utc::now().to_rfc3339(),
        "reason": reason,
    }));
    history
}
```

- [ ] **Step 2: Teach `SettlementService` to create rich settlement rows**

```rust
// crates/ramp-core/src/service/settlement.rs
#[derive(Debug, Clone)]
pub struct TriggerSettlementRequest {
    pub tenant_id: TenantId,
    pub offramp_intent_id: String,
    pub rfq_id: Option<String>,
    pub winning_lp_id: Option<String>,
    pub final_rate: Option<Decimal>,
    pub bank_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    pub id: String,
    pub tenant_id: String,
    pub offramp_intent_id: String,
    pub rfq_id: Option<String>,
    pub winning_lp_id: Option<String>,
    pub final_rate: Option<Decimal>,
    pub status: SettlementStatus,
    pub bank_reference: Option<String>,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

pub async fn trigger_settlement_async(&self, req: TriggerSettlementRequest) -> Result<Settlement> {
    let now = Utc::now();
    let settlement = Settlement {
        id: format!("stl_{}", Uuid::now_v7()),
        tenant_id: req.tenant_id.0,
        offramp_intent_id: req.offramp_intent_id,
        rfq_id: req.rfq_id,
        winning_lp_id: req.winning_lp_id,
        final_rate: req.final_rate,
        status: SettlementStatus::Processing,
        bank_reference: req.bank_reference.or_else(|| Some(format!("RAMP-{}", &Uuid::now_v7().to_string()[..8].to_uppercase()))),
        error_message: None,
        created_at: now,
        updated_at: now,
    };

    if let Some(repo) = &self.repo {
        repo.create(&settlement.to_row()).await?;
    }

    self.store
        .lock()
        .map_err(|e| Error::Internal(format!("Settlement store lock poisoned: {}", e)))?
        .insert(settlement.id.clone(), settlement.clone());

    Ok(settlement)
}
```

- [ ] **Step 3: Wire both RFQ handlers through the coordinator**

```rust
// crates/ramp-api/src/handlers/portal/rfq.rs
fn make_linked_execution_service(pool: sqlx::PgPool, state: &AppState) -> LinkedOfframpExecutionService {
    let rfq_repo: Arc<dyn RfqRepository> = Arc::new(PgRfqRepository::new(pool.clone()));
    let offramp_repo: Arc<dyn OfframpIntentRepository> = Arc::new(PgOfframpIntentRepository::new(pool.clone()));
    let settlement_repo: Arc<dyn SettlementRepository> = Arc::new(PgSettlementRepository::new(pool.clone()));
    let rfq_service = RfqService::new(rfq_repo.clone(), state.event_publisher.clone());
    let settlement_service = SettlementService::with_repository(settlement_repo.clone());

    LinkedOfframpExecutionService::new(
        rfq_service,
        rfq_repo,
        offramp_repo,
        settlement_repo,
        settlement_service,
        state.event_publisher.clone(),
    )
}

let svc = make_linked_execution_service(pool.clone(), &app_state);
let result = svc
    .finalize_linked_offramp_rfq(&tenant_id, &id)
    .await
    .map_err(ApiError::from)?;
```

```rust
// crates/ramp-api/src/handlers/admin/rfq.rs
let svc = make_linked_execution_service(pool, &app_state);
let result = svc
    .finalize_linked_offramp_rfq(&tenant_id, &id)
    .await
    .map_err(ApiError::from)?;
```

```rust
// crates/ramp-core/src/service/mod.rs
pub mod linked_offramp_execution;
pub use linked_offramp_execution::{LinkedOfframpExecutionResult, LinkedOfframpExecutionService};
```

- [ ] **Step 4: Run the kickoff test to verify it passes**

Run:
```bash
cargo test -p ramp-api --test e2e_offramp_test test_portal_accept_linked_offramp_rfq_creates_settlement -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit the kickoff path**

```bash
git add crates/ramp-core/src/service/linked_offramp_execution.rs crates/ramp-core/src/service/settlement.rs crates/ramp-core/src/service/mod.rs crates/ramp-api/src/handlers/portal/rfq.rs crates/ramp-api/src/handlers/admin/rfq.rs
git commit -m "feat: kick off settlement from matched offramp rfqs"
```

---

### Task 4: Apply settlement outcomes back to the linked off-ramp

**Files:**
- Modify: `crates/ramp-core/src/service/linked_offramp_execution.rs`
- Modify: `crates/ramp-core/src/service/settlement.rs`
- Modify: `crates/ramp-api/src/handlers/admin/settlement.rs`
- Modify: `crates/ramp-api/src/router.rs`
- Test: `crates/ramp-api/tests/e2e_offramp_test.rs`

- [ ] **Step 1: Write the failing outcome test**

```rust
#[tokio::test]
async fn test_admin_settlement_outcome_completes_linked_offramp() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }

    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);

    let pool = setup_db().await;
    let (app, _api_key, jwt) = build_test_app(pool.clone()).await;
    let tenant_id = TenantId("00000000-0000-0000-0000-000000000001".to_string());
    let admin_jwt = build_admin_jwt("operator");

    // Build this test's own linked-offramp fixture in the same pool/app.
    // Do not call another test function here.
    seed_linked_offramp_fixture(&pool, &tenant_id, "ofr_linked_outcome_1", "rfq_linked_outcome_1", "lp_beta_outcome_1").await;
    accept_linked_rfq_via_portal(app.clone(), jwt.clone(), "rfq_linked_outcome_1").await;

    let settlement = PgSettlementRepository::new(pool.clone())
        .get_by_rfq_id(&tenant_id, "rfq_linked_outcome_1")
        .await
        .unwrap()
        .unwrap();

    let request = Request::builder()
        .uri(format!("/v1/admin/settlement/{}/status", settlement.id))
        .method("POST")
        .header("Authorization", "Bearer offramp_api_key")
        .header("X-Admin-Authorization", format!("Bearer {}", admin_jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({ "status": "COMPLETED" }).to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let updated_intent = PgOfframpIntentRepository::new(pool.clone())
        .get_intent(&tenant_id, "ofr_linked_outcome_1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_intent.state, "COMPLETED");

    let latest_snapshot = PgRfqRepository::new(pool.clone())
        .get_latest_reliability_snapshot(&tenant_id, "lp_beta_outcome_1", "OFFRAMP", "ROLLING_30D")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest_snapshot.settlement_count, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
cargo test -p ramp-api --test e2e_offramp_test test_admin_settlement_outcome_completes_linked_offramp -- --nocapture
```

Expected: FAIL with 404 on `/v1/admin/settlement/:id/status` or unchanged off-ramp state/reliability snapshot.

- [ ] **Step 3: Implement the outcome route and coordinator callback**

```rust
// crates/ramp-api/src/handlers/admin/settlement.rs
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplySettlementOutcomeRequest {
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementOutcomeResponse {
    pub settlement_id: String,
    pub offramp_intent_id: String,
    pub settlement_status: String,
    pub offramp_state: String,
}

pub async fn apply_settlement_outcome(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ApplySettlementOutcomeRequest>,
) -> Result<Json<SettlementOutcomeResponse>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Settlement service unavailable".to_string()))?
        .clone();
    let tenant_id = TenantId(tenant_ctx.tenant_id.0.clone());

    let status = match req.status.as_str() {
        "COMPLETED" => SettlementStatus::Completed,
        "FAILED" => SettlementStatus::Failed,
        other => return Err(ApiError::Validation(format!("Unsupported settlement status {}", other))),
    };

    let svc = make_linked_execution_service(pool, &app_state);
    let result = svc
        .apply_settlement_outcome(&tenant_id, &id, status, req.error_message)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(SettlementOutcomeResponse {
        settlement_id: result.settlement.id,
        offramp_intent_id: result.offramp.id,
        settlement_status: result.settlement.status.as_db_str().to_string(),
        offramp_state: result.offramp.state,
    }))
}
```

```rust
// crates/ramp-core/src/service/linked_offramp_execution.rs
pub async fn apply_settlement_outcome(
    &self,
    tenant_id: &TenantId,
    settlement_id: &str,
    status: SettlementStatus,
    error_message: Option<String>,
) -> Result<LinkedOfframpExecutionResult> {
    let settlement = self
        .settlement_service
        .apply_outcome_async(settlement_id, status.clone(), error_message.as_deref())
        .await?;

    let rfq_id = settlement
        .rfq_id
        .clone()
        .ok_or_else(|| Error::Validation("Linked settlement is missing rfq_id".to_string()))?;
    let rfq = self
        .rfq_repo
        .get_request(tenant_id, &rfq_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("RFQ {} not found", rfq_id)))?;

    let winning_bid = self
        .rfq_repo
        .list_bids_for_request(tenant_id, &rfq.id)
        .await?
        .into_iter()
        .find(|bid| bid.state == "ACCEPTED")
        .ok_or_else(|| Error::Conflict(format!("RFQ {} has no accepted bid", rfq.id)))?;

    let mut intent = self
        .offramp_repo
        .get_intent(tenant_id, &settlement.offramp_intent_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Off-ramp {} not found", settlement.offramp_intent_id)))?;

    let target_state = match settlement.status {
        SettlementStatus::Completed => "COMPLETED",
        SettlementStatus::Failed => "FAILED",
        SettlementStatus::Pending | SettlementStatus::Processing => return Err(Error::Validation("Settlement outcome must be terminal".to_string())),
    };

    if intent.state != target_state {
        intent.state_history = append_state_transition(
            intent.state_history.clone(),
            &intent.state,
            target_state,
            error_message.as_deref().or(Some("Settlement outcome applied")),
        );
        intent.state = target_state.to_string();
        intent.updated_at = Utc::now();
        self.offramp_repo.update_intent(&intent).await?;
        let _ = self
            .event_publisher
            .publish_intent_status_changed(&IntentId::new(&intent.id), tenant_id, &intent.state)
            .await;
    }

    self.settlement_service
        .ingest_reliability_outcome(
            self.rfq_repo.clone(),
            tenant_id,
            winning_bid.lp_id.as_str(),
            "OFFRAMP",
            &settlement,
        )
        .await?;

    Ok(LinkedOfframpExecutionResult {
        rfq,
        winning_bid,
        settlement,
        offramp: intent,
    })
}
```

```rust
// crates/ramp-core/src/service/settlement.rs
pub async fn apply_outcome_async(
    &self,
    id: &str,
    status: SettlementStatus,
    error_message: Option<&str>,
) -> Result<Settlement> {
    let current = self.check_settlement_status_async(id).await?;

    if current.status == status {
        return Ok(current);
    }

    if matches!(current.status, SettlementStatus::Completed | SettlementStatus::Failed) {
        return Err(Error::Conflict(format!(
            "Settlement {} is already terminal in status {}",
            id, current.status.as_db_str()
        )));
    }

    self.update_settlement_status_async(id, status).await?;
    if let Some(repo) = &self.repo {
        repo.update_status(id, status.as_db_str(), error_message).await?;
    }
    self.check_settlement_status_async(id).await
}
```

```rust
// crates/ramp-api/src/router.rs
.route(
    "/settlement/:id/status",
    post(handlers::admin::settlement::apply_settlement_outcome),
)
```

- [ ] **Step 4: Run the outcome test to verify it passes**

Run:
```bash
cargo test -p ramp-api --test e2e_offramp_test test_admin_settlement_outcome_completes_linked_offramp -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit the outcome propagation**

```bash
git add crates/ramp-core/src/service/linked_offramp_execution.rs crates/ramp-core/src/service/settlement.rs crates/ramp-api/src/handlers/admin/settlement.rs crates/ramp-api/src/router.rs
git commit -m "feat: apply settlement outcomes to linked offramps"
```

---

### Task 5: Add replay safety and expose the linkage in status responses

**Files:**
- Modify: `crates/ramp-core/src/service/settlement.rs`
- Modify: `crates/ramp-api/src/handlers/portal/offramp.rs`
- Modify: `crates/ramp-api/src/handlers/admin/offramp.rs`
- Modify: `crates/ramp-api/tests/e2e_offramp_test.rs`
- Test: `crates/ramp-core/src/service/settlement.rs`
- Test: `crates/ramp-api/tests/e2e_offramp_test.rs`

- [ ] **Step 1: Write the replay/idempotency tests**

```rust
// crates/ramp-core/src/service/settlement.rs
#[tokio::test]
async fn test_apply_outcome_async_is_idempotent_for_replayed_completed() {
    use crate::repository::settlement::InMemorySettlementRepository;

    let repo = Arc::new(InMemorySettlementRepository::new());
    let svc = SettlementService::with_repository(repo);
    let settlement = svc
        .trigger_settlement_async(TriggerSettlementRequest {
            tenant_id: TenantId::new("tenant_replay"),
            offramp_intent_id: "ofr_replay".to_string(),
            rfq_id: Some("rfq_replay".to_string()),
            winning_lp_id: Some("lp_replay".to_string()),
            final_rate: Some(Decimal::from(25500)),
            bank_reference: Some("RAMP-REPLAY".to_string()),
        })
        .await
        .unwrap();

    let first = svc
        .apply_outcome_async(&settlement.id, SettlementStatus::Completed, None)
        .await
        .unwrap();
    let second = svc
        .apply_outcome_async(&settlement.id, SettlementStatus::Completed, None)
        .await
        .unwrap();

    assert_eq!(first.status, SettlementStatus::Completed);
    assert_eq!(second.status, SettlementStatus::Completed);
}
```

```rust
// crates/ramp-api/tests/e2e_offramp_test.rs
#[tokio::test]
async fn test_replayed_failed_callback_does_not_reopen_completed_linked_offramp() {
    if !docker_available() {
        eprintln!("Skipping e2e_offramp_test: Docker daemon unavailable");
        return;
    }

    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);

    let pool = setup_db().await;
    let (app, _api_key, _jwt) = build_test_app(pool.clone()).await;
    let tenant_id = TenantId("00000000-0000-0000-0000-000000000001".to_string());
    let admin_jwt = build_admin_jwt("operator");

    // Build and complete this test's own linked settlement fixture in the same pool/app.
    // Do not rely on another test having run, and do not use hard-coded IDs that were never seeded.
    let seeded = seed_completed_linked_settlement_fixture(
        &pool,
        &tenant_id,
        "ofr_replayed_terminal",
        "rfq_replayed_terminal",
        "lp_replayed_terminal",
    )
    .await;
    let settlement_id = seeded.settlement_id;

    let request = Request::builder()
        .uri(format!("/v1/admin/settlement/{}/status", settlement_id))
        .method("POST")
        .header("Authorization", "Bearer offramp_api_key")
        .header("X-Admin-Authorization", format!("Bearer {}", admin_jwt))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "status": "FAILED",
            "errorMessage": "replayed failure after completion"
        }).to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
cargo test -p ramp-core test_apply_outcome_async_is_idempotent_for_replayed_completed -- --nocapture
cargo test -p ramp-api --test e2e_offramp_test test_replayed_failed_callback_does_not_reopen_completed_linked_offramp -- --nocapture
```

Expected: first FAILS because repeated identical outcomes are not idempotent yet; second FAILS because the route/service allows an invalid terminal-state replay or returns the wrong status.

- [ ] **Step 3: Surface linkage fields on portal/admin responses**

```rust
// crates/ramp-api/src/handlers/portal/offramp.rs
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfframpIntentResponse {
    pub id: String,
    pub state: String,
    pub crypto_asset: String,
    pub crypto_amount: String,
    pub exchange_rate: String,
    pub net_vnd_amount: String,
    pub gross_vnd_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_rfq_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winning_lp_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn map_intent_response(intent: &OfframpIntentRow) -> OfframpIntentResponse {
    OfframpIntentResponse {
        id: intent.id.clone(),
        state: intent.state.clone(),
        crypto_asset: intent.crypto_asset.clone(),
        crypto_amount: intent.crypto_amount.to_string(),
        exchange_rate: intent.exchange_rate.to_string(),
        net_vnd_amount: intent.net_vnd_amount.to_string(),
        gross_vnd_amount: intent.gross_vnd_amount.to_string(),
        deposit_address: intent.deposit_address.clone(),
        chain_id: intent.chain_id,
        tx_hash: intent.tx_hash.clone(),
        bank_reference: intent.bank_reference.clone(),
        linked_rfq_id: intent.linked_rfq_id.clone(),
        winning_lp_id: intent.winning_lp_id.clone(),
        matched_rate: intent.matched_rate.map(|rate| rate.to_string()),
        settlement_id: intent.settlement_id.clone(),
        created_at: intent.created_at.to_rfc3339(),
        updated_at: intent.updated_at.to_rfc3339(),
    }
}
```

```rust
// crates/ramp-api/src/handlers/admin/offramp.rs
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminOfframpResponse {
    pub id: String,
    pub user_id: String,
    pub state: String,
    pub crypto_asset: String,
    pub crypto_amount: String,
    pub exchange_rate: String,
    pub net_vnd_amount: String,
    pub gross_vnd_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_rfq_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winning_lp_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 4: Run the replay + visibility checks**

Run:
```bash
cargo test -p ramp-core test_apply_outcome_async_is_idempotent_for_replayed_completed -- --nocapture
cargo test -p ramp-api --test e2e_offramp_test test_portal_accept_linked_offramp_rfq_creates_settlement -- --nocapture
cargo test -p ramp-api --test e2e_offramp_test test_admin_settlement_outcome_completes_linked_offramp -- --nocapture
cargo test -p ramp-api --test e2e_offramp_test test_replayed_failed_callback_does_not_reopen_completed_linked_offramp -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit replay safety and status visibility**

```bash
git add crates/ramp-core/src/service/settlement.rs crates/ramp-api/src/handlers/portal/offramp.rs crates/ramp-api/src/handlers/admin/offramp.rs crates/ramp-api/tests/e2e_offramp_test.rs
git commit -m "feat: expose linked offramp settlement state safely"
```

---

### Task 6: Fix the stale payout rejection regression

**Files:**
- Modify: `crates/ramp-api/tests/e2e_payout_test.rs`
- Test: `crates/ramp-api/tests/e2e_payout_test.rs`

- [ ] **Step 1: Add the correct regression assertion**

```rust
#[tokio::test]
async fn test_payout_bank_rejection_transitions_to_reversed() {
    let app = setup_app().await;
    let amount = 500_000i64;

    app.ledger_repo.set_balance(
        &TenantId::new(&app.tenant_id),
        Some(&UserId::new(&app.user_id)),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        Decimal::from(1_000_000),
    );

    // Existing request-building code from test_payout_bank_rejection stays the same.
    // Only the terminal-state expectation changes.

    let intents = app.intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == intent_id_str).unwrap();
    assert_eq!(intent.state, "REVERSED");

    let events = app.event_publisher.get_events().await;
    let reversed_event = events.iter().find(|e| {
        e.get("type").and_then(|v| v.as_str()) == Some("intent.status_changed")
            && e.get("new_status").and_then(|v| v.as_str()) == Some("REVERSED")
    });
    assert!(reversed_event.is_some());
}
```

- [ ] **Step 2: Run the payout regression test**

Run:
```bash
cargo test -p ramp-api --test e2e_payout_test test_payout_bank_rejection -- --nocapture
```

Expected: FAIL until the stale `BANK_REJECTED` expectation is replaced.

- [ ] **Step 3: Replace the stale assertion in the existing test**

```rust
assert_eq!(intent.state, "REVERSED");

let reject_event = events.iter().find(|e| {
    e.get("type").and_then(|v| v.as_str()) == Some("intent.status_changed")
        && e.get("new_status").and_then(|v| v.as_str()) == Some("REVERSED")
});
assert!(reject_event.is_some());
```

- [ ] **Step 4: Run the targeted regression suite**

Run:
```bash
cargo test -p ramp-api --test e2e_payout_test test_payout_bank_rejection -- --nocapture
cargo test -p ramp-api --test e2e_offramp_test test_portal_accept_linked_offramp_rfq_creates_settlement -- --nocapture
cargo test -p ramp-api --test e2e_offramp_test test_admin_settlement_outcome_completes_linked_offramp -- --nocapture
cargo test -p ramp-api --test e2e_offramp_test test_replayed_failed_callback_does_not_reopen_completed_linked_offramp -- --nocapture
```

Expected: PASS locally with Docker available; if Docker is unavailable, the off-ramp tests should print the existing skip message and exit cleanly.

- [ ] **Step 5: Commit the regression cleanup**

```bash
git add crates/ramp-api/tests/e2e_payout_test.rs
git commit -m "test: align payout rejection with reversed terminal state"
```

---

## Final verification checkpoint

- [ ] `cargo test -p ramp-core test_apply_outcome_async_is_idempotent_for_replayed_completed -- --nocapture`
- [ ] `cargo test -p ramp-api --test e2e_offramp_test test_portal_accept_linked_offramp_rfq_creates_settlement -- --nocapture`
- [ ] `cargo test -p ramp-api --test e2e_offramp_test test_admin_settlement_outcome_completes_linked_offramp -- --nocapture`
- [ ] `cargo test -p ramp-api --test e2e_offramp_test test_replayed_failed_callback_does_not_reopen_completed_linked_offramp -- --nocapture`
- [ ] `cargo test -p ramp-api --test e2e_payout_test test_payout_bank_rejection -- --nocapture`
- [ ] Portal off-ramp status now exposes `linkedRfqId`, `winningLpId`, `matchedRate`, and `settlementId` once a linked RFQ is matched.
- [ ] Admin settlement outcome endpoint moves the linked off-ramp to `COMPLETED` or `FAILED` and updates LP reliability exactly once.

## Risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `rfq.matched` succeeds but settlement row creation fails | RFQ is matched but off-ramp stays disconnected | Make `finalize_linked_offramp_rfq(...)` idempotent for `MATCHED` RFQs and key settlement lookup by `rfq_id` before creating a new row |
| Replayed settlement callback reopens a terminal off-ramp | Double-application corrupts operator state | `SettlementService::apply_outcome_async(...)` must return the current row for identical replay and reject conflicting terminal replays |
| Portal/admin responses hide the new linkage | Ops still need to grep the DB | Persist linkage fields on `offramp_intents` and expose them on existing status responses |
| This slice accidentally redesigns manual admin approval | Scope creep | Leave `/v1/admin/offramp/:id/approve` behavior untouched except for response shape |

## Self-review

**Spec coverage:** This plan covers the handoff seam, settlement creation, outcome propagation, idempotency/replay protection, visibility on existing surfaces, and the stale payout regression drift discovered during review.

**Placeholder scan:** No `TODO`, `TBD`, or “handle appropriately” placeholders remain. Each task has specific files, code blocks, commands, and expected outcomes.

**Type consistency:** The plan uses one coordinator name (`LinkedOfframpExecutionService`), one settlement creation request (`TriggerSettlementRequest`), and one outcome method (`apply_outcome_async(...)`) across all tasks.
