//! On-chain Deposit Service
//!
//! Handles crypto deposits from external wallets to user smart wallet addresses.
//! Flow: Detect on-chain transfer -> Confirm -> KYT check -> Credit ledger

use chrono::Utc;
use ramp_common::{
    intent::DepositState,
    ledger::{patterns, LedgerCurrency},
    types::*,
    Error, Result,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, warn};

use crate::repository::{
    intent::{IntentRepository, IntentRow},
    ledger::LedgerRepository,
    user::UserRepository,
};
use crate::event::EventPublisher;

/// Request to create a deposit intent when an on-chain transfer is detected
#[derive(Debug, Clone)]
pub struct CreateDepositRequest {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub chain_id: ChainId,
    pub token_address: Option<WalletAddress>,
    pub amount: Decimal,
    pub symbol: CryptoSymbol,
    pub from_address: WalletAddress,
    pub to_address: WalletAddress,
    pub tx_hash: TxHash,
    pub idempotency_key: Option<IdempotencyKey>,
    pub metadata: serde_json::Value,
}

/// Response from creating a deposit intent
#[derive(Debug, Clone)]
pub struct CreateDepositResponse {
    pub intent_id: IntentId,
    pub deposit_address: WalletAddress,
    pub status: DepositState,
    pub required_confirmations: u32,
}

/// Request to confirm a deposit after sufficient block confirmations
#[derive(Debug, Clone)]
pub struct ConfirmDepositRequest {
    pub tenant_id: TenantId,
    pub intent_id: IntentId,
    pub confirmations: u32,
    pub block_number: u64,
}

/// Request to update deposit with KYT (Know Your Transaction) check result
#[derive(Debug, Clone)]
pub struct KytCheckRequest {
    pub tenant_id: TenantId,
    pub intent_id: IntentId,
    pub kyt_score: f64,
    pub kyt_provider: String,
    pub risk_flags: Vec<String>,
}

/// Deposit service for handling on-chain crypto deposits
pub struct DepositService {
    intent_repo: Arc<dyn IntentRepository>,
    ledger_repo: Arc<dyn LedgerRepository>,
    user_repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    /// Required confirmations per chain (simplified)
    required_confirmations: u32,
    /// KYT score threshold for flagging (0.0 - 1.0, higher = riskier)
    kyt_threshold: f64,
}

