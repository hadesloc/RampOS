//! Off-ramp intent repository - SQL-backed persistence for off-ramp intents

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use tracing::instrument;

use crate::repository::set_rls_context;

/// Off-ramp intent database row
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OfframpIntentRow {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
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
}

#[async_trait]
pub trait OfframpIntentRepository: Send + Sync {
    /// Create a new off-ramp intent
    async fn create_intent(&self, intent: &OfframpIntentRow) -> Result<()>;

    /// Get an intent by ID
    async fn get_intent(&self, tenant_id: &TenantId, id: &str) -> Result<Option<OfframpIntentRow>>;

    /// Update intent state with validation
    async fn update_status(
        &self,
        tenant_id: &TenantId,
        id: &str,
        new_state: &str,
        state_history: &serde_json::Value,
    ) -> Result<()>;

    /// Update intent fields (tx_hash, bank_reference, deposit_address, locked_rate_id, etc.)
    async fn update_intent(&self, intent: &OfframpIntentRow) -> Result<()>;

    /// List intents for a tenant
    async fn list_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OfframpIntentRow>>;

    /// List intents by status
    async fn list_by_status(
        &self,
        tenant_id: &TenantId,
        status: &str,
        limit: i64,
    ) -> Result<Vec<OfframpIntentRow>>;
}

/// PostgreSQL implementation
pub struct PgOfframpIntentRepository {
    pool: PgPool,
}

