# RampOS Ledger Documentation

## Overview

RampOS implements a **double-entry bookkeeping** system to ensure financial integrity and auditability. Every money movement in the system creates balanced ledger entries where debits equal credits.

This design provides:
- **Accuracy**: Mathematical guarantee that all funds are accounted for
- **Auditability**: Complete transaction history with attribution
- **Reconciliation**: Easy verification against external systems
- **Compliance**: Required for financial regulations

## Double-Entry Principle

In double-entry accounting:
- Every transaction has at least two entries
- **Debits** = **Credits** (always balanced)
- Assets + Expenses = Liabilities + Equity + Revenue

```
+------------------+     +------------------+
|     DEBIT        |     |     CREDIT       |
|   (Increases)    |     |   (Increases)    |
| - Assets         |     | - Liabilities    |
| - Expenses       |     | - Revenue        |
+------------------+     +------------------+
```

## Account Types

RampOS uses a chart of accounts organized by category:

### Asset Accounts (What RampOS Controls)

| Account | Code | Description |
|---------|------|-------------|
| `AssetBank` | 1000 | Cash held in bank accounts |
| `AssetCrypto` | 1100 | Cryptocurrency in custody |
| `AssetReceivable` | 1200 | Money owed to the system |

### Liability Accounts (What RampOS Owes)

| Account | Code | Description |
|---------|------|-------------|
| `LiabilityUserVnd` | 2000 | VND balance owed to users |
| `LiabilityUserCrypto` | 2100 | Crypto balance owed to users |
| `LiabilityPayable` | 2200 | Money owed to external parties |

### Clearing Accounts (Temporary Holding)

| Account | Code | Description |
|---------|------|-------------|
| `ClearingBankPending` | 3000 | Bank transfers awaiting confirmation |
| `ClearingCryptoPending` | 3100 | Crypto transfers awaiting confirmation |
| `ClearingTrade` | 3200 | Trade settlement clearing |

### Revenue Accounts

| Account | Code | Description |
|---------|------|-------------|
| `RevenueFee` | 4000 | Transaction fees earned |
| `RevenueSpread` | 4100 | Trading spread revenue |

### Expense Accounts

| Account | Code | Description |
|---------|------|-------------|
| `ExpenseGas` | 5000 | Blockchain gas fees paid |
| `ExpenseProvider` | 5100 | Payments to rails providers |

## Ledger Data Structures

### Ledger Entry

A single side of a transaction:

```rust
pub struct LedgerEntry {
    pub id: LedgerEntryId,           // Unique entry ID (le_xxx)
    pub tenant_id: TenantId,         // Tenant isolation
    pub user_id: Option<UserId>,     // User for user-specific accounts
    pub intent_id: IntentId,         // Link to originating intent
    pub account_type: AccountType,   // Account being affected
    pub direction: EntryDirection,   // Debit or Credit
    pub amount: Decimal,             // Amount (always positive)
    pub currency: LedgerCurrency,    // VND, BTC, ETH, etc.
    pub balance_after: Decimal,      // Running balance after this entry
    pub description: String,         // Human-readable description
    pub metadata: serde_json::Value, // Additional data
    pub created_at: DateTime<Utc>,   // Timestamp
    pub sequence: i64,               // Ordering sequence
}
```

### Ledger Transaction

A complete balanced transaction:

```rust
pub struct LedgerTransaction {
    pub id: String,                  // Transaction ID (ltx_xxx)
    pub tenant_id: TenantId,         // Tenant isolation
    pub intent_id: IntentId,         // Link to originating intent
    pub entries: Vec<LedgerEntry>,   // All entries in this transaction
    pub description: String,         // Transaction description
    pub created_at: DateTime<Utc>,   // Timestamp
}

impl LedgerTransaction {
    /// Verify that debits equal credits
    pub fn is_balanced(&self) -> bool {
        let mut debit_total = Decimal::ZERO;
        let mut credit_total = Decimal::ZERO;

        for entry in &self.entries {
            match entry.direction {
                EntryDirection::Debit => debit_total += entry.amount,
                EntryDirection::Credit => credit_total += entry.amount,
            }
        }

        debit_total == credit_total
    }
}
```

