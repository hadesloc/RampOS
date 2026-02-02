use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{IntentId, TenantId, UserId};

/// Ledger Entry ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LedgerEntryId(pub String);

impl LedgerEntryId {
    pub fn new() -> Self {
        Self(format!("le_{}", Uuid::now_v7()))
    }
}

impl Default for LedgerEntryId {
    fn default() -> Self {
        Self::new()
    }
}

/// Account types in the ledger
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
    // Asset accounts (what the system owns/controls)
    AssetBank,       // Cash in bank accounts
    AssetCrypto,     // Crypto held in custody
    AssetReceivable, // Money owed to the system

    // Liability accounts (what the system owes)
    LiabilityUserVnd,    // VND owed to users
    LiabilityUserCrypto, // Crypto owed to users
    LiabilityPayable,    // Money owed to external parties

    // Clearing accounts (temporary holding)
    ClearingBankPending,   // Bank transfers pending confirmation
    ClearingCryptoPending, // Crypto transfers pending confirmation
    ClearingTrade,         // Trade settlement clearing

    // Revenue accounts
    RevenueFee,    // Fees earned
    RevenueSpread, // Trading spread revenue

    // Expense accounts
    ExpenseGas,      // Gas fees paid
    ExpenseProvider, // Payment to rails providers
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountType::AssetBank => write!(f, "Asset:Bank"),
            AccountType::AssetCrypto => write!(f, "Asset:Crypto"),
            AccountType::AssetReceivable => write!(f, "Asset:Receivable"),
            AccountType::LiabilityUserVnd => write!(f, "Liability:UserVND"),
            AccountType::LiabilityUserCrypto => write!(f, "Liability:UserCrypto"),
            AccountType::LiabilityPayable => write!(f, "Liability:Payable"),
            AccountType::ClearingBankPending => write!(f, "Clearing:BankPending"),
            AccountType::ClearingCryptoPending => write!(f, "Clearing:CryptoPending"),
            AccountType::ClearingTrade => write!(f, "Clearing:Trade"),
            AccountType::RevenueFee => write!(f, "Revenue:Fee"),
            AccountType::RevenueSpread => write!(f, "Revenue:Spread"),
            AccountType::ExpenseGas => write!(f, "Expense:Gas"),
            AccountType::ExpenseProvider => write!(f, "Expense:Provider"),
        }
    }
}

/// Currency for ledger entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LedgerCurrency {
    VND,
    BTC,
    ETH,
    USDT,
    USDC,
    Other,
}

impl std::fmt::Display for LedgerCurrency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerCurrency::VND => write!(f, "VND"),
            LedgerCurrency::BTC => write!(f, "BTC"),
            LedgerCurrency::ETH => write!(f, "ETH"),
            LedgerCurrency::USDT => write!(f, "USDT"),
            LedgerCurrency::USDC => write!(f, "USDC"),
            LedgerCurrency::Other => write!(f, "OTHER"),
        }
    }
}

/// Direction of ledger entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryDirection {
    Debit,
    Credit,
}

impl std::fmt::Display for EntryDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryDirection::Debit => write!(f, "DEBIT"),
            EntryDirection::Credit => write!(f, "CREDIT"),
        }
    }
}

/// A single ledger entry (one side of double-entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: LedgerEntryId,
    pub tenant_id: TenantId,
    pub user_id: Option<UserId>,
    pub intent_id: IntentId,
    pub account_type: AccountType,
    pub direction: EntryDirection,
    pub amount: Decimal,
    pub currency: LedgerCurrency,
    pub balance_after: Decimal,
    pub description: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub sequence: i64,
}

/// A complete transaction with balanced entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerTransaction {
    pub id: String,
    pub tenant_id: TenantId,
    pub intent_id: IntentId,
    pub entries: Vec<LedgerEntry>,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

impl LedgerTransaction {
    /// Verify that debits equal credits (double-entry balance)
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

    /// Get total amount (from debit side)
    pub fn total_amount(&self) -> Decimal {
        self.entries
            .iter()
            .filter(|e| e.direction == EntryDirection::Debit)
            .map(|e| e.amount)
            .sum()
    }
}

/// Builder for creating balanced ledger transactions
#[derive(Debug)]
pub struct LedgerTransactionBuilder {
    tenant_id: TenantId,
    intent_id: IntentId,
    description: String,
    entries: Vec<PendingEntry>,
}

