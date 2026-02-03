use crate::repository::{
    intent::{IntentRepository, IntentRow},
    ledger::{BalanceRow, LedgerEntryRow, LedgerRepository},
    tenant::{TenantRepository, TenantRow},
    user::{UserRepository, UserRow},
};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Utc};
use ramp_common::{ledger::*, types::*, Result};
use rust_decimal::Decimal;
use std::sync::{Arc, Mutex};
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct MockIntentRepository {
    pub intents: Arc<Mutex<Vec<IntentRow>>>,
}

impl MockIntentRepository {
    pub fn new() -> Self {
        Self {
            intents: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for MockIntentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IntentRepository for MockIntentRepository {
    async fn create(&self, intent: &IntentRow) -> Result<()> {
        self.intents.lock().unwrap().push(intent.clone());
        Ok(())
    }

    async fn get_by_id(&self, _tenant_id: &TenantId, id: &IntentId) -> Result<Option<IntentRow>> {
        let intents = self.intents.lock().unwrap();
        Ok(intents.iter().find(|i| i.id == id.0).cloned())
    }

    async fn get_by_idempotency_key(
        &self,
        _tenant_id: &TenantId,
        key: &IdempotencyKey,
    ) -> Result<Option<IntentRow>> {
        let intents = self.intents.lock().unwrap();
        Ok(intents
            .iter()
            .find(|i| i.idempotency_key == Some(key.0.clone()))
            .cloned())
    }

    async fn get_by_reference_code(
        &self,
        _tenant_id: &TenantId,
        code: &ReferenceCode,
    ) -> Result<Option<IntentRow>> {
        let intents = self.intents.lock().unwrap();
        Ok(intents
            .iter()
            .find(|i| i.reference_code == Some(code.0.clone()))
            .cloned())
    }

    async fn update_state(
        &self,
        _tenant_id: &TenantId,
        id: &IntentId,
        new_state: &str,
    ) -> Result<()> {
        let mut intents = self.intents.lock().unwrap();
        if let Some(intent) = intents.iter_mut().find(|i| i.id == id.0) {
            intent.state = new_state.to_string();
        }
        Ok(())
    }

    async fn update_bank_confirmed(
        &self,
        _tenant_id: &TenantId,
        id: &IntentId,
        bank_tx_id: &str,
        actual_amount: Decimal,
    ) -> Result<()> {
        let mut intents = self.intents.lock().unwrap();
        if let Some(intent) = intents.iter_mut().find(|i| i.id == id.0) {
            intent.bank_tx_id = Some(bank_tx_id.to_string());
            intent.actual_amount = Some(actual_amount);
        }
        Ok(())
    }

    async fn list_by_user(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        limit: i64,
        _offset: i64,
    ) -> Result<Vec<IntentRow>> {
        let intents = self.intents.lock().unwrap();
        let filtered: Vec<_> = intents
            .iter()
            .filter(|i| i.tenant_id == tenant_id.0 && i.user_id == user_id.0)
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn list_expired(&self, limit: i64) -> Result<Vec<IntentRow>> {
        let intents = self.intents.lock().unwrap();
        let now = Utc::now();
        let expired: Vec<_> = intents
            .iter()
            .filter(|i| {
                if let Some(expires_at) = i.expires_at {
                    expires_at < now
                        && ![
                            "COMPLETED",
                            "EXPIRED",
                            "CANCELLED",
                            "TIMEOUT",
                            "REJECTED_BY_POLICY",
                            "BANK_REJECTED",
                            "SUSPECTED_FRAUD",
                            "REVERSED",
                        ]
                        .contains(&i.state.as_str())
                } else {
                    false
                }
            })
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(expired)
    }

    async fn get_daily_payin_amount(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Decimal> {
        let intents = self.intents.lock().unwrap();
        let now = Utc::now();
        let start_of_day = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        let amount: Decimal = intents
            .iter()
            .filter(|i| {
                i.tenant_id == tenant_id.0
                    && i.user_id == user_id.0
                    && i.intent_type == "PAYIN_VND"
                    && [
                        "COMPLETED",
                        "INSTRUCTION_ISSUED",
                        "FUNDS_PENDING",
                        "FUNDS_CONFIRMED",
                        "VND_CREDITED",
                    ]
                    .contains(&i.state.as_str())
                    && i.created_at >= start_of_day
            })
            .map(|i| i.amount)
            .sum();

        Ok(amount)
    }

    async fn get_daily_payout_amount(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Decimal> {
        let intents = self.intents.lock().unwrap();
        let now = Utc::now();
        let start_of_day = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        let amount: Decimal = intents
            .iter()
            .filter(|i| {
                i.tenant_id == tenant_id.0
                    && i.user_id == user_id.0
                    && i.intent_type == "PAYOUT_VND"
                    && [
                        "COMPLETED",
                        "PAYOUT_CREATED",
                        "POLICY_APPROVED",
                        "PAYOUT_SUBMITTED",
                        "PAYOUT_CONFIRMED",
                    ]
                    .contains(&i.state.as_str())
                    && i.created_at >= start_of_day
            })
            .map(|i| i.amount)
            .sum();

        Ok(amount)
    }

    async fn get_daily_withdraw_amount(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Decimal> {
        let intents = self.intents.lock().unwrap();
        let now = Utc::now();
        let start_of_day = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        let amount: Decimal = intents
            .iter()
            .filter(|i| {
                i.tenant_id == tenant_id.0
                    && i.user_id == user_id.0
                    && i.intent_type == "WITHDRAW_ONCHAIN"
                    && [
                        "CREATED",
                        "POLICY_APPROVED",
                        "KYT_CHECKED",
                        "SIGNED",
                        "BROADCASTED",
                        "CONFIRMING",
                        "CONFIRMED",
                        "COMPLETED",
                    ]
                    .contains(&i.state.as_str())
                    && i.created_at >= start_of_day
            })
            .map(|i| i.amount)
            .sum();

        Ok(amount)
    }

    async fn get_monthly_withdraw_amount(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Decimal> {
        let intents = self.intents.lock().unwrap();
        let now = Utc::now();
        let start_of_month = now
            .date_naive()
            .with_day(1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        let amount: Decimal = intents
            .iter()
            .filter(|i| {
                i.tenant_id == tenant_id.0
                    && i.user_id == user_id.0
                    && i.intent_type == "WITHDRAW_ONCHAIN"
                    && [
                        "CREATED",
                        "POLICY_APPROVED",
                        "KYT_CHECKED",
                        "SIGNED",
                        "BROADCASTED",
                        "CONFIRMING",
                        "CONFIRMED",
                        "COMPLETED",
                    ]
                    .contains(&i.state.as_str())
                    && i.created_at >= start_of_month
            })
            .map(|i| i.amount)
            .sum();

        Ok(amount)
    }

    async fn get_hourly_withdraw_count(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<u32> {
        let intents = self.intents.lock().unwrap();
        let now = Utc::now();
        let hour_ago = now - chrono::Duration::hours(1);

        let count = intents
            .iter()
            .filter(|i| {
                i.tenant_id == tenant_id.0
                    && i.user_id == user_id.0
                    && i.intent_type == "WITHDRAW_ONCHAIN"
                    && ![
                        "CANCELLED",
                        "REJECTED_BY_POLICY",
                        "REJECTED_INSUFFICIENT_BALANCE",
                    ]
                    .contains(&i.state.as_str())
                    && i.created_at >= hour_ago
            })
            .count();

        Ok(count as u32)
    }

    async fn get_daily_withdraw_count(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<u32> {
        let intents = self.intents.lock().unwrap();
        let now = Utc::now();
        let start_of_day = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        let count = intents
            .iter()
            .filter(|i| {
                i.tenant_id == tenant_id.0
                    && i.user_id == user_id.0
                    && i.intent_type == "WITHDRAW_ONCHAIN"
                    && ![
                        "CANCELLED",
                        "REJECTED_BY_POLICY",
                        "REJECTED_INSUFFICIENT_BALANCE",
                    ]
                    .contains(&i.state.as_str())
                    && i.created_at >= start_of_day
            })
            .count();

        Ok(count as u32)
    }

    async fn get_last_withdraw_time(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Option<DateTime<Utc>>> {
        let intents = self.intents.lock().unwrap();

        let last = intents
            .iter()
            .filter(|i| {
                i.tenant_id == tenant_id.0
                    && i.user_id == user_id.0
                    && i.intent_type == "WITHDRAW_ONCHAIN"
                    && ![
                        "CANCELLED",
                        "REJECTED_BY_POLICY",
                        "REJECTED_INSUFFICIENT_BALANCE",
                    ]
                    .contains(&i.state.as_str())
            })
            .max_by_key(|i| i.created_at)
            .map(|i| i.created_at);

        Ok(last)
    }
}

#[derive(Clone)]
pub struct MockLedgerRepository {
    pub transactions: Arc<Mutex<Vec<LedgerTransaction>>>,
    pub balances: Arc<Mutex<Vec<BalanceRow>>>,
}

impl Default for MockLedgerRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockLedgerRepository {
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(Mutex::new(Vec::new())),
            balances: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn set_balance(
        &self,
        _tenant_id: &TenantId,
        _user_id: Option<&UserId>,
        account_type: &AccountType,
        currency: &LedgerCurrency,
        amount: Decimal,
    ) {
        let mut balances = self.balances.lock().unwrap();
        balances.push(BalanceRow {
            account_type: account_type.to_string(),
            currency: currency.to_string(),
            balance: amount,
        });
    }
}

#[async_trait]
impl LedgerRepository for MockLedgerRepository {
    async fn record_transaction(&self, tx: LedgerTransaction) -> Result<()> {
        self.transactions.lock().unwrap().push(tx);
        Ok(())
    }

    async fn get_entries_by_intent(
        &self,
        tenant_id: &TenantId,
        intent_id: &IntentId,
    ) -> Result<Vec<LedgerEntryRow>> {
        // Return entries from recorded transactions matching the intent
        let txs = self.transactions.lock().unwrap();
        let entries: Vec<LedgerEntryRow> = txs
            .iter()
            .filter(|tx| tx.intent_id == *intent_id && tx.tenant_id == *tenant_id)
            .flat_map(|tx| {
                tx.entries.iter().map(|e| LedgerEntryRow {
                    id: uuid::Uuid::new_v4().to_string(),
                    tenant_id: tx.tenant_id.0.clone(),
                    user_id: None,
                    intent_id: tx.intent_id.0.clone(),
                    transaction_id: tx.id.clone(),
                    account_type: e.account_type.to_string(),
                    direction: e.direction.to_string(),
                    amount: e.amount,
                    currency: e.currency.to_string(),
                    balance_after: Decimal::ZERO,
                    sequence: 0,
                    description: Some(e.description.clone()),
                    metadata: serde_json::json!({}),
                    created_at: Utc::now(),
                })
            })
            .collect();
        Ok(entries)
    }

    async fn get_balance(
        &self,
        _tenant_id: &TenantId,
        _user_id: Option<&UserId>,
        account_type: &AccountType,
        currency: &LedgerCurrency,
    ) -> Result<Decimal> {
        let balances = self.balances.lock().unwrap();
        for b in balances.iter() {
            if b.account_type == account_type.to_string() && b.currency == currency.to_string() {
                return Ok(b.balance);
            }
        }
        Ok(Decimal::ZERO)
    }

    async fn get_user_balances(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
    ) -> Result<Vec<BalanceRow>> {
        Ok(self.balances.lock().unwrap().clone())
    }

    async fn check_balance_and_record_transaction(
        &self,
        required_balance: Decimal,
        _user_id: &UserId,
        account_type: &AccountType,
        currency: &LedgerCurrency,
        tx: LedgerTransaction,
    ) -> Result<Decimal> {
        // Check balance first
        let balances = self.balances.lock().unwrap();
        let current_balance = balances
            .iter()
            .find(|b| {
                b.account_type == account_type.to_string() && b.currency == currency.to_string()
            })
            .map(|b| b.balance)
            .unwrap_or(Decimal::ZERO);
        drop(balances);

        if current_balance < required_balance {
            return Err(ramp_common::Error::InsufficientBalance {
                required: required_balance.to_string(),
                available: current_balance.to_string(),
            });
        }

        // Record the transaction
        self.transactions.lock().unwrap().push(tx);

        Ok(current_balance)
    }
}

#[derive(Clone)]
pub struct MockUserRepository {
    pub users: Arc<Mutex<Vec<UserRow>>>,
}

impl Default for MockUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockUserRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_user(&self, user: UserRow) {
        self.users.lock().unwrap().push(user);
    }
}

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn get_by_id(&self, _tenant_id: &TenantId, user_id: &UserId) -> Result<Option<UserRow>> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|u| u.id == user_id.0).cloned())
    }

    async fn create(&self, user: &UserRow) -> Result<()> {
        self.users.lock().unwrap().push(user.clone());
        Ok(())
    }

    async fn update_kyc_tier(
        &self,
        _tenant_id: &TenantId,
        user_id: &UserId,
        tier: i16,
    ) -> Result<()> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.iter_mut().find(|u| u.id == user_id.0) {
            user.kyc_tier = tier;
        }
        Ok(())
    }

    async fn update_risk_score(
        &self,
        _tenant_id: &TenantId,
        user_id: &UserId,
        score: Decimal,
    ) -> Result<()> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.iter_mut().find(|u| u.id == user_id.0) {
            user.risk_score = Some(score);
        }
        Ok(())
    }

    async fn update_status(
        &self,
        _tenant_id: &TenantId,
        user_id: &UserId,
        status: &str,
    ) -> Result<()> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.iter_mut().find(|u| u.id == user_id.0) {
            user.status = status.to_string();
        }
        Ok(())
    }

    async fn update_limits(
        &self,
        _tenant_id: &TenantId,
        user_id: &UserId,
        daily_payin_limit_vnd: Option<Decimal>,
        daily_payout_limit_vnd: Option<Decimal>,
    ) -> Result<()> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.iter_mut().find(|u| u.id == user_id.0) {
            if daily_payin_limit_vnd.is_some() {
                user.daily_payin_limit_vnd = daily_payin_limit_vnd;
            }
            if daily_payout_limit_vnd.is_some() {
                user.daily_payout_limit_vnd = daily_payout_limit_vnd;
            }
        }
        Ok(())
    }

    async fn list_users(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
        kyc_tier: Option<i16>,
        status: Option<&str>,
        search: Option<&str>,
    ) -> Result<Vec<UserRow>> {
        let users = self.users.lock().unwrap();
        let mut filtered: Vec<UserRow> = users
            .iter()
            .filter(|user| user.tenant_id == tenant_id.0)
            .filter(|user| kyc_tier.is_none_or(|tier| user.kyc_tier == tier))
            .filter(|user| status.is_none_or(|status| user.status == status))
            .filter(|user| search.is_none_or(|query| user.id.contains(query)))
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let start = offset.max(0) as usize;
        let end = (start + limit.max(0) as usize).min(filtered.len());

        if start >= filtered.len() {
            return Ok(vec![]);
        }

        Ok(filtered[start..end].to_vec())
    }

    async fn count_users(
        &self,
        tenant_id: &TenantId,
        kyc_tier: Option<i16>,
        status: Option<&str>,
        search: Option<&str>,
    ) -> Result<i64> {
        let users = self.users.lock().unwrap();
        let count = users
            .iter()
            .filter(|user| user.tenant_id == tenant_id.0)
            .filter(|user| kyc_tier.is_none_or(|tier| user.kyc_tier == tier))
            .filter(|user| status.is_none_or(|status| user.status == status))
            .filter(|user| search.is_none_or(|query| user.id.contains(query)))
            .count();
        Ok(count as i64)
    }

    async fn count_users_by_kyc_status(
        &self,
        tenant_id: &TenantId,
        kyc_status: &str,
    ) -> Result<i64> {
        let users = self.users.lock().unwrap();
        let count = users
            .iter()
            .filter(|user| user.tenant_id == tenant_id.0)
            .filter(|user| user.kyc_status == kyc_status)
            .count();
        Ok(count as i64)
    }

    async fn count_users_created_since(
        &self,
        tenant_id: &TenantId,
        since: DateTime<Utc>,
    ) -> Result<i64> {
        let users = self.users.lock().unwrap();
        let count = users
            .iter()
            .filter(|user| user.tenant_id == tenant_id.0)
            .filter(|user| user.created_at >= since)
            .count();
        Ok(count as i64)
    }
}

#[derive(Clone)]
pub struct MockTenantRepository {
    pub tenants: Arc<Mutex<Vec<TenantRow>>>,
}

impl Default for MockTenantRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTenantRepository {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_tenant(&self, tenant: TenantRow) {
        self.tenants.lock().unwrap().push(tenant);
    }
}

#[async_trait]
impl TenantRepository for MockTenantRepository {
    async fn get_by_id(&self, id: &TenantId) -> Result<Option<TenantRow>> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.iter().find(|t| t.id == id.0).cloned())
    }

    async fn get_by_api_key_hash(&self, hash: &str) -> Result<Option<TenantRow>> {
        let tenants = self.tenants.lock().unwrap();
        // SECURITY: Use constant-time comparison to prevent timing attacks
        Ok(tenants
            .iter()
            .find(|t| {
                let stored = t.api_key_hash.as_bytes();
                let provided = hash.as_bytes();
                // Only compare if lengths match to avoid early return on length mismatch
                stored.len() == provided.len() && bool::from(stored.ct_eq(provided))
            })
            .cloned())
    }

    async fn create(&self, tenant: &TenantRow) -> Result<()> {
        self.tenants.lock().unwrap().push(tenant.clone());
        Ok(())
    }

    async fn update_status(&self, id: &TenantId, status: &str) -> Result<()> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(tenant) = tenants.iter_mut().find(|t| t.id == id.0) {
            tenant.status = status.to_string();
        }
        Ok(())
    }

