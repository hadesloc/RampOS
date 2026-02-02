# Integration Testing Guide

This document covers end-to-end (E2E) and integration testing for RampOS.

## Overview

Integration tests verify that multiple components work together correctly. RampOS uses:

- **Testcontainers**: For spinning up PostgreSQL in tests
- **Mock services**: For external dependencies
- **In-memory publishers**: For event verification

## Test Structure

Integration tests are located in:
- `crates/ramp-api/tests/e2e_tests.rs` - Complete flow tests
- `crates/ramp-api/tests/e2e_payin_test.rs` - Payin flow with real DB
- `crates/ramp-api/tests/e2e_payout_test.rs` - Payout flow with real DB
- `crates/ramp-api/tests/integration_tests.rs` - API integration tests
- `crates/ramp-compliance/tests/integration_tests.rs` - Compliance integration

## E2E Test Setup

### Using Testcontainers

```rust
use testcontainers::clients;
use testcontainers_modules::postgres::Postgres;
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn test_e2e_flow() {
    // 1. Start PostgreSQL container
    let docker = clients::Cli::default();
    let pg_container = docker.run(Postgres::default());
    let pg_port = pg_container.get_host_port_ipv4(5432);
    let db_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        pg_port
    );

    // 2. Create connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    // 3. Run migrations
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // 4. Setup repositories and services
    let intent_repo = Arc::new(PgIntentRepository::new(pool.clone()));
    let ledger_repo = Arc::new(PgLedgerRepository::new(pool.clone()));
    // ... more setup

    // 5. Run tests
    // ...
}
```

### Test Context Pattern

```rust
struct TestContext {
    app: Router,
    intent_repo: Arc<MockIntentRepository>,
    ledger_repo: Arc<MockLedgerRepository>,
    event_publisher: Arc<InMemoryEventPublisher>,
    payout_service: Arc<PayoutService>,
    api_key: String,
    tenant_id: String,
    user_id: String,
}

async fn setup_test_app() -> TestContext {
    // Setup repositories
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // Setup tenant with API key
    let tenant_id = "tenant_test_123".to_string();
    let api_key = "test_api_key";

    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: tenant_id.clone(),
        name: "Test Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash,
        // ... other fields
    });

    // Setup user
    let user_id = "user_test_123".to_string();
    user_repo.add_user(UserRow {
        id: user_id.clone(),
        tenant_id: tenant_id.clone(),
        status: "ACTIVE".to_string(),
        kyc_tier: 1,
        kyc_status: "VERIFIED".to_string(),
        // ... other fields
    });

    // Setup services
    let payin_service = Arc::new(PayinService::new(
        intent_repo.clone(),
        ledger_repo.clone(),
        user_repo.clone(),
        event_publisher.clone(),
    ));
    // ... more services

    let app_state = AppState {
        payin_service,
        payout_service: payout_service.clone(),
        // ... other services
    };

    let app = create_router(app_state);

    TestContext {
        app,
        intent_repo,
        ledger_repo,
        event_publisher,
        payout_service,
        api_key: api_key.to_string(),
        tenant_id,
        user_id,
    }
}
```

## E2E Flow Tests

### Complete Payin Flow