## Ledger Transaction Builder

The builder pattern ensures balanced transactions:

```rust
// Create a balanced transaction
let tx = LedgerTransactionBuilder::new(
    tenant_id,
    intent_id,
    "VND Pay-in confirmed"
)
.debit(AccountType::AssetBank, dec!(1_000_000), LedgerCurrency::VND)
.credit_user(
    user_id,
    AccountType::LiabilityUserVnd,
    dec!(1_000_000),
    LedgerCurrency::VND,
)
.build()?;

// Transaction is guaranteed to be balanced
assert!(tx.is_balanced());
```

### Builder Methods

| Method | Description |
|--------|-------------|
| `debit(account, amount, currency)` | Add a debit entry (system account) |
| `credit(account, amount, currency)` | Add a credit entry (system account) |
| `debit_user(user_id, account, amount, currency)` | Add a user-specific debit |
| `credit_user(user_id, account, amount, currency)` | Add a user-specific credit |
| `build()` | Validate balance and create transaction |

## Common Transaction Patterns

### 1. VND Pay-in (Bank Confirmed)

When a user deposits VND and the bank confirms receipt:

```
User deposits 1,000,000 VND:

+-------------------+-------+-------------+
| Account           | Debit | Credit      |
+-------------------+-------+-------------+
| Asset:Bank        | 1M    |             |
| Liability:UserVND |       | 1M          |
+-------------------+-------+-------------+
| TOTAL             | 1M    | 1M          |
+-------------------+-------+-------------+
```

```rust
pub fn payin_vnd_confirmed(
    tenant_id: TenantId,
    user_id: UserId,
    intent_id: IntentId,
    amount: Decimal,
) -> Result<LedgerTransaction, LedgerError> {
    LedgerTransactionBuilder::new(tenant_id, intent_id, "VND Pay-in confirmed")
        .debit(AccountType::AssetBank, amount, LedgerCurrency::VND)
        .credit_user(user_id, AccountType::LiabilityUserVnd, amount, LedgerCurrency::VND)
        .build()
}
```

### 2. VND Pay-out (Initiated)

When a user requests a VND withdrawal:

```
User withdraws 500,000 VND:

+----------------------+--------+--------+
| Account              | Debit  | Credit |
+----------------------+--------+--------+
| Liability:UserVND    | 500K   |        |
| Clearing:BankPending |        | 500K   |
+----------------------+--------+--------+
| TOTAL                | 500K   | 500K   |
+----------------------+--------+--------+
```

```rust
pub fn payout_vnd_initiated(
    tenant_id: TenantId,
    user_id: UserId,
    intent_id: IntentId,
    amount: Decimal,
) -> Result<LedgerTransaction, LedgerError> {
    LedgerTransactionBuilder::new(tenant_id, intent_id, "VND Pay-out initiated")
        .debit_user(user_id, AccountType::LiabilityUserVnd, amount, LedgerCurrency::VND)
        .credit(AccountType::ClearingBankPending, amount, LedgerCurrency::VND)
        .build()
}
```

### 3. VND Pay-out (Confirmed)

When the bank confirms the withdrawal:

```
Bank confirms 500,000 VND transfer:

+----------------------+--------+--------+
| Account              | Debit  | Credit |
+----------------------+--------+--------+
| Clearing:BankPending | 500K   |        |
| Asset:Bank           |        | 500K   |
+----------------------+--------+--------+
| TOTAL                | 500K   | 500K   |
+----------------------+--------+--------+
```

```rust
pub fn payout_vnd_confirmed(
    tenant_id: TenantId,
    intent_id: IntentId,
    amount: Decimal,
) -> Result<LedgerTransaction, LedgerError> {
    LedgerTransactionBuilder::new(tenant_id, intent_id, "VND Pay-out confirmed")
        .debit(AccountType::ClearingBankPending, amount, LedgerCurrency::VND)
        .credit(AccountType::AssetBank, amount, LedgerCurrency::VND)
        .build()
}
```

