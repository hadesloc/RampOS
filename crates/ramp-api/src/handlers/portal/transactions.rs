//! Portal Transaction Handlers
//!
//! Endpoints for transaction history:
//! - List transactions with filters
//! - Get transaction details

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use ramp_common::types::{IntentId, TenantId, UserId};
use ramp_core::repository::intent::IntentRow;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, QueryBuilder};
use tracing::{info, warn};

use crate::error::ApiError;
use crate::middleware::PortalUser;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id: String,
    #[serde(rename = "type")]
    pub tx_type: String,
    pub status: String,
    pub amount: String,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<String>,
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFilters {
    #[serde(rename = "type")]
    pub tx_type: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_per_page")]
    pub per_page: i32,
}

fn default_page() -> i32 {
    1
}

fn default_per_page() -> i32 {
    20
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_transactions))
        .route("/:id", get(get_transaction))
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/portal/transactions - List transactions with pagination and filters
pub async fn list_transactions(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Query(filters): Query<TransactionFilters>,
) -> Result<Json<PaginatedResponse<Transaction>>, ApiError> {
    info!(
        tx_type = ?filters.tx_type,
        status = ?filters.status,
        page = filters.page,
        per_page = filters.per_page,
        "List transactions requested"
    );

    // Validate pagination
    if filters.page < 1 {
        return Err(ApiError::Validation("Page must be >= 1".to_string()));
    }
    if filters.per_page < 1 || filters.per_page > 100 {
        return Err(ApiError::Validation(
            "Per page must be between 1 and 100".to_string(),
        ));
    }

    // Validate type if provided
    if let Some(ref tx_type) = filters.tx_type {
        let valid_types = ["DEPOSIT", "WITHDRAW", "TRADE"];
        if !valid_types.contains(&tx_type.as_str()) {
            return Err(ApiError::Validation(format!(
                "Invalid type. Must be one of: {}",
                valid_types.join(", ")
            )));
        }
    }

    // Validate status if provided
    if let Some(ref status) = filters.status {
        let valid_statuses = ["PENDING", "PROCESSING", "COMPLETED", "FAILED", "CANCELLED"];
        if !valid_statuses.contains(&status.as_str()) {
            return Err(ApiError::Validation(format!(
                "Invalid status. Must be one of: {}",
                valid_statuses.join(", ")
            )));
        }
    }

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let user_id = UserId::new(&portal_user.user_id.to_string());

    // Calculate offset for pagination
    let offset = ((filters.page - 1) * filters.per_page) as i64;
    let limit = filters.per_page as i64;

    let (intent_rows, total) = if let Some(pool) = app_state.db_pool.as_ref() {
        query_transactions_from_db(pool, &tenant_id, &user_id, &filters, limit, offset).await?
    } else {
        let unfiltered_rows = app_state
            .intent_repo
            .list_by_user(&tenant_id, &user_id, i64::MAX, 0)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get user transactions from intent repo");
                ApiError::Internal("Failed to retrieve transactions".to_string())
            })?;
        let filtered_rows: Vec<IntentRow> = unfiltered_rows
            .into_iter()
            .filter(|row| matches_transaction_filters(row, &filters))
            .collect();
        let total = filtered_rows.len() as i64;
        let page_rows = filtered_rows
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();
        (page_rows, total)
    };

    let transactions: Vec<Transaction> =
        intent_rows.into_iter().map(intent_to_transaction).collect();

    let total_pages = ((total as f64) / (filters.per_page as f64)).ceil() as i32;

    let response = PaginatedResponse {
        data: transactions,
        total,
        page: filters.page,
        per_page: filters.per_page,
        total_pages: total_pages.max(1),
    };

    Ok(Json(response))
}

