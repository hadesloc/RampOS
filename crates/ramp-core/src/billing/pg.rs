use super::{
    BillingDataProvider, BillingEvent, BillingEventType, BillingPlan, Invoice, InvoiceItem,
    InvoiceStatus, PlanFeature, PlanLimits, Subscription, TenantBillingStatus, UsageSummary,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

/// PostgreSQL-backed billing data provider.
///
/// Uses existing schema only:
/// - `pricing_plans` for plans and overage rates.
/// - `daily_usage` / `usage_events` for current usage summaries.
/// - `invoices` for latest invoice details.
/// - `tenants.config->'billing_status'` for subscription/cycle fields not modeled by
///   dedicated billing tables yet.
/// - `tenants.config->'billing_events'` for billing event history because there is
///   no dedicated billing events table in the current migrations.
pub struct PgBillingDataProvider {
    pool: PgPool,
}

impl PgBillingDataProvider {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn with_pool(pool: PgPool) -> Self {
        Self::new(pool)
    }
}

#[derive(Debug, Clone)]
struct PricingPlanRow {
    id: String,
    name: String,
    description: Option<String>,
    currency: String,
    period: String,
    base_fee: Decimal,
    included_api_calls: i64,
    included_mau: i64,
    included_volume: Decimal,
    api_call_unit_price: Decimal,
    mau_unit_price: Decimal,
    volume_percentage_fee: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredBillingStatus {
    tenant_id: TenantId,
    plan_id: String,
    subscription: Option<Subscription>,
    billing_cycle_start: DateTime<Utc>,
    billing_cycle_end: DateTime<Utc>,
    is_overdue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredBillingEvent {
    id: String,
    tenant_id: TenantId,
    event_type: BillingEventType,
    details: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl From<BillingEvent> for StoredBillingEvent {
    fn from(event: BillingEvent) -> Self {
        Self {
            id: event.id,
            tenant_id: event.tenant_id,
            event_type: event.event_type,
            details: event.details,
            created_at: event.created_at,
        }
    }
}

impl From<StoredBillingEvent> for BillingEvent {
    fn from(event: StoredBillingEvent) -> Self {
        Self {
            id: event.id,
            tenant_id: event.tenant_id,
            event_type: event.event_type,
            details: event.details,
            created_at: event.created_at,
        }
    }
}

#[async_trait]
impl BillingDataProvider for PgBillingDataProvider {
    async fn get_tenant_billing(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Option<TenantBillingStatus>> {
        let config = self.get_tenant_config(tenant_id).await?;
        let Some(stored) = config
            .get("billing_status")
            .cloned()
            .map(serde_json::from_value::<StoredBillingStatus>)
            .transpose()
            .map_err(|e| Error::Serialization(e.to_string()))?
        else {
            return Ok(None);
        };

        let plan_row = self
            .get_pricing_plan_row(&stored.plan_id)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!("Billing plan not found: {}", stored.plan_id))
            })?;
        let plan = billing_plan_from_row(&plan_row);
        let current_usage = self
            .usage_summary(
                tenant_id,
                stored.billing_cycle_start,
                stored.billing_cycle_end,
            )
            .await?;
        let last_invoice = self.latest_invoice(tenant_id).await?;
        let overage_charges = calculate_overage(&plan_row, &current_usage);

        Ok(Some(TenantBillingStatus {
            tenant_id: tenant_id.clone(),
            plan,
            subscription: stored.subscription,
            current_usage,
            overage_charges,
            billing_cycle_start: stored.billing_cycle_start,
            billing_cycle_end: stored.billing_cycle_end,
            is_overdue: stored.is_overdue,
            last_invoice,
        }))
    }

    async fn store_tenant_billing(&self, status: &TenantBillingStatus) -> Result<()> {
        let mut config = self.get_tenant_config(&status.tenant_id).await?;
        let stored = StoredBillingStatus {
            tenant_id: status.tenant_id.clone(),
            plan_id: status.plan.id.clone(),
            subscription: status.subscription.clone(),
            billing_cycle_start: status.billing_cycle_start,
            billing_cycle_end: status.billing_cycle_end,
            is_overdue: status.is_overdue,
        };
        config["billing_status"] =
            serde_json::to_value(stored).map_err(|e| Error::Serialization(e.to_string()))?;
        self.update_tenant_config(&status.tenant_id, &config).await
    }

    async fn get_plan(&self, plan_id: &str) -> Result<Option<BillingPlan>> {
        Ok(self
            .get_pricing_plan_row(plan_id)
            .await?
            .map(|row| billing_plan_from_row(&row)))
    }

    async fn list_plans(&self) -> Result<Vec<BillingPlan>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, currency, period::text AS period, base_fee,
                   included_api_calls, included_mau, included_volume,
                   api_call_unit_price, mau_unit_price, volume_percentage_fee
            FROM pricing_plans
            ORDER BY base_fee ASC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(pricing_plan_row_from_pg_row)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|row| billing_plan_from_row(&row))
            .collect())
    }

    async fn record_event(&self, event: &BillingEvent) -> Result<()> {
        let mut config = self.get_tenant_config(&event.tenant_id).await?;
        let mut events = config
            .get("billing_events")
            .cloned()
            .map(serde_json::from_value::<Vec<StoredBillingEvent>>)
            .transpose()
            .map_err(|e| Error::Serialization(e.to_string()))?
            .unwrap_or_default();
        events.push(event.clone().into());
        config["billing_events"] =
            serde_json::to_value(events).map_err(|e| Error::Serialization(e.to_string()))?;
        self.update_tenant_config(&event.tenant_id, &config).await
    }

    async fn get_events(
        &self,
        tenant_id: &TenantId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<BillingEvent>> {
        let config = self.get_tenant_config(tenant_id).await?;
        let events = config
            .get("billing_events")
            .cloned()
            .map(serde_json::from_value::<Vec<StoredBillingEvent>>)
            .transpose()
            .map_err(|e| Error::Serialization(e.to_string()))?
            .unwrap_or_default();

        Ok(events
            .into_iter()
            .filter(|event| event.created_at >= from && event.created_at <= to)
            .map(BillingEvent::from)
            .collect())
    }
}

