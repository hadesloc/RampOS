use crate::repository::set_rls_context;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{
    ledger::{AccountType, EntryDirection, LedgerCurrency, LedgerTransaction},
    types::{IntentId, TenantId, UserId},
    Result,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

/// Ledger entry database row
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LedgerEntryRow {
    pub id: String,
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub intent_id: String,
    pub transaction_id: String,
    pub account_type: String,
    pub direction: String,
    pub amount: Decimal,
    pub currency: String,
    pub balance_after: Decimal,
    pub sequence: i64,
    pub description: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait LedgerRepository: Send + Sync {
    /// Record a complete transaction with balanced entries
    async fn record_transaction(&self, tx: LedgerTransaction) -> Result<()>;

    /// Get entries for an intent
    async fn get_entries_by_intent(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
    ) -> Result<Vec<LedgerEntryRow>>;

    /// Get balance for an account
    async fn get_balance(
        &self,
        tenant_id: &TenantId,
        user_id: Option<&UserId>,
        account_type: &AccountType,
        currency: &LedgerCurrency,
    ) -> Result<Decimal>;

    /// Get all balances for a user
    async fn get_user_balances(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Vec<BalanceRow>>;

    /// SECURITY: Atomically check balance and record transaction.
    /// Uses SELECT FOR UPDATE to prevent race conditions in concurrent withdrawals.
    /// Returns the balance before the transaction was applied.
    ///
    /// This method:
    /// 1. Acquires a row lock on the account balance
    /// 2. Verifies sufficient balance
    /// 3. Records the transaction
    /// 4. All within a single database transaction
    async fn check_balance_and_record_transaction(
        &self,
        required_balance: Decimal,
        user_id: &UserId,
        account_type: &AccountType,
        currency: &LedgerCurrency,
        tx: LedgerTransaction,
    ) -> Result<Decimal>;
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct BalanceRow {
    pub account_type: String,
    pub currency: String,
    pub balance: Decimal,
}

/// PostgreSQL implementation
pub struct PgLedgerRepository {
    pool: PgPool,
}

impl PgLedgerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LedgerRepository for PgLedgerRepository {
    async fn record_transaction(&self, tx: LedgerTransaction) -> Result<()> {
        // Use a database transaction
        let mut db_tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        // Set RLS context for the transaction
        set_rls_context(&mut db_tx, &tx.tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        for entry in &tx.entries {
            // Get current balance for this account
            let current_balance: Decimal = sqlx::query_scalar(
                r#"
                SELECT COALESCE(balance, 0)
                FROM account_balances
                WHERE tenant_id = $1
                  AND COALESCE(user_id, '') = COALESCE($2, '')
                  AND account_type = $3
                  AND currency = $4
                FOR UPDATE
                "#,
            )
            .bind(&tx.tenant_id.0)
            .bind(entry.user_id.as_ref().map(|u| &u.0))
            .bind(entry.account_type.to_string())
            .bind(entry.currency.to_string())
            .fetch_optional(&mut *db_tx)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?
            .unwrap_or(Decimal::ZERO);

            // Calculate new balance
            let balance_change = match entry.direction {
                EntryDirection::Debit => entry.amount,
                EntryDirection::Credit => -entry.amount,
            };
            let new_balance = current_balance + balance_change;

            // Insert ledger entry
            sqlx::query(
                r#"
                INSERT INTO ledger_entries (
                    id, tenant_id, user_id, intent_id, transaction_id,
                    account_type, direction, amount, currency, balance_after,
                    description, metadata, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                "#,
            )
            .bind(&entry.id.0)
            .bind(&tx.tenant_id.0)
            .bind(entry.user_id.as_ref().map(|u| &u.0))
            .bind(&tx.intent_id.0)
            .bind(&tx.id)
            .bind(entry.account_type.to_string())
            .bind(match entry.direction {
                EntryDirection::Debit => "DEBIT",
                EntryDirection::Credit => "CREDIT",
            })
            .bind(entry.amount)
            .bind(entry.currency.to_string())
            .bind(new_balance)
            .bind(&entry.description)
            .bind(&entry.metadata)
            .bind(entry.created_at)
            .execute(&mut *db_tx)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

            // Upsert balance
            sqlx::query(
                r#"
                INSERT INTO account_balances (tenant_id, user_id, account_type, currency, balance, last_entry_id, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, NOW())
                ON CONFLICT (tenant_id, COALESCE(user_id, ''), account_type, currency)
                DO UPDATE SET
                    balance = $5,
                    last_entry_id = $6,
                    updated_at = NOW()
                "#,
            )
            .bind(&tx.tenant_id.0)
            .bind(entry.user_id.as_ref().map(|u| &u.0))
            .bind(entry.account_type.to_string())
            .bind(entry.currency.to_string())
            .bind(new_balance)
            .bind(&entry.id.0)
            .execute(&mut *db_tx)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        }

        db_tx
            .commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Get entries for an intent
    async fn get_entries_by_intent(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
    ) -> Result<Vec<LedgerEntryRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, LedgerEntryRow>(
            "SELECT * FROM ledger_entries WHERE tenant_id = $1 AND intent_id = $2 ORDER BY sequence",
        )
        .bind(&tenant_id.0)
        .bind(&intent_id.0)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn get_balance(
        &self,
        tenant_id: &TenantId,
        user_id: Option<&UserId>,
        account_type: &AccountType,
        currency: &LedgerCurrency,
    ) -> Result<Decimal> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let balance: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT balance
            FROM account_balances
            WHERE tenant_id = $1
              AND COALESCE(user_id, '') = COALESCE($2, '')
              AND account_type = $3
              AND currency = $4
            "#,
        )
        .bind(&tenant_id.0)
        .bind(user_id.map(|u| &u.0))
        .bind(account_type.to_string())
        .bind(currency.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(balance.unwrap_or(Decimal::ZERO))
    }

    async fn get_user_balances(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Vec<BalanceRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, BalanceRow>(
            r#"
            SELECT account_type, currency, balance
            FROM account_balances
            WHERE tenant_id = $1 AND user_id = $2
            ORDER BY account_type, currency
            "#,
        )
        .bind(&tenant_id.0)
        .bind(&user_id.0)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(rows)
    }

    /// SECURITY: Atomically check balance and record transaction with row locking.
    /// Prevents race conditions in concurrent withdrawals.
    async fn check_balance_and_record_transaction(
        &self,
        required_balance: Decimal,
        user_id: &UserId,
        account_type: &AccountType,
        currency: &LedgerCurrency,
        ledger_tx: LedgerTransaction,
    ) -> Result<Decimal> {
        // Start database transaction
        let mut db_tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        // Set RLS context
        set_rls_context(&mut db_tx, &ledger_tx.tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        // CRITICAL: Get balance WITH LOCK to prevent concurrent modifications
        // FOR UPDATE acquires a row-level exclusive lock
        let current_balance: Decimal = sqlx::query_scalar(
            r#"
            SELECT COALESCE(balance, 0)
            FROM account_balances
            WHERE tenant_id = $1
              AND user_id = $2
              AND account_type = $3
              AND currency = $4
            FOR UPDATE
            "#,
        )
        .bind(&ledger_tx.tenant_id.0)
        .bind(&user_id.0)
        .bind(account_type.to_string())
        .bind(currency.to_string())
        .fetch_optional(&mut *db_tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?
        .unwrap_or(Decimal::ZERO);

        // Check if sufficient balance
        if current_balance < required_balance {
            // Rollback and return error
            db_tx
                .rollback()
                .await
                .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

            return Err(ramp_common::Error::InsufficientBalance {
                required: required_balance.to_string(),
                available: current_balance.to_string(),
            });
        }

        // Balance is sufficient, now record all entries atomically
        for entry in &ledger_tx.entries {
            // Get current balance for this specific entry's account
            let entry_current_balance: Decimal = sqlx::query_scalar(
                r#"
                SELECT COALESCE(balance, 0)
                FROM account_balances
                WHERE tenant_id = $1
                  AND COALESCE(user_id, '') = COALESCE($2, '')
                  AND account_type = $3
                  AND currency = $4
                FOR UPDATE
                "#,
            )
            .bind(&ledger_tx.tenant_id.0)
            .bind(entry.user_id.as_ref().map(|u| &u.0))
            .bind(entry.account_type.to_string())
            .bind(entry.currency.to_string())
            .fetch_optional(&mut *db_tx)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?
            .unwrap_or(Decimal::ZERO);

            // Calculate new balance
            let balance_change = match entry.direction {
                EntryDirection::Debit => entry.amount,
                EntryDirection::Credit => -entry.amount,
            };
            let new_balance = entry_current_balance + balance_change;

            // Insert ledger entry
            sqlx::query(
                r#"
                INSERT INTO ledger_entries (
                    id, tenant_id, user_id, intent_id, transaction_id,
                    account_type, direction, amount, currency, balance_after,
                    description, metadata, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                "#,
            )
            .bind(&entry.id.0)
            .bind(&ledger_tx.tenant_id.0)
            .bind(entry.user_id.as_ref().map(|u| &u.0))
            .bind(&ledger_tx.intent_id.0)
            .bind(&ledger_tx.id)
            .bind(entry.account_type.to_string())
            .bind(match entry.direction {
                EntryDirection::Debit => "DEBIT",
                EntryDirection::Credit => "CREDIT",
            })
            .bind(entry.amount)
            .bind(entry.currency.to_string())
            .bind(new_balance)
            .bind(&entry.description)
            .bind(&entry.metadata)
            .bind(entry.created_at)
            .execute(&mut *db_tx)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

            // Upsert balance
            sqlx::query(
                r#"
                INSERT INTO account_balances (tenant_id, user_id, account_type, currency, balance, last_entry_id, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, NOW())
                ON CONFLICT (tenant_id, COALESCE(user_id, ''), account_type, currency)
                DO UPDATE SET
                    balance = $5,
                    last_entry_id = $6,
                    updated_at = NOW()
                "#,
            )
            .bind(&ledger_tx.tenant_id.0)
            .bind(entry.user_id.as_ref().map(|u| &u.0))
            .bind(entry.account_type.to_string())
            .bind(entry.currency.to_string())
            .bind(new_balance)
            .bind(&entry.id.0)
            .execute(&mut *db_tx)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        }

        // Commit the transaction
        db_tx
            .commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(current_balance)
    }
}
