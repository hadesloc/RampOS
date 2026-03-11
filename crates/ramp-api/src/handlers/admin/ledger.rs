use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use ramp_core::repository::ledger::{BalanceRow, LedgerEntryRow};
use serde::{Deserialize, Serialize};
use tracing::info;

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEntriesQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub user_id: Option<String>,
    pub account_type: Option<String>,
    pub direction: Option<String>,
    pub currency: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBalancesQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub user_id: Option<String>,
    pub account_type: Option<String>,
    pub currency: Option<String>,
}

fn default_limit() -> i64 {
    20
}

// ============================================================================
// Response DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntriesResponse {
    pub data: Vec<LedgerEntryRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerBalancesResponse {
    pub data: Vec<BalanceRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ============================================================================
// Handlers
// ============================================================================

// GET /v1/admin/ledger/entries
pub async fn list_entries(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    axum::extract::Query(params): axum::extract::Query<ListEntriesQuery>,
) -> Result<Json<LedgerEntriesResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, "Listing ledger entries with filters");

    let limit = params.limit.min(100);
    let offset = params.offset;

    // If we have a db_pool and filters, use direct SQL queries
    if let Some(pool) = &state.db_pool {
        if params.user_id.is_some()
            || params.account_type.is_some()
            || params.direction.is_some()
            || params.currency.is_some()
            || params.from.is_some()
            || params.to.is_some()
        {
            let (entries, total) =
                query_entries_filtered(pool, &tenant_ctx.tenant_id.0, &params, limit, offset)
                    .await?;

            return Ok(Json(LedgerEntriesResponse {
                data: entries,
                total,
                limit,
                offset,
            }));
        }
    }

    // Fallback: use service with basic pagination
    let entries = state
        .ledger_service
        .list_entries(&tenant_ctx.tenant_id, limit, offset)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let total = entries.len() as i64 + offset;
    Ok(Json(LedgerEntriesResponse {
        data: entries,
        total,
        limit,
        offset,
    }))
}

// GET /v1/admin/ledger/balances
pub async fn list_balances(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    axum::extract::Query(params): axum::extract::Query<ListBalancesQuery>,
) -> Result<Json<LedgerBalancesResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, "Listing ledger balances with filters");

    let limit = params.limit.min(100);
    let offset = params.offset;

    // If we have a db_pool and filters, use direct SQL queries
    if let Some(pool) = &state.db_pool {
        if params.user_id.is_some() || params.account_type.is_some() || params.currency.is_some() {
            let (balances, total) =
                query_balances_filtered(pool, &tenant_ctx.tenant_id.0, &params, limit, offset)
                    .await?;

            return Ok(Json(LedgerBalancesResponse {
                data: balances,
                total,
                limit,
                offset,
            }));
        }
    }

    // Fallback: use service with basic pagination
    let balances = state
        .ledger_service
        .list_balances(&tenant_ctx.tenant_id, limit, offset)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let total = balances.len() as i64 + offset;
    Ok(Json(LedgerBalancesResponse {
        data: balances,
        total,
        limit,
        offset,
    }))
}

// ============================================================================
// Database Query Helpers
// ============================================================================