impl DepositService {
    pub fn new(
        intent_repo: Arc<dyn IntentRepository>,
        ledger_repo: Arc<dyn LedgerRepository>,
        user_repo: Arc<dyn UserRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            intent_repo,
            ledger_repo,
            user_repo,
            event_publisher,
            required_confirmations: 12, // Default for EVM chains
            kyt_threshold: 0.7,         // 70% risk score triggers review
        }
    }

    /// Create a deposit service with custom confirmation requirements
    pub fn with_confirmations(mut self, confirmations: u32) -> Self {
        self.required_confirmations = confirmations;
        self
    }

    /// Create a deposit service with custom KYT threshold
    pub fn with_kyt_threshold(mut self, threshold: f64) -> Self {
        self.kyt_threshold = threshold;
        self
    }

    /// Get required confirmations for a chain
    fn get_required_confirmations(&self, chain_id: &ChainId) -> u32 {
        match chain_id {
            ChainId::Ethereum => 12,
            ChainId::Polygon => 128,
            ChainId::BnbChain => 15,
            ChainId::Arbitrum => 1,  // L2 finality is faster
            ChainId::Optimism => 1,
            ChainId::Base => 1,
            ChainId::Solana => 32,
        }
    }

    /// Create a new deposit intent when an on-chain transfer is detected
    pub async fn create_deposit(&self, req: CreateDepositRequest) -> Result<CreateDepositResponse> {
        // Check idempotency
        if let Some(ref key) = req.idempotency_key {
            if let Some(existing) = self
                .intent_repo
                .get_by_idempotency_key(&req.tenant_id, key)
                .await?
            {
                info!("Returning existing deposit intent for idempotency key");
                return Ok(CreateDepositResponse {
                    intent_id: IntentId(existing.id),
                    deposit_address: req.to_address.clone(),
                    status: parse_deposit_state(&existing.state),
                    required_confirmations: self.get_required_confirmations(&req.chain_id),
                });
            }
        }

        // Verify user exists and is active
        let user = self
            .user_repo
            .get_by_id(&req.tenant_id, &req.user_id)
            .await?
            .ok_or_else(|| Error::UserNotFound(req.user_id.0.clone()))?;

        if user.status != "ACTIVE" {
            return Err(Error::UserKycNotVerified(req.user_id.0.clone()));
        }

        // Validate deposit amount (minimum check)
        if req.amount <= Decimal::ZERO {
            return Err(Error::Validation("Deposit amount must be positive".into()));
        }

        // Generate intent ID
        let intent_id = IntentId::new_deposit();
        let now = Utc::now();
        let required_confirmations = self.get_required_confirmations(&req.chain_id);

        // Create intent row
        let intent_row = IntentRow {
            id: intent_id.0.clone(),
            tenant_id: req.tenant_id.0.clone(),
            user_id: req.user_id.0.clone(),
            intent_type: "DEPOSIT_ONCHAIN".to_string(),
            state: "DETECTED".to_string(),
            state_history: serde_json::json!([{
                "from": null,
                "to": "DETECTED",
                "at": now
            }]),
            amount: req.amount,
            currency: req.symbol.to_string(),
            actual_amount: Some(req.amount),
            rails_provider: None,
            reference_code: None,
            bank_tx_id: None,
            chain_id: req.chain_id.evm_chain_id().map(|c| c.to_string()),
            tx_hash: Some(req.tx_hash.0.clone()),
            from_address: Some(req.from_address.0.clone()),
            to_address: Some(req.to_address.0.clone()),
            metadata: serde_json::json!({
                "token_address": req.token_address.as_ref().map(|a| &a.0),
                "required_confirmations": required_confirmations,
                "original_metadata": req.metadata
            }),
            idempotency_key: req.idempotency_key.map(|k| k.0),
            created_at: now,
            updated_at: now,
            expires_at: None, // Deposits don't expire
            completed_at: None,
        };

        // Save to database
        self.intent_repo.create(&intent_row).await?;

        // Publish event
        self.event_publisher
            .publish_intent_created(&intent_id, &req.tenant_id)
            .await?;

        info!(
            intent_id = %intent_id,
            tx_hash = %req.tx_hash,
            amount = %req.amount,
            symbol = %req.symbol,
            "Deposit detected"
        );

        Ok(CreateDepositResponse {
            intent_id,
            deposit_address: req.to_address,
            status: DepositState::Detected,
            required_confirmations,
        })
    }

    /// Update deposit confirmations and check if confirmed
    pub async fn update_confirmations(&self, req: ConfirmDepositRequest) -> Result<DepositState> {
        // Find intent
        let intent = self
            .intent_repo
            .get_by_id(&req.tenant_id, &req.intent_id)
            .await?
            .ok_or_else(|| Error::IntentNotFound(req.intent_id.0.clone()))?;

        let current_state = parse_deposit_state(&intent.state);

        // Only process if in valid state
        if !matches!(current_state, DepositState::Detected | DepositState::Confirming) {
            return Ok(current_state);
        }

        // Get required confirmations from metadata
        let required_confirmations: u32 = intent.metadata
            .get("required_confirmations")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(self.required_confirmations);

        // Update to confirming state if not already
        if current_state == DepositState::Detected {
            self.intent_repo
                .update_state(&req.tenant_id, &req.intent_id, "CONFIRMING")
                .await?;

            self.event_publisher
                .publish_intent_status_changed(&req.intent_id, &req.tenant_id, "CONFIRMING")
                .await?;
        }

        // Check if we have enough confirmations
        if req.confirmations >= required_confirmations {
            self.intent_repo
                .update_state(&req.tenant_id, &req.intent_id, "CONFIRMED")
                .await?;

            self.event_publisher
                .publish_intent_status_changed(&req.intent_id, &req.tenant_id, "CONFIRMED")
                .await?;

            info!(
                intent_id = %req.intent_id,
                confirmations = req.confirmations,
                required = required_confirmations,
                "Deposit confirmed"
            );

            return Ok(DepositState::Confirmed);
        }

        Ok(DepositState::Confirming)
    }

    /// Process KYT (Know Your Transaction) check result
    pub async fn process_kyt_check(&self, req: KytCheckRequest) -> Result<DepositState> {
        // Find intent
        let intent = self
            .intent_repo
            .get_by_id(&req.tenant_id, &req.intent_id)
            .await?
            .ok_or_else(|| Error::IntentNotFound(req.intent_id.0.clone()))?;

        // Validate state
        if intent.state != "CONFIRMED" {
            return Err(Error::InvalidStateTransition {
                from: intent.state,
                to: "KYT_CHECKED".to_string(),
            });
        }

        // Check KYT score against threshold
        if req.kyt_score >= self.kyt_threshold {
            warn!(
                intent_id = %req.intent_id,
                kyt_score = req.kyt_score,
                threshold = self.kyt_threshold,
                risk_flags = ?req.risk_flags,
                "Deposit flagged by KYT"
            );

            self.intent_repo
                .update_state(&req.tenant_id, &req.intent_id, "KYT_FLAGGED")
                .await?;

            self.event_publisher
                .publish_risk_review_required(&req.intent_id, &req.tenant_id)
                .await?;

            return Ok(DepositState::KytFlagged);
        }

        // KYT passed
        self.intent_repo
            .update_state(&req.tenant_id, &req.intent_id, "KYT_CHECKED")
            .await?;

        self.event_publisher
            .publish_intent_status_changed(&req.intent_id, &req.tenant_id, "KYT_CHECKED")
            .await?;

        info!(
            intent_id = %req.intent_id,
            kyt_score = req.kyt_score,
            "Deposit passed KYT check"
        );

        Ok(DepositState::KytChecked)
    }

    /// Credit the deposit to user's ledger balance
    pub async fn credit_deposit(&self, tenant_id: &TenantId, intent_id: &IntentId) -> Result<()> {
        // Find intent
        let intent = self
            .intent_repo
            .get_by_id(tenant_id, intent_id)
            .await?
            .ok_or_else(|| Error::IntentNotFound(intent_id.0.clone()))?;

        // Validate state
        if intent.state != "KYT_CHECKED" {
            return Err(Error::InvalidStateTransition {
                from: intent.state,
                to: "CREDITED".to_string(),
            });
        }

        let user_id = UserId::new(&intent.user_id);
        let crypto_currency = LedgerCurrency::from_symbol(&intent.currency);

        // Create ledger entries to credit crypto
        let tx = patterns::deposit_crypto_confirmed(
            tenant_id.clone(),
            user_id,
            intent_id.clone(),
            intent.amount,
            crypto_currency,
        )?;

        self.ledger_repo.record_transaction(tx).await?;

        // Update state to credited
        self.intent_repo
            .update_state(tenant_id, intent_id, "CREDITED")
            .await?;

        self.event_publisher
            .publish_intent_status_changed(intent_id, tenant_id, "CREDITED")
            .await?;

        info!(
            intent_id = %intent_id,
            amount = %intent.amount,
            currency = %intent.currency,
            "Deposit credited to user"
        );

        Ok(())
    }

    /// Complete the deposit (final state)
    pub async fn complete_deposit(&self, tenant_id: &TenantId, intent_id: &IntentId) -> Result<()> {
        // Find intent
        let intent = self
            .intent_repo
            .get_by_id(tenant_id, intent_id)
            .await?
            .ok_or_else(|| Error::IntentNotFound(intent_id.0.clone()))?;

        // Validate state
        if intent.state != "CREDITED" {
            return Err(Error::InvalidStateTransition {
                from: intent.state,
                to: "COMPLETED".to_string(),
            });
        }

        // Update to completed
        self.intent_repo
            .update_state(tenant_id, intent_id, "COMPLETED")
            .await?;

        self.event_publisher
            .publish_intent_status_changed(intent_id, tenant_id, "COMPLETED")
            .await?;

        info!(intent_id = %intent_id, "Deposit completed");

        Ok(())
    }

    /// Process the full deposit flow: KYT -> Credit -> Complete
    /// Used for automated processing after confirmation
    pub async fn process_confirmed_deposit(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
        kyt_score: f64,
        kyt_provider: &str,
        risk_flags: Vec<String>,
    ) -> Result<DepositState> {
        // Run KYT check
        let kyt_req = KytCheckRequest {
            tenant_id: tenant_id.clone(),
            intent_id: intent_id.clone(),
            kyt_score,
            kyt_provider: kyt_provider.to_string(),
            risk_flags,
        };

        let state = self.process_kyt_check(kyt_req).await?;

        if state == DepositState::KytFlagged {
            return Ok(state);
        }

        // Credit deposit
        self.credit_deposit(tenant_id, intent_id).await?;

        // Complete deposit
        self.complete_deposit(tenant_id, intent_id).await?;

        Ok(DepositState::Completed)
    }
}

