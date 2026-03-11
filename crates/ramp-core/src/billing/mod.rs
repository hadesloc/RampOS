//! Billing Module
//!
//! Usage-based billing with Stripe integration for enterprise tenants.
//! Tracks API calls, transaction volume, storage, and active users.

mod metering;
mod stripe;

pub use metering::{
    MeterEvent, MeterType, MetricAggregation, MetricValue, UsageMeter, UsageMetrics, UsagePeriod,
    UsageRecord, UsageSummary,
};
pub use stripe::{
    BillingPlan, BillingPlanTier, Invoice, InvoiceItem, InvoiceStatus, PlanFeature, PricingModel,
    StripeClient, StripeConfig, StripeError, Subscription, SubscriptionStatus,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Billing service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingConfig {
    /// Stripe configuration
    pub stripe: StripeConfig,
    /// Enable usage metering
    pub metering_enabled: bool,
    /// Usage sync interval in seconds
    pub sync_interval_secs: u64,
    /// Default billing plan ID
    pub default_plan_id: String,
    /// Free tier limits
    pub free_tier: FreeTierLimits,
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            stripe: StripeConfig::default(),
            metering_enabled: true,
            sync_interval_secs: 3600, // 1 hour
            default_plan_id: "plan_free".to_string(),
            free_tier: FreeTierLimits::default(),
        }
    }
}

/// Free tier usage limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeTierLimits {
    /// API calls per month
    pub api_calls_monthly: u64,
    /// Transaction volume per month (VND)
    pub transaction_volume_monthly: Decimal,
    /// Storage in bytes
    pub storage_bytes: u64,
    /// Active users
    pub active_users: u32,
}

impl Default for FreeTierLimits {
    fn default() -> Self {
        Self {
            api_calls_monthly: 10_000,
            transaction_volume_monthly: Decimal::from(100_000_000), // 100M VND
            storage_bytes: 1_073_741_824,                           // 1 GB
            active_users: 100,
        }
    }
}

/// Tenant billing status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantBillingStatus {
    pub tenant_id: TenantId,
    pub plan: BillingPlan,
    pub subscription: Option<Subscription>,
    pub current_usage: UsageSummary,
    pub overage_charges: Decimal,
    pub billing_cycle_start: DateTime<Utc>,
    pub billing_cycle_end: DateTime<Utc>,
    pub is_overdue: bool,
    pub last_invoice: Option<Invoice>,
}

/// Billing event for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingEvent {
    pub id: String,
    pub tenant_id: TenantId,
    pub event_type: BillingEventType,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingEventType {
    SubscriptionCreated,
    SubscriptionUpdated,
    SubscriptionCancelled,
    InvoiceCreated,
    InvoicePaid,
    InvoiceFailed,
    UsageRecorded,
    OverageTriggered,
    PlanUpgraded,
    PlanDowngraded,
}

/// Billing data provider trait
#[async_trait]
pub trait BillingDataProvider: Send + Sync {
    /// Get tenant billing status
    async fn get_tenant_billing(&self, tenant_id: &TenantId)
        -> Result<Option<TenantBillingStatus>>;

    /// Store tenant billing status
    async fn store_tenant_billing(&self, status: &TenantBillingStatus) -> Result<()>;

    /// Get billing plan by ID
    async fn get_plan(&self, plan_id: &str) -> Result<Option<BillingPlan>>;

    /// List all available plans
    async fn list_plans(&self) -> Result<Vec<BillingPlan>>;

    /// Record billing event
    async fn record_event(&self, event: &BillingEvent) -> Result<()>;

    /// Get billing events for tenant
    async fn get_events(
        &self,
        tenant_id: &TenantId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<BillingEvent>>;
}

/// Main billing service
pub struct BillingService {
    config: BillingConfig,
    stripe_client: Arc<StripeClient>,
    usage_meter: Arc<UsageMeter>,
    data_provider: Arc<dyn BillingDataProvider>,
}

impl BillingService {
    pub fn new(config: BillingConfig, data_provider: Arc<dyn BillingDataProvider>) -> Self {
        let stripe_client = Arc::new(StripeClient::new(config.stripe.clone()));
        let usage_meter = Arc::new(UsageMeter::new());

        Self {
            config,
            stripe_client,
            usage_meter,
            data_provider,
        }
    }