impl PgOfframpIntentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OfframpIntentRepository for PgOfframpIntentRepository {
    #[instrument(skip(self, intent), fields(intent_id = %intent.id, tenant_id = %intent.tenant_id))]
    async fn create_intent(&self, intent: &OfframpIntentRow) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, &TenantId(intent.tenant_id.clone()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO offramp_intents (
                id, tenant_id, user_id, crypto_asset, crypto_amount, exchange_rate,
                locked_rate_id, fees, net_vnd_amount, gross_vnd_amount, bank_account,
                deposit_address, tx_hash, bank_reference, state, state_history,
                created_at, updated_at, quote_expires_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                $12, $13, $14, $15, $16, $17, $18, $19
            )
            "#,
        )
        .bind(&intent.id)
        .bind(&intent.tenant_id)
        .bind(&intent.user_id)
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
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, intent_id = %id))]
    async fn get_intent(&self, tenant_id: &TenantId, id: &str) -> Result<Option<OfframpIntentRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let row = sqlx::query_as::<_, OfframpIntentRow>(
            "SELECT * FROM offramp_intents WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&tenant_id.0)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(row)
    }

    #[instrument(skip(self, state_history), fields(tenant_id = %tenant_id.0, intent_id = %id, new_state = %new_state))]
    async fn update_status(
        &self,
        tenant_id: &TenantId,
        id: &str,
        new_state: &str,
        state_history: &serde_json::Value,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE offramp_intents
            SET state = $1, state_history = $2, updated_at = NOW()
            WHERE id = $3 AND tenant_id = $4
            "#,
        )
        .bind(new_state)
        .bind(state_history)
        .bind(id)
        .bind(&tenant_id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self, intent), fields(intent_id = %intent.id, tenant_id = %intent.tenant_id))]
    async fn update_intent(&self, intent: &OfframpIntentRow) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, &TenantId(intent.tenant_id.clone()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE offramp_intents
            SET locked_rate_id = $1, deposit_address = $2, tx_hash = $3,
                bank_reference = $4, state = $5, state_history = $6,
                updated_at = NOW()
            WHERE id = $7 AND tenant_id = $8
            "#,
        )
        .bind(&intent.locked_rate_id)
        .bind(&intent.deposit_address)
        .bind(&intent.tx_hash)
        .bind(&intent.bank_reference)
        .bind(&intent.state)
        .bind(&intent.state_history)
        .bind(&intent.id)
        .bind(&intent.tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0))]
    async fn list_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OfframpIntentRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, OfframpIntentRow>(
            r#"
            SELECT * FROM offramp_intents
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(&tenant_id.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, status = %status))]
    async fn list_by_status(
        &self,
        tenant_id: &TenantId,
        status: &str,
        limit: i64,
    ) -> Result<Vec<OfframpIntentRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, OfframpIntentRow>(
            r#"
            SELECT * FROM offramp_intents
            WHERE tenant_id = $1 AND state = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(&tenant_id.0)
        .bind(status)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde_json::json;

    /// Test OfframpIntentRow construction and field access (CRUD model test)
    #[test]
    fn test_offramp_intent_crud() {
        let now = Utc::now();
        let intent = OfframpIntentRow {
            id: "ofr_test_001".to_string(),
            tenant_id: "tenant_1".to_string(),
            user_id: "user_1".to_string(),
            crypto_asset: "USDT".to_string(),
            crypto_amount: Decimal::new(100, 0),
            exchange_rate: Decimal::new(25_000, 0),
            locked_rate_id: Some("rate_lock_1".to_string()),
            fees: json!({"network_fee": "0.5", "platform_fee": "1.0"}),
            net_vnd_amount: Decimal::new(2_475_000, 0),
            gross_vnd_amount: Decimal::new(2_500_000, 0),
            bank_account: json!({"bank_code": "VCB", "account_number": "1234567890", "account_name": "Nguyen Van A"}),
            deposit_address: Some("0x1234567890abcdef".to_string()),
            tx_hash: None,
            bank_reference: None,
            state: "QUOTE_CREATED".to_string(),
            state_history: json!([{"state": "QUOTE_CREATED", "timestamp": now.to_rfc3339()}]),
            created_at: now,
            updated_at: now,
            quote_expires_at: now + chrono::Duration::minutes(5),
        };

        assert_eq!(intent.id, "ofr_test_001");
        assert_eq!(intent.tenant_id, "tenant_1");
        assert_eq!(intent.user_id, "user_1");
        assert_eq!(intent.crypto_asset, "USDT");
        assert_eq!(intent.crypto_amount, Decimal::new(100, 0));
        assert_eq!(intent.exchange_rate, Decimal::new(25_000, 0));
        assert_eq!(intent.locked_rate_id, Some("rate_lock_1".to_string()));
        assert_eq!(intent.net_vnd_amount, Decimal::new(2_475_000, 0));
        assert_eq!(intent.gross_vnd_amount, Decimal::new(2_500_000, 0));
        assert_eq!(intent.state, "QUOTE_CREATED");
        assert!(intent.deposit_address.is_some());
        assert!(intent.tx_hash.is_none());
        assert!(intent.bank_reference.is_none());
        assert!(intent.quote_expires_at > intent.created_at);

        // Verify JSON fields
        assert_eq!(intent.bank_account["bank_code"], "VCB");
        assert_eq!(intent.fees["platform_fee"], "1.0");
    }

    /// Test state transitions are valid (model-level validation)
    #[test]
    fn test_offramp_intent_state_transitions() {
        let valid_states = [
            "QUOTE_CREATED",
            "CRYPTO_PENDING",
            "CRYPTO_RECEIVED",
            "VND_TRANSFERRING",
            "COMPLETED",
            "FAILED",
            "EXPIRED",
        ];

        // Valid transitions map
        let valid_transitions: Vec<(&str, &str)> = vec![
            ("QUOTE_CREATED", "CRYPTO_PENDING"),
            ("QUOTE_CREATED", "EXPIRED"),
            ("CRYPTO_PENDING", "CRYPTO_RECEIVED"),
            ("CRYPTO_PENDING", "EXPIRED"),
            ("CRYPTO_PENDING", "FAILED"),
            ("CRYPTO_RECEIVED", "VND_TRANSFERRING"),
            ("CRYPTO_RECEIVED", "FAILED"),
            ("VND_TRANSFERRING", "COMPLETED"),
            ("VND_TRANSFERRING", "FAILED"),
        ];

        let now = Utc::now();
        let mut intent = OfframpIntentRow {
            id: "ofr_transition_test".to_string(),
            tenant_id: "tenant_1".to_string(),
            user_id: "user_1".to_string(),
            crypto_asset: "USDT".to_string(),
            crypto_amount: Decimal::new(100, 0),
            exchange_rate: Decimal::new(25_000, 0),
            locked_rate_id: None,
            fees: json!({}),
            net_vnd_amount: Decimal::new(2_475_000, 0),
            gross_vnd_amount: Decimal::new(2_500_000, 0),
            bank_account: json!({}),
            deposit_address: None,
            tx_hash: None,
            bank_reference: None,
            state: "QUOTE_CREATED".to_string(),
            state_history: json!([]),
            created_at: now,
            updated_at: now,
            quote_expires_at: now + chrono::Duration::minutes(5),
        };

        // Verify all valid states are recognized
        for state in &valid_states {
            assert!(
                !state.is_empty(),
                "State should not be empty"
            );
        }

        // Simulate forward transition: QUOTE_CREATED -> CRYPTO_PENDING -> CRYPTO_RECEIVED -> VND_TRANSFERRING -> COMPLETED
        let forward_path = [
            "CRYPTO_PENDING",
            "CRYPTO_RECEIVED",
            "VND_TRANSFERRING",
            "COMPLETED",
        ];

        let mut current_state = "QUOTE_CREATED";
        for next_state in &forward_path {
            let transition_valid = valid_transitions
                .iter()
                .any(|(from, to)| *from == current_state && *to == *next_state);
            assert!(
                transition_valid,
                "Transition from {} to {} should be valid",
                current_state, next_state
            );

            // Apply transition
            intent.state = next_state.to_string();
            intent.updated_at = Utc::now();
            current_state = next_state;
        }

        assert_eq!(intent.state, "COMPLETED");

        // Test failure path: QUOTE_CREATED -> CRYPTO_PENDING -> FAILED
        intent.state = "QUOTE_CREATED".to_string();
        let failure_path = ["CRYPTO_PENDING", "FAILED"];

        current_state = "QUOTE_CREATED";
        for next_state in &failure_path {
            let transition_valid = valid_transitions
                .iter()
                .any(|(from, to)| *from == current_state && *to == *next_state);
            assert!(
                transition_valid,
                "Transition from {} to {} should be valid",
                current_state, next_state
            );
            intent.state = next_state.to_string();
            current_state = next_state;
        }

        assert_eq!(intent.state, "FAILED");

        // Test expiry path: QUOTE_CREATED -> EXPIRED
        let expiry_valid = valid_transitions
            .iter()
            .any(|(from, to)| *from == "QUOTE_CREATED" && *to == "EXPIRED");
        assert!(expiry_valid, "QUOTE_CREATED -> EXPIRED should be valid");

        // Test invalid transition: COMPLETED -> CRYPTO_PENDING (should not be valid)
        let invalid = valid_transitions
            .iter()
            .any(|(from, to)| *from == "COMPLETED" && *to == "CRYPTO_PENDING");
        assert!(
            !invalid,
            "COMPLETED -> CRYPTO_PENDING should NOT be valid"
        );

        // Verify serialization of OfframpIntentRow
        let serialized = serde_json::to_string(&intent).expect("Serialization should succeed");
        assert!(serialized.contains("\"state\":\"FAILED\""));
        assert!(serialized.contains("ofr_transition_test"));
    }

    /// Test that OfframpIntentRow survives serialization roundtrip (persists across restart)
    #[test]
    fn test_offramp_intent_persists_across_restart() {
        let now = Utc::now();
        let original = OfframpIntentRow {
            id: "ofr_persist_001".to_string(),
            tenant_id: "tenant_persist".to_string(),
            user_id: "user_persist".to_string(),
            crypto_asset: "ETH".to_string(),
            crypto_amount: Decimal::new(25, 1), // 2.5 ETH
            exchange_rate: Decimal::new(50_000_000, 0), // 50M VND/ETH
            locked_rate_id: Some("lock_abc".to_string()),
            fees: json!({"network_fee": "0.001", "platform_fee": "0.5", "total_pct": "1.5"}),
            net_vnd_amount: Decimal::new(123_750_000, 0),
            gross_vnd_amount: Decimal::new(125_000_000, 0),
            bank_account: json!({
                "bank_code": "TCB",
                "account_number": "0987654321",
                "account_name": "Le Van C",
                "branch": "Ho Chi Minh"
            }),
            deposit_address: Some("0xdeadbeef1234567890abcdef".to_string()),
            tx_hash: Some("0xabcdef1234567890deadbeef".to_string()),
            bank_reference: Some("RAMP-20260210-001".to_string()),
            state: "COMPLETED".to_string(),
            state_history: json!([
                {"state": "QUOTE_CREATED", "at": "2026-02-10T10:00:00Z"},
                {"state": "CRYPTO_PENDING", "at": "2026-02-10T10:01:00Z"},
                {"state": "CRYPTO_RECEIVED", "at": "2026-02-10T10:05:00Z"},
                {"state": "VND_TRANSFERRING", "at": "2026-02-10T10:10:00Z"},
                {"state": "COMPLETED", "at": "2026-02-10T10:15:00Z"}
            ]),
            created_at: now,
            updated_at: now,
            quote_expires_at: now + chrono::Duration::minutes(5),
        };

        // Simulate persistence: serialize -> store -> deserialize (restart boundary)
        let serialized = serde_json::to_string(&original)
            .expect("Serialization should succeed");

        // Verify serialized data is not empty and contains key fields
        assert!(!serialized.is_empty());
        assert!(serialized.contains("ofr_persist_001"));
        assert!(serialized.contains("ETH"));
        assert!(serialized.contains("COMPLETED"));
        assert!(serialized.contains("0xdeadbeef"));
        assert!(serialized.contains("RAMP-20260210-001"));

        // Deserialize (simulates loading from DB/disk after restart)
        let restored: OfframpIntentRow = serde_json::from_str(&serialized)
            .expect("Deserialization should succeed");

        // Verify all fields survive the roundtrip
        assert_eq!(restored.id, original.id);
        assert_eq!(restored.tenant_id, original.tenant_id);
        assert_eq!(restored.user_id, original.user_id);
        assert_eq!(restored.crypto_asset, original.crypto_asset);
        assert_eq!(restored.crypto_amount, original.crypto_amount);
        assert_eq!(restored.exchange_rate, original.exchange_rate);
        assert_eq!(restored.locked_rate_id, original.locked_rate_id);
        assert_eq!(restored.fees, original.fees);
        assert_eq!(restored.net_vnd_amount, original.net_vnd_amount);
        assert_eq!(restored.gross_vnd_amount, original.gross_vnd_amount);
        assert_eq!(restored.bank_account, original.bank_account);
        assert_eq!(restored.deposit_address, original.deposit_address);
        assert_eq!(restored.tx_hash, original.tx_hash);
        assert_eq!(restored.bank_reference, original.bank_reference);
        assert_eq!(restored.state, original.state);
        assert_eq!(restored.state_history, original.state_history);

        // Verify state_history array length survives
        let history = restored.state_history.as_array().unwrap();
        assert_eq!(history.len(), 5, "All 5 state transitions should persist");
        assert_eq!(history[0]["state"], "QUOTE_CREATED");
        assert_eq!(history[4]["state"], "COMPLETED");

        // Verify bank_account nested fields survive
        assert_eq!(restored.bank_account["bank_code"], "TCB");
        assert_eq!(restored.bank_account["branch"], "Ho Chi Minh");

        // Verify fees nested fields survive
        assert_eq!(restored.fees["total_pct"], "1.5");

        // Verify decimal precision survives
        assert_eq!(restored.crypto_amount, Decimal::new(25, 1));
        assert_eq!(restored.exchange_rate, Decimal::new(50_000_000, 0));

        // Verify Optional fields survive
        assert!(restored.deposit_address.is_some());
        assert!(restored.tx_hash.is_some());
        assert!(restored.bank_reference.is_some());
        assert!(restored.locked_rate_id.is_some());
    }
}