### 4. Crypto/VND Trade (Buy Crypto)

When a user buys crypto with VND:

```
User buys 0.01 BTC for 500,000,000 VND:

+------------------------+------------+------------+
| Account                | Debit      | Credit     |
+------------------------+------------+------------+
| Liability:UserVND      | 500M VND   |            |
| Asset:Bank             |            | 500M VND   |
| Asset:Crypto           | 0.01 BTC   |            |
| Liability:UserCrypto   |            | 0.01 BTC   |
+------------------------+------------+------------+
| TOTAL (VND)            | 500M       | 500M       |
| TOTAL (BTC)            | 0.01       | 0.01       |
+------------------------+------------+------------+
```

```rust
pub fn trade_crypto_vnd(
    tenant_id: TenantId,
    user_id: UserId,
    intent_id: IntentId,
    vnd_amount: Decimal,
    crypto_amount: Decimal,
    crypto_currency: LedgerCurrency,
    is_buy: bool,  // true = user buys crypto with VND
) -> Result<LedgerTransaction, LedgerError> {
    let mut builder = LedgerTransactionBuilder::new(
        tenant_id,
        intent_id,
        if is_buy { "Buy crypto with VND" } else { "Sell crypto for VND" },
    );

    if is_buy {
        // User pays VND, receives crypto
        builder = builder
            .debit_user(user_id.clone(), AccountType::LiabilityUserVnd, vnd_amount, LedgerCurrency::VND)
            .credit(AccountType::AssetBank, vnd_amount, LedgerCurrency::VND)
            .debit(AccountType::AssetCrypto, crypto_amount, crypto_currency)
            .credit_user(user_id, AccountType::LiabilityUserCrypto, crypto_amount, crypto_currency);
    } else {
        // User pays crypto, receives VND
        builder = builder
            .debit_user(user_id.clone(), AccountType::LiabilityUserCrypto, crypto_amount, crypto_currency)
            .credit(AccountType::AssetCrypto, crypto_amount, crypto_currency)
            .debit(AccountType::AssetBank, vnd_amount, LedgerCurrency::VND)
            .credit_user(user_id, AccountType::LiabilityUserVnd, vnd_amount, LedgerCurrency::VND);
    }

    builder.build()
}
```

## Supported Currencies

```rust
pub enum LedgerCurrency {
    VND,    // Vietnamese Dong
    BTC,    // Bitcoin
    ETH,    // Ethereum
    USDT,   // Tether
    USDC,   // USD Coin
    Other,  // Custom tokens
}
```

## Balance Queries

### User Balance

Get a user's current balance for a specific currency:

```sql
SELECT
    SUM(CASE WHEN direction = 'CREDIT' THEN amount ELSE -amount END) as balance
FROM ledger_entries
WHERE tenant_id = $1
  AND user_id = $2
  AND currency = $3
  AND account_type IN ('LIABILITY_USER_VND', 'LIABILITY_USER_CRYPTO')
```

### System Balance

Get total system balance by account type:

```sql
SELECT
    account_type,
    currency,
    SUM(CASE WHEN direction = 'DEBIT' THEN amount ELSE -amount END) as balance
FROM ledger_entries
WHERE tenant_id = $1
GROUP BY account_type, currency
```

## Transaction Flow Diagrams

### Complete Pay-in Flow

```
+--------+    +----------+    +-------+    +--------+    +--------+
|  User  |--->| Bank API |--->|RampOS |--->| Ledger |--->|  User  |
| (Bank) |    | Webhook  |    | Core  |    |  Post  |    |Balance |
+--------+    +----------+    +-------+    +--------+    +--------+
    |              |              |            |             |
    |  Transfer    |              |            |             |
    +------------->|              |            |             |
    |              | Notification |            |             |
    |              +------------->|            |             |
    |              |              | Record Tx  |             |
    |              |              +----------->|             |
    |              |              |            | Update      |
    |              |              |            +------------>|
    |              |              |            |             |
```