    /// Get billing status for a tenant
    pub async fn get_billing_status(&self, tenant_id: &TenantId) -> Result<TenantBillingStatus> {
        if let Some(status) = self.data_provider.get_tenant_billing(tenant_id).await? {
            return Ok(status);
        }

        // Return default status for new tenants
        let default_plan = self
            .data_provider
            .get_plan(&self.config.default_plan_id)
            .await?
            .unwrap_or_else(BillingPlan::free);

        let now = Utc::now();
        let cycle_start = now;
        let cycle_end = now + chrono::Duration::days(30);

        Ok(TenantBillingStatus {
            tenant_id: tenant_id.clone(),
            plan: default_plan,
            subscription: None,
            current_usage: UsageSummary::default(),
            overage_charges: Decimal::ZERO,
            billing_cycle_start: cycle_start,
            billing_cycle_end: cycle_end,
            is_overdue: false,
            last_invoice: None,
        })
    }

    /// Create subscription for tenant
    pub async fn create_subscription(
        &self,
        tenant_id: &TenantId,
        plan_id: &str,
        stripe_customer_id: &str,
    ) -> Result<Subscription> {
        let plan =
            self.data_provider.get_plan(plan_id).await?.ok_or_else(|| {
                ramp_common::Error::Validation(format!("Plan {} not found", plan_id))
            })?;

        let subscription = self
            .stripe_client
            .create_subscription(stripe_customer_id, &plan)
            .await?;

        // Record event
        let event = BillingEvent {
            id: format!("evt_{}", Utc::now().timestamp_millis()),
            tenant_id: tenant_id.clone(),
            event_type: BillingEventType::SubscriptionCreated,
            details: serde_json::json!({
                "plan_id": plan_id,
                "subscription_id": subscription.id,
            }),
            created_at: Utc::now(),
        };
        self.data_provider.record_event(&event).await?;

        Ok(subscription)
    }

    /// Cancel subscription
    pub async fn cancel_subscription(&self, tenant_id: &TenantId) -> Result<()> {
        let status = self.get_billing_status(tenant_id).await?;

        if let Some(subscription) = status.subscription {
            self.stripe_client
                .cancel_subscription(&subscription.id)
                .await?;

            let event = BillingEvent {
                id: format!("evt_{}", Utc::now().timestamp_millis()),
                tenant_id: tenant_id.clone(),
                event_type: BillingEventType::SubscriptionCancelled,
                details: serde_json::json!({
                    "subscription_id": subscription.id,
                }),
                created_at: Utc::now(),
            };
            self.data_provider.record_event(&event).await?;
        }

        Ok(())
    }

    /// Record usage for a tenant
    pub async fn record_usage(
        &self,
        tenant_id: &TenantId,
        meter_type: MeterType,
        value: MetricValue,
    ) -> Result<()> {
        let record = UsageRecord {
            id: format!("usage_{}", Utc::now().timestamp_millis()),
            tenant_id: tenant_id.clone(),
            meter_type,
            value,
            recorded_at: Utc::now(),
            synced_to_stripe: false,
        };

        self.usage_meter.record(record).await?;

        Ok(())
    }

    /// Get current usage for tenant
    pub async fn get_usage(&self, tenant_id: &TenantId) -> Result<UsageSummary> {
        self.usage_meter.get_summary(tenant_id).await
    }

    /// Sync usage to Stripe
    pub async fn sync_usage_to_stripe(&self, tenant_id: &TenantId) -> Result<()> {
        let status = self.get_billing_status(tenant_id).await?;

        if let Some(subscription) = status.subscription {
            let usage = self.usage_meter.get_summary(tenant_id).await?;

            self.stripe_client
                .report_usage(&subscription.id, &usage)
                .await?;

            self.usage_meter.mark_synced(tenant_id).await?;
        }

        Ok(())
    }

    /// Check if tenant has exceeded free tier
    pub async fn check_free_tier_exceeded(&self, tenant_id: &TenantId) -> Result<bool> {
        let usage = self.usage_meter.get_summary(tenant_id).await?;

        let exceeded = usage.api_calls > self.config.free_tier.api_calls_monthly
            || usage.transaction_volume > self.config.free_tier.transaction_volume_monthly
            || usage.storage_bytes > self.config.free_tier.storage_bytes
            || usage.active_users > self.config.free_tier.active_users;

        Ok(exceeded)
    }