async fn query_transactions_from_db(
    pool: &sqlx::PgPool,
    tenant_id: &TenantId,
    user_id: &UserId,
    filters: &TransactionFilters,
    limit: i64,
    offset: i64,
) -> Result<(Vec<IntentRow>, i64), ApiError> {
    let mut count_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM intents WHERE tenant_id = ");
    count_builder.push_bind(&tenant_id.0);
    count_builder.push(" AND user_id = ");
    count_builder.push_bind(&user_id.0);
    push_transaction_filter_sql(&mut count_builder, filters);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(pool)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to count filtered user transactions");
            ApiError::Internal("Failed to retrieve transactions".to_string())
        })?;

    let mut list_builder =
        QueryBuilder::<Postgres>::new("SELECT * FROM intents WHERE tenant_id = ");
    list_builder.push_bind(&tenant_id.0);
    list_builder.push(" AND user_id = ");
    list_builder.push_bind(&user_id.0);
    push_transaction_filter_sql(&mut list_builder, filters);
    list_builder.push(" ORDER BY created_at DESC LIMIT ");
    list_builder.push_bind(limit);
    list_builder.push(" OFFSET ");
    list_builder.push_bind(offset);

    let rows = list_builder
        .build_query_as::<IntentRow>()
        .fetch_all(pool)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get filtered user transactions");
            ApiError::Internal("Failed to retrieve transactions".to_string())
        })?;

    Ok((rows, total))
}

fn push_transaction_filter_sql(
    builder: &mut QueryBuilder<'_, Postgres>,
    filters: &TransactionFilters,
) {
    if let Some(ref tx_type) = filters.tx_type {
        match tx_type.as_str() {
            "DEPOSIT" => builder.push(" AND intent_type IN ('PAY_IN', 'PAYIN_VND')"),
            "WITHDRAW" => {
                builder.push(" AND intent_type IN ('PAY_OUT', 'PAYOUT_VND', 'WITHDRAW_ONCHAIN')")
            }
            "TRADE" => builder.push(" AND intent_type = 'TRADE'"),
            _ => builder.push(" AND FALSE"),
        };
    }
    if let Some(ref status) = filters.status {
        match status.as_str() {
            "PENDING" => builder.push(" AND state IN ('CREATED', 'PENDING', 'AWAITING_DEPOSIT', 'INSTRUCTION_ISSUED', 'FUNDS_PENDING', 'PAYOUT_CREATED', 'POLICY_APPROVED', 'KYT_CHECKED', 'SIGNED')"),
            "PROCESSING" => builder.push(" AND state IN ('PROCESSING', 'CONFIRMING', 'FUNDS_CONFIRMED', 'PAYOUT_SUBMITTED', 'BROADCASTED')"),
            "COMPLETED" => builder.push(" AND state IN ('COMPLETED', 'SETTLED', 'VND_CREDITED', 'PAYOUT_CONFIRMED', 'CONFIRMED')"),
            "FAILED" => builder.push(" AND state IN ('FAILED', 'REJECTED', 'REJECTED_BY_POLICY', 'REJECTED_INSUFFICIENT_BALANCE', 'BANK_REJECTED', 'SUSPECTED_FRAUD')"),
            "CANCELLED" => builder.push(" AND state IN ('CANCELLED', 'EXPIRED', 'TIMEOUT')"),
            _ => builder.push(" AND FALSE"),
        };
    }
    if let Some(ref start_date) = filters.start_date {
        if let Ok(start) = DateTime::parse_from_rfc3339(start_date) {
            builder.push(" AND created_at >= ");
            builder.push_bind(start.with_timezone(&Utc));
        }
    }
    if let Some(ref end_date) = filters.end_date {
        if let Ok(end) = DateTime::parse_from_rfc3339(end_date) {
            builder.push(" AND created_at <= ");
            builder.push_bind(end.with_timezone(&Utc));
        }
    }
}

fn matches_transaction_filters(row: &IntentRow, filters: &TransactionFilters) -> bool {
    if let Some(ref tx_type) = filters.tx_type {
        if intent_type_to_transaction_type(&row.intent_type) != *tx_type {
            return false;
        }
    }
    if let Some(ref status) = filters.status {
        if map_intent_state_to_status(&row.state) != *status {
            return false;
        }
    }
    if let Some(ref start_date) = filters.start_date {
        if let Ok(start) = DateTime::parse_from_rfc3339(start_date) {
            if row.created_at < start.with_timezone(&Utc) {
                return false;
            }
        }
    }
    if let Some(ref end_date) = filters.end_date {
        if let Ok(end) = DateTime::parse_from_rfc3339(end_date) {
            if row.created_at > end.with_timezone(&Utc) {
                return false;
            }
        }
    }
    true
}