```rust
#[tokio::test]
async fn test_payin_complete_flow() {
    let ctx = setup_test_app().await;
    let amount = 1_000_000i64;

    // Step 1: Create pay-in intent
    let create_request = json!({
        "tenantId": ctx.tenant_id,
        "userId": ctx.user_id,
        "amountVnd": amount,
        "railsProvider": "VIETCOMBANK",
        "metadata": { "source": "e2e_test" }
    });

    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", ctx.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&create_request).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await.unwrap();
    let intent_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let intent_id = intent_resp["intentId"].as_str().unwrap().to_string();
    let reference_code = intent_resp["referenceCode"].as_str().unwrap().to_string();

    assert_eq!(intent_resp["status"], "INSTRUCTION_ISSUED");

    // Step 2: Simulate bank confirmation
    let confirm_request = json!({
        "tenantId": ctx.tenant_id,
        "referenceCode": reference_code,
        "status": "FUNDS_CONFIRMED",
        "bankTxId": format!("BANK_TX_{}", Uuid::now_v7()),
        "amountVnd": amount,
        "settledAt": Utc::now().to_rfc3339(),
        "rawPayloadHash": "abc123def456"
    });

    let request = Request::builder()
        .uri("/v1/intents/payin/confirm")
        .method("POST")
        .header("Authorization", format!("Bearer {}", ctx.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&confirm_request).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Step 3: Verify intent final state
    let intents = ctx.intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == intent_id).unwrap();
    assert_eq!(intent.state, "COMPLETED");

    // Step 4: Verify ledger entries
    let txs = ctx.ledger_repo.transactions.lock().unwrap();
    assert_eq!(txs.len(), 1);

    let has_credit = txs[0].entries.iter().any(|e|
        e.account_type == AccountType::LiabilityUserVnd &&
        e.direction == EntryDirection::Credit &&
        e.amount == Decimal::from(amount)
    );
    assert!(has_credit, "Should have credited user liability");

    // Step 5: Verify webhook event
    let events = ctx.event_publisher.get_events().await;
    let completed_event = events.iter().find(|e|
        e["type"] == "intent.status_changed" &&
        e["new_status"] == "COMPLETED"
    );
    assert!(completed_event.is_some());
}
```

### Complete Payout Flow

```rust
#[tokio::test]
async fn test_payout_complete_flow() {
    let ctx = setup_test_app().await;
    let amount = 500_000i64;

    // Prerequisite: Fund user account
    ctx.ledger_repo.set_balance(
        &TenantId::new(&ctx.tenant_id),
        Some(&UserId::new(&ctx.user_id)),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        Decimal::from(1_000_000), // Enough balance
    );

    // Step 1: Create pay-out intent
    let create_request = json!({
        "tenantId": ctx.tenant_id,
        "userId": ctx.user_id,
        "amountVnd": amount,
        "railsProvider": "VIETCOMBANK",
        "bankAccount": {
            "bankCode": "VCB",
            "accountNumber": "1234567890",
            "accountName": "NGUYEN VAN A"
        },
        "metadata": { "source": "e2e_test" }
    });

    let request = Request::builder()
        .uri("/v1/intents/payout")
        .method("POST")
        .header("Authorization", format!("Bearer {}", ctx.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&create_request).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await.unwrap();
    let intent_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let intent_id_str = intent_resp["intentId"].as_str().unwrap().to_string();

    assert_eq!(intent_resp["status"], "PAYOUT_SUBMITTED");

    // Step 2: Confirm payout via service
    let confirm_req = ConfirmPayoutRequest {
        tenant_id: TenantId::new(&ctx.tenant_id),
        intent_id: IntentId::new(&intent_id_str),
        bank_tx_id: format!("BANK_TX_{}", Uuid::now_v7()),
        status: PayoutBankStatus::Success,
    };

    ctx.payout_service.confirm_payout(confirm_req).await
        .expect("Failed to confirm payout");

    // Step 3: Verify intent final state
    let intents = ctx.intent_repo.intents.lock().unwrap();
    let intent = intents.iter().find(|i| i.id == intent_id_str).unwrap();
    assert_eq!(intent.state, "COMPLETED");

    // Step 4: Verify final balance
    let final_balance = ctx.ledger_repo.get_balance(
        &TenantId::new(&ctx.tenant_id),
        Some(&UserId::new(&ctx.user_id)),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND
    ).await.unwrap();

    assert_eq!(final_balance, dec!(500_000)); // 1M - 500k = 500k
}
```

### Trade Execution Flow