impl PgBillingDataProvider {
    async fn get_tenant_config(&self, tenant_id: &TenantId) -> Result<serde_json::Value> {
        let config =
            sqlx::query_scalar::<_, serde_json::Value>("SELECT config FROM tenants WHERE id = $1")
                .bind(&tenant_id.0)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| Error::TenantNotFound(tenant_id.0.clone()))?;

        Ok(config)
    }

    async fn update_tenant_config(
        &self,
        tenant_id: &TenantId,
        config: &serde_json::Value,
    ) -> Result<()> {
        sqlx::query("UPDATE tenants SET config = $1, updated_at = NOW() WHERE id = $2")
            .bind(config)
            .bind(&tenant_id.0)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_pricing_plan_row(&self, plan_id: &str) -> Result<Option<PricingPlanRow>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, currency, period::text AS period, base_fee,
                   included_api_calls, included_mau, included_volume,
                   api_call_unit_price, mau_unit_price, volume_percentage_fee
            FROM pricing_plans
            WHERE id = $1
            "#,
        )
        .bind(plan_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(pricing_plan_row_from_pg_row).transpose()
    }

    async fn usage_summary(
        &self,
        tenant_id: &TenantId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<UsageSummary> {
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(total_amount) FILTER (WHERE meter_slug = 'api_requests'), 0) AS api_calls,
                COALESCE(SUM(total_amount) FILTER (WHERE meter_slug IN ('tx_volume_usd', 'transaction_volume')), 0) AS transaction_volume,
                COALESCE(MAX(total_amount) FILTER (WHERE meter_slug = 'active_users'), 0) AS active_users
            FROM daily_usage
            WHERE tenant_id = $1
              AND date >= $2::date
              AND date <= $3::date
            "#,
        )
        .bind(&tenant_id.0)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await?;

        let api_calls: Decimal = row.try_get("api_calls")?;
        let transaction_volume: Decimal = row.try_get("transaction_volume")?;
        let active_users: Decimal = row.try_get("active_users")?;

        Ok(UsageSummary {
            api_calls: decimal_to_u64(api_calls, "api_calls")?,
            transaction_volume,
            active_users: decimal_to_u32(active_users, "active_users")?,
            // The current schema has no storage meter. Returning 0 here is a derivation from
            // absent rows for a metric the database cannot currently collect.
            storage_bytes: 0,
        })
    }

    async fn latest_invoice(&self, tenant_id: &TenantId) -> Result<Option<Invoice>> {
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, period_start, period_end, status::text AS status,
                   currency, subtotal, tax, total, line_items
            FROM invoices
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&tenant_id.0)
        .fetch_optional(&self.pool)
        .await?;

        row.map(invoice_from_row).transpose()
    }
}

