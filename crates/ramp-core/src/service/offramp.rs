//! Off-Ramp Service (F16.02)
//!
//! Implements the full off-ramp flow: crypto -> VND bank transfer.
//!
//! State Machine:
//! QUOTE_CREATED -> CRYPTO_PENDING -> CRYPTO_RECEIVED -> CONVERTING ->
//! VND_TRANSFERRING -> COMPLETED
//!
//! Error states: FAILED, EXPIRED, CANCELLED

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use ramp_common::types::{BankAccount, CryptoSymbol};
use ramp_common::{Error, Result};

use super::exchange_rate::ExchangeRateService;
use super::offramp_fees::{OffRampFeeCalculator, FeeBreakdown};

// ============================================================================
// Off-Ramp State Machine
// ============================================================================

/// States for an off-ramp intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OffRampState {
    /// Quote has been created, waiting for user confirmation
    QuoteCreated,
    /// User confirmed, waiting for crypto deposit
    CryptoPending,
    /// Crypto deposit received on-chain
    CryptoReceived,
    /// Converting crypto to VND
    Converting,
    /// VND bank transfer in progress
    VndTransferring,
    /// Successfully completed
    Completed,
    /// Failed at some step
    Failed,
    /// Quote or intent expired
    Expired,
    /// Cancelled by user
    Cancelled,
}

impl OffRampState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            OffRampState::Completed | OffRampState::Failed | OffRampState::Expired | OffRampState::Cancelled
        )
    }

    pub fn allowed_transitions(&self) -> Vec<OffRampState> {
        match self {
            OffRampState::QuoteCreated => vec![
                OffRampState::CryptoPending,
                OffRampState::Expired,
                OffRampState::Cancelled,
            ],
            OffRampState::CryptoPending => vec![
                OffRampState::CryptoReceived,
                OffRampState::Expired,
                OffRampState::Cancelled,
            ],
            OffRampState::CryptoReceived => vec![
                OffRampState::Converting,
                OffRampState::Failed,
            ],
            OffRampState::Converting => vec![
                OffRampState::VndTransferring,
                OffRampState::Failed,
            ],
            OffRampState::VndTransferring => vec![
                OffRampState::Completed,
                OffRampState::Failed,
            ],
            // Terminal states
            OffRampState::Completed
            | OffRampState::Failed
            | OffRampState::Expired
            | OffRampState::Cancelled => vec![],
        }
    }

    pub fn can_transition_to(&self, target: OffRampState) -> bool {
        self.allowed_transitions().contains(&target)
    }
}

impl fmt::Display for OffRampState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OffRampState::QuoteCreated => write!(f, "QUOTE_CREATED"),
            OffRampState::CryptoPending => write!(f, "CRYPTO_PENDING"),
            OffRampState::CryptoReceived => write!(f, "CRYPTO_RECEIVED"),
            OffRampState::Converting => write!(f, "CONVERTING"),
            OffRampState::VndTransferring => write!(f, "VND_TRANSFERRING"),
            OffRampState::Completed => write!(f, "COMPLETED"),
            OffRampState::Failed => write!(f, "FAILED"),
            OffRampState::Expired => write!(f, "EXPIRED"),
            OffRampState::Cancelled => write!(f, "CANCELLED"),
        }
    }
}

impl FromStr for OffRampState {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "QUOTE_CREATED" => Ok(OffRampState::QuoteCreated),
            "CRYPTO_PENDING" => Ok(OffRampState::CryptoPending),
            "CRYPTO_RECEIVED" => Ok(OffRampState::CryptoReceived),
            "CONVERTING" => Ok(OffRampState::Converting),
            "VND_TRANSFERRING" => Ok(OffRampState::VndTransferring),
            "COMPLETED" => Ok(OffRampState::Completed),
            "FAILED" => Ok(OffRampState::Failed),
            "EXPIRED" => Ok(OffRampState::Expired),
            "CANCELLED" => Ok(OffRampState::Cancelled),
            _ => Err(format!("Unknown OffRampState: {}", s)),
        }
    }
}

// ============================================================================
// Off-Ramp Intent
// ============================================================================

