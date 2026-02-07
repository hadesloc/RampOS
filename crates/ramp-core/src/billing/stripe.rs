//! Stripe Integration
//!
//! Adapter for Stripe API with real HTTP calls.
//! Falls back to mock responses when `secret_key` is empty (dev/test mode).

use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

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
    client: reqwest::Client,
}

impl StripeClient {
    pub fn new(config: StripeConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Returns true if a real Stripe API key is configured.
    fn has_api_key(&self) -> bool {
        !self.config.secret_key.is_empty()
    }

    // -----------------------------------------------------------------------
    // Low-level HTTP helpers
    // -----------------------------------------------------------------------

    /// POST to a Stripe endpoint with form-encoded params.
    async fn stripe_post(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let url = format!("{}/{}", STRIPE_API_BASE, endpoint);
        debug!(url = %url, "Stripe POST request");

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.config.secret_key)
            .form(params)
            .send()
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "stripe".into(),
                message: format!("HTTP request failed: {}", e),
            })?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "stripe".into(),
                message: format!("Failed to read response body: {}", e),
            })?;

        if !status.is_success() {
            let msg = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(String::from))
                .unwrap_or_else(|| format!("HTTP {} - {}", status, body));

            return Err(ramp_common::Error::ExternalService {
                service: "stripe".into(),
                message: msg,
            });
        }

        serde_json::from_str(&body).map_err(|e| ramp_common::Error::ExternalService {
            service: "stripe".into(),
            message: format!("Failed to parse Stripe response: {}", e),
        })
    }

    /// GET from a Stripe endpoint.
    async fn stripe_get(&self, endpoint: &str) -> Result<serde_json::Value> {
        let url = format!("{}/{}", STRIPE_API_BASE, endpoint);
        debug!(url = %url, "Stripe GET request");

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.config.secret_key)
            .send()
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "stripe".into(),
                message: format!("HTTP request failed: {}", e),
            })?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "stripe".into(),
                message: format!("Failed to read response body: {}", e),
            })?;

        if !status.is_success() {
            let msg = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(String::from))
                .unwrap_or_else(|| format!("HTTP {} - {}", status, body));

            return Err(ramp_common::Error::ExternalService {
                service: "stripe".into(),
                message: msg,
            });
        }

        serde_json::from_str(&body).map_err(|e| ramp_common::Error::ExternalService {
            service: "stripe".into(),
            message: format!("Failed to parse Stripe response: {}", e),
        })
    }

    // -----------------------------------------------------------------------
    // Public API methods -- each falls back to mock data when no key is set.
    // -----------------------------------------------------------------------

    /// Create a customer in Stripe
    pub async fn create_customer(&self, tenant_id: &TenantId, email: &str) -> Result<String> {
        if !self.has_api_key() {
            warn!("Stripe API key not configured -- returning mock customer ID");
            return Ok(format!("cus_mock_{}_{}", tenant_id.0, Utc::now().timestamp()));
        }

        let tenant_str = tenant_id.0.clone();
        let resp = self
            .stripe_post(
                "customers",
                &[
                    ("email", email),
                    ("metadata[tenant_id]", &tenant_str),
                ],
            )
            .await?;

        resp["id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                ramp_common::Error::ExternalService {
                    service: "stripe".into(),
                    message: "Missing customer ID in Stripe response".into(),
                }
            })
    }

    /// Create a subscription
    pub async fn create_subscription(
        &self,
        customer_id: &str,
        plan: &BillingPlan,
    ) -> Result<Subscription> {
        if !self.has_api_key() {
            warn!("Stripe API key not configured -- returning mock subscription");
            return Ok(Subscription {
                id: format!("sub_mock_{}", Utc::now().timestamp()),
                customer_id: customer_id.to_string(),
                status: SubscriptionStatus::Active,
                plan_id: plan.id.clone(),
                current_period_start: Utc::now(),
                current_period_end: Utc::now() + chrono::Duration::days(30),
                cancel_at_period_end: false,
            });
        }

        let resp = self
            .stripe_post(
                "subscriptions",
                &[
                    ("customer", customer_id),
                    ("items[0][price]", &plan.id),
                ],
            )
            .await?;

        parse_subscription(&resp)
    }

    /// Update subscription (e.g. plan change / upgrade / downgrade)
    pub async fn update_subscription(
        &self,
        subscription_id: &str,
        plan: &BillingPlan,
    ) -> Result<Subscription> {
        if !self.has_api_key() {
            warn!("Stripe API key not configured -- returning mock updated subscription");
            return Ok(Subscription {
                id: subscription_id.to_string(),
                customer_id: "cus_mock".to_string(),
                status: SubscriptionStatus::Active,
                plan_id: plan.id.clone(),
                current_period_start: Utc::now(),
                current_period_end: Utc::now() + chrono::Duration::days(30),
                cancel_at_period_end: false,
            });
        }

        // First retrieve current subscription to get the subscription item ID
        let current = self
            .stripe_get(&format!("subscriptions/{}", subscription_id))
            .await?;

        let item_id = current["items"]["data"][0]["id"]
            .as_str()
            .ok_or_else(|| ramp_common::Error::ExternalService {
                service: "stripe".into(),
                message: "Cannot find subscription item to update".into(),
            })?;

        let endpoint = format!("subscriptions/{}", subscription_id);
        let resp = self
            .stripe_post(
                &endpoint,
                &[
                    ("items[0][id]", item_id),
                    ("items[0][price]", &plan.id),
                    ("proration_behavior", "create_prorations"),
                ],
            )
            .await?;

        parse_subscription(&resp)
    }

    /// Cancel subscription at period end
    pub async fn cancel_subscription(&self, subscription_id: &str) -> Result<()> {
        if !self.has_api_key() {
            warn!("Stripe API key not configured -- mock cancellation");
            return Ok(());
        }

        let endpoint = format!("subscriptions/{}", subscription_id);
        self.stripe_post(&endpoint, &[("cancel_at_period_end", "true")])
            .await?;

        Ok(())
    }

    /// Report usage for metered billing (Stripe Usage Records)
    pub async fn report_usage(
        &self,
        subscription_id: &str,
        usage: &super::UsageSummary,
    ) -> Result<()> {
        if !self.has_api_key() {
            warn!("Stripe API key not configured -- skipping usage report");
            return Ok(());
        }

        // Retrieve subscription to find the metered subscription item
        let sub = self
            .stripe_get(&format!("subscriptions/{}", subscription_id))
            .await?;

        let item_id = sub["items"]["data"][0]["id"]
            .as_str()
            .ok_or_else(|| ramp_common::Error::ExternalService {
                service: "stripe".into(),
                message: "No subscription item found for usage reporting".into(),
            })?;

        let quantity = usage.api_calls.to_string();
        let timestamp = Utc::now().timestamp().to_string();

        let endpoint = format!("subscription_items/{}/usage_records", item_id);
        self.stripe_post(
            &endpoint,
            &[
                ("quantity", &quantity),
                ("timestamp", &timestamp),
                ("action", "increment"),
            ],
        )
        .await?;

        Ok(())
    }

    /// Create an invoice for a tenant
    pub async fn create_invoice(
        &self,
        tenant_id: &TenantId,
        plan: &BillingPlan,
        usage: &super::UsageSummary,
    ) -> Result<Invoice> {
        let now = Utc::now();

        if !self.has_api_key() {
            warn!("Stripe API key not configured -- returning mock invoice");
            return Ok(Invoice {
                id: format!("in_mock_{}", now.timestamp()),
                customer_id: format!("cus_mock_{}", tenant_id.0),
                status: InvoiceStatus::Draft,
                currency: "usd".to_string(),
                amount_due: plan.price + calculate_overage(plan, usage),
                amount_paid: Decimal::ZERO,
                period_start: now - chrono::Duration::days(30),
                period_end: now,
                lines: vec![],
            });
        }

        let tenant_str = tenant_id.0.clone();
        let resp = self
            .stripe_post(
                "invoices",
                &[
                    ("auto_advance", "false"),
                    ("collection_method", "send_invoice"),
                    ("days_until_due", "30"),
                    ("metadata[tenant_id]", &tenant_str),
                ],
            )
            .await?;

        let invoice_id = resp["id"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| format!("in_{}", now.timestamp()));

        let customer_id = resp["customer"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| format!("cus_{}", tenant_id.0));

        // Add a line item for the base plan
        let price_cents = (plan.price * Decimal::from(100)).to_string();
        let plan_desc = format!("{} plan", plan.name);
        let _line = self
            .stripe_post(
                "invoiceitems",
                &[
                    ("invoice", &invoice_id),
                    ("amount", &price_cents),
                    ("currency", &plan.currency),
                    ("description", &plan_desc),
                ],
            )
            .await?;

        // Add overage line item if applicable
        let overage = calculate_overage(plan, usage);
        if overage > Decimal::ZERO {
            let overage_cents = (overage * Decimal::from(100)).to_string();
            let _overage_line = self
                .stripe_post(
                    "invoiceitems",
                    &[
                        ("invoice", &invoice_id),
                        ("amount", &overage_cents),
                        ("currency", &plan.currency),
                        ("description", "API overage charges"),
                    ],
                )
                .await?;
        }

        let amount_due = plan.price + overage;

        Ok(Invoice {
            id: invoice_id,
            customer_id,
            status: InvoiceStatus::Draft,
            currency: plan.currency.clone(),
            amount_due,
            amount_paid: Decimal::ZERO,
            period_start: now - chrono::Duration::days(30),
            period_end: now,
            lines: vec![],
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a Stripe subscription JSON object into our domain `Subscription`.
fn parse_subscription(resp: &serde_json::Value) -> Result<Subscription> {
    let id = resp["id"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| ramp_common::Error::ExternalService {
            service: "stripe".into(),
            message: "Missing subscription ID".into(),
        })?;

    let customer_id = resp["customer"]
        .as_str()
        .map(String::from)
        .unwrap_or_default();

    let status = match resp["status"].as_str().unwrap_or("active") {
        "active" => SubscriptionStatus::Active,
        "past_due" => SubscriptionStatus::PastDue,
        "canceled" => SubscriptionStatus::Canceled,
        "unpaid" => SubscriptionStatus::Unpaid,
        "trialing" => SubscriptionStatus::Trialing,
        "incomplete" => SubscriptionStatus::Incomplete,
        "incomplete_expired" => SubscriptionStatus::IncompleteExpired,
        _ => SubscriptionStatus::Active,
    };

    let plan_id = resp["items"]["data"][0]["price"]["id"]
        .as_str()
        .map(String::from)
        .unwrap_or_default();

    let period_start = resp["current_period_start"]
        .as_i64()
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        .unwrap_or_else(Utc::now);

    let period_end = resp["current_period_end"]
        .as_i64()
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        .unwrap_or_else(|| Utc::now() + chrono::Duration::days(30));

    let cancel_at_period_end = resp["cancel_at_period_end"].as_bool().unwrap_or(false);

    Ok(Subscription {
        id,
        customer_id,
        status,
        plan_id,
        current_period_start: period_start,
        current_period_end: period_end,
        cancel_at_period_end,
    })
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
