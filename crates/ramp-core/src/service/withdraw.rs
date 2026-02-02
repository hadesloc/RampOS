//! On-chain Withdraw Service
//!
//! Handles crypto withdrawals from user accounts to external wallet addresses.
//! Flow: Create -> Policy Check -> KYT Check -> Sign -> Broadcast -> Confirm

use chrono::{Duration, Utc};
use ramp_common::{
    intent::WithdrawState,
    ledger::{patterns, AccountType, LedgerCurrency},
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

/// Request to create a withdraw intent
#[derive(Debug, Clone)]
pub struct CreateWithdrawRequest {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub chain_id: ChainId,
    pub token_address: Option<WalletAddress>,
    pub amount: Decimal,
    pub symbol: CryptoSymbol,
    pub to_address: WalletAddress,
    pub idempotency_key: Option<IdempotencyKey>,
    pub metadata: serde_json::Value,
}

/// Response from creating a withdraw intent
#[derive(Debug, Clone)]
pub struct CreateWithdrawResponse {
    pub intent_id: IntentId,
    pub status: WithdrawState,
    pub estimated_gas: Option<Decimal>,
}

/// Request to execute a withdraw (submit UserOp)
#[derive(Debug, Clone)]
pub struct ExecuteWithdrawRequest {
    pub tenant_id: TenantId,
    pub intent_id: IntentId,
    /// UserOperation hash from bundler
    pub user_op_hash: String,
}

/// Request to confirm a withdraw when tx is mined
#[derive(Debug, Clone)]
pub struct ConfirmWithdrawRequest {
    pub tenant_id: TenantId,
    pub intent_id: IntentId,
    pub tx_hash: TxHash,
    pub block_number: u64,
    pub success: bool,
}

/// KYT check request for withdraw destination
#[derive(Debug, Clone)]
pub struct WithdrawKytRequest {
    pub tenant_id: TenantId,
    pub intent_id: IntentId,
    pub kyt_score: f64,
    pub kyt_provider: String,
    pub risk_flags: Vec<String>,
}

/// Withdraw service for handling on-chain crypto withdrawals
pub struct WithdrawService {
    intent_repo: Arc<dyn IntentRepository>,
    ledger_repo: Arc<dyn LedgerRepository>,
    user_repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    /// KYT score threshold for flagging (0.0 - 1.0, higher = riskier)
    kyt_threshold: f64,
    /// Maximum withdraw amount per transaction (in crypto units)
    max_withdraw_amount: Decimal,
}

impl WithdrawService {
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
            kyt_threshold: 0.7,
            max_withdraw_amount: Decimal::from(100), // 100 units max per tx
        }
    }

    /// Create a withdraw service with custom KYT threshold
    pub fn with_kyt_threshold(mut self, threshold: f64) -> Self {
        self.kyt_threshold = threshold;
        self
    }

    /// Create a withdraw service with custom max amount
    pub fn with_max_amount(mut self, max_amount: Decimal) -> Self {
        self.max_withdraw_amount = max_amount;
        self
    }

    /// Create a new withdraw intent
    pub async fn create_withdraw(&self, req: CreateWithdrawRequest) -> Result<CreateWithdrawResponse> {
        // Check idempotency
        if let Some(ref key) = req.idempotency_key {
            if let Some(existing) = self
                .intent_repo
                .get_by_idempotency_key(&req.tenant_id, key)
                .await?
            {
                info!("Returning existing withdraw intent for idempotency key");
                return Ok(CreateWithdrawResponse {
                    intent_id: IntentId(existing.id),
                    status: parse_withdraw_state(&existing.state),
                    estimated_gas: None,
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

        // Validate withdraw amount
        if req.amount <= Decimal::ZERO {
            return Err(Error::Validation("Withdraw amount must be positive".into()));
        }

        if req.amount > self.max_withdraw_amount {
            return Err(Error::UserLimitExceeded {
                limit_type: format!(
                    "Withdraw amount {} exceeds maximum {}",
                    req.amount, self.max_withdraw_amount
                ),
            });
        }

        // Validate destination address
        if !req.to_address.is_valid_evm() {
            return Err(Error::Validation("Invalid destination address".into()));
        }

        // SECURITY: We do NOT check balance here anymore to prevent race conditions.
        // The balance check is now done atomically with the ledger transaction
        // using SELECT FOR UPDATE to acquire a row lock.
        let crypto_currency = symbol_to_ledger_currency(&req.symbol);

        // Generate intent ID
        let intent_id = IntentId::new_withdraw();
        let now = Utc::now();
        // Withdraw expires in 24 hours
        let expires_at = now + Duration::hours(24);

        // Create intent row
        let intent_row = IntentRow {
            id: intent_id.0.clone(),
            tenant_id: req.tenant_id.0.clone(),
            user_id: req.user_id.0.clone(),
            intent_type: "WITHDRAW_ONCHAIN".to_string(),
            state: "CREATED".to_string(),
            state_history: serde_json::json!([{
                "from": null,
                "to": "CREATED",
                "at": now
            }]),
            amount: req.amount,
            currency: req.symbol.to_string(),
            actual_amount: None,
            rails_provider: None,
            reference_code: None,
            bank_tx_id: None,
            chain_id: req.chain_id.evm_chain_id().map(|c| c.to_string()),
            tx_hash: None,
            from_address: None, // Will be set when UserOp is created (smart wallet address)
            to_address: Some(req.to_address.0.clone()),
            metadata: serde_json::json!({
                "token_address": req.token_address.as_ref().map(|a| &a.0),
                "original_metadata": req.metadata
            }),
            idempotency_key: req.idempotency_key.as_ref().map(|k| k.0.clone()),
            created_at: now,
            updated_at: now,
            expires_at: Some(expires_at),
            completed_at: None,
        };

        // Save to database
        self.intent_repo.create(&intent_row).await?;

        // Run initial policy check
        let policy_approved = self.check_withdraw_policy(&req).await?;

        if policy_approved {
            self.intent_repo
                .update_state(&req.tenant_id, &intent_id, "POLICY_APPROVED")
                .await?;

            // SECURITY FIX: Use atomic balance check and transaction recording
            // This prevents race conditions where concurrent withdrawals could
            // exceed the available balance.
            let tx = patterns::withdraw_crypto_initiated(
                req.tenant_id.clone(),
                req.user_id.clone(),
                intent_id.clone(),
                req.amount,
                crypto_currency.clone(),
            )?;

            // Atomically check balance and record transaction with row locking
            match self.ledger_repo.check_balance_and_record_transaction(
                req.amount,
                &req.user_id,
                &AccountType::LiabilityUserCrypto,
                &crypto_currency,
                tx,
            ).await {
                Ok(_balance) => {
                    // Transaction recorded successfully
                }
                Err(Error::InsufficientBalance { required, available }) => {
                    // Rollback the intent state
                    self.intent_repo
                        .update_state(&req.tenant_id, &intent_id, "REJECTED_INSUFFICIENT_BALANCE")
                        .await?;
                    return Err(Error::InsufficientBalance { required, available });
                }
                Err(e) => {
                    // Other database error
                    return Err(e);
                }
            }
        } else {
            self.intent_repo
                .update_state(&req.tenant_id, &intent_id, "REJECTED_BY_POLICY")
                .await?;
        }

        // Publish event
        self.event_publisher
            .publish_intent_created(&intent_id, &req.tenant_id)
            .await?;

        info!(
            intent_id = %intent_id,
            amount = %req.amount,
            symbol = %req.symbol,
            to_address = %req.to_address,
            policy_approved = policy_approved,
            "Withdraw intent created"
        );

        Ok(CreateWithdrawResponse {
            intent_id,
            status: if policy_approved {
                WithdrawState::PolicyApproved
            } else {
                WithdrawState::RejectedByPolicy
            },
            estimated_gas: None, // Would be populated by gas estimator
        })
    }

    /// Process KYT check for withdraw destination
    pub async fn process_kyt_check(&self, req: WithdrawKytRequest) -> Result<WithdrawState> {
        // Find intent
        let intent = self
            .intent_repo
            .get_by_id(&req.tenant_id, &req.intent_id)
            .await?
            .ok_or_else(|| Error::IntentNotFound(req.intent_id.0.clone()))?;

        // Validate state
        if intent.state != "POLICY_APPROVED" {
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
                "Withdraw destination flagged by KYT"
            );

            self.intent_repo
                .update_state(&req.tenant_id, &req.intent_id, "KYT_FLAGGED")
                .await?;

            self.event_publisher
                .publish_risk_review_required(&req.intent_id, &req.tenant_id)
                .await?;

            return Ok(WithdrawState::KytFlagged);
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
            "Withdraw passed KYT check"
        );

        Ok(WithdrawState::KytChecked)
    }

    /// Mark withdraw as signed (UserOp signed and ready to broadcast)
    pub async fn mark_signed(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
        from_address: &WalletAddress,
    ) -> Result<()> {
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
                to: "SIGNED".to_string(),
            });
        }

        // Update state
        self.intent_repo
            .update_state(tenant_id, intent_id, "SIGNED")
            .await?;

        self.event_publisher
            .publish_intent_status_changed(intent_id, tenant_id, "SIGNED")
            .await?;

        info!(
            intent_id = %intent_id,
            from_address = %from_address,
            "Withdraw signed"
        );

        Ok(())
    }

    /// Execute withdraw by submitting UserOp to bundler
    pub async fn execute_withdraw(&self, req: ExecuteWithdrawRequest) -> Result<WithdrawState> {
        // Find intent
        let intent = self
            .intent_repo
            .get_by_id(&req.tenant_id, &req.intent_id)
            .await?
            .ok_or_else(|| Error::IntentNotFound(req.intent_id.0.clone()))?;

        // Validate state
        if intent.state != "SIGNED" {
            return Err(Error::InvalidStateTransition {
                from: intent.state,
                to: "BROADCASTED".to_string(),
            });
        }

        // Update state to broadcasted
        self.intent_repo
            .update_state(&req.tenant_id, &req.intent_id, "BROADCASTED")
            .await?;

        self.event_publisher
            .publish_intent_status_changed(&req.intent_id, &req.tenant_id, "BROADCASTED")
            .await?;

        info!(
            intent_id = %req.intent_id,
            user_op_hash = %req.user_op_hash,
            "Withdraw broadcasted"
        );

        // Move to confirming state
        self.intent_repo
            .update_state(&req.tenant_id, &req.intent_id, "CONFIRMING")
            .await?;

        Ok(WithdrawState::Confirming)
    }

    /// Confirm withdraw when on-chain transaction is mined
    pub async fn confirm_withdraw(&self, req: ConfirmWithdrawRequest) -> Result<()> {
        // Find intent
        let intent = self
            .intent_repo
            .get_by_id(&req.tenant_id, &req.intent_id)
            .await?
            .ok_or_else(|| Error::IntentNotFound(req.intent_id.0.clone()))?;

        // Validate state
        if intent.state != "CONFIRMING" && intent.state != "BROADCASTED" {
            return Err(Error::InvalidStateTransition {
                from: intent.state,
                to: "CONFIRMED".to_string(),
            });
        }

        let crypto_currency = LedgerCurrency::from_symbol(&intent.currency);

        if req.success {
            // Update state to confirmed
            self.intent_repo
                .update_state(&req.tenant_id, &req.intent_id, "CONFIRMED")
                .await?;

            // Complete clearing entries - finalize the withdraw
            let tx = patterns::withdraw_crypto_confirmed(
                req.tenant_id.clone(),
                req.intent_id.clone(),
                intent.amount,
                crypto_currency,
            )?;

            self.ledger_repo.record_transaction(tx).await?;

            // Mark as completed
            self.intent_repo
                .update_state(&req.tenant_id, &req.intent_id, "COMPLETED")
                .await?;

            self.event_publisher
                .publish_intent_status_changed(&req.intent_id, &req.tenant_id, "COMPLETED")
                .await?;

            info!(
                intent_id = %req.intent_id,
                tx_hash = %req.tx_hash,
                block_number = req.block_number,
                "Withdraw confirmed and completed"
            );
        } else {
            // Transaction failed - reverse the hold
            let user_id = UserId::new(&intent.user_id);

            let tx = patterns::withdraw_crypto_reversed(
                req.tenant_id.clone(),
                user_id,
                req.intent_id.clone(),
                intent.amount,
                crypto_currency,
            )?;

            self.ledger_repo.record_transaction(tx).await?;

            self.intent_repo
                .update_state(&req.tenant_id, &req.intent_id, "BROADCAST_FAILED")
                .await?;

            self.event_publisher
                .publish_intent_status_changed(&req.intent_id, &req.tenant_id, "BROADCAST_FAILED")
                .await?;

            warn!(
                intent_id = %req.intent_id,
                tx_hash = %req.tx_hash,
                "Withdraw transaction failed - funds returned to user"
            );
        }

        Ok(())
    }

    /// Cancel a withdraw that hasn't been broadcasted yet
    pub async fn cancel_withdraw(&self, tenant_id: &TenantId, intent_id: &IntentId) -> Result<()> {
        // Find intent
        let intent = self
            .intent_repo
            .get_by_id(tenant_id, intent_id)
            .await?
            .ok_or_else(|| Error::IntentNotFound(intent_id.0.clone()))?;

        // Can only cancel if not yet broadcasted
        let cancellable_states = [
            "CREATED",
            "POLICY_APPROVED",
            "KYT_CHECKED",
            "SIGNED",
            "KYT_FLAGGED",
        ];

        if !cancellable_states.contains(&intent.state.as_str()) {
            return Err(Error::InvalidStateTransition {
                from: intent.state,
                to: "CANCELLED".to_string(),
            });
        }

        // If funds were held, return them
        if intent.state != "CREATED" && intent.state != "REJECTED_BY_POLICY" {
            let user_id = UserId::new(&intent.user_id);
            let crypto_currency = LedgerCurrency::from_symbol(&intent.currency);

            let tx = patterns::withdraw_crypto_reversed(
                tenant_id.clone(),
                user_id,
                intent_id.clone(),
                intent.amount,
                crypto_currency,
            )?;

            self.ledger_repo.record_transaction(tx).await?;
        }

        // Update state
        self.intent_repo
            .update_state(tenant_id, intent_id, "CANCELLED")
            .await?;

        self.event_publisher
            .publish_intent_status_changed(intent_id, tenant_id, "CANCELLED")
            .await?;

        info!(intent_id = %intent_id, "Withdraw cancelled");

        Ok(())
    }

    /// Simple policy check (placeholder)
    async fn check_withdraw_policy(&self, req: &CreateWithdrawRequest) -> Result<bool> {
        // In production, this would:
        // - Check velocity limits
        // - Check amount limits based on KYC tier
        // - Run AML rules
        // - Check if destination is whitelisted
        // - Check cooling-off periods for new addresses

        // For now, approve all withdrawals
        Ok(true)
    }
}