async fn query_entries_filtered(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    params: &ListEntriesQuery,
    limit: i64,
    offset: i64,
) -> Result<(Vec<LedgerEntryRow>, i64), ApiError> {
    // Build dynamic WHERE clause
    let mut where_clauses = vec!["tenant_id = $1".to_string()];
    let mut param_idx = 2u32;

    if params.user_id.is_some() {
        where_clauses.push(format!("user_id = ${}", param_idx));
        param_idx += 1;
    }
    if params.account_type.is_some() {
        where_clauses.push(format!("account_type = ${}", param_idx));
        param_idx += 1;
    }
    if params.direction.is_some() {
        where_clauses.push(format!("direction = ${}", param_idx));
        param_idx += 1;
    }
    if params.currency.is_some() {
        where_clauses.push(format!("currency = ${}", param_idx));
        param_idx += 1;
    }
    if params.from.is_some() {
        where_clauses.push(format!("created_at >= ${}", param_idx));
        param_idx += 1;
    }
    if params.to.is_some() {
        where_clauses.push(format!("created_at <= ${}", param_idx));
        param_idx += 1;
    }

    let where_sql = where_clauses.join(" AND ");
    let count_sql = format!("SELECT COUNT(*) FROM ledger_entries WHERE {}", where_sql);
    let data_sql = format!(
        "SELECT * FROM ledger_entries WHERE {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        where_sql,
        param_idx,
        param_idx + 1
    );

    // Build count query
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql).bind(tenant_id);
    if let Some(ref user_id) = params.user_id {
        count_query = count_query.bind(user_id);
    }
    if let Some(ref account_type) = params.account_type {
        count_query = count_query.bind(account_type);
    }
    if let Some(ref direction) = params.direction {
        count_query = count_query.bind(direction.to_uppercase());
    }
    if let Some(ref currency) = params.currency {
        count_query = count_query.bind(currency);
    }
    if let Some(from) = params.from {
        count_query = count_query.bind(from);
    }
    if let Some(to) = params.to {
        count_query = count_query.bind(to);
    }

    let total = count_query
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to count entries: {}", e)))?;

    // Build data query
    let mut data_query = sqlx::query_as::<_, LedgerEntryRow>(&data_sql).bind(tenant_id);
    if let Some(ref user_id) = params.user_id {
        data_query = data_query.bind(user_id);
    }
    if let Some(ref account_type) = params.account_type {
        data_query = data_query.bind(account_type);
    }
    if let Some(ref direction) = params.direction {
        data_query = data_query.bind(direction.to_uppercase());
    }
    if let Some(ref currency) = params.currency {
        data_query = data_query.bind(currency);
    }
    if let Some(from) = params.from {
        data_query = data_query.bind(from);
    }
    if let Some(to) = params.to {
        data_query = data_query.bind(to);
    }
    data_query = data_query.bind(limit).bind(offset);

    let entries = data_query
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to query entries: {}", e)))?;

    Ok((entries, total))
}

async fn query_balances_filtered(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    params: &ListBalancesQuery,
    limit: i64,
    offset: i64,
) -> Result<(Vec<BalanceRow>, i64), ApiError> {
    // Build dynamic WHERE clause
    let mut where_clauses = vec!["tenant_id = $1".to_string()];
    let mut param_idx = 2u32;

    if params.user_id.is_some() {
        where_clauses.push(format!("user_id = ${}", param_idx));
        param_idx += 1;
    }
    if params.account_type.is_some() {
        where_clauses.push(format!("account_type = ${}", param_idx));
        param_idx += 1;
    }
    if params.currency.is_some() {
        where_clauses.push(format!("currency = ${}", param_idx));
        param_idx += 1;
    }

    let where_sql = where_clauses.join(" AND ");
    let count_sql = format!("SELECT COUNT(*) FROM account_balances WHERE {}", where_sql);
    let data_sql = format!(
        "SELECT account_type, currency, balance FROM account_balances WHERE {} ORDER BY balance DESC LIMIT ${} OFFSET ${}",
        where_sql, param_idx, param_idx + 1
    );

    // Build count query
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql).bind(tenant_id);
    if let Some(ref user_id) = params.user_id {
        count_query = count_query.bind(user_id);
    }
    if let Some(ref account_type) = params.account_type {
        count_query = count_query.bind(account_type);
    }
    if let Some(ref currency) = params.currency {
        count_query = count_query.bind(currency);
    }

    let total = count_query
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to count balances: {}", e)))?;

    // Build data query
    let mut data_query = sqlx::query_as::<_, BalanceRow>(&data_sql).bind(tenant_id);
    if let Some(ref user_id) = params.user_id {
        data_query = data_query.bind(user_id);
    }
    if let Some(ref account_type) = params.account_type {
        data_query = data_query.bind(account_type);
    }
    if let Some(ref currency) = params.currency {
        data_query = data_query.bind(currency);
    }
    data_query = data_query.bind(limit).bind(offset);

    let balances = data_query
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to query balances: {}", e)))?;

    Ok((balances, total))
}