fn pricing_plan_row_from_pg_row(row: sqlx::postgres::PgRow) -> Result<PricingPlanRow> {
    Ok(PricingPlanRow {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        currency: row.try_get("currency")?,
        period: row.try_get("period")?,
        base_fee: row.try_get("base_fee")?,
        included_api_calls: row.try_get("included_api_calls")?,
        included_mau: row.try_get("included_mau")?,
        included_volume: row.try_get("included_volume")?,
        api_call_unit_price: row.try_get("api_call_unit_price")?,
        mau_unit_price: row.try_get("mau_unit_price")?,
        volume_percentage_fee: row.try_get("volume_percentage_fee")?,
    })
}

fn billing_plan_from_row(row: &PricingPlanRow) -> BillingPlan {
    BillingPlan {
        id: row.id.clone(),
        name: row.name.clone(),
        description: row.description.clone().unwrap_or_default(),
        price: row.base_fee,
        currency: row.currency.to_lowercase(),
        interval: match row.period.as_str() {
            "yearly" => "year".to_string(),
            _ => "month".to_string(),
        },
        features: Vec::<PlanFeature>::new(),
        limits: PlanLimits {
            api_calls: row.included_api_calls.max(0) as u64,
            transaction_volume: row.included_volume,
            users: row.included_mau.max(0) as u32,
        },
    }
}

fn invoice_from_row(row: sqlx::postgres::PgRow) -> Result<Invoice> {
    let status: String = row.try_get("status")?;
    let subtotal: Decimal = row.try_get("subtotal")?;
    let tax: Decimal = row.try_get("tax")?;
    let total: Decimal = row.try_get("total")?;
    let line_items_value: serde_json::Value = row.try_get("line_items")?;
    let lines = parse_invoice_lines(line_items_value)?;

    Ok(Invoice {
        id: row.try_get("id")?,
        customer_id: row.try_get("tenant_id")?,
        status: parse_invoice_status(&status)?,
        currency: row.try_get::<String, _>("currency")?.to_lowercase(),
        amount_due: total,
        amount_paid: match parse_invoice_status(&status)? {
            InvoiceStatus::Paid => total,
            _ => Decimal::ZERO,
        },
        period_start: row.try_get("period_start")?,
        period_end: row.try_get("period_end")?,
        lines: if lines.is_empty() {
            vec![InvoiceItem {
                id: "subtotal".to_string(),
                amount: subtotal + tax,
                currency: row.try_get::<String, _>("currency")?.to_lowercase(),
                description: Some("Invoice subtotal plus tax".to_string()),
                quantity: None,
            }]
        } else {
            lines
        },
    })
}

fn parse_invoice_lines(value: serde_json::Value) -> Result<Vec<InvoiceItem>> {
    if value.is_null() {
        return Ok(Vec::new());
    }

    serde_json::from_value::<Vec<InvoiceItem>>(value)
        .map_err(|e| Error::Serialization(e.to_string()))
}