fn parse_withdraw_state(state: &str) -> WithdrawState {
    match state {
        "CREATED" => WithdrawState::Created,
        "POLICY_APPROVED" => WithdrawState::PolicyApproved,
        "KYT_CHECKED" => WithdrawState::KytChecked,
        "SIGNED" => WithdrawState::Signed,
        "BROADCASTED" => WithdrawState::Broadcasted,
        "CONFIRMING" => WithdrawState::Confirming,
        "CONFIRMED" => WithdrawState::Confirmed,
        "COMPLETED" => WithdrawState::Completed,
        "REJECTED_BY_POLICY" => WithdrawState::RejectedByPolicy,
        "KYT_FLAGGED" => WithdrawState::KytFlagged,
        "BROADCAST_FAILED" => WithdrawState::BroadcastFailed,
        "MANUAL_REVIEW" => WithdrawState::ManualReview,
        "CANCELLED" => WithdrawState::Cancelled,
        _ => WithdrawState::Created,
    }
}

fn symbol_to_ledger_currency(symbol: &CryptoSymbol) -> LedgerCurrency {
    match symbol {
        CryptoSymbol::BTC => LedgerCurrency::BTC,
        CryptoSymbol::ETH => LedgerCurrency::ETH,
        CryptoSymbol::USDT => LedgerCurrency::USDT,
        CryptoSymbol::USDC => LedgerCurrency::USDC,
        _ => LedgerCurrency::Other,
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
    async fn test_create_withdraw() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        // Set up balance
        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserCrypto,
            &LedgerCurrency::ETH,
            dec!(10.0),
        );

        let service = WithdrawService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        let req = CreateWithdrawRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(1.5),
            symbol: CryptoSymbol::ETH,
            to_address: WalletAddress::new("0x1234567890123456789012345678901234567890"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let res = service.create_withdraw(req).await.unwrap();

        assert_eq!(res.status, WithdrawState::PolicyApproved);

        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].state, "POLICY_APPROVED");
        assert_eq!(intents[0].intent_type, "WITHDRAW_ONCHAIN");

        // Check that funds were held
        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 1);
        assert!(txs[0].is_balanced());
    }

    #[tokio::test]
    async fn test_create_withdraw_insufficient_balance() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        // No balance set - defaults to 0

        let service = WithdrawService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        let req = CreateWithdrawRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(1.0),
            symbol: CryptoSymbol::ETH,
            to_address: WalletAddress::new("0x1234567890123456789012345678901234567890"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let result = service.create_withdraw(req).await;

        assert!(result.is_err());
        match result {
            Err(Error::InsufficientBalance { .. }) => {}
            _ => panic!("Expected InsufficientBalance error"),
        }
    }

    #[tokio::test]
    async fn test_withdraw_kyt_pass() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserCrypto,
            &LedgerCurrency::USDT,
            dec!(1000.0),
        );

        let service = WithdrawService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        )
        .with_kyt_threshold(0.7);

        // Create withdraw
        let create_req = CreateWithdrawRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(50.0),
            symbol: CryptoSymbol::USDT,
            to_address: WalletAddress::new("0xabcdef1234567890123456789012345678901234"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let create_res = service.create_withdraw(create_req).await.unwrap();
        let intent_id = create_res.intent_id;

        // Process KYT with low risk score
        let kyt_req = WithdrawKytRequest {
            tenant_id: TenantId::new("tenant1"),
            intent_id: intent_id.clone(),
            kyt_score: 0.2,
            kyt_provider: "chainalysis".to_string(),
            risk_flags: vec![],
        };

        let state = service.process_kyt_check(kyt_req).await.unwrap();
        assert_eq!(state, WithdrawState::KytChecked);
    }

    #[tokio::test]
    async fn test_withdraw_kyt_flagged() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserCrypto,
            &LedgerCurrency::ETH,
            dec!(100.0),
        );

        let service = WithdrawService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        )
        .with_kyt_threshold(0.7);

        // Create withdraw
        let create_req = CreateWithdrawRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(10.0),
            symbol: CryptoSymbol::ETH,
            to_address: WalletAddress::new("0x1111111111111111111111111111111111111111"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let create_res = service.create_withdraw(create_req).await.unwrap();
        let intent_id = create_res.intent_id;

        // Process KYT with high risk score
        let kyt_req = WithdrawKytRequest {
            tenant_id: TenantId::new("tenant1"),
            intent_id: intent_id.clone(),
            kyt_score: 0.9,
            kyt_provider: "chainalysis".to_string(),
            risk_flags: vec!["sanctioned".to_string()],
        };

        let state = service.process_kyt_check(kyt_req).await.unwrap();
        assert_eq!(state, WithdrawState::KytFlagged);
    }

    #[tokio::test]
    async fn test_withdraw_full_flow() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserCrypto,
            &LedgerCurrency::ETH,
            dec!(5.0),
        );

        let service = WithdrawService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        let tenant_id = TenantId::new("tenant1");

        // 1. Create withdraw
        let create_req = CreateWithdrawRequest {
            tenant_id: tenant_id.clone(),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(1.0),
            symbol: CryptoSymbol::ETH,
            to_address: WalletAddress::new("0x2222222222222222222222222222222222222222"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let create_res = service.create_withdraw(create_req).await.unwrap();
        let intent_id = create_res.intent_id;

        // 2. KYT check
        let kyt_req = WithdrawKytRequest {
            tenant_id: tenant_id.clone(),
            intent_id: intent_id.clone(),
            kyt_score: 0.1,
            kyt_provider: "test".to_string(),
            risk_flags: vec![],
        };
        service.process_kyt_check(kyt_req).await.unwrap();

        // 3. Mark signed
        let from_addr = WalletAddress::new("0x3333333333333333333333333333333333333333");
        service.mark_signed(&tenant_id, &intent_id, &from_addr).await.unwrap();

        // 4. Execute (broadcast)
        let exec_req = ExecuteWithdrawRequest {
            tenant_id: tenant_id.clone(),
            intent_id: intent_id.clone(),
            user_op_hash: "0xuserophash".to_string(),
        };
        service.execute_withdraw(exec_req).await.unwrap();

        // 5. Confirm
        let confirm_req = ConfirmWithdrawRequest {
            tenant_id: tenant_id.clone(),
            intent_id: intent_id.clone(),
            tx_hash: TxHash::new("0xtxhash"),
            block_number: 12345,
            success: true,
        };
        service.confirm_withdraw(confirm_req).await.unwrap();

        // Verify final state
        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents[0].state, "COMPLETED");

        // Verify ledger transactions (initiate + confirm)
        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 2);
    }

    #[tokio::test]
    async fn test_cancel_withdraw() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserCrypto,
            &LedgerCurrency::ETH,
            dec!(10.0),
        );

        let service = WithdrawService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        let tenant_id = TenantId::new("tenant1");

        // Create withdraw
        let create_req = CreateWithdrawRequest {
            tenant_id: tenant_id.clone(),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(2.0),
            symbol: CryptoSymbol::ETH,
            to_address: WalletAddress::new("0x4444444444444444444444444444444444444444"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let create_res = service.create_withdraw(create_req).await.unwrap();
        let intent_id = create_res.intent_id;

        // Cancel
        service.cancel_withdraw(&tenant_id, &intent_id).await.unwrap();

        // Verify cancelled
        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents[0].state, "CANCELLED");

        // Verify funds returned (initiate + reverse = 2 transactions)
        let txs = ledger_repo.transactions.lock().unwrap();
        assert_eq!(txs.len(), 2);
    }

    #[tokio::test]
    async fn test_withdraw_idempotency() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserCrypto,
            &LedgerCurrency::ETH,
            dec!(10.0),
        );

        let service = WithdrawService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        let req = CreateWithdrawRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(1.0),
            symbol: CryptoSymbol::ETH,
            to_address: WalletAddress::new("0x5555555555555555555555555555555555555555"),
            idempotency_key: Some(IdempotencyKey::new("withdraw-abc")),
            metadata: serde_json::json!({}),
        };

        // First call
        let res1 = service.create_withdraw(req.clone()).await.unwrap();

        // Second call with same idempotency key
        let res2 = service.create_withdraw(req).await.unwrap();

        // Should return same intent
        assert_eq!(res1.intent_id, res2.intent_id);

        // Should only have one intent
        let intents = intent_repo.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
    }

    #[tokio::test]
    async fn test_invalid_address() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        user_repo.add_user(create_test_user());

        ledger_repo.set_balance(
            &TenantId::new("tenant1"),
            Some(&UserId::new("user1")),
            &AccountType::LiabilityUserCrypto,
            &LedgerCurrency::ETH,
            dec!(10.0),
        );

        let service = WithdrawService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        );

        let req = CreateWithdrawRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            chain_id: ChainId::Ethereum,
            token_address: None,
            amount: dec!(1.0),
            symbol: CryptoSymbol::ETH,
            to_address: WalletAddress::new("invalid-address"),
            idempotency_key: None,
            metadata: serde_json::json!({}),
        };

        let result = service.create_withdraw(req).await;

        assert!(result.is_err());
        match result {
            Err(Error::Validation(msg)) => {
                assert!(msg.contains("Invalid destination address"));
            }
            _ => panic!("Expected Validation error"),
        }
    }
}
