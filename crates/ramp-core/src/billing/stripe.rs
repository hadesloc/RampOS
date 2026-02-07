//! Stripe Integration
//!
//! Adapter for Stripe API.

use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stripe Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeConfig {
    pub secret_key: String,
    pub publishable_key: String,
    pub webhook_secret: String,
}

impl Default for StripeConfig {
    fn default() -> Self {
        Self {
            secret_key: std::env::var("STRIPE_SECRET_KEY")
                .unwrap_or_else(|_| String::new()),
            publishable_key: std::env::var("STRIPE_PUBLISHABLE_KEY")
                .unwrap_or_else(|_| String::new()),
            webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET")
                .unwrap_or_else(|_| String::new()),
        }
    }
}

/// Stripe Client
pub struct StripeClient {
    config: StripeConfig,
    // client: reqwest::Client, // Uncomment when integrating real Stripe
}

impl StripeClient {
    pub fn new(config: StripeConfig) -> Self {
        Self {
            config,
            // client: reqwest::Client::new(),
        }
    }

    /// Create a customer in Stripe
    pub async fn create_customer(&self, tenant_id: &TenantId, email: &str) -> Result<String> {
        // Mock implementation
        Ok(format!("cus_{}_{}", tenant_id.0, Utc::now().timestamp()))
    }

    /// Create a subscription
    pub async fn create_subscription(
        &self,
        customer_id: &str,
        plan: &BillingPlan,
    ) -> Result<Subscription> {
        // Mock implementation
        Ok(Subscription {
            id: format!("sub_{}", Utc::now().timestamp()),
            customer_id: customer_id.to_string(),
            status: SubscriptionStatus::Active,
            plan_id: plan.id.clone(),
            current_period_start: Utc::now(),
            current_period_end: Utc::now() + chrono::Duration::days(30),
            cancel_at_period_end: false,
        })
    }

    /// Update subscription
    pub async fn update_subscription(
        &self,
        subscription_id: &str,
        plan: &BillingPlan,
    ) -> Result<Subscription> {
        // Mock implementation
        Ok(Subscription {
            id: subscription_id.to_string(),
            customer_id: "cus_mock".to_string(),
            status: SubscriptionStatus::Active,
            plan_id: plan.id.clone(),
            current_period_start: Utc::now(),
            current_period_end: Utc::now() + chrono::Duration::days(30),
            cancel_at_period_end: false,
        })
    }

    /// Cancel subscription
    pub async fn cancel_subscription(&self, subscription_id: &str) -> Result<()> {
        // Mock implementation
        Ok(())
    }

    /// Report usage for metered billing
    pub async fn report_usage(
        &self,
        subscription_id: &str,
        usage: &super::UsageSummary,
    ) -> Result<()> {
        // Mock implementation
        Ok(())
    }

    /// Create an invoice
    pub async fn create_invoice(
        &self,
        tenant_id: &TenantId,
        plan: &BillingPlan,
        usage: &super::UsageSummary,
    ) -> Result<Invoice> {
        let now = Utc::now();
        // Mock implementation
        Ok(Invoice {
            id: format!("in_{}", now.timestamp()),
            customer_id: format!("cus_{}", tenant_id.0),
            status: InvoiceStatus::Draft,
            currency: "usd".to_string(),
            amount_due: plan.price + calculate_overage(plan, usage),
            amount_paid: Decimal::ZERO,
            period_start: now - chrono::Duration::days(30),
            period_end: now,
            lines: vec![],
        })
    }
}

fn calculate_overage(plan: &BillingPlan, usage: &super::UsageSummary) -> Decimal {
    // Simplified overage calculation
    let mut overage = Decimal::ZERO;

    if usage.api_calls > plan.limits.api_calls {
        let excess = Decimal::from(usage.api_calls - plan.limits.api_calls);
        overage += excess * Decimal::new(1, 4); // $0.0001 per call
    }

    overage
}

/// Stripe Error wrapper
#[derive(Debug, thiserror::Error)]
pub enum StripeError {
    #[error("API Error: {0}")]
    ApiError(String),
    #[error("Invalid Request: {0}")]
    InvalidRequest(String),
}

