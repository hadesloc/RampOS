# Unit Testing Guide

This document covers running and writing unit tests for the RampOS Rust codebase.

## Overview

RampOS uses Rust's built-in testing framework with `#[tokio::test]` for async tests. The codebase is organized into multiple crates, each with its own tests.

## Crate Structure

| Crate | Description | Test Location |
|-------|-------------|---------------|
| `ramp-core` | Core domain logic and services | `crates/ramp-core/src/**/*_test.rs` |
| `ramp-api` | HTTP API handlers and middleware | `crates/ramp-api/tests/*.rs` |
| `ramp-compliance` | KYC, AML, and compliance rules | `crates/ramp-compliance/tests/*.rs` |
| `ramp-common` | Shared types and utilities | `crates/ramp-common/src/tests/*.rs` |
| `ramp-ledger` | Ledger and accounting logic | `crates/ramp-ledger/src/**/*.rs` |
| `ramp-adapter` | Bank and payment rail adapters | `crates/ramp-adapter/src/**/*.rs` |
| `ramp-aa` | Account Abstraction (ERC-4337) | `crates/ramp-aa/src/**/*.rs` |

## Running Unit Tests

### Run All Tests

```bash
# Run all tests in the workspace
cargo test

# Run with output displayed
cargo test -- --nocapture

# Run tests in release mode (faster execution)
cargo test --release
```

### Run Tests for Specific Crate

```bash
# Run tests for ramp-api crate
cargo test -p ramp-api

# Run tests for ramp-compliance crate
cargo test -p ramp-compliance

# Run tests for ramp-core crate
cargo test -p ramp-core
```

### Run Specific Test

```bash
# Run a specific test by name
cargo test test_health_check

# Run tests matching a pattern
cargo test payin

# Run tests in a specific module
cargo test api_tests::

# Run a specific test with full output
cargo test test_payin_endpoint_success -- --nocapture
```

### Run Tests with Filtering

```bash
# Run only tests that match a pattern
cargo test --test api_tests

# Run ignored tests
cargo test -- --ignored

# Run all tests including ignored
cargo test -- --include-ignored
```

## Test Coverage

### Using cargo-tarpaulin

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html

# Generate coverage with specific output
cargo tarpaulin --out Lcov --output-dir ./coverage

# Coverage for specific crate
cargo tarpaulin -p ramp-api --out Html
```

### Using cargo-llvm-cov

```bash
# Install llvm-cov
cargo install cargo-llvm-cov

# Generate HTML coverage report
cargo llvm-cov --html

# Generate coverage for specific package
cargo llvm-cov -p ramp-core --html

# Open coverage report
cargo llvm-cov --open
```

### Coverage Thresholds

Target coverage thresholds for the project:

| Module | Minimum Coverage |
|--------|------------------|
| Core Services | 80% |
| API Handlers | 75% |
| Compliance Rules | 90% |
| Ledger Logic | 95% |
| Common Types | 70% |

## Mocking Patterns

### Using Test Utilities

RampOS provides mock implementations in `ramp-core/src/test_utils.rs`:

```rust
use ramp_core::test_utils::*;

// Mock repositories
let intent_repo = Arc::new(MockIntentRepository::new());
let ledger_repo = Arc::new(MockLedgerRepository::new());
let user_repo = Arc::new(MockUserRepository::new());
let tenant_repo = Arc::new(MockTenantRepository::new());

// Event publisher for testing webhooks
let event_publisher = Arc::new(InMemoryEventPublisher::new());
```

### MockIntentRepository

```rust
use ramp_core::test_utils::MockIntentRepository;
use ramp_core::repository::intent::IntentRow;

let repo = MockIntentRepository::new();

// Create an intent
let intent = IntentRow {
    id: "intent-1".to_string(),
    tenant_id: "tenant-1".to_string(),
    user_id: "user-1".to_string(),
    intent_type: "PAYIN_VND".to_string(),
    state: "CREATED".to_string(),
    // ... other fields
};

repo.create(&intent).await.unwrap();

// Access stored intents directly for assertions
let intents = repo.intents.lock().unwrap();
assert_eq!(intents.len(), 1);
```

### MockLedgerRepository

```rust
use ramp_core::test_utils::MockLedgerRepository;
use ramp_common::ledger::{AccountType, LedgerCurrency};
use rust_decimal::Decimal;

let repo = MockLedgerRepository::new();

// Set a balance for testing
repo.set_balance(
    &TenantId::new("tenant1"),
    Some(&UserId::new("user1")),
    &AccountType::LiabilityUserVnd,
    &LedgerCurrency::VND,
    Decimal::from(1_000_000),
);

// Record and verify transactions
let txs = repo.transactions.lock().unwrap();
assert_eq!(txs.len(), 1);
```

### MockUserRepository

```rust
use ramp_core::test_utils::MockUserRepository;
use ramp_core::repository::user::UserRow;

let repo = MockUserRepository::new();