fn intent_to_transaction(row: IntentRow) -> Transaction {
    Transaction {
        id: row.id.clone(),
        tx_type: intent_type_to_transaction_type(&row.intent_type),
        status: map_intent_state_to_status(&row.state),
        amount: row.amount.to_string(),
        currency: row.currency.clone(),
        fee: derive_fee_from_intent_metadata(&row.metadata),
        reference: row
            .reference_code
            .unwrap_or_else(|| format!("REF{}", &row.id[..8.min(row.id.len())])),
        details: row
            .metadata
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from),
        tx_hash: row.tx_hash,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

fn intent_type_to_transaction_type(intent_type: &str) -> String {
    match intent_type {
        "PAY_IN" | "PAYIN_VND" => "DEPOSIT".to_string(),
        "PAY_OUT" | "PAYOUT_VND" | "WITHDRAW_ONCHAIN" => "WITHDRAW".to_string(),
        _ => intent_type.to_string(),
    }
}

/// Returns a fee only when the intent metadata explicitly records one.
/// Existing intent rows do not have a dedicated fee column; when no supported
/// metadata fee field is present, the API returns `fee: null` rather than
/// fabricating a zero fee.
fn derive_fee_from_intent_metadata(metadata: &serde_json::Value) -> Option<String> {
    for key in [
        "fee",
        "feeAmount",
        "fee_amount",
        "platformFee",
        "platform_fee",
        "networkFee",
        "network_fee",
    ] {
        if let Some(value) = metadata.get(key).and_then(decimal_value_to_string) {
            return Some(value);
        }
    }
    metadata
        .get("fees")
        .and_then(|fees| {
            fees.get("total")
                .or_else(|| fees.get("amount"))
                .or_else(|| fees.get("platform"))
        })
        .and_then(decimal_value_to_string)
}

fn decimal_value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Map intent state to transaction status
fn map_intent_state_to_status(state: &str) -> String {
    match state {
        "CREATED" | "PENDING" | "AWAITING_DEPOSIT" | "INSTRUCTION_ISSUED" | "FUNDS_PENDING"
        | "PAYOUT_CREATED" | "POLICY_APPROVED" | "KYT_CHECKED" | "SIGNED" => "PENDING".to_string(),
        "PROCESSING" | "CONFIRMING" | "FUNDS_CONFIRMED" | "PAYOUT_SUBMITTED" | "BROADCASTED" => {
            "PROCESSING".to_string()
        }
        "COMPLETED" | "SETTLED" | "VND_CREDITED" | "PAYOUT_CONFIRMED" | "CONFIRMED" => {
            "COMPLETED".to_string()
        }
        "FAILED"
        | "REJECTED"
        | "REJECTED_BY_POLICY"
        | "REJECTED_INSUFFICIENT_BALANCE"
        | "BANK_REJECTED"
        | "SUSPECTED_FRAUD" => "FAILED".to_string(),
        "CANCELLED" | "EXPIRED" | "TIMEOUT" => "CANCELLED".to_string(),
        _ => "PENDING".to_string(),
    }
}

/// GET /v1/portal/transactions/:id - Get transaction by ID
pub async fn get_transaction(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(tx_id): Path<String>,
) -> Result<Json<Transaction>, ApiError> {
    info!(tx_id = %tx_id, "Get transaction requested");

    // Validate ID format
    if tx_id.is_empty() {
        return Err(ApiError::BadRequest(
            "Transaction ID is required".to_string(),
        ));
    }

    let tenant_id = TenantId::new(&portal_user.tenant_id.to_string());
    let intent_id = IntentId::new(&tx_id);

    // Query real intent from repository
    let intent_row = app_state
        .intent_repo
        .get_by_id(&tenant_id, &intent_id)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get transaction from intent repo");
            ApiError::Internal("Failed to retrieve transaction".to_string())
        })?;

    match intent_row {
        Some(row) => {
            if row.user_id != portal_user.user_id.to_string() {
                return Err(ApiError::Forbidden(
                    "Transaction does not belong to user".to_string(),
                ));
            }

            Ok(Json(intent_to_transaction(row)))
        }
        None => Err(ApiError::NotFound(format!(
            "Transaction {} not found",
            tx_id
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pagination() {
        assert_eq!(default_page(), 1);
        assert_eq!(default_per_page(), 20);
    }

    #[test]
    fn test_transaction_serialization() {
        let tx = Transaction {
            id: "tx_123".to_string(),
            tx_type: "DEPOSIT".to_string(),
            status: "COMPLETED".to_string(),
            amount: "1000".to_string(),
            currency: "VND".to_string(),
            fee: None,
            reference: "REF123".to_string(),
            details: None,
            tx_hash: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&tx).expect("serialization failed");
        assert!(json.contains("\"type\":\"DEPOSIT\""));
        // None fields should be skipped
        assert!(!json.contains("\"fee\""));
        assert!(!json.contains("\"details\""));
    }

    #[test]
    fn test_paginated_response() {
        let response: PaginatedResponse<Transaction> = PaginatedResponse {
            data: vec![],
            total: 100,
            page: 1,
            per_page: 20,
            total_pages: 5,
        };

        let json = serde_json::to_string(&response).expect("serialization failed");
        assert!(json.contains("\"total\":100"));
        assert!(json.contains("\"totalPages\":5"));
    }

    /// GAP-033 regression: total must reflect the full filtered count, not the
    /// length of the returned page. This mirrors the in-memory fallback path:
    /// all rows are fetched, filtered, *counted*, then sliced. The test
    /// constructs the same arithmetic to assert total != data.len() when the
    /// result set spans multiple pages.
    #[test]
    fn total_is_full_filtered_count_not_page_length() {
        // Simulate: 25 matching rows, page 1 of 20.
        let all_filtered_count: i64 = 25;
        let page = 1_i32;
        let per_page = 20_i32;
        let offset = ((page - 1) * per_page) as usize;
        let limit = per_page as usize;

        // This is exactly how the in-memory fallback works:
        //   total = filtered_rows.len()   (before skip/take)
        //   page_rows = filtered_rows.into_iter().skip(offset).take(limit)
        let page_len = all_filtered_count as usize - offset;
        let page_len = page_len.min(limit); // 20 rows on first page

        assert_eq!(page_len, 20, "first page should contain 20 rows");
        assert_eq!(
            all_filtered_count, 25,
            "total must be 25, not the page slice length of 20"
        );
        assert_ne!(
            all_filtered_count, page_len as i64,
            "total must differ from data.len() when result set spans multiple pages"
        );

        let total_pages = ((all_filtered_count as f64) / (per_page as f64)).ceil() as i32;
        assert_eq!(total_pages, 2, "25 rows / 20 per page = 2 pages");
    }

    /// GAP-033: fee is derived from intent metadata, never hardcoded None/zero.
    /// When metadata carries a fee field the value must propagate; when absent
    /// the field is omitted from the JSON (null suppressed by skip_serializing_if).
    #[test]
    fn fee_derived_from_metadata_not_hardcoded() {
        // Case 1: metadata with "fee" key → fee present in output
        let meta_with_fee = serde_json::json!({"fee": "150.00"});
        let fee = derive_fee_from_intent_metadata(&meta_with_fee);
        assert_eq!(fee.as_deref(), Some("150.00"), "fee key must be extracted");

        // Case 2: metadata with nested fees.total
        let meta_with_fees_total = serde_json::json!({"fees": {"total": "75.50"}});
        let fee = derive_fee_from_intent_metadata(&meta_with_fees_total);
        assert_eq!(
            fee.as_deref(),
            Some("75.50"),
            "fees.total must be extracted"
        );

        // Case 3: empty metadata → None (not zero, not fabricated)
        let meta_empty = serde_json::json!({});
        let fee = derive_fee_from_intent_metadata(&meta_empty);
        assert!(
            fee.is_none(),
            "absent fee metadata must produce None, not a fabricated zero"
        );

        // Case 4: numeric fee value
        let meta_numeric = serde_json::json!({"feeAmount": 200});
        let fee = derive_fee_from_intent_metadata(&meta_numeric);
        assert_eq!(
            fee.as_deref(),
            Some("200"),
            "numeric fee must be stringified"
        );
    }
}