fn parse_invoice_status(status: &str) -> Result<InvoiceStatus> {
    match status {
        "draft" => Ok(InvoiceStatus::Draft),
        "open" => Ok(InvoiceStatus::Open),
        "paid" => Ok(InvoiceStatus::Paid),
        "void" => Ok(InvoiceStatus::Void),
        "uncollectible" => Ok(InvoiceStatus::Uncollectible),
        other => Err(Error::Validation(format!(
            "Unknown invoice status: {}",
            other
        ))),
    }
}

fn calculate_overage(plan: &PricingPlanRow, usage: &UsageSummary) -> Decimal {
    let api_overage = usage
        .api_calls
        .saturating_sub(plan.included_api_calls.max(0) as u64);
    let user_overage = usage
        .active_users
        .saturating_sub(plan.included_mau.max(0) as u32);
    let volume_overage = if usage.transaction_volume > plan.included_volume {
        usage.transaction_volume - plan.included_volume
    } else {
        Decimal::ZERO
    };

    Decimal::from(api_overage) * plan.api_call_unit_price
        + Decimal::from(user_overage) * plan.mau_unit_price
        + volume_overage * plan.volume_percentage_fee
}

fn decimal_to_u64(value: Decimal, metric: &str) -> Result<u64> {
    value
        .trunc()
        .to_string()
        .parse::<u64>()
        .map_err(|_| Error::Validation(format!("{} value is outside u64 range", metric)))
}

fn decimal_to_u32(value: Decimal, metric: &str) -> Result<u32> {
    let value = decimal_to_u64(value, metric)?;
    u32::try_from(value)
        .map_err(|_| Error::Validation(format!("{} value is outside u32 range", metric)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    async fn constructs_with_lazy_pool() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://user:password@localhost/rampos")
            .expect("lazy pool URL should parse");
        let _provider = PgBillingDataProvider::with_pool(pool);
    }

    #[test]
    fn maps_pricing_plan_row_without_fabricating_features() {
        let row = PricingPlanRow {
            id: "plan_growth".to_string(),
            name: "Growth".to_string(),
            description: Some("Scaling companies".to_string()),
            currency: "USD".to_string(),
            period: "monthly".to_string(),
            base_fee: Decimal::new(9900, 2),
            included_api_calls: 1_000_000,
            included_mau: 100,
            included_volume: Decimal::from(10_000_000),
            api_call_unit_price: Decimal::new(1, 4),
            mau_unit_price: Decimal::new(100, 2),
            volume_percentage_fee: Decimal::new(10, 4),
        };

        let plan = billing_plan_from_row(&row);

        assert_eq!(plan.id, "plan_growth");
        assert_eq!(plan.price, Decimal::new(9900, 2));
        assert_eq!(plan.currency, "usd");
        assert!(plan.features.is_empty());
        assert_eq!(plan.limits.api_calls, 1_000_000);
        assert_eq!(plan.limits.users, 100);
    }

    #[test]
    fn calculates_overage_from_pricing_plan_rates() {
        let row = PricingPlanRow {
            id: "plan".to_string(),
            name: "Plan".to_string(),
            description: None,
            currency: "USD".to_string(),
            period: "monthly".to_string(),
            base_fee: Decimal::ZERO,
            included_api_calls: 10,
            included_mau: 1,
            included_volume: Decimal::from(100),
            api_call_unit_price: Decimal::new(5, 1),
            mau_unit_price: Decimal::from(2),
            volume_percentage_fee: Decimal::new(10, 2),
        };
        let usage = UsageSummary {
            api_calls: 12,
            transaction_volume: Decimal::from(150),
            active_users: 3,
            storage_bytes: 0,
        };

        assert_eq!(calculate_overage(&row, &usage), Decimal::from(10));
    }
}