repo.add_user(UserRow {
    id: "user1".to_string(),
    tenant_id: "tenant1".to_string(),
    status: "ACTIVE".to_string(),
    kyc_tier: 1,
    kyc_status: "VERIFIED".to_string(),
    kyc_verified_at: Some(Utc::now()),
    // ... other fields
});
```

### MockTenantRepository

```rust
use ramp_core::test_utils::MockTenantRepository;
use ramp_core::repository::tenant::TenantRow;
use sha2::{Digest, Sha256};

let repo = MockTenantRepository::new();

// Hash the API key for testing
let api_key = "test_api_key";
let mut hasher = Sha256::new();
hasher.update(api_key.as_bytes());
let api_key_hash = hex::encode(hasher.finalize());

repo.add_tenant(TenantRow {
    id: "tenant1".to_string(),
    name: "Test Tenant".to_string(),
    status: "ACTIVE".to_string(),
    api_key_hash,
    webhook_secret_hash: "secret".to_string(),
    // ... other fields
});
```

## Writing Unit Tests

### Basic Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synchronous_function() {
        // Arrange
        let input = "test";

        // Act
        let result = my_function(input);

        // Assert
        assert_eq!(result, expected_value);
    }

    #[tokio::test]
    async fn test_async_function() {
        // Arrange
        let repo = MockIntentRepository::new();

        // Act
        let result = my_async_function(&repo).await;

        // Assert
        assert!(result.is_ok());
    }
}
```

### Testing API Handlers

```rust
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn test_api_endpoint() {
    let app = setup_test_app().await;

    let payload = serde_json::json!({
        "tenant_id": "tenant1",
        "user_id": "user1",
        "amount_vnd": 100000,
        "rails_provider": "VIETCOMBANK",
        "metadata": {}
    });

    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

### Testing Error Cases

```rust
#[tokio::test]
async fn test_validation_error() {
    let app = setup_test_app().await;

    // Missing required field
    let payload = serde_json::json!({
        "tenant_id": "tenant1",
        "user_id": "user1",
        // "amount_vnd" is missing
    });

    let request = Request::builder()
        .uri("/v1/intents/payin")
        .method("POST")
        .header("Authorization", format!("Bearer {}", app.api_key))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
```

### Testing State Transitions

```rust
#[tokio::test]
async fn test_intent_state_transition() {
    let repo = MockIntentRepository::new();

    // Create intent in initial state
    let intent = IntentRow {
        id: "intent-1".to_string(),
        state: "INSTRUCTION_ISSUED".to_string(),
        // ...
    };
    repo.create(&intent).await.unwrap();

    // Update state
    repo.update_state(
        &TenantId::new("tenant-1"),
        &IntentId::new("intent-1"),
        "COMPLETED"
    ).await.unwrap();

    // Verify
    let updated = repo.get_by_id(
        &TenantId::new("tenant-1"),
        &IntentId::new("intent-1")
    ).await.unwrap().unwrap();

    assert_eq!(updated.state, "COMPLETED");
}
```

## Test Fixtures

### Reusable Test Data

```rust
pub mod fixtures {
    use super::*;

    pub fn test_tenant_id() -> String {
        "tenant_test_123".to_string()
    }

    pub fn test_user_id() -> String {
        "user_test_123".to_string()
    }

    pub fn test_payin_request(
        tenant_id: &str,
        user_id: &str,
        amount_vnd: i64,
    ) -> serde_json::Value {
        json!({
            "tenantId": tenant_id,
            "userId": user_id,
            "amountVnd": amount_vnd,
            "railsProvider": "VIETCOMBANK",
            "metadata": {
                "source": "test"
            }
        })
    }

    pub fn test_payout_request(
        tenant_id: &str,
        user_id: &str,
        amount_vnd: i64,
    ) -> serde_json::Value {
        json!({
            "tenantId": tenant_id,
            "userId": user_id,
            "amountVnd": amount_vnd,
            "railsProvider": "VIETCOMBANK",
            "bankAccount": {
                "bankCode": "VCB",
                "accountNumber": "1234567890",
                "accountName": "NGUYEN VAN A"
            },
            "metadata": {
                "source": "test"
            }
        })
    }
}
```

## Best Practices

1. **Isolate tests**: Each test should be independent and not rely on other tests' state
2. **Use descriptive names**: Test names should describe what is being tested
3. **Follow AAA pattern**: Arrange, Act, Assert
4. **Test edge cases**: Include tests for error conditions and boundary values
5. **Mock external dependencies**: Use mock repositories and publishers
6. **Clean up resources**: Ensure tests don't leave state that affects other tests
7. **Use `#[ignore]` for slow tests**: Mark integration tests that require external services

## Debugging Tests

```bash
# Run tests with backtrace
RUST_BACKTRACE=1 cargo test

# Run tests with full backtrace
RUST_BACKTRACE=full cargo test

# Run tests with logging
RUST_LOG=debug cargo test -- --nocapture

# Run single-threaded for debugging
cargo test -- --test-threads=1
```

## CI Integration

Tests are automatically run in CI. See `.github/workflows/` for configuration.

```yaml
# Example CI step
- name: Run Tests
  run: cargo test --all-features --workspace
  env:
    RUST_BACKTRACE: 1
```