```rust
#[tokio::test]
async fn test_trade_execution_ledger_balancing() {
    let ctx = setup_test_app().await;

    // Trade: Buy 0.1 BTC for 100M VND
    let trade_request = json!({
        "tenantId": ctx.tenant_id,
        "userId": ctx.user_id,
        "tradeId": "trade_001",
        "symbol": "BTC/VND",
        "price": "50000000",
        "vndDelta": -100_000_000,  // User pays VND
        "cryptoDelta": "0.1",       // User gets BTC
        "timestamp": Utc::now().to_rfc3339(),
        "metadata": { "source": "e2e_test" }
    });

    let request = Request::builder()
        .uri("/v1/events/trade-executed")
        .method("POST")
        .header("Authorization", format!("Bearer {}", ctx.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&trade_request).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify ledger balancing
    let txs = ctx.ledger_repo.transactions.lock().unwrap();
    let entries = &txs[0].entries;

    // Verify VND entries balance
    let vnd_debits: Decimal = entries.iter()
        .filter(|e| e.currency == LedgerCurrency::VND &&
                    e.direction == EntryDirection::Debit)
        .map(|e| e.amount)
        .sum();
    let vnd_credits: Decimal = entries.iter()
        .filter(|e| e.currency == LedgerCurrency::VND &&
                    e.direction == EntryDirection::Credit)
        .map(|e| e.amount)
        .sum();
    assert_eq!(vnd_debits, vnd_credits, "VND entries must balance");

    // Verify BTC entries balance
    let btc_debits: Decimal = entries.iter()
        .filter(|e| e.currency == LedgerCurrency::BTC &&
                    e.direction == EntryDirection::Debit)
        .map(|e| e.amount)
        .sum();
    let btc_credits: Decimal = entries.iter()
        .filter(|e| e.currency == LedgerCurrency::BTC &&
                    e.direction == EntryDirection::Credit)
        .map(|e| e.amount)
        .sum();
    assert_eq!(btc_debits, btc_credits, "BTC entries must balance");
}
```

## Running Integration Tests

### Basic Commands

```bash
# Run all integration tests
cargo test --test integration_tests

# Run E2E tests
cargo test --test e2e_tests

# Run specific E2E test file
cargo test --test e2e_payin_test

# Run with testcontainers (requires Docker)
cargo test e2e -- --nocapture

# Run ignored integration tests (requires external services)
cargo test -- --ignored
```

### Environment Variables

```bash
# Database URL for integration tests
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/rampos_test"

# Run integration tests with real database
cargo test -- --ignored
```

## Test Data Management

### Seed Data for Tests

```rust
async fn seed_test_data(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Create tenant
    let tenant_id = "test_tenant";
    let api_key = "secret_api_key";
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    sqlx::query(r#"
        INSERT INTO tenants (id, name, status, api_key_hash, webhook_secret_hash)
        VALUES ($1, $2, $3, $4, $5)
    "#)
    .bind(tenant_id)
    .bind("E2E Test Tenant")
    .bind("ACTIVE")
    .bind(&api_key_hash)
    .bind("secret")
    .execute(pool)
    .await?;

    // Create user
    sqlx::query(r#"
        INSERT INTO users (id, tenant_id, status, kyc_tier, kyc_status)
        VALUES ($1, $2, $3, $4, $5)
    "#)
    .bind("test_user")
    .bind(tenant_id)
    .bind("ACTIVE")
    .bind(1i16)
    .bind("VERIFIED")
    .execute(pool)
    .await?;

    // Create rails adapter
    sqlx::query(r#"
        INSERT INTO rails_adapters
        (id, tenant_id, provider_code, provider_name, adapter_type,
         config_encrypted, supports_payin, status)
        VALUES ($1, $2, $3, $4, 'BANK', $5, true, 'ACTIVE')
    "#)
    .bind("rails_vcb_test")
    .bind(tenant_id)
    .bind("VIETCOMBANK")
    .bind("Vietcombank")
    .bind(vec![0u8; 16]) // Mock encrypted config
    .execute(pool)
    .await?;

    Ok(())
}
```

## API Testing

### HTTP Request Building

```rust
fn build_request(
    method: &str,
    uri: &str,
    api_key: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .uri(uri)
        .method(method)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json");

    if let Some(b) = body {
        builder.body(Body::from(serde_json::to_string(&b).unwrap())).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    }
}
```

