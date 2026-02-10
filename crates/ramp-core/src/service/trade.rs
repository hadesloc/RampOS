use chrono::{Duration, Utc};
use ramp_common::{
    intent::TradeState,
    ledger::{patterns, LedgerCurrency, LedgerError},
    types::*,
    Error, Result,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::info;

use crate::event::EventPublisher;
use crate::repository::{
    intent::{IntentRepository, IntentRow},
    ledger::LedgerRepository,
};

/// Trade executed event from exchange
#[derive(Debug, Clone)]
pub struct TradeExecutedRequest {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub trade_id: String,
    pub symbol: String, // e.g., "BTC/VND"
    pub price: Decimal,
    pub vnd_delta: VndAmount, // negative = user paid VND, positive = user received VND
    pub crypto_delta: Decimal, // negative = user paid crypto, positive = user received crypto
    pub timestamp: Timestamp,
    pub idempotency_key: Option<IdempotencyKey>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct TradeExecutedResponse {
    pub intent_id: IntentId,
    pub status: TradeState,
}

pub struct TradeService {
    intent_repo: Arc<dyn IntentRepository>,
    ledger_repo: Arc<dyn LedgerRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    /// Optional database pool for cross-repo atomic transactions.
    db_pool: Option<sqlx::PgPool>,
}

impl TradeService {
    pub fn new(
        intent_repo: Arc<dyn IntentRepository>,
        ledger_repo: Arc<dyn LedgerRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            intent_repo,
            ledger_repo,
            event_publisher,
            db_pool: None,
        }
    }

    /// Create a TradeService with a database pool for atomic transactions
    pub fn with_pool(
        intent_repo: Arc<dyn IntentRepository>,
        ledger_repo: Arc<dyn LedgerRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        pool: sqlx::PgPool,
    ) -> Self {
        Self {
            intent_repo,
            ledger_repo,
            event_publisher,
            db_pool: Some(pool),
        }
    }

    /// Record a trade executed event
    pub async fn record_trade(&self, req: TradeExecutedRequest) -> Result<TradeExecutedResponse> {
        // Check idempotency
        if let Some(ref key) = req.idempotency_key {
            if let Some(existing) = self
                .intent_repo
                .get_by_idempotency_key(&req.tenant_id, key)
                .await?
            {
                return Ok(TradeExecutedResponse {
                    intent_id: IntentId(existing.id),
                    status: TradeState::Completed,
                });
            }
        }

        // Generate intent ID
        let intent_id = IntentId::new_trade();
        let now = Utc::now();
        // Trade expires in 5 minutes
        let expires_at = now + Duration::minutes(5);

        // Create intent row
        let intent_row = IntentRow {
            id: intent_id.0.clone(),
            tenant_id: req.tenant_id.0.clone(),
            user_id: req.user_id.0.clone(),
            intent_type: "TRADE_EXECUTED".to_string(),
            state: TradeState::Recorded.to_string(),
            state_history: serde_json::json!([]),
            amount: req.vnd_delta.0.abs(),
            currency: "VND".to_string(),
            actual_amount: Some(req.crypto_delta.abs()),
            rails_provider: None,
            reference_code: Some(req.trade_id.clone()),
            bank_tx_id: None,
            chain_id: None,
            tx_hash: None,
            from_address: None,
            to_address: None,
            metadata: serde_json::json!({
                "trade_id": req.trade_id,
                "symbol": req.symbol,
                "price": req.price.to_string(),
                "vnd_delta": req.vnd_delta.0.to_string(),
                "crypto_delta": req.crypto_delta.to_string(),
                "timestamp": req.timestamp.0.to_rfc3339(),
                "original_metadata": req.metadata,
            }),
            idempotency_key: req.idempotency_key.as_ref().map(|k| k.0.clone()),
            created_at: now,
            updated_at: now,
            expires_at: Some(expires_at),
            completed_at: None,
        };

        // Save to database + create ledger entries + update state atomically
        let compliance_ok = self.post_trade_check(&req).await?;

        if let Some(ref pool) = self.db_pool {
            // Atomic path
            let mut db_tx = pool
                .begin()
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            // Set RLS context
            sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
                .bind(&req.tenant_id.0)
                .execute(&mut *db_tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            // 1. Create intent
            sqlx::query(
                r#"INSERT INTO intents (
                    id, tenant_id, user_id, intent_type, state, state_history,
                    amount, currency, actual_amount, rails_provider, reference_code,
                    bank_tx_id, chain_id, tx_hash, from_address, to_address,
                    metadata, idempotency_key, created_at, updated_at, expires_at
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                    $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
                )"#,
            )
            .bind(&intent_row.id)
            .bind(&intent_row.tenant_id)
            .bind(&intent_row.user_id)
            .bind(&intent_row.intent_type)
            .bind(&intent_row.state)
            .bind(&intent_row.state_history)
            .bind(intent_row.amount)
            .bind(&intent_row.currency)
            .bind(intent_row.actual_amount)
            .bind(&intent_row.rails_provider)
            .bind(&intent_row.reference_code)
            .bind(&intent_row.bank_tx_id)
            .bind(&intent_row.chain_id)
            .bind(&intent_row.tx_hash)
            .bind(&intent_row.from_address)
            .bind(&intent_row.to_address)
            .bind(&intent_row.metadata)
            .bind(&intent_row.idempotency_key)
            .bind(intent_row.created_at)
            .bind(intent_row.updated_at)
            .bind(intent_row.expires_at)
            .execute(&mut *db_tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

            if !compliance_ok {
                sqlx::query(
                    "UPDATE intents SET state = $1, updated_at = NOW() WHERE tenant_id = $2 AND id = $3",
                )
                .bind(&TradeState::ComplianceHold.to_string())
                .bind(&req.tenant_id.0)
                .bind(&intent_id.0)
                .execute(&mut *db_tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

                db_tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;

                self.event_publisher
                    .publish_risk_review_required(&intent_id, &req.tenant_id)
                    .await?;

                return Ok(TradeExecutedResponse {
                    intent_id,
                    status: TradeState::ComplianceHold,
                });
            }

            // 2. Update state to PostTradeChecked
            sqlx::query(
                "UPDATE intents SET state = $1, updated_at = NOW() WHERE tenant_id = $2 AND id = $3",
            )
            .bind(&TradeState::PostTradeChecked.to_string())
            .bind(&req.tenant_id.0)
            .bind(&intent_id.0)
            .execute(&mut *db_tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

            // 3. Create ledger entries
            let crypto_currency = self.parse_crypto_currency(&req.symbol);
            let is_buy = req.crypto_delta.is_sign_positive();

            let ledger_tx = patterns::trade_crypto_vnd(
                req.tenant_id.clone(),
                req.user_id.clone(),
                intent_id.clone(),
                req.vnd_delta.0.abs(),
                req.crypto_delta.abs(),
                crypto_currency,
                is_buy,
            )
            .map_err(|e: LedgerError| Error::LedgerError(e.to_string()))?;

            for entry in &ledger_tx.entries {
                sqlx::query(
                    r#"INSERT INTO ledger_entries
                       (id, tenant_id, user_id, intent_id, transaction_id,
                        account_type, direction, amount, currency,
                        balance_after, sequence, description, metadata, created_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())"#,
                )
                .bind(&entry.id.0)
                .bind(&req.tenant_id.0)
                .bind(&entry.user_id.as_ref().map(|u| &u.0))
                .bind(&intent_id.0)
                .bind(&ledger_tx.id)
                .bind(&entry.account_type.to_string())
                .bind(&entry.direction.to_string())
                .bind(entry.amount)
                .bind(&entry.currency.to_string())
                .bind(entry.amount)
                .bind(0i64)
                .bind(&entry.description)
                .bind(&serde_json::json!({}))
                .execute(&mut *db_tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
            }

            // 4. Update state to Completed
            sqlx::query(
                "UPDATE intents SET state = $1, completed_at = NOW(), updated_at = NOW() WHERE tenant_id = $2 AND id = $3",
            )
            .bind(&TradeState::Completed.to_string())
            .bind(&req.tenant_id.0)
            .bind(&intent_id.0)
            .execute(&mut *db_tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

            db_tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        } else {
            // Non-atomic fallback (tests / mock repos)
            self.intent_repo.create(&intent_row).await?;

            if !compliance_ok {
                self.intent_repo
                    .update_state(&req.tenant_id, &intent_id, &TradeState::ComplianceHold.to_string())
                    .await?;

                self.event_publisher
                    .publish_risk_review_required(&intent_id, &req.tenant_id)
                    .await?;

                return Ok(TradeExecutedResponse {
                    intent_id,
                    status: TradeState::ComplianceHold,
                });
            }

            self.intent_repo
                .update_state(&req.tenant_id, &intent_id, &TradeState::PostTradeChecked.to_string())
                .await?;

            let crypto_currency = self.parse_crypto_currency(&req.symbol);
            let is_buy = req.crypto_delta.is_sign_positive();

            let ledger_tx = patterns::trade_crypto_vnd(
                req.tenant_id.clone(),
                req.user_id.clone(),
                intent_id.clone(),
                req.vnd_delta.0.abs(),
                req.crypto_delta.abs(),
                crypto_currency,
                is_buy,
            )
            .map_err(|e: LedgerError| Error::LedgerError(e.to_string()))?;

            self.ledger_repo.record_transaction(ledger_tx).await?;

            self.intent_repo
                .update_state(&req.tenant_id, &intent_id, &TradeState::SettledLedger.to_string())
                .await?;

            self.intent_repo
                .update_state(&req.tenant_id, &intent_id, &TradeState::Completed.to_string())
                .await?;
        }

        // Publish event
        self.event_publisher
            .publish_intent_status_changed(&intent_id, &req.tenant_id, &TradeState::Completed.to_string())
            .await?;

        info!(
            intent_id = %intent_id,
            trade_id = %req.trade_id,
            symbol = %req.symbol,
            "Trade recorded and settled"
        );

        Ok(TradeExecutedResponse {
            intent_id,
            status: TradeState::Completed,
        })
    }

    /// Simple post-trade compliance check (placeholder)
    async fn post_trade_check(&self, req: &TradeExecutedRequest) -> Result<bool> {
        // In production, this would:
        // - Check for wash trading patterns
        // - Check velocity limits
        // - Check unusual price deviations
        // - Flag large trades for review

        // For now, flag trades over 1B VND
        Ok(req.vnd_delta.0.abs() <= Decimal::from(1_000_000_000))
    }

    /// Parse crypto currency from trading pair symbol
    fn parse_crypto_currency(&self, symbol: &str) -> LedgerCurrency {
        let base = symbol.split('/').next().unwrap_or("");
        match base.to_uppercase().as_str() {
            "BTC" => LedgerCurrency::BTC,
            "ETH" => LedgerCurrency::ETH,
            "USDT" => LedgerCurrency::USDT,
            "USDC" => LedgerCurrency::USDC,
            _ => LedgerCurrency::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::InMemoryEventPublisher;
    use crate::test_utils::{MockIntentRepository, MockLedgerRepository};
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_test_service() -> (
        TradeService,
        Arc<MockIntentRepository>,
        Arc<MockLedgerRepository>,
    ) {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        let service = TradeService::new(intent_repo.clone(), ledger_repo.clone(), event_publisher);

        (service, intent_repo, ledger_repo)
    }

    #[tokio::test]
    async fn test_record_trade_buy_success() {
        let (service, intent_repo, ledger_repo) = create_test_service();

        let req = TradeExecutedRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            trade_id: "trade123".to_string(),
            symbol: "BTC/VND".to_string(),
            price: dec!(1000000000),                      // 1B VND per BTC
            vnd_delta: VndAmount::from_i64(-100_000_000), // User paid 100M VND
            crypto_delta: dec!(0.1),                      // User received 0.1 BTC
            timestamp: Timestamp::now(),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let res = service.record_trade(req).await.unwrap();

        assert_eq!(res.status, TradeState::Completed);

        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].intent_type, "TRADE_EXECUTED");
        assert_eq!(intents[0].state, "COMPLETED");

        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);
    }

    #[tokio::test]
    async fn test_record_trade_sell_success() {
        let (service, intent_repo, _) = create_test_service();

        let req = TradeExecutedRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            trade_id: "trade456".to_string(),
            symbol: "ETH/VND".to_string(),
            price: dec!(50000000),                       // 50M VND per ETH
            vnd_delta: VndAmount::from_i64(100_000_000), // User received 100M VND
            crypto_delta: dec!(-2.0),                    // User paid 2 ETH
            timestamp: Timestamp::now(),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let res = service.record_trade(req).await.unwrap();

        assert_eq!(res.status, TradeState::Completed);

        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
    }

    #[tokio::test]
    async fn test_record_trade_idempotency() {
        let (service, intent_repo, _) = create_test_service();

        let req = TradeExecutedRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            trade_id: "trade789".to_string(),
            symbol: "BTC/VND".to_string(),
            price: dec!(1000000000),
            vnd_delta: VndAmount::from_i64(-100_000_000),
            crypto_delta: dec!(0.1),
            timestamp: Timestamp::now(),
            idempotency_key: Some(IdempotencyKey::new("idem-trade-1")),
            metadata: serde_json::json!({}),
        };

        // First call
        let res1 = service.record_trade(req.clone()).await.unwrap();

        // Second call with same idempotency key
        let req2 = TradeExecutedRequest {
            idempotency_key: Some(IdempotencyKey::new("idem-trade-1")),
            ..req
        };
        let res2 = service.record_trade(req2).await.unwrap();

        // Should return same intent
        assert_eq!(res1.intent_id, res2.intent_id);

        // Should only have created one intent
        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
    }

    #[tokio::test]
    async fn test_record_large_trade_compliance_hold() {
        let (service, intent_repo, _) = create_test_service();

        // Trade over 1B VND should trigger compliance hold
        let req = TradeExecutedRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            trade_id: "large-trade".to_string(),
            symbol: "BTC/VND".to_string(),
            price: dec!(1500000000),
            vnd_delta: VndAmount::from_i64(-1_500_000_000), // 1.5B VND
            crypto_delta: dec!(1.0),
            timestamp: Timestamp::now(),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let res = service.record_trade(req).await.unwrap();

        assert_eq!(res.status, TradeState::ComplianceHold);

        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents[0].state, "COMPLIANCE_HOLD");
    }

    #[test]
    fn test_parse_crypto_currency() {
        let service = TradeService::new(
            Arc::new(MockIntentRepository::new()),
            Arc::new(MockLedgerRepository::new()),
            Arc::new(InMemoryEventPublisher::new()),
        );

        assert_eq!(
            service.parse_crypto_currency("BTC/VND"),
            LedgerCurrency::BTC
        );
        assert_eq!(
            service.parse_crypto_currency("ETH/VND"),
            LedgerCurrency::ETH
        );
        assert_eq!(
            service.parse_crypto_currency("USDT/VND"),
            LedgerCurrency::USDT
        );
        assert_eq!(
            service.parse_crypto_currency("USDC/VND"),
            LedgerCurrency::USDC
        );
        assert_eq!(
            service.parse_crypto_currency("XRP/VND"),
            LedgerCurrency::Other
        );
    }
}
