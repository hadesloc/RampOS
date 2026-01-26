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
}