#[derive(Debug)]
struct PendingEntry {
    user_id: Option<UserId>,
    account_type: AccountType,
    direction: EntryDirection,
    amount: Decimal,
    currency: LedgerCurrency,
    description: String,
    metadata: serde_json::Value,
}

impl LedgerTransactionBuilder {
    pub fn new(tenant_id: TenantId, intent_id: IntentId, description: impl Into<String>) -> Self {
        Self {
            tenant_id,
            intent_id,
            description: description.into(),
            entries: Vec::new(),
        }
    }

    pub fn debit(
        mut self,
        account_type: AccountType,
        amount: Decimal,
        currency: LedgerCurrency,
    ) -> Self {
        self.entries.push(PendingEntry {
            user_id: None,
            account_type,
            direction: EntryDirection::Debit,
            amount,
            currency,
            description: String::new(),
            metadata: serde_json::Value::Null,
        });
        self
    }

    pub fn credit(
        mut self,
        account_type: AccountType,
        amount: Decimal,
        currency: LedgerCurrency,
    ) -> Self {
        self.entries.push(PendingEntry {
            user_id: None,
            account_type,
            direction: EntryDirection::Credit,
            amount,
            currency,
            description: String::new(),
            metadata: serde_json::Value::Null,
        });
        self
    }

    pub fn debit_user(
        mut self,
        user_id: UserId,
        account_type: AccountType,
        amount: Decimal,
        currency: LedgerCurrency,
    ) -> Self {
        self.entries.push(PendingEntry {
            user_id: Some(user_id),
            account_type,
            direction: EntryDirection::Debit,
            amount,
            currency,
            description: String::new(),
            metadata: serde_json::Value::Null,
        });
        self
    }

    pub fn credit_user(
        mut self,
        user_id: UserId,
        account_type: AccountType,
        amount: Decimal,
        currency: LedgerCurrency,
    ) -> Self {
        self.entries.push(PendingEntry {
            user_id: Some(user_id),
            account_type,
            direction: EntryDirection::Credit,
            amount,
            currency,
            description: String::new(),
            metadata: serde_json::Value::Null,
        });
        self
    }