    async fn update_webhook_url(&self, id: &TenantId, url: &str) -> Result<()> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(tenant) = tenants.iter_mut().find(|t| t.id == id.0) {
            tenant.webhook_url = Some(url.to_string());
        }
        Ok(())
    }

    async fn update_api_key_hash(&self, id: &TenantId, hash: &str) -> Result<()> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(tenant) = tenants.iter_mut().find(|t| t.id == id.0) {
            tenant.api_key_hash = hash.to_string();
        }
        Ok(())
    }

    async fn update_webhook_secret(
        &self,
        id: &TenantId,
        hash: &str,
        encrypted: &[u8],
    ) -> Result<()> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(tenant) = tenants.iter_mut().find(|t| t.id == id.0) {
            tenant.webhook_secret_hash = hash.to_string();
            tenant.webhook_secret_encrypted = Some(encrypted.to_vec());
        }
        Ok(())
    }

    async fn update_limits(
        &self,
        id: &TenantId,
        daily_payin: Option<Decimal>,
        daily_payout: Option<Decimal>,
    ) -> Result<()> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(tenant) = tenants.iter_mut().find(|t| t.id == id.0) {
            tenant.daily_payin_limit_vnd = daily_payin;
            tenant.daily_payout_limit_vnd = daily_payout;
        }
        Ok(())
    }

    async fn update_config(&self, id: &TenantId, config: &serde_json::Value) -> Result<()> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(tenant) = tenants.iter_mut().find(|t| t.id == id.0) {
            tenant.config = config.clone();
        }
        Ok(())
    }

    async fn list_ids(&self) -> Result<Vec<TenantId>> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.iter().map(|t| TenantId(t.id.clone())).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_intent_repository_create_and_get() {
        let repo = MockIntentRepository::new();
        let intent = IntentRow {
            id: "intent-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-1".to_string(),
            intent_type: "PAYIN_VND".to_string(),
            state: "CREATED".to_string(),
            state_history: serde_json::json!([]),
            amount: Decimal::from(100000),
            currency: "VND".to_string(),
            actual_amount: None,
            rails_provider: Some("VCB".to_string()),
            reference_code: Some("REF123".to_string()),
            bank_tx_id: None,
            chain_id: None,
            tx_hash: None,
            from_address: None,
            to_address: None,
            metadata: serde_json::json!({}),
            idempotency_key: Some("idem-1".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            completed_at: None,
        };

        repo.create(&intent).await.unwrap();

        let found = repo
            .get_by_id(
                &TenantId::new("tenant-1"),
                &IntentId("intent-1".to_string()),
            )
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "intent-1");

        let by_idem = repo
            .get_by_idempotency_key(&TenantId::new("tenant-1"), &IdempotencyKey::new("idem-1"))
            .await
            .unwrap();
        assert!(by_idem.is_some());

        let by_ref = repo
            .get_by_reference_code(
                &TenantId::new("tenant-1"),
                &ReferenceCode("REF123".to_string()),
            )
            .await
            .unwrap();
        assert!(by_ref.is_some());
    }

    #[tokio::test]
    async fn test_mock_user_repository() {
        let repo = MockUserRepository::new();
        let user = UserRow {
            id: "user-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            status: "ACTIVE".to_string(),
            kyc_tier: 1,
            kyc_status: "VERIFIED".to_string(),
            kyc_verified_at: Some(Utc::now()),
            risk_score: None,
            risk_flags: serde_json::json!([]),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        repo.add_user(user);

        let found = repo
            .get_by_id(&TenantId::new("tenant-1"), &UserId::new("user-1"))
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().kyc_tier, 1);

        repo.update_kyc_tier(&TenantId::new("tenant-1"), &UserId::new("user-1"), 2)
            .await
            .unwrap();
        let updated = repo
            .get_by_id(&TenantId::new("tenant-1"), &UserId::new("user-1"))
            .await
            .unwrap();
        assert_eq!(updated.unwrap().kyc_tier, 2);
    }

    #[tokio::test]
    async fn test_mock_tenant_repository() {
        let repo = MockTenantRepository::new();
        let tenant = TenantRow {
            id: "tenant-1".to_string(),
            name: "Test Tenant".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash: "hash123".to_string(),
            webhook_secret_hash: "secret".to_string(),
            webhook_secret_encrypted: None,
            webhook_url: None,
            config: serde_json::json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        repo.add_tenant(tenant);

        let found = repo.get_by_id(&TenantId::new("tenant-1")).await.unwrap();
        assert!(found.is_some());

        let by_hash = repo.get_by_api_key_hash("hash123").await.unwrap();
        assert!(by_hash.is_some());
    }
}
