use async_trait::async_trait;
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorridorPackRecord {
    pub corridor_pack_id: String,
    pub tenant_id: Option<String>,
    pub corridor_code: String,
    pub source_market: String,
    pub destination_market: String,
    pub source_currency: String,
    pub destination_currency: String,
    pub settlement_direction: String,
    pub fee_model: String,
    pub lifecycle_state: String,
    pub rollout_state: String,
    pub eligibility_state: String,
    pub metadata: serde_json::Value,
    pub endpoints: Vec<CorridorEndpointRecord>,
    pub fee_profiles: Vec<CorridorFeeProfileRecord>,
    pub cutoff_policies: Vec<CorridorCutoffPolicyRecord>,
    pub compliance_hooks: Vec<CorridorComplianceHookRecord>,
    pub rollout_scopes: Vec<CorridorRolloutScopeRecord>,
    pub eligibility_rules: Vec<CorridorEligibilityRuleRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorridorEndpointRecord {
    pub endpoint_id: String,
    pub endpoint_role: String,
    pub partner_id: Option<String>,
    pub provider_key: Option<String>,
    pub adapter_key: Option<String>,
    pub entity_type: String,
    pub rail: String,
    pub method_family: Option<String>,
    pub settlement_mode: Option<String>,
    pub instrument_family: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorridorFeeProfileRecord {
    pub fee_profile_id: String,
    pub fee_currency: String,
    pub base_fee: Option<String>,
    pub fx_spread_bps: Option<i32>,
    pub liquidity_cost_bps: Option<i32>,
    pub surcharge_bps: Option<i32>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorridorCutoffPolicyRecord {
    pub cutoff_policy_id: String,
    pub timezone: String,
    pub cutoff_windows: serde_json::Value,
    pub holiday_calendar: Option<String>,
    pub retry_rule: Option<String>,
    pub exception_policy: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorridorComplianceHookRecord {
    pub compliance_hook_id: String,
    pub hook_kind: String,
    pub provider_key: Option<String>,
    pub required: bool,
    pub config: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorridorRolloutScopeRecord {
    pub rollout_scope_id: String,
    pub tenant_id: Option<String>,
    pub environment: String,
    pub geography: Option<String>,
    pub method_family: Option<String>,
    pub rollout_state: String,
    pub approval_reference: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorridorEligibilityRuleRecord {
    pub eligibility_rule_id: String,
    pub partner_id: Option<String>,
    pub entity_type: Option<String>,
    pub method_family: Option<String>,
    pub amount_bounds: serde_json::Value,
    pub compliance_requirements: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCorridorPackRequest {
    pub corridor_pack_id: String,
    pub tenant_id: Option<String>,
    pub corridor_code: String,
    pub source_market: String,
    pub destination_market: String,
    pub source_currency: String,
    pub destination_currency: String,
    pub settlement_direction: String,
    pub fee_model: String,
    pub lifecycle_state: String,
    pub rollout_state: String,
    pub eligibility_state: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCorridorEndpointRequest {
    pub endpoint_id: String,
    pub corridor_pack_id: String,
    pub endpoint_role: String,
    pub partner_id: Option<String>,
    pub provider_key: Option<String>,
    pub adapter_key: Option<String>,
    pub entity_type: String,
    pub rail: String,
    pub method_family: Option<String>,
    pub settlement_mode: Option<String>,
    pub instrument_family: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCorridorFeeProfileRequest {
    pub fee_profile_id: String,
    pub corridor_pack_id: String,
    pub fee_currency: String,
    pub base_fee: Option<String>,
    pub fx_spread_bps: Option<i32>,
    pub liquidity_cost_bps: Option<i32>,
    pub surcharge_bps: Option<i32>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCorridorCutoffPolicyRequest {
    pub cutoff_policy_id: String,
    pub corridor_pack_id: String,
    pub timezone: String,
    pub cutoff_windows: serde_json::Value,
    pub holiday_calendar: Option<String>,
    pub retry_rule: Option<String>,
    pub exception_policy: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCorridorComplianceHookRequest {
    pub compliance_hook_id: String,
    pub corridor_pack_id: String,
    pub hook_kind: String,
    pub provider_key: Option<String>,
    pub required: bool,
    pub config: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCorridorRolloutScopeRequest {
    pub rollout_scope_id: String,
    pub corridor_pack_id: String,
    pub tenant_id: Option<String>,
    pub environment: String,
    pub geography: Option<String>,
    pub method_family: Option<String>,
    pub rollout_state: String,
    pub approval_reference: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCorridorEligibilityRuleRequest {
    pub eligibility_rule_id: String,
    pub corridor_pack_id: String,
    pub partner_id: Option<String>,
    pub entity_type: Option<String>,
    pub method_family: Option<String>,
    pub amount_bounds: serde_json::Value,
    pub compliance_requirements: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[async_trait]
pub trait CorridorPackRepository: Send + Sync {
    async fn upsert_corridor_pack(&self, request: &UpsertCorridorPackRequest) -> Result<()>;
    async fn upsert_endpoint(&self, request: &UpsertCorridorEndpointRequest) -> Result<()>;
    async fn upsert_fee_profile(&self, request: &UpsertCorridorFeeProfileRequest) -> Result<()>;
    async fn upsert_cutoff_policy(&self, request: &UpsertCorridorCutoffPolicyRequest) -> Result<()>;
    async fn upsert_compliance_hook(&self, request: &UpsertCorridorComplianceHookRequest) -> Result<()>;
    async fn upsert_rollout_scope(&self, request: &UpsertCorridorRolloutScopeRequest) -> Result<()>;
    async fn upsert_eligibility_rule(&self, request: &UpsertCorridorEligibilityRuleRequest) -> Result<()>;
    async fn list_corridor_packs(&self, tenant_id: Option<&str>) -> Result<Vec<CorridorPackRecord>>;
    async fn get_corridor_pack(&self, tenant_id: Option<&str>, corridor_code: &str) -> Result<Option<CorridorPackRecord>>;
}

pub struct PgCorridorPackRepository {
    pool: PgPool,
}

impl PgCorridorPackRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone, FromRow)]
struct CorridorPackRow {
    id: String,
    tenant_id: Option<String>,
    corridor_code: String,
    source_market: String,
    destination_market: String,
    source_currency: String,
    destination_currency: String,
    settlement_direction: String,
    fee_model: String,
    lifecycle_state: String,
    rollout_state: String,
    eligibility_state: String,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct EndpointRow {
    id: String,
    corridor_pack_id: String,
    endpoint_role: String,
    partner_id: Option<String>,
    provider_key: Option<String>,
    adapter_key: Option<String>,
    entity_type: String,
    rail: String,
    method_family: Option<String>,
    settlement_mode: Option<String>,
    instrument_family: Option<String>,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct FeeProfileRow {
    id: String,
    corridor_pack_id: String,
    fee_currency: String,
    base_fee: Option<rust_decimal::Decimal>,
    fx_spread_bps: Option<i32>,
    liquidity_cost_bps: Option<i32>,
    surcharge_bps: Option<i32>,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct CutoffPolicyRow {
    id: String,
    corridor_pack_id: String,
    timezone: String,
    cutoff_windows: serde_json::Value,
    holiday_calendar: Option<String>,
    retry_rule: Option<String>,
    exception_policy: Option<String>,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct ComplianceHookRow {
    id: String,
    corridor_pack_id: String,
    hook_kind: String,
    provider_key: Option<String>,
    required: bool,
    config: serde_json::Value,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct RolloutScopeRow {
    id: String,
    corridor_pack_id: String,
    tenant_id: Option<String>,
    environment: String,
    geography: Option<String>,
    method_family: Option<String>,
    rollout_state: String,
    approval_reference: Option<String>,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct EligibilityRuleRow {
    id: String,
    corridor_pack_id: String,
    partner_id: Option<String>,
    entity_type: Option<String>,
    method_family: Option<String>,
    amount_bounds: serde_json::Value,
    compliance_requirements: serde_json::Value,
    metadata: serde_json::Value,
}

#[async_trait]
impl CorridorPackRepository for PgCorridorPackRepository {
    async fn upsert_corridor_pack(&self, request: &UpsertCorridorPackRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO corridor_packs (
                id, tenant_id, corridor_code, source_market, destination_market,
                source_currency, destination_currency, settlement_direction, fee_model,
                lifecycle_state, rollout_state, eligibility_state, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                corridor_code = EXCLUDED.corridor_code,
                source_market = EXCLUDED.source_market,
                destination_market = EXCLUDED.destination_market,
                source_currency = EXCLUDED.source_currency,
                destination_currency = EXCLUDED.destination_currency,
                settlement_direction = EXCLUDED.settlement_direction,
                fee_model = EXCLUDED.fee_model,
                lifecycle_state = EXCLUDED.lifecycle_state,
                rollout_state = EXCLUDED.rollout_state,
                eligibility_state = EXCLUDED.eligibility_state,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.corridor_pack_id)
        .bind(&request.tenant_id)
        .bind(&request.corridor_code)
        .bind(&request.source_market)
        .bind(&request.destination_market)
        .bind(&request.source_currency)
        .bind(&request.destination_currency)
        .bind(&request.settlement_direction)
        .bind(&request.fee_model)
        .bind(&request.lifecycle_state)
        .bind(&request.rollout_state)
        .bind(&request.eligibility_state)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_endpoint(&self, request: &UpsertCorridorEndpointRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO corridor_pack_endpoints (
                id, corridor_pack_id, endpoint_role, partner_id, provider_key, adapter_key,
                entity_type, rail, method_family, settlement_mode, instrument_family, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            ON CONFLICT (id) DO UPDATE SET
                corridor_pack_id = EXCLUDED.corridor_pack_id,
                endpoint_role = EXCLUDED.endpoint_role,
                partner_id = EXCLUDED.partner_id,
                provider_key = EXCLUDED.provider_key,
                adapter_key = EXCLUDED.adapter_key,
                entity_type = EXCLUDED.entity_type,
                rail = EXCLUDED.rail,
                method_family = EXCLUDED.method_family,
                settlement_mode = EXCLUDED.settlement_mode,
                instrument_family = EXCLUDED.instrument_family,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.endpoint_id)
        .bind(&request.corridor_pack_id)
        .bind(&request.endpoint_role)
        .bind(&request.partner_id)
        .bind(&request.provider_key)
        .bind(&request.adapter_key)
        .bind(&request.entity_type)
        .bind(&request.rail)
        .bind(&request.method_family)
        .bind(&request.settlement_mode)
        .bind(&request.instrument_family)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_fee_profile(&self, request: &UpsertCorridorFeeProfileRequest) -> Result<()> {
        let base_fee = request.base_fee.as_deref().map(str::parse::<rust_decimal::Decimal>).transpose()
            .map_err(|e| ramp_common::Error::Validation(format!("Invalid base_fee: {e}")))?;
        sqlx::query(
            r#"
            INSERT INTO corridor_fee_profiles (
                id, corridor_pack_id, fee_currency, base_fee, fx_spread_bps, liquidity_cost_bps, surcharge_bps, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (id) DO UPDATE SET
                corridor_pack_id = EXCLUDED.corridor_pack_id,
                fee_currency = EXCLUDED.fee_currency,
                base_fee = EXCLUDED.base_fee,
                fx_spread_bps = EXCLUDED.fx_spread_bps,
                liquidity_cost_bps = EXCLUDED.liquidity_cost_bps,
                surcharge_bps = EXCLUDED.surcharge_bps,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.fee_profile_id)
        .bind(&request.corridor_pack_id)
        .bind(&request.fee_currency)
        .bind(base_fee)
        .bind(request.fx_spread_bps)
        .bind(request.liquidity_cost_bps)
        .bind(request.surcharge_bps)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_cutoff_policy(&self, request: &UpsertCorridorCutoffPolicyRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO corridor_cutoff_policies (
                id, corridor_pack_id, timezone, cutoff_windows, holiday_calendar, retry_rule, exception_policy, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (id) DO UPDATE SET
                corridor_pack_id = EXCLUDED.corridor_pack_id,
                timezone = EXCLUDED.timezone,
                cutoff_windows = EXCLUDED.cutoff_windows,
                holiday_calendar = EXCLUDED.holiday_calendar,
                retry_rule = EXCLUDED.retry_rule,
                exception_policy = EXCLUDED.exception_policy,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.cutoff_policy_id)
        .bind(&request.corridor_pack_id)
        .bind(&request.timezone)
        .bind(&request.cutoff_windows)
        .bind(&request.holiday_calendar)
        .bind(&request.retry_rule)
        .bind(&request.exception_policy)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_compliance_hook(&self, request: &UpsertCorridorComplianceHookRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO corridor_compliance_hooks (
                id, corridor_pack_id, hook_kind, provider_key, required, config, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            ON CONFLICT (id) DO UPDATE SET
                corridor_pack_id = EXCLUDED.corridor_pack_id,
                hook_kind = EXCLUDED.hook_kind,
                provider_key = EXCLUDED.provider_key,
                required = EXCLUDED.required,
                config = EXCLUDED.config,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.compliance_hook_id)
        .bind(&request.corridor_pack_id)
        .bind(&request.hook_kind)
        .bind(&request.provider_key)
        .bind(request.required)
        .bind(&request.config)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_rollout_scope(&self, request: &UpsertCorridorRolloutScopeRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO corridor_rollout_scopes (
                id, corridor_pack_id, tenant_id, environment, geography, method_family, rollout_state, approval_reference, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (id) DO UPDATE SET
                corridor_pack_id = EXCLUDED.corridor_pack_id,
                tenant_id = EXCLUDED.tenant_id,
                environment = EXCLUDED.environment,
                geography = EXCLUDED.geography,
                method_family = EXCLUDED.method_family,
                rollout_state = EXCLUDED.rollout_state,
                approval_reference = EXCLUDED.approval_reference,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.rollout_scope_id)
        .bind(&request.corridor_pack_id)
        .bind(&request.tenant_id)
        .bind(&request.environment)
        .bind(&request.geography)
        .bind(&request.method_family)
        .bind(&request.rollout_state)
        .bind(&request.approval_reference)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_eligibility_rule(&self, request: &UpsertCorridorEligibilityRuleRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO corridor_eligibility_rules (
                id, corridor_pack_id, partner_id, entity_type, method_family, amount_bounds, compliance_requirements, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (id) DO UPDATE SET
                corridor_pack_id = EXCLUDED.corridor_pack_id,
                partner_id = EXCLUDED.partner_id,
                entity_type = EXCLUDED.entity_type,
                method_family = EXCLUDED.method_family,
                amount_bounds = EXCLUDED.amount_bounds,
                compliance_requirements = EXCLUDED.compliance_requirements,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.eligibility_rule_id)
        .bind(&request.corridor_pack_id)
        .bind(&request.partner_id)
        .bind(&request.entity_type)
        .bind(&request.method_family)
        .bind(&request.amount_bounds)
        .bind(&request.compliance_requirements)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn list_corridor_packs(&self, tenant_id: Option<&str>) -> Result<Vec<CorridorPackRecord>> {
        let rows = if let Some(tenant_id) = tenant_id {
            sqlx::query_as::<_, CorridorPackRow>(
                r#"
                SELECT id, tenant_id, corridor_code, source_market, destination_market, source_currency,
                       destination_currency, settlement_direction, fee_model, lifecycle_state,
                       rollout_state, eligibility_state, metadata
                FROM corridor_packs
                WHERE tenant_id = $1 OR tenant_id IS NULL
                ORDER BY corridor_code ASC
                "#,
            )
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?
        } else {
            sqlx::query_as::<_, CorridorPackRow>(
                r#"
                SELECT id, tenant_id, corridor_code, source_market, destination_market, source_currency,
                       destination_currency, settlement_direction, fee_model, lifecycle_state,
                       rollout_state, eligibility_state, metadata
                FROM corridor_packs
                ORDER BY corridor_code ASC
                "#,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?
        };

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            records.push(load_corridor_pack(&self.pool, row).await?);
        }
        Ok(records)
    }

    async fn get_corridor_pack(&self, tenant_id: Option<&str>, corridor_code: &str) -> Result<Option<CorridorPackRecord>> {
        let row = if let Some(tenant_id) = tenant_id {
            sqlx::query_as::<_, CorridorPackRow>(
                r#"
                SELECT id, tenant_id, corridor_code, source_market, destination_market, source_currency,
                       destination_currency, settlement_direction, fee_model, lifecycle_state,
                       rollout_state, eligibility_state, metadata
                FROM corridor_packs
                WHERE corridor_code = $1 AND (tenant_id = $2 OR tenant_id IS NULL)
                ORDER BY tenant_id NULLS LAST
                LIMIT 1
                "#,
            )
            .bind(corridor_code)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?
        } else {
            sqlx::query_as::<_, CorridorPackRow>(
                r#"
                SELECT id, tenant_id, corridor_code, source_market, destination_market, source_currency,
                       destination_currency, settlement_direction, fee_model, lifecycle_state,
                       rollout_state, eligibility_state, metadata
                FROM corridor_packs
                WHERE corridor_code = $1
                LIMIT 1
                "#,
            )
            .bind(corridor_code)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?
        };

        match row {
            Some(row) => Ok(Some(load_corridor_pack(&self.pool, row).await?)),
            None => Ok(None),
        }
    }
}

async fn load_corridor_pack(pool: &PgPool, row: CorridorPackRow) -> Result<CorridorPackRecord> {
    let endpoints = sqlx::query_as::<_, EndpointRow>(
        r#"
        SELECT id, corridor_pack_id, endpoint_role, partner_id, provider_key, adapter_key,
               entity_type, rail, method_family, settlement_mode, instrument_family, metadata
        FROM corridor_pack_endpoints
        WHERE corridor_pack_id = $1
        ORDER BY endpoint_role ASC, id ASC
        "#,
    )
    .bind(&row.id)
    .fetch_all(pool)
    .await
    .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

    let fee_profiles = sqlx::query_as::<_, FeeProfileRow>(
        r#"
        SELECT id, corridor_pack_id, fee_currency, base_fee, fx_spread_bps, liquidity_cost_bps, surcharge_bps, metadata
        FROM corridor_fee_profiles
        WHERE corridor_pack_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(&row.id)
    .fetch_all(pool)
    .await
    .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

    let cutoff_policies = sqlx::query_as::<_, CutoffPolicyRow>(
        r#"
        SELECT id, corridor_pack_id, timezone, cutoff_windows, holiday_calendar, retry_rule, exception_policy, metadata
        FROM corridor_cutoff_policies
        WHERE corridor_pack_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(&row.id)
    .fetch_all(pool)
    .await
    .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

    let compliance_hooks = sqlx::query_as::<_, ComplianceHookRow>(
        r#"
        SELECT id, corridor_pack_id, hook_kind, provider_key, required, config, metadata
        FROM corridor_compliance_hooks
        WHERE corridor_pack_id = $1
        ORDER BY hook_kind ASC, id ASC
        "#,
    )
    .bind(&row.id)
    .fetch_all(pool)
    .await
    .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

    let rollout_scopes = sqlx::query_as::<_, RolloutScopeRow>(
        r#"
        SELECT id, corridor_pack_id, tenant_id, environment, geography, method_family, rollout_state, approval_reference, metadata
        FROM corridor_rollout_scopes
        WHERE corridor_pack_id = $1
        ORDER BY environment ASC, id ASC
        "#,
    )
    .bind(&row.id)
    .fetch_all(pool)
    .await
    .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

    let eligibility_rules = sqlx::query_as::<_, EligibilityRuleRow>(
        r#"
        SELECT id, corridor_pack_id, partner_id, entity_type, method_family, amount_bounds, compliance_requirements, metadata
        FROM corridor_eligibility_rules
        WHERE corridor_pack_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(&row.id)
    .fetch_all(pool)
    .await
    .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

    Ok(CorridorPackRecord {
        corridor_pack_id: row.id,
        tenant_id: row.tenant_id,
        corridor_code: row.corridor_code,
        source_market: row.source_market,
        destination_market: row.destination_market,
        source_currency: row.source_currency,
        destination_currency: row.destination_currency,
        settlement_direction: row.settlement_direction,
        fee_model: row.fee_model,
        lifecycle_state: row.lifecycle_state,
        rollout_state: row.rollout_state,
        eligibility_state: row.eligibility_state,
        metadata: row.metadata,
        endpoints: endpoints.into_iter().map(|item| CorridorEndpointRecord {
            endpoint_id: item.id,
            endpoint_role: item.endpoint_role,
            partner_id: item.partner_id,
            provider_key: item.provider_key,
            adapter_key: item.adapter_key,
            entity_type: item.entity_type,
            rail: item.rail,
            method_family: item.method_family,
            settlement_mode: item.settlement_mode,
            instrument_family: item.instrument_family,
            metadata: item.metadata,
        }).collect(),
        fee_profiles: fee_profiles.into_iter().map(|item| CorridorFeeProfileRecord {
            fee_profile_id: item.id,
            fee_currency: item.fee_currency,
            base_fee: item.base_fee.map(|v| v.to_string()),
            fx_spread_bps: item.fx_spread_bps,
            liquidity_cost_bps: item.liquidity_cost_bps,
            surcharge_bps: item.surcharge_bps,
            metadata: item.metadata,
        }).collect(),
        cutoff_policies: cutoff_policies.into_iter().map(|item| CorridorCutoffPolicyRecord {
            cutoff_policy_id: item.id,
            timezone: item.timezone,
            cutoff_windows: item.cutoff_windows,
            holiday_calendar: item.holiday_calendar,
            retry_rule: item.retry_rule,
            exception_policy: item.exception_policy,
            metadata: item.metadata,
        }).collect(),
        compliance_hooks: compliance_hooks.into_iter().map(|item| CorridorComplianceHookRecord {
            compliance_hook_id: item.id,
            hook_kind: item.hook_kind,
            provider_key: item.provider_key,
            required: item.required,
            config: item.config,
            metadata: item.metadata,
        }).collect(),
        rollout_scopes: rollout_scopes.into_iter().map(|item| CorridorRolloutScopeRecord {
            rollout_scope_id: item.id,
            tenant_id: item.tenant_id,
            environment: item.environment,
            geography: item.geography,
            method_family: item.method_family,
            rollout_state: item.rollout_state,
            approval_reference: item.approval_reference,
            metadata: item.metadata,
        }).collect(),
        eligibility_rules: eligibility_rules.into_iter().map(|item| CorridorEligibilityRuleRecord {
            eligibility_rule_id: item.id,
            partner_id: item.partner_id,
            entity_type: item.entity_type,
            method_family: item.method_family,
            amount_bounds: item.amount_bounds,
            compliance_requirements: item.compliance_requirements,
            metadata: item.metadata,
        }).collect(),
    })
}