    /// Build the transaction, returns error if not balanced
    pub fn build(self) -> Result<LedgerTransaction, LedgerError> {
        // Check balance
        let mut debit_total = Decimal::ZERO;
        let mut credit_total = Decimal::ZERO;

        for entry in &self.entries {
            match entry.direction {
                EntryDirection::Debit => debit_total += entry.amount,
                EntryDirection::Credit => credit_total += entry.amount,
            }
        }

        if debit_total != credit_total {
            return Err(LedgerError::Imbalanced {
                debit: debit_total,
                credit: credit_total,
            });
        }

        let now = Utc::now();
        let tx_id = format!("ltx_{}", Uuid::now_v7());

        let entries = self
            .entries
            .into_iter()
            .map(|e| LedgerEntry {
                id: LedgerEntryId::new(),
                tenant_id: self.tenant_id.clone(),
                user_id: e.user_id,
                intent_id: self.intent_id.clone(),
                account_type: e.account_type,
                direction: e.direction,
                amount: e.amount,
                currency: e.currency,
                balance_after: Decimal::ZERO, // Will be set by repository
                description: e.description,
                metadata: e.metadata,
                created_at: now,
                sequence: 0, // Will be set by repository
            })
            .collect();

        Ok(LedgerTransaction {
            id: tx_id,
            tenant_id: self.tenant_id,
            intent_id: self.intent_id,
            entries,
            description: self.description,
            created_at: now,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("Transaction not balanced: debit={debit}, credit={credit}")]
    Imbalanced { debit: Decimal, credit: Decimal },

    #[error("Insufficient balance in account")]
    InsufficientBalance,

    #[error("Account not found: {0}")]
    AccountNotFound(String),
}

/// Convert string to LedgerCurrency
impl LedgerCurrency {
    pub fn from_symbol(symbol: &str) -> Self {
        match symbol.to_uppercase().as_str() {
            "VND" => LedgerCurrency::VND,
            "BTC" => LedgerCurrency::BTC,
            "ETH" => LedgerCurrency::ETH,
            "USDT" => LedgerCurrency::USDT,
            "USDC" => LedgerCurrency::USDC,
            _ => LedgerCurrency::Other,
        }
    }
}

/// Common ledger transaction patterns for RampOS
pub mod patterns {
    use super::*;

    /// Create ledger entries for VND pay-in (bank confirmed)
    pub fn payin_vnd_confirmed(
        tenant_id: TenantId,
        user_id: UserId,
        intent_id: IntentId,
        amount: Decimal,
    ) -> Result<LedgerTransaction, LedgerError> {
        LedgerTransactionBuilder::new(tenant_id, intent_id, "VND Pay-in confirmed")
            .debit(AccountType::AssetBank, amount, LedgerCurrency::VND)
            .credit_user(
                user_id,
                AccountType::LiabilityUserVnd,
                amount,
                LedgerCurrency::VND,
            )
            .build()
    }

    /// Create ledger entries for VND pay-out (initiated)
    pub fn payout_vnd_initiated(
        tenant_id: TenantId,
        user_id: UserId,
        intent_id: IntentId,
        amount: Decimal,
    ) -> Result<LedgerTransaction, LedgerError> {
        LedgerTransactionBuilder::new(tenant_id, intent_id, "VND Pay-out initiated")
            .debit_user(
                user_id,
                AccountType::LiabilityUserVnd,
                amount,
                LedgerCurrency::VND,
            )
            .credit(
                AccountType::ClearingBankPending,
                amount,
                LedgerCurrency::VND,
            )
            .build()
    }

    /// Create ledger entries for VND pay-out (confirmed)
    pub fn payout_vnd_confirmed(
        tenant_id: TenantId,
        intent_id: IntentId,
        amount: Decimal,
    ) -> Result<LedgerTransaction, LedgerError> {
        LedgerTransactionBuilder::new(tenant_id, intent_id, "VND Pay-out confirmed")
            .debit(
                AccountType::ClearingBankPending,
                amount,
                LedgerCurrency::VND,
            )
            .credit(AccountType::AssetBank, amount, LedgerCurrency::VND)
            .build()
    }

    /// Create ledger entries for VND pay-out reversal (funds returned to user)
    ///
    /// This reverses the payout_vnd_initiated transaction:
    /// - Debits the ClearingBankPending account (clearing the held funds)
    /// - Credits back the user's VND balance (restoring their funds)
    ///
    /// Used when:
    /// - Bank rejects the payout
    /// - Payout times out
    /// - Payout is cancelled after funds were held
    pub fn payout_vnd_reversed(
        tenant_id: TenantId,
        user_id: UserId,
        intent_id: IntentId,
        amount: Decimal,
        reason: &str,
    ) -> Result<LedgerTransaction, LedgerError> {
        LedgerTransactionBuilder::new(
            tenant_id,
            intent_id,
            format!("VND Pay-out reversed: {}", reason),
        )
        .debit(
            AccountType::ClearingBankPending,
            amount,
            LedgerCurrency::VND,
        )
        .credit_user(
            user_id,
            AccountType::LiabilityUserVnd,
            amount,
            LedgerCurrency::VND,
        )
        .build()
    }

    /// Create ledger entries for partial VND pay-out reversal
    ///
    /// Used when a payout partially succeeds and we need to return the remainder
    /// - original_amount: The total amount initially held
    /// - settled_amount: The amount that was actually settled by the bank
    /// - Returns the difference (original - settled) to the user
    pub fn payout_vnd_partial_reversed(
        tenant_id: TenantId,
        user_id: UserId,
        intent_id: IntentId,
        original_amount: Decimal,
        settled_amount: Decimal,
        reason: &str,
    ) -> Result<LedgerTransaction, LedgerError> {
        let reversal_amount = original_amount - settled_amount;

        if reversal_amount <= Decimal::ZERO {
            return Err(LedgerError::Imbalanced {
                debit: Decimal::ZERO,
                credit: Decimal::ZERO,
            });
        }

        LedgerTransactionBuilder::new(
            tenant_id,
            intent_id,
            format!(
                "VND Pay-out partial reversal: {} (settled: {}, returned: {})",
                reason, settled_amount, reversal_amount
            ),
        )
        .debit(
            AccountType::ClearingBankPending,
            reversal_amount,
            LedgerCurrency::VND,
        )
        .credit_user(
            user_id,
            AccountType::LiabilityUserVnd,
            reversal_amount,
            LedgerCurrency::VND,
        )
        .build()
    }

    /// Create ledger entries for crypto/VND trade
    pub fn trade_crypto_vnd(
        tenant_id: TenantId,
        user_id: UserId,
        intent_id: IntentId,
        vnd_amount: Decimal,
        crypto_amount: Decimal,
        crypto_currency: LedgerCurrency,
        is_buy: bool, // true = user buys crypto with VND
    ) -> Result<LedgerTransaction, LedgerError> {
        let mut builder = LedgerTransactionBuilder::new(
            tenant_id,
            intent_id.clone(),
            if is_buy {
                "Buy crypto with VND"
            } else {
                "Sell crypto for VND"
            },
        );

        if is_buy {
            // User pays VND, receives crypto
            builder = builder
                .debit_user(
                    user_id.clone(),
                    AccountType::LiabilityUserVnd,
                    vnd_amount,
                    LedgerCurrency::VND,
                )
                .credit(AccountType::AssetBank, vnd_amount, LedgerCurrency::VND)
                .debit(AccountType::AssetCrypto, crypto_amount, crypto_currency)
                .credit_user(
                    user_id,
                    AccountType::LiabilityUserCrypto,
                    crypto_amount,
                    crypto_currency,
                );
        } else {
            // User pays crypto, receives VND
            builder = builder
                .debit_user(
                    user_id.clone(),
                    AccountType::LiabilityUserCrypto,
                    crypto_amount,
                    crypto_currency,
                )
                .credit(AccountType::AssetCrypto, crypto_amount, crypto_currency)
                .debit(AccountType::AssetBank, vnd_amount, LedgerCurrency::VND)
                .credit_user(
                    user_id,
                    AccountType::LiabilityUserVnd,
                    vnd_amount,
                    LedgerCurrency::VND,
                );
        }

        builder.build()
    }

    /// Create ledger entries for crypto deposit confirmed (on-chain)
    /// Credits crypto to user's account from on-chain deposit
    pub fn deposit_crypto_confirmed(
        tenant_id: TenantId,
        user_id: UserId,
        intent_id: IntentId,
        amount: Decimal,
        crypto_currency: LedgerCurrency,
    ) -> Result<LedgerTransaction, LedgerError> {
        LedgerTransactionBuilder::new(tenant_id, intent_id, "Crypto deposit confirmed")
            .debit(AccountType::AssetCrypto, amount, crypto_currency)
            .credit_user(
                user_id,
                AccountType::LiabilityUserCrypto,
                amount,
                crypto_currency,
            )
            .build()
    }

    /// Create ledger entries for crypto withdraw initiated
    /// Holds crypto from user's account pending on-chain transfer
    pub fn withdraw_crypto_initiated(
        tenant_id: TenantId,
        user_id: UserId,
        intent_id: IntentId,
        amount: Decimal,
        crypto_currency: LedgerCurrency,
    ) -> Result<LedgerTransaction, LedgerError> {
        LedgerTransactionBuilder::new(tenant_id, intent_id, "Crypto withdraw initiated")
            .debit_user(
                user_id,
                AccountType::LiabilityUserCrypto,
                amount,
                crypto_currency,
            )
            .credit(
                AccountType::ClearingCryptoPending,
                amount,
                crypto_currency,
            )
            .build()
    }

    /// Create ledger entries for crypto withdraw confirmed (on-chain tx mined)
    /// Finalizes the withdraw by clearing the pending crypto
    pub fn withdraw_crypto_confirmed(
        tenant_id: TenantId,
        intent_id: IntentId,
        amount: Decimal,
        crypto_currency: LedgerCurrency,
    ) -> Result<LedgerTransaction, LedgerError> {
        LedgerTransactionBuilder::new(tenant_id, intent_id, "Crypto withdraw confirmed")
            .debit(
                AccountType::ClearingCryptoPending,
                amount,
                crypto_currency,
            )
            .credit(AccountType::AssetCrypto, amount, crypto_currency)
            .build()
    }

    /// Create ledger entries for crypto withdraw failed/reversed
    /// Returns crypto to user's account when withdraw fails
    pub fn withdraw_crypto_reversed(
        tenant_id: TenantId,
        user_id: UserId,
        intent_id: IntentId,
        amount: Decimal,
        crypto_currency: LedgerCurrency,
    ) -> Result<LedgerTransaction, LedgerError> {
        LedgerTransactionBuilder::new(tenant_id, intent_id, "Crypto withdraw reversed")
            .debit(
                AccountType::ClearingCryptoPending,
                amount,
                crypto_currency,
            )
            .credit_user(
                user_id,
                AccountType::LiabilityUserCrypto,
                amount,
                crypto_currency,
            )
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balanced_transaction() {
        let tx = LedgerTransactionBuilder::new(
            TenantId::new("test"),
            IntentId::new_payin(),
            "Test transaction",
        )
        .debit(
            AccountType::AssetBank,
            Decimal::from(1000000),
            LedgerCurrency::VND,
        )
        .credit(
            AccountType::LiabilityUserVnd,
            Decimal::from(1000000),
            LedgerCurrency::VND,
        )
        .build();

        assert!(tx.is_ok());
        assert!(tx.unwrap().is_balanced());
    }

    #[test]
    fn test_imbalanced_transaction() {
        let tx = LedgerTransactionBuilder::new(
            TenantId::new("test"),
            IntentId::new_payin(),
            "Test transaction",
        )
        .debit(
            AccountType::AssetBank,
            Decimal::from(1000000),
            LedgerCurrency::VND,
        )
        .credit(
            AccountType::LiabilityUserVnd,
            Decimal::from(999999),
            LedgerCurrency::VND,
        )
        .build();

        assert!(tx.is_err());
    }

    #[test]
    fn test_payout_vnd_reversed_pattern() {
        let tx = patterns::payout_vnd_reversed(
            TenantId::new("test"),
            UserId::new("user1"),
            IntentId::new_payout(),
            Decimal::from(1000000),
            "Bank rejected",
        );

        assert!(tx.is_ok());
        let tx = tx.unwrap();
        assert!(tx.is_balanced());
        assert_eq!(tx.entries.len(), 2);
        assert!(tx.description.contains("reversed"));
    }

    #[test]
    fn test_payout_vnd_partial_reversed_pattern() {
        let tx = patterns::payout_vnd_partial_reversed(
            TenantId::new("test"),
            UserId::new("user1"),
            IntentId::new_payout(),
            Decimal::from(1000000),  // original
            Decimal::from(700000),   // settled
            "Partial settlement",
        );

        assert!(tx.is_ok());
        let tx = tx.unwrap();
        assert!(tx.is_balanced());
        // Should reverse 300000 (1000000 - 700000)
        assert_eq!(tx.total_amount(), Decimal::from(300000));
    }

    #[test]
    fn test_payout_vnd_partial_reversed_fails_when_invalid() {
        // Settled amount > original should fail
        let tx = patterns::payout_vnd_partial_reversed(
            TenantId::new("test"),
            UserId::new("user1"),
            IntentId::new_payout(),
            Decimal::from(500000),   // original
            Decimal::from(600000),   // settled (more than original)
            "Invalid",
        );

        assert!(tx.is_err());
    }

    #[test]
    fn test_payout_lifecycle_ledger_entries() {
        // Test the complete payout lifecycle:
        // 1. Initiated (hold funds)
        // 2. Either confirmed (complete) or reversed (refund)

        let tenant_id = TenantId::new("test");
        let user_id = UserId::new("user1");
        let intent_id = IntentId::new_payout();
        let amount = Decimal::from(500000);

        // Step 1: Initiate payout (hold funds)
        let initiated = patterns::payout_vnd_initiated(
            tenant_id.clone(),
            user_id.clone(),
            intent_id.clone(),
            amount,
        ).unwrap();

        assert!(initiated.is_balanced());
        // User balance debited, ClearingBankPending credited
        let user_entry = initiated.entries.iter()
            .find(|e| e.user_id.is_some())
            .unwrap();
        assert_eq!(user_entry.direction, EntryDirection::Debit);
        assert_eq!(user_entry.account_type, AccountType::LiabilityUserVnd);

        // Step 2a: Confirm payout (success path)
        let confirmed = patterns::payout_vnd_confirmed(
            tenant_id.clone(),
            intent_id.clone(),
            amount,
        ).unwrap();

        assert!(confirmed.is_balanced());
        // ClearingBankPending debited, AssetBank credited
        let clearing_entry = confirmed.entries.iter()
            .find(|e| e.account_type == AccountType::ClearingBankPending)
            .unwrap();
        assert_eq!(clearing_entry.direction, EntryDirection::Debit);

        // Step 2b: Reverse payout (failure path)
        let reversed = patterns::payout_vnd_reversed(
            tenant_id.clone(),
            user_id.clone(),
            intent_id.clone(),
            amount,
            "Bank rejected",
        ).unwrap();

        assert!(reversed.is_balanced());
        // ClearingBankPending debited, User balance credited (refund)
        let refund_entry = reversed.entries.iter()
            .find(|e| e.user_id.is_some())
            .unwrap();
        assert_eq!(refund_entry.direction, EntryDirection::Credit);
        assert_eq!(refund_entry.account_type, AccountType::LiabilityUserVnd);
    }
}