fn parse_deposit_state(state: &str) -> DepositState {
    match state {
        "DETECTED" => DepositState::Detected,
        "CONFIRMING" => DepositState::Confirming,
        "CONFIRMED" => DepositState::Confirmed,
        "KYT_CHECKED" => DepositState::KytChecked,
        "CREDITED" => DepositState::Credited,
        "COMPLETED" => DepositState::Completed,
        "KYT_FLAGGED" => DepositState::KytFlagged,
        "MANUAL_REVIEW" => DepositState::ManualReview,
        "REJECTED" => DepositState::Rejected,
        _ => DepositState::Detected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{MockIntentRepository, MockLedgerRepository, MockUserRepository};
    use crate::event::InMemoryEventPublisher;
    use crate::repository::user::UserRow;
    use rust_decimal_macros::dec;

    fn create_test_user() -> UserRow {
        UserRow {
            id: "user1".to_string(),
            tenant_id: "tenant1".to_string(),
            status: "ACTIVE".to_string(),
            kyc_tier: 1,
            kyc_status: "VERIFIED".to_string(),
            kyc_verified_at: Some(Utc::now()),
            risk_score: None,
            risk_flags: serde_json::json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_create_deposit() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        let service = DepositService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        let req = CreateDepositRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(1.5),
            symbol: CryptoSymbol::ETH,
            from_address: WalletAddress::new("0x1234567890123456789012345678901234567890"),
            to_address: WalletAddress::new("0xabcdef1234567890123456789012345678901234"),
            tx_hash: TxHash::new("0xdeadbeef"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let res = service.create_deposit(req).await.unwrap();

        assert_eq!(res.status, DepositState::Detected);
        assert_eq!(res.required_confirmations, 12); // Ethereum default

        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].state, "DETECTED");
        assert_eq!(intents[0].intent_type, "DEPOSIT_ONCHAIN");
    }

    #[tokio::test]
    async fn test_deposit_confirmation_flow() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        let service = DepositService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        )
        .with_confirmations(12);

        // Create deposit
        let create_req = CreateDepositRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(1.0),
            symbol: CryptoSymbol::ETH,
            from_address: WalletAddress::new("0x1111111111111111111111111111111111111111"),
            to_address: WalletAddress::new("0x2222222222222222222222222222222222222222"),
            tx_hash: TxHash::new("0x3333"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let create_res = service.create_deposit(create_req).await.unwrap();
        let intent_id = create_res.intent_id;

        // Update with partial confirmations
        let confirm_req = ConfirmDepositRequest {
            tenant_id: TenantId::new("tenant1"),
            intent_id: intent_id.clone(),
            confirmations: 5,
            block_number: 100,
        };

        let state = service.update_confirmations(confirm_req).await.unwrap();
        assert_eq!(state, DepositState::Confirming);

        // Update with sufficient confirmations
        let confirm_req = ConfirmDepositRequest {
            tenant_id: TenantId::new("tenant1"),
            intent_id: intent_id.clone(),
            confirmations: 12,
            block_number: 107,
        };

        let state = service.update_confirmations(confirm_req).await.unwrap();
        assert_eq!(state, DepositState::Confirmed);
    }

    #[tokio::test]
    async fn test_deposit_kyt_pass() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        let service = DepositService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        )
        .with_kyt_threshold(0.7);

        // Create and confirm deposit
        let create_req = CreateDepositRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(0.5),
            symbol: CryptoSymbol::USDT,
            from_address: WalletAddress::new("0x1111111111111111111111111111111111111111"),
            to_address: WalletAddress::new("0x2222222222222222222222222222222222222222"),
            tx_hash: TxHash::new("0x4444"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let create_res = service.create_deposit(create_req).await.unwrap();
        let intent_id = create_res.intent_id;

        // Manually set state to CONFIRMED for testing
        intent_repo
            .update_state(&TenantId::new("tenant1"), &intent_id, "CONFIRMED")
            .await
            .unwrap();

        // Process KYT with low risk score (should pass)
        let kyt_req = KytCheckRequest {
            tenant_id: TenantId::new("tenant1"),
            intent_id: intent_id.clone(),
            kyt_score: 0.2, // Low risk
            kyt_provider: "chainalysis".to_string(),
            risk_flags: vec![],
        };

        let state = service.process_kyt_check(kyt_req).await.unwrap();
        assert_eq!(state, DepositState::KytChecked);
    }

    #[tokio::test]
    async fn test_deposit_kyt_flagged() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        let service = DepositService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        )
        .with_kyt_threshold(0.7);

        // Create deposit
        let create_req = CreateDepositRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(100.0),
            symbol: CryptoSymbol::ETH,
            from_address: WalletAddress::new("0x1111111111111111111111111111111111111111"),
            to_address: WalletAddress::new("0x2222222222222222222222222222222222222222"),
            tx_hash: TxHash::new("0x5555"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let create_res = service.create_deposit(create_req).await.unwrap();
        let intent_id = create_res.intent_id;

        // Manually set state to CONFIRMED
        intent_repo
            .update_state(&TenantId::new("tenant1"), &intent_id, "CONFIRMED")
            .await
            .unwrap();

        // Process KYT with high risk score (should flag)
        let kyt_req = KytCheckRequest {
            tenant_id: TenantId::new("tenant1"),
            intent_id: intent_id.clone(),
            kyt_score: 0.85, // High risk
            kyt_provider: "chainalysis".to_string(),
            risk_flags: vec!["sanctions".to_string(), "mixer".to_string()],
        };

        let state = service.process_kyt_check(kyt_req).await.unwrap();
        assert_eq!(state, DepositState::KytFlagged);
    }

    #[tokio::test]
    async fn test_credit_deposit() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        let service = DepositService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        // Create deposit
        let create_req = CreateDepositRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(2.0),
            symbol: CryptoSymbol::ETH,
            from_address: WalletAddress::new("0x1111111111111111111111111111111111111111"),
            to_address: WalletAddress::new("0x2222222222222222222222222222222222222222"),
            tx_hash: TxHash::new("0x6666"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let create_res = service.create_deposit(create_req).await.unwrap();
        let intent_id = create_res.intent_id;
        let tenant_id = TenantId::new("tenant1");

        // Move through states: DETECTED -> CONFIRMED -> KYT_CHECKED
        intent_repo.update_state(&tenant_id, &intent_id, "KYT_CHECKED").await.unwrap();

        // Credit deposit
        service.credit_deposit(&tenant_id, &intent_id).await.unwrap();

        // Check ledger transaction was recorded
        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);
        assert!(txs[0].is_balanced());

        // Check state updated
        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents[0].state, "CREDITED");
    }

    #[tokio::test]
    async fn test_idempotency() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        let service = DepositService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        let req = CreateDepositRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(1.0),
            symbol: CryptoSymbol::ETH,
            from_address: WalletAddress::new("0x1111111111111111111111111111111111111111"),
            to_address: WalletAddress::new("0x2222222222222222222222222222222222222222"),
            tx_hash: TxHash::new("0x7777"),
            idempotency_key: Some(IdempotencyKey::new("deposit-123")),
            metadata: serde_json::json!({}),
        };

        // First call
        let res1 = service.create_deposit(req.clone()).await.unwrap();

        // Second call with same idempotency key
        let res2 = service.create_deposit(req).await.unwrap();

        // Should return same intent
        assert_eq!(res1.intent_id, res2.intent_id);

        // Should only have one intent
        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
    }
}