/// Billing Plan Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingPlan {
    pub id: String,
    pub name: String,
    pub description: String,
    pub price: Decimal,
    pub currency: String,
    pub interval: String, // month, year
    pub features: Vec<PlanFeature>,
    pub limits: PlanLimits,
}

impl BillingPlan {
    pub fn free() -> Self {
        Self {
            id: "plan_free".to_string(),
            name: "Free Tier".to_string(),
            description: "For small teams just getting started".to_string(),
            price: Decimal::ZERO,
            currency: "usd".to_string(),
            interval: "month".to_string(),
            features: vec![
                PlanFeature { code: "api_access".to_string(), name: "API Access".to_string(), included: true },
                PlanFeature { code: "sso".to_string(), name: "SSO".to_string(), included: false },
            ],
            limits: PlanLimits {
                api_calls: 10_000,
                transaction_volume: Decimal::from(100_000),
                users: 5,
            },
        }
    }

    pub fn starter() -> Self {
        Self {
            id: "plan_starter".to_string(),
            name: "Starter".to_string(),
            description: "Growing businesses".to_string(),
            price: Decimal::new(2900, 2), // $29.00
            currency: "usd".to_string(),
            interval: "month".to_string(),
            features: vec![
                PlanFeature { code: "api_access".to_string(), name: "API Access".to_string(), included: true },
                PlanFeature { code: "sso".to_string(), name: "SSO".to_string(), included: false },
            ],
            limits: PlanLimits {
                api_calls: 100_000,
                transaction_volume: Decimal::from(1_000_000),
                users: 20,
            },
        }
    }

    pub fn growth() -> Self {
        Self {
            id: "plan_growth".to_string(),
            name: "Growth".to_string(),
            description: "Scaling companies".to_string(),
            price: Decimal::new(9900, 2), // $99.00
            currency: "usd".to_string(),
            interval: "month".to_string(),
            features: vec![
                PlanFeature { code: "api_access".to_string(), name: "API Access".to_string(), included: true },
                PlanFeature { code: "sso".to_string(), name: "SSO".to_string(), included: true },
            ],
            limits: PlanLimits {
                api_calls: 1_000_000,
                transaction_volume: Decimal::from(10_000_000),
                users: 100,
            },
        }
    }

    pub fn enterprise() -> Self {
        Self {
            id: "plan_enterprise".to_string(),
            name: "Enterprise".to_string(),
            description: "Mission critical applications".to_string(),
            price: Decimal::new(49900, 2), // $499.00
            currency: "usd".to_string(),
            interval: "month".to_string(),
            features: vec![
                PlanFeature { code: "api_access".to_string(), name: "API Access".to_string(), included: true },
                PlanFeature { code: "sso".to_string(), name: "SSO".to_string(), included: true },
                PlanFeature { code: "sla".to_string(), name: "99.9% SLA".to_string(), included: true },
            ],
            limits: PlanLimits {
                api_calls: 10_000_000,
                transaction_volume: Decimal::from(100_000_000),
                users: 1000,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanFeature {
    pub code: String,
    pub name: String,
    pub included: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanLimits {
    pub api_calls: u64,
    pub transaction_volume: Decimal,
    pub users: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingPlanTier {
    pub up_to: Option<u64>,
    pub unit_amount: Decimal,
    pub flat_amount: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PricingModel {
    FlatRate,
    PerSeat,
    UsageBased,
    Tiered,
}

/// Subscription Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub customer_id: String,
    pub status: SubscriptionStatus,
    pub plan_id: String,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub cancel_at_period_end: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Canceled,
    Unpaid,
    Trialing,
    Incomplete,
    IncompleteExpired,
}

/// Invoice Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub customer_id: String,
    pub status: InvoiceStatus,
    pub currency: String,
    pub amount_due: Decimal,
    pub amount_paid: Decimal,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub lines: Vec<InvoiceItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Open,
    Paid,
    Uncollectible,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    pub id: String,
    pub amount: Decimal,
    pub currency: String,
    pub description: Option<String>,
    pub quantity: Option<u64>,
}