### Complete Trade Flow

```
+---------+    +---------+    +--------+    +--------+    +--------+
| Trading |--->| RampOS  |--->|  AML   |--->| Ledger |--->| Update |
| Engine  |    |  Core   |    | Check  |    |  Post  |    |Balances|
+---------+    +---------+    +--------+    +--------+    +--------+
     |              |              |             |             |
     | Trade        |              |             |             |
     | Executed     |              |             |             |
     +------------->|              |             |             |
     |              | Run Checks   |             |             |
     |              +------------->|             |             |
     |              |              | Passed      |             |
     |              |<-------------+             |             |
     |              | Post Entries |             |             |
     |              +-------------------------->|             |
     |              |              |             | VND +/-     |
     |              |              |             | Crypto +/-  |
     |              |              |             +------------>|
```

## Error Handling

### Imbalanced Transaction Error

```rust
#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("Transaction not balanced: debit={debit}, credit={credit}")]
    Imbalanced { debit: Decimal, credit: Decimal },

    #[error("Insufficient balance in account")]
    InsufficientBalance,

    #[error("Account not found: {0}")]
    AccountNotFound(String),
}
```

### Validation

The builder rejects imbalanced transactions at build time:

```rust
let result = LedgerTransactionBuilder::new(tenant_id, intent_id, "Test")
    .debit(AccountType::AssetBank, dec!(1000), LedgerCurrency::VND)
    .credit(AccountType::LiabilityUserVnd, dec!(999), LedgerCurrency::VND)  // Wrong!
    .build();

assert!(matches!(result, Err(LedgerError::Imbalanced { .. })));
```

## Database Schema

```sql
CREATE TABLE ledger_entries (
    id VARCHAR(36) PRIMARY KEY,
    tenant_id VARCHAR(36) NOT NULL,
    user_id VARCHAR(36),
    intent_id VARCHAR(36) NOT NULL,
    transaction_id VARCHAR(36) NOT NULL,
    account_type VARCHAR(50) NOT NULL,
    direction VARCHAR(10) NOT NULL,  -- 'DEBIT' or 'CREDIT'
    amount DECIMAL(38, 18) NOT NULL,
    currency VARCHAR(10) NOT NULL,
    balance_after DECIMAL(38, 18) NOT NULL,
    description TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sequence BIGSERIAL NOT NULL,

    -- Indexes
    CONSTRAINT fk_intent FOREIGN KEY (intent_id) REFERENCES intents(id),
    INDEX idx_tenant_user (tenant_id, user_id),
    INDEX idx_tenant_account (tenant_id, account_type, currency),
    INDEX idx_transaction (transaction_id),
    INDEX idx_created (created_at)
);

-- Ensure sequence ordering
CREATE SEQUENCE ledger_sequence;
```

## Reconciliation

### Daily Reconciliation Process

```
1. Export ledger entries for the day
2. Compare against bank statements
3. Compare against blockchain transactions
4. Identify discrepancies
5. Create adjustment entries if needed
6. Generate reconciliation report
```

### Reconciliation Query

```sql
-- Compare ledger against external source
SELECT
    l.intent_id,
    l.amount as ledger_amount,
    e.amount as external_amount,
    l.amount - e.amount as discrepancy
FROM ledger_entries l
LEFT JOIN external_transactions e ON l.intent_id = e.reference_id
WHERE l.created_at BETWEEN $1 AND $2
  AND l.amount != COALESCE(e.amount, 0)
```

## Best Practices

1. **Always use the builder** - Never create entries manually
2. **Link to intents** - Every transaction should reference an intent
3. **Include metadata** - Store relevant context for debugging
4. **Use transactions** - Wrap ledger posts in database transactions
5. **Verify balances** - Run periodic reconciliation checks
6. **Immutable entries** - Never modify or delete ledger entries
7. **Sequence ordering** - Use sequences for deterministic ordering