### Response Verification

```rust
async fn verify_response<T: serde::de::DeserializeOwned>(
    response: axum::response::Response,
    expected_status: StatusCode,
) -> T {
    assert_eq!(response.status(), expected_status);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    serde_json::from_slice(&body).unwrap()
}
```

## Compliance Integration Tests

### Case Management Tests

```rust
#[tokio::test]
#[ignore] // Requires database
async fn test_case_store_integration() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await
        .expect("Failed to connect to database");

    let store = Arc::new(PostgresCaseStore::new(pool.clone()));
    let manager = CaseManager::new(store);

    let tenant_id = TenantId::new("test_tenant");
    let user_id = UserId::new("test_user");
    let intent_id = IntentId::new_payin();

    // 1. Create a case
    let case_id = manager
        .create_case(
            &tenant_id,
            Some(&user_id),
            Some(&intent_id),
            CaseType::LargeTransaction,
            CaseSeverity::High,
            json!({"amount": 1000000000}),
        )
        .await
        .expect("Failed to create case");

    // 2. Add note
    manager.note_manager
        .add_note(
            &case_id,
            Some("analyst_1".to_string()),
            "Investigating large transaction".to_string(),
            NoteType::Comment,
            true,
        )
        .await
        .expect("Failed to add note");

    // 3. Update status
    manager
        .update_status(&case_id, CaseStatus::Review, Some("analyst_1".to_string()))
        .await
        .expect("Failed to update status");

    // 4. Resolve case
    manager
        .resolve_case(
            &case_id,
            "False positive, user provided documentation",
            CaseStatus::Closed,
            Some("analyst_1".to_string()),
        )
        .await
        .expect("Failed to resolve case");

    // Verify resolution
    let final_cases = manager
        .get_user_cases(&tenant_id, &user_id)
        .await
        .expect("Failed to get user cases");
    let final_case = final_cases.iter().find(|c| c.id == case_id).unwrap();

    assert_eq!(final_case.status, CaseStatus::Closed);
    assert!(final_case.resolved_at.is_some());
}
```

### Reconciliation Tests

```rust
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

    let rampos_txs = vec![
        // Perfect match
        RamposTransaction {
            intent_id: "tx1".to_string(),
            reference_code: "REF1".to_string(),
            amount: Decimal::from(1000),
            // ...
        },
        // Amount mismatch
        RamposTransaction {
            intent_id: "tx2".to_string(),
            reference_code: "REF2".to_string(),
            amount: Decimal::from(2000),
            // ...
        },
    ];

    let rails_txs = vec![
        RailsTransaction {
            tx_id: "bank1".to_string(),
            reference_code: Some("REF1".to_string()),
            amount: Decimal::from(1000),
            // ...
        },
        RailsTransaction {
            tx_id: "bank2".to_string(),
            reference_code: Some("REF2".to_string()),
            amount: Decimal::from(2500), // Mismatch
            // ...
        },
    ];

    engine.reconcile(&mut batch, rampos_txs, rails_txs);

    assert_eq!(batch.matched_count, 2);
    assert_eq!(batch.discrepancy_count, 1);
}
```

## Best Practices

1. **Use testcontainers for database tests**: Avoids polluting local database
2. **Run integration tests separately**: Use `--ignored` flag for slow tests
3. **Clean up after tests**: Reset database state between tests
4. **Mock external services**: Use mock adapters for bank APIs
5. **Verify all side effects**: Check database, events, and logs
6. **Test error scenarios**: Insufficient balance, AML blocks, etc.
7. **Use realistic test data**: Amount ranges, user scenarios

## Debugging Integration Tests

```bash
# Run with full output
cargo test e2e -- --nocapture --test-threads=1

# Run with logging
RUST_LOG=debug cargo test e2e -- --nocapture

# Run with database logging
RUST_LOG=sqlx=trace cargo test e2e -- --nocapture
```
