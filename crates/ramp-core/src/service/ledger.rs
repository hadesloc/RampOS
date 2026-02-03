use ramp_common::{
    ledger::{AccountType, LedgerCurrency, LedgerTransaction},
    types::{IntentId, TenantId, UserId},
    Result,
};
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::repository::ledger::{BalanceRow, LedgerRepository};

pub struct LedgerService {
    repo: Arc<dyn LedgerRepository>,
}

impl LedgerService {
    pub fn new(repo: Arc<dyn LedgerRepository>) -> Self {
        Self { repo }
    }

    /// Record a transaction to the ledger
    pub async fn record_transaction(&self, tx: LedgerTransaction) -> Result<()> {
        // Verify transaction is balanced
        if !tx.is_balanced() {
            return Err(ramp_common::Error::LedgerImbalance {
                debit: tx
                    .entries
                    .iter()
                    .filter(|e| matches!(e.direction, ramp_common::ledger::EntryDirection::Debit))
                    .map(|e| e.amount)
                    .sum::<Decimal>()
                    .to_string(),
                credit: tx
                    .entries
                    .iter()
                    .filter(|e| matches!(e.direction, ramp_common::ledger::EntryDirection::Credit))
                    .map(|e| e.amount)
                    .sum::<Decimal>()
                    .to_string(),
            });
        }

        self.repo.record_transaction(tx).await
    }

    /// Get entries for an intent
    pub async fn get_entries_by_intent(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
    ) -> Result<Vec<crate::repository::ledger::LedgerEntryRow>> {
        self.repo.get_entries_by_intent(tenant_id, intent_id).await
    }

    /// Get balance for a specific account
    pub async fn get_balance(
        &self,
        tenant_id: &TenantId,
        user_id: Option<&UserId>,
        account_type: &AccountType,
        currency: &LedgerCurrency,
    ) -> Result<Decimal> {
        self.repo
            .get_balance(tenant_id, user_id, account_type, currency)
            .await
    }

    /// Get all balances for a user
    pub async fn get_user_balances(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Vec<BalanceRow>> {
        self.repo.get_user_balances(tenant_id, user_id).await
    }

    /// Get user's VND balance
    pub async fn get_user_vnd_balance(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Decimal> {
        self.repo
            .get_balance(
                tenant_id,
                Some(user_id),
                &AccountType::LiabilityUserVnd,
                &LedgerCurrency::VND,
            )
            .await
    }

    /// List ledger entries
    pub async fn list_entries(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::repository::ledger::LedgerEntryRow>> {
        self.repo.list_entries(tenant_id, limit, offset).await
    }

    /// List balances
    pub async fn list_balances(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BalanceRow>> {
        self.repo.list_all_balances(tenant_id, limit, offset).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockLedgerRepository;
    use chrono::Utc;
    use ramp_common::ledger::{
        AccountType, EntryDirection, LedgerCurrency, LedgerEntry, LedgerTransaction,
    };
    use rust_decimal_macros::dec;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_balance_calculations() {
        let repo = Arc::new(MockLedgerRepository::new());
        let service = LedgerService::new(repo.clone());

        let tenant_id = TenantId::new("tenant1");
        let user_id = UserId::new("user1");

        repo.set_balance(
            &tenant_id,
            Some(&user_id),
            &AccountType::LiabilityUserVnd,
            &LedgerCurrency::VND,
            dec!(1000000),
        );

        let balance = service
            .get_user_vnd_balance(&tenant_id, &user_id)
            .await
            .unwrap();
        assert_eq!(balance, dec!(1000000));
    }

    #[tokio::test]
    async fn test_record_transaction_balanced() {
        let repo = Arc::new(MockLedgerRepository::new());
        let service = LedgerService::new(repo.clone());

        let tx = LedgerTransaction {
            id: Uuid::new_v4().to_string(),
            tenant_id: TenantId::new("tenant1"),
            intent_id: IntentId::new_payin(),
            entries: vec![
                LedgerEntry {
                    id: ramp_common::ledger::LedgerEntryId::new(),
                    tenant_id: TenantId::new("tenant1"),
                    user_id: None,
                    intent_id: IntentId::new_payin(),
                    account_type: AccountType::AssetBank,
                    direction: EntryDirection::Debit,
                    amount: dec!(100),
                    currency: LedgerCurrency::VND,
                    description: "test".to_string(),
                    metadata: serde_json::json!({}),
                    created_at: Utc::now(),
                    balance_after: dec!(0),
                    sequence: 0,
                },
                LedgerEntry {
                    id: ramp_common::ledger::LedgerEntryId::new(),
                    tenant_id: TenantId::new("tenant1"),
                    user_id: Some(UserId::new("user1")),
                    intent_id: IntentId::new_payin(),
                    account_type: AccountType::LiabilityUserVnd,
                    direction: EntryDirection::Credit,
                    amount: dec!(100),
                    currency: LedgerCurrency::VND,
                    description: "test".to_string(),
                    metadata: serde_json::json!({}),
                    created_at: Utc::now(),
                    balance_after: dec!(0),
                    sequence: 0,
                },
            ],
            description: "test".to_string(),
            created_at: Utc::now(),
        };

        assert!(service.record_transaction(tx).await.is_ok());
    }

    #[tokio::test]
    async fn test_record_transaction_unbalanced() {
        let repo = Arc::new(MockLedgerRepository::new());
        let service = LedgerService::new(repo.clone());

        let tx = LedgerTransaction {
            id: Uuid::new_v4().to_string(),
            tenant_id: TenantId::new("tenant1"),
            intent_id: IntentId::new_payin(),
            entries: vec![
                LedgerEntry {
                    id: ramp_common::ledger::LedgerEntryId::new(),
                    tenant_id: TenantId::new("tenant1"),
                    user_id: None,
                    intent_id: IntentId::new_payin(),
                    account_type: AccountType::AssetBank,
                    direction: EntryDirection::Debit,
                    amount: dec!(100),
                    currency: LedgerCurrency::VND,
                    description: "test".to_string(),
                    metadata: serde_json::json!({}),
                    created_at: Utc::now(),
                    balance_after: dec!(0),
                    sequence: 0,
                },
                LedgerEntry {
                    id: ramp_common::ledger::LedgerEntryId::new(),
                    tenant_id: TenantId::new("tenant1"),
                    user_id: Some(UserId::new("user1")),
                    intent_id: IntentId::new_payin(),
                    account_type: AccountType::LiabilityUserVnd,
                    direction: EntryDirection::Credit,
                    amount: dec!(90), // Unbalanced
                    currency: LedgerCurrency::VND,
                    description: "test".to_string(),
                    metadata: serde_json::json!({}),
                    created_at: Utc::now(),
                    balance_after: dec!(0),
                    sequence: 0,
                },
            ],
            description: "test".to_string(),
            created_at: Utc::now(),
        };

        assert!(service.record_transaction(tx).await.is_err());
    }
}