/// Full off-ramp intent with all tracking data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffRampIntent {
    /// Unique intent ID
    pub id: String,
    /// User who initiated the off-ramp
    pub user_id: String,
    /// Crypto asset being sold
    pub crypto_asset: CryptoSymbol,
    /// Amount of crypto being sold
    pub crypto_amount: Decimal,
    /// Exchange rate used (VND per 1 unit of crypto)
    pub exchange_rate: Decimal,
    /// Locked rate ID (if rate was locked)
    pub locked_rate_id: Option<String>,
    /// Fee breakdown
    pub fees: FeeBreakdown,
    /// Net VND amount after fees
    pub net_vnd_amount: Decimal,
    /// Gross VND amount before fees
    pub gross_vnd_amount: Decimal,
    /// Destination bank account
    pub bank_account: BankAccount,
    /// Escrow deposit address for crypto
    pub deposit_address: Option<String>,
    /// On-chain transaction hash (when crypto is received)
    pub tx_hash: Option<String>,
    /// Bank transfer reference
    pub bank_reference: Option<String>,
    /// Current state
    pub state: OffRampState,
    /// State history
    pub state_history: Vec<StateTransition>,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Updated at
    pub updated_at: DateTime<Utc>,
    /// Quote expires at
    pub quote_expires_at: DateTime<Utc>,
}

/// A state transition record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub timestamp: DateTime<Utc>,
    pub reason: Option<String>,
}

/// Quote response returned to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffRampQuote {
    pub quote_id: String,
    pub crypto_asset: CryptoSymbol,
    pub crypto_amount: Decimal,
    pub exchange_rate: Decimal,
    pub fees: FeeBreakdown,
    pub net_vnd_amount: Decimal,
    pub gross_vnd_amount: Decimal,
    pub expires_at: DateTime<Utc>,
}

// ============================================================================
// Intent Store Trait (abstracts storage backend)
// ============================================================================

/// Abstracts intent storage so service works with both in-memory (tests) and SQL (production)
pub trait OffRampIntentStore: Send + Sync {
    fn save(&self, intent: OffRampIntent) -> Result<()>;
    fn get(&self, id: &str) -> Result<Option<OffRampIntent>>;
    fn update(&self, intent: &OffRampIntent) -> Result<()>;
}

/// In-memory store for unit tests (NOT for production)
#[cfg(test)]
pub struct InMemoryOffRampStore {
    intents: std::sync::Mutex<Vec<OffRampIntent>>,
}