    /// Generate invoice for tenant
    pub async fn generate_invoice(&self, tenant_id: &TenantId) -> Result<Invoice> {
        let status = self.get_billing_status(tenant_id).await?;
        let usage = self.usage_meter.get_summary(tenant_id).await?;

        let invoice = self
            .stripe_client
            .create_invoice(tenant_id, &status.plan, &usage)
            .await?;

        let event = BillingEvent {
            id: format!("evt_{}", Utc::now().timestamp_millis()),
            tenant_id: tenant_id.clone(),
            event_type: BillingEventType::InvoiceCreated,
            details: serde_json::json!({
                "invoice_id": invoice.id,
                "amount": invoice.amount_due.to_string(),
            }),
            created_at: Utc::now(),
        };
        self.data_provider.record_event(&event).await?;

        Ok(invoice)
    }

    /// Get available billing plans
    pub async fn list_plans(&self) -> Result<Vec<BillingPlan>> {
        self.data_provider.list_plans().await
    }

    /// Upgrade tenant to a new plan
    pub async fn upgrade_plan(&self, tenant_id: &TenantId, new_plan_id: &str) -> Result<()> {
        let status = self.get_billing_status(tenant_id).await?;

        if let Some(subscription) = status.subscription {
            let new_plan = self
                .data_provider
                .get_plan(new_plan_id)
                .await?
                .ok_or_else(|| {
                    ramp_common::Error::Validation(format!("Plan {} not found", new_plan_id))
                })?;

            self.stripe_client
                .update_subscription(&subscription.id, &new_plan)
                .await?;

            let event = BillingEvent {
                id: format!("evt_{}", Utc::now().timestamp_millis()),
                tenant_id: tenant_id.clone(),
                event_type: BillingEventType::PlanUpgraded,
                details: serde_json::json!({
                    "old_plan_id": status.plan.id,
                    "new_plan_id": new_plan_id,
                }),
                created_at: Utc::now(),
            };
            self.data_provider.record_event(&event).await?;
        }

        Ok(())
    }

    /// Get billing configuration
    pub fn get_config(&self) -> &BillingConfig {
        &self.config
    }

    /// Get Stripe client for direct operations
    pub fn stripe_client(&self) -> Arc<StripeClient> {
        Arc::clone(&self.stripe_client)
    }

    /// Get usage meter for direct recording
    pub fn usage_meter(&self) -> Arc<UsageMeter> {
        Arc::clone(&self.usage_meter)
    }
}

pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    pub struct MockBillingDataProvider {
        billings: Mutex<HashMap<String, TenantBillingStatus>>,
        plans: Mutex<HashMap<String, BillingPlan>>,
        events: Mutex<Vec<BillingEvent>>,
    }

    impl MockBillingDataProvider {
        pub fn new() -> Self {
            let mut plans = HashMap::new();
            plans.insert("plan_free".to_string(), BillingPlan::free());
            plans.insert("plan_starter".to_string(), BillingPlan::starter());
            plans.insert("plan_growth".to_string(), BillingPlan::growth());
            plans.insert("plan_enterprise".to_string(), BillingPlan::enterprise());

            Self {
                billings: Mutex::new(HashMap::new()),
                plans: Mutex::new(plans),
                events: Mutex::new(Vec::new()),
            }
        }
    }

    impl Default for MockBillingDataProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl BillingDataProvider for MockBillingDataProvider {
        async fn get_tenant_billing(
            &self,
            tenant_id: &TenantId,
        ) -> Result<Option<TenantBillingStatus>> {
            Ok(self
                .billings
                .lock()
                .expect("Lock poisoned")
                .get(&tenant_id.0)
                .cloned())
        }

        async fn store_tenant_billing(&self, status: &TenantBillingStatus) -> Result<()> {
            self.billings
                .lock()
                .expect("Lock poisoned")
                .insert(status.tenant_id.0.clone(), status.clone());
            Ok(())
        }