#[cfg(test)]
impl InMemoryOffRampStore {
    pub fn new() -> Self {
        Self {
            intents: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[cfg(test)]
impl OffRampIntentStore for InMemoryOffRampStore {
    fn save(&self, intent: OffRampIntent) -> Result<()> {
        let mut intents = self.intents.lock().map_err(|_| {
            Error::Internal("Failed to acquire intents lock".to_string())
        })?;
        intents.push(intent);
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<OffRampIntent>> {
        let intents = self.intents.lock().map_err(|_| {
            Error::Internal("Failed to acquire intents lock".to_string())
        })?;
        Ok(intents.iter().find(|i| i.id == id).cloned())
    }

    fn update(&self, intent: &OffRampIntent) -> Result<()> {
        let mut intents = self.intents.lock().map_err(|_| {
            Error::Internal("Failed to acquire intents lock".to_string())
        })?;
        if let Some(existing) = intents.iter_mut().find(|i| i.id == intent.id) {
            *existing = intent.clone();
            Ok(())
        } else {
            Err(Error::NotFound(format!("Off-ramp intent not found: {}", intent.id)))
        }
    }
}

// ============================================================================
// Off-Ramp Service
// ============================================================================

pub struct OffRampService {
    exchange_rate_service: ExchangeRateService,
    fee_calculator: OffRampFeeCalculator,
    store: Arc<dyn OffRampIntentStore>,
}

impl OffRampService {
    #[cfg(test)]
    pub fn new(
        exchange_rate_service: ExchangeRateService,
        fee_calculator: OffRampFeeCalculator,
    ) -> Self {
        Self {
            exchange_rate_service,
            fee_calculator,
            store: Arc::new(InMemoryOffRampStore::new()),
        }
    }

    pub fn with_store(
        exchange_rate_service: ExchangeRateService,
        fee_calculator: OffRampFeeCalculator,
        store: Arc<dyn OffRampIntentStore>,
    ) -> Self {
        Self {
            exchange_rate_service,
            fee_calculator,
            store,
        }
    }

    /// Create a quote for an off-ramp transaction
    pub fn create_quote(
        &self,
        user_id: &str,
        crypto_asset: CryptoSymbol,
        crypto_amount: Decimal,
        bank_account: BankAccount,
    ) -> Result<OffRampQuote> {
        if crypto_amount <= Decimal::ZERO {
            return Err(Error::Validation("Amount must be positive".to_string()));
        }

        // Get current exchange rate
        let rate = self.exchange_rate_service.get_rate(crypto_asset, "VND")?;

        // Calculate gross VND amount (using sell price since user is selling crypto)
        let gross_vnd = crypto_amount * rate.sell_price;

        // Calculate fees
        let bank_type = if bank_account.bank_code == "VCB" || bank_account.bank_code == "TCB" {
            "domestic"
        } else {
            "domestic"
        };
        let fees = self.fee_calculator.calculate_fees(
            gross_vnd,
            crypto_asset,
            bank_type,
        );

        let net_vnd = fees.net_amount_vnd;
        let quote_expires_at = Utc::now() + Duration::minutes(5);

        let intent_id = format!("ofr_{}", Uuid::now_v7());

        // Create the intent in QUOTE_CREATED state
        let intent = OffRampIntent {
            id: intent_id.clone(),
            user_id: user_id.to_string(),
            crypto_asset,
            crypto_amount,
            exchange_rate: rate.sell_price,
            locked_rate_id: None,
            fees: fees.clone(),
            net_vnd_amount: net_vnd,
            gross_vnd_amount: gross_vnd,
            bank_account,
            deposit_address: None,
            tx_hash: None,
            bank_reference: None,
            state: OffRampState::QuoteCreated,
            state_history: vec![StateTransition {
                from: "NONE".to_string(),
                to: OffRampState::QuoteCreated.to_string(),
                timestamp: Utc::now(),
                reason: Some("Quote created".to_string()),
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            quote_expires_at,
        };

        self.store.save(intent)?;

        info!(quote_id = %intent_id, user_id = %user_id, crypto = %crypto_asset, "Off-ramp quote created");

        Ok(OffRampQuote {
            quote_id: intent_id,
            crypto_asset,
            crypto_amount,
            exchange_rate: rate.sell_price,
            fees,
            net_vnd_amount: net_vnd,
            gross_vnd_amount: gross_vnd,
            expires_at: quote_expires_at,
        })
    }

    /// Confirm a quote, locking the rate for 60 seconds
    pub fn confirm_quote(&self, quote_id: &str) -> Result<OffRampIntent> {
        let mut intent = self.store.get(quote_id)?.ok_or_else(|| {
            Error::NotFound(format!("Off-ramp intent not found: {}", quote_id))
        })?;

        if intent.state != OffRampState::QuoteCreated {
            return Err(Error::InvalidStateTransition {
                from: intent.state.to_string(),
                to: OffRampState::CryptoPending.to_string(),
            });
        }

        // Check if quote has expired
        if Utc::now() >= intent.quote_expires_at {
            self.transition_state_internal(&mut intent, OffRampState::Expired, Some("Quote expired"))?;
            self.store.update(&intent)?;
            return Err(Error::IntentExpired(format!(
                "Quote {} expired at {}",
                quote_id, intent.quote_expires_at
            )));
        }

        // Lock the rate for 60 seconds
        let locked = self.exchange_rate_service.lock_rate(
            intent.crypto_asset,
            "VND",
            60,
        )?;

        intent.locked_rate_id = Some(locked.id);
        intent.deposit_address = Some(format!(
            "0x{:040x}",
            uuid::Uuid::now_v7().as_u128() & u128::MAX
        ));

        self.transition_state_internal(&mut intent, OffRampState::CryptoPending, Some("Quote confirmed, awaiting crypto deposit"))?;
        self.store.update(&intent)?;

        info!(intent_id = %quote_id, "Off-ramp quote confirmed");
        Ok(intent)
    }

    /// Confirm that crypto has been received on-chain
    pub fn confirm_crypto_received(
        &self,
        intent_id: &str,
        tx_hash: &str,
    ) -> Result<OffRampIntent> {
        let mut intent = self.store.get(intent_id)?.ok_or_else(|| {
            Error::NotFound(format!("Off-ramp intent not found: {}", intent_id))
        })?;

        if intent.state != OffRampState::CryptoPending {
            return Err(Error::InvalidStateTransition {
                from: intent.state.to_string(),
                to: OffRampState::CryptoReceived.to_string(),
            });
        }

        intent.tx_hash = Some(tx_hash.to_string());
        self.transition_state_internal(
            &mut intent,
            OffRampState::CryptoReceived,
            Some("Crypto deposit confirmed on-chain"),
        )?;
        self.store.update(&intent)?;

        info!(intent_id = %intent_id, tx_hash = %tx_hash, "Crypto received for off-ramp");
        Ok(intent)
    }

    /// Initiate the bank transfer (VND payout)
    pub fn initiate_bank_transfer(&self, intent_id: &str) -> Result<OffRampIntent> {
        let mut intent = self.store.get(intent_id)?.ok_or_else(|| {
            Error::NotFound(format!("Off-ramp intent not found: {}", intent_id))
        })?;

        if intent.state != OffRampState::CryptoReceived {
            return Err(Error::InvalidStateTransition {
                from: intent.state.to_string(),
                to: OffRampState::Converting.to_string(),
            });
        }

        // Move through Converting to VndTransferring
        self.transition_state_internal(&mut intent, OffRampState::Converting, Some("Converting crypto to VND"))?;

        // Generate bank reference
        let bank_ref = format!("RAMP-{}", &Uuid::now_v7().to_string()[..8].to_uppercase());
        intent.bank_reference = Some(bank_ref);

        self.transition_state_internal(
            &mut intent,
            OffRampState::VndTransferring,
            Some("VND bank transfer initiated"),
        )?;
        self.store.update(&intent)?;

        info!(intent_id = %intent_id, "Bank transfer initiated for off-ramp");
        Ok(intent)
    }

    /// Mark the off-ramp as completed
    pub fn complete(&self, intent_id: &str) -> Result<OffRampIntent> {
        let mut intent = self.store.get(intent_id)?.ok_or_else(|| {
            Error::NotFound(format!("Off-ramp intent not found: {}", intent_id))
        })?;

        if intent.state != OffRampState::VndTransferring {
            return Err(Error::InvalidStateTransition {
                from: intent.state.to_string(),
                to: OffRampState::Completed.to_string(),
            });
        }

        self.transition_state_internal(&mut intent, OffRampState::Completed, Some("Off-ramp completed successfully"))?;
        self.store.update(&intent)?;

        info!(intent_id = %intent_id, "Off-ramp completed");
        Ok(intent)
    }

    /// Cancel an off-ramp (only from cancellable states)
    pub fn cancel(&self, intent_id: &str) -> Result<OffRampIntent> {
        let mut intent = self.store.get(intent_id)?.ok_or_else(|| {
            Error::NotFound(format!("Off-ramp intent not found: {}", intent_id))
        })?;

        if !intent.state.can_transition_to(OffRampState::Cancelled) {
            return Err(Error::InvalidStateTransition {
                from: intent.state.to_string(),
                to: OffRampState::Cancelled.to_string(),
            });
        }

        self.transition_state_internal(&mut intent, OffRampState::Cancelled, Some("Cancelled by user"))?;
        self.store.update(&intent)?;

        info!(intent_id = %intent_id, "Off-ramp cancelled");
        Ok(intent)
    }

    /// Get an off-ramp intent by ID
    pub fn get_offramp(&self, intent_id: &str) -> Result<OffRampIntent> {
        self.store.get(intent_id)?.ok_or_else(|| {
            Error::NotFound(format!("Off-ramp intent not found: {}", intent_id))
        })
    }

    /// Internal helper to transition state with history tracking
    fn transition_state_internal(
        &self,
        intent: &mut OffRampIntent,
        to: OffRampState,
        reason: Option<&str>,
    ) -> Result<()> {
        let from = intent.state;
        intent.state_history.push(StateTransition {
            from: from.to_string(),
            to: to.to_string(),
            timestamp: Utc::now(),
            reason: reason.map(|s| s.to_string()),
        });
        intent.state = to;
        intent.updated_at = Utc::now();
        Ok(())
    }
}