        async fn get_plan(&self, plan_id: &str) -> Result<Option<BillingPlan>> {
            Ok(self
                .plans
                .lock()
                .expect("Lock poisoned")
                .get(plan_id)
                .cloned())
        }

        async fn list_plans(&self) -> Result<Vec<BillingPlan>> {
            Ok(self
                .plans
                .lock()
                .expect("Lock poisoned")
                .values()
                .cloned()
                .collect())
        }

        async fn record_event(&self, event: &BillingEvent) -> Result<()> {
            self.events
                .lock()
                .expect("Lock poisoned")
                .push(event.clone());
            Ok(())
        }

        async fn get_events(
            &self,
            tenant_id: &TenantId,
            from: DateTime<Utc>,
            to: DateTime<Utc>,
        ) -> Result<Vec<BillingEvent>> {
            Ok(self
                .events
                .lock()
                .expect("Lock poisoned")
                .iter()
                .filter(|e| {
                    e.tenant_id.0 == tenant_id.0 && e.created_at >= from && e.created_at <= to
                })
                .cloned()
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock::MockBillingDataProvider;

    fn create_test_service() -> BillingService {
        let provider = Arc::new(MockBillingDataProvider::new());
        BillingService::new(BillingConfig::default(), provider)
    }

    #[tokio::test]
    async fn test_get_billing_status_default() {
        let service = create_test_service();
        let tenant_id = TenantId::new("test_tenant");

        let status = service.get_billing_status(&tenant_id).await.unwrap();

        assert_eq!(status.tenant_id.0, "test_tenant");
        assert_eq!(status.plan.id, "plan_free");
        assert!(!status.is_overdue);
    }

    #[tokio::test]
    async fn test_record_usage() {
        let service = create_test_service();
        let tenant_id = TenantId::new("test_tenant");

        service
            .record_usage(&tenant_id, MeterType::ApiCalls, MetricValue::Count(100))
            .await
            .unwrap();

        let usage = service.get_usage(&tenant_id).await.unwrap();
        assert_eq!(usage.api_calls, 100);
    }

    #[tokio::test]
    async fn test_check_free_tier() {
        let service = create_test_service();
        let tenant_id = TenantId::new("test_tenant");

        // Record usage below free tier
        service
            .record_usage(&tenant_id, MeterType::ApiCalls, MetricValue::Count(1000))
            .await
            .unwrap();

        let exceeded = service.check_free_tier_exceeded(&tenant_id).await.unwrap();
        assert!(!exceeded);

        // Record usage above free tier
        service
            .record_usage(&tenant_id, MeterType::ApiCalls, MetricValue::Count(100_000))
            .await
            .unwrap();

        let exceeded = service.check_free_tier_exceeded(&tenant_id).await.unwrap();
        assert!(exceeded);
    }

    #[tokio::test]
    async fn test_list_plans() {
        let service = create_test_service();

        let plans = service.list_plans().await.unwrap();

        assert!(plans.len() >= 4);
        assert!(plans.iter().any(|p| p.id == "plan_free"));
        assert!(plans.iter().any(|p| p.id == "plan_enterprise"));
    }

    #[tokio::test]
    async fn test_record_usage_transaction_volume() {
        let service = create_test_service();
        let tenant_id = TenantId::new("test_tenant");

        service
            .record_usage(
                &tenant_id,
                MeterType::TransactionVolume,
                MetricValue::Volume(rust_decimal::Decimal::from(50_000_000)),
            )
            .await
            .unwrap();

        let usage = service.get_usage(&tenant_id).await.unwrap();
        assert_eq!(
            usage.transaction_volume,
            rust_decimal::Decimal::from(50_000_000)
        );
    }

    #[tokio::test]
    async fn test_record_usage_storage_bytes() {
        let service = create_test_service();
        let tenant_id = TenantId::new("test_tenant");

        service
            .record_usage(
                &tenant_id,
                MeterType::StorageBytes,
                MetricValue::Bytes(1024 * 1024),
            )
            .await
            .unwrap();

        let usage = service.get_usage(&tenant_id).await.unwrap();
        assert_eq!(usage.storage_bytes, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_check_free_tier_not_exceeded_initially() {
        let service = create_test_service();
        let tenant_id = TenantId::new("test_tenant");

        let exceeded = service.check_free_tier_exceeded(&tenant_id).await.unwrap();
        assert!(!exceeded);
    }

    #[tokio::test]
    async fn test_check_free_tier_exceeded_by_volume() {
        let service = create_test_service();
        let tenant_id = TenantId::new("test_tenant");

        service
            .record_usage(
                &tenant_id,
                MeterType::TransactionVolume,
                MetricValue::Volume(rust_decimal::Decimal::from(200_000_000)),
            )
            .await
            .unwrap();

        let exceeded = service.check_free_tier_exceeded(&tenant_id).await.unwrap();
        assert!(exceeded);
    }

    #[tokio::test]
    async fn test_billing_config_default() {
        let config = BillingConfig::default();
        assert!(config.metering_enabled);
        assert_eq!(config.sync_interval_secs, 3600);
        assert_eq!(config.default_plan_id, "plan_free");
    }

    #[tokio::test]
    async fn test_free_tier_limits_default() {
        let limits = FreeTierLimits::default();
        assert_eq!(limits.api_calls_monthly, 10_000);
        assert_eq!(limits.storage_bytes, 1_073_741_824);
        assert_eq!(limits.active_users, 100);
    }

    #[tokio::test]
    async fn test_billing_status_default_plan() {
        let service = create_test_service();
        let tenant_id = TenantId::new("new_tenant");

        let status = service.get_billing_status(&tenant_id).await.unwrap();
        assert_eq!(status.plan.id, "plan_free");
        assert!(status.subscription.is_none());
        assert!(!status.is_overdue);
        assert_eq!(status.overage_charges, rust_decimal::Decimal::ZERO);
    }

    #[test]
    fn test_billing_event_type_values() {
        // Ensure all event types exist and are distinct
        let types = vec![
            BillingEventType::SubscriptionCreated,
            BillingEventType::SubscriptionUpdated,
            BillingEventType::SubscriptionCancelled,
            BillingEventType::InvoiceCreated,
            BillingEventType::InvoicePaid,
            BillingEventType::InvoiceFailed,
            BillingEventType::UsageRecorded,
            BillingEventType::OverageTriggered,
            BillingEventType::PlanUpgraded,
            BillingEventType::PlanDowngraded,
        ];
        // Just ensure they're all constructed without panic
        assert_eq!(types.len(), 10);
    }

    #[tokio::test]
    async fn test_service_exposes_config() {
        let service = create_test_service();
        let config = service.get_config();
        assert_eq!(config.default_plan_id, "plan_free");
    }

    #[tokio::test]
    async fn test_service_exposes_stripe_client() {
        let service = create_test_service();
        let _client = service.stripe_client();
        // Just ensure it doesn't panic
    }

    #[tokio::test]
    async fn test_service_exposes_usage_meter() {
        let service = create_test_service();
        let _meter = service.usage_meter();
    }

    #[tokio::test]
    async fn test_mock_provider_record_and_get_events() {
        let provider = MockBillingDataProvider::new();
        let tenant_id = TenantId::new("t1");

        let event = BillingEvent {
            id: "evt_1".to_string(),
            tenant_id: tenant_id.clone(),
            event_type: BillingEventType::UsageRecorded,
            details: serde_json::json!({"calls": 100}),
            created_at: Utc::now(),
        };
        provider.record_event(&event).await.unwrap();

        let events = provider
            .get_events(
                &tenant_id,
                Utc::now() - chrono::Duration::hours(1),
                Utc::now() + chrono::Duration::hours(1),
            )
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "evt_1");
    }

    #[tokio::test]
    async fn test_mock_provider_store_and_get_billing() {
        let provider = MockBillingDataProvider::new();
        let tenant_id = TenantId::new("t1");
        let now = Utc::now();

        let status = TenantBillingStatus {
            tenant_id: tenant_id.clone(),
            plan: BillingPlan::starter(),
            subscription: None,
            current_usage: UsageSummary::default(),
            overage_charges: rust_decimal::Decimal::ZERO,
            billing_cycle_start: now,
            billing_cycle_end: now + chrono::Duration::days(30),
            is_overdue: false,
            last_invoice: None,
        };
        provider.store_tenant_billing(&status).await.unwrap();

        let retrieved = provider.get_tenant_billing(&tenant_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().plan.id, "plan_starter");
    }
}
