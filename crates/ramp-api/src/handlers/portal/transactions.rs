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
use ramp_common::types::{IntentId, TenantId, UserId};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::ApiError;
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

    // TODO: Extract user from PortalUser middleware extractor
    // For now, use placeholder values - in production this comes from JWT claims
    let tenant_id = TenantId::new(&get_default_tenant_id());
    let user_id = UserId::new("placeholder-user-id"); // TODO: Get from auth middleware

    // Calculate offset for pagination
    let offset = ((filters.page - 1) * filters.per_page) as i64;
    let limit = filters.per_page as i64;

    // Query real transactions from intent repository
    let intent_rows = app_state
        .intent_repo
        .list_by_user(&tenant_id, &user_id, limit, offset)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get user transactions from intent repo");
            ApiError::Internal("Failed to retrieve transactions".to_string())
        })?;

    // Convert IntentRow to Transaction DTO
    let transactions: Vec<Transaction> = intent_rows
        .into_iter()
        .filter(|row| {
            // Apply type filter if provided
            if let Some(ref tx_type) = filters.tx_type {
                let row_type = match row.intent_type.as_str() {
                    "PAY_IN" => "DEPOSIT",
                    "PAY_OUT" => "WITHDRAW",
                    _ => &row.intent_type,
                };
                if row_type != tx_type {
                    return false;
                }
            }
            // Apply status filter if provided
            if let Some(ref status) = filters.status {
                let row_status = map_intent_state_to_status(&row.state);
                if row_status != *status {
                    return false;
                }
            }
            true
        })
        .map(|row| {
            let tx_type = match row.intent_type.as_str() {
                "PAY_IN" => "DEPOSIT".to_string(),
                "PAY_OUT" => "WITHDRAW".to_string(),
                _ => row.intent_type.clone(),
            };
            let status = map_intent_state_to_status(&row.state);

            Transaction {
                id: row.id.clone(),
                tx_type,
                status,
                amount: row.amount.to_string(),
                currency: row.currency.clone(),
                fee: None, // TODO: Calculate fee from metadata
                reference: row
                    .reference_code
                    .unwrap_or_else(|| format!("REF{}", &row.id[..8])),
                details: row
                    .metadata
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                tx_hash: row.tx_hash,
                created_at: row.created_at.to_rfc3339(),
                updated_at: row.updated_at.to_rfc3339(),
            }
        })
        .collect();

    // TODO: Get total count from repository for proper pagination
    // For now, estimate based on returned results
    let total = transactions.len() as i64;
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

/// Map intent state to transaction status
fn map_intent_state_to_status(state: &str) -> String {
    match state {
        "CREATED" | "PENDING" | "AWAITING_DEPOSIT" => "PENDING".to_string(),
        "PROCESSING" | "CONFIRMING" => "PROCESSING".to_string(),
        "COMPLETED" | "SETTLED" => "COMPLETED".to_string(),
        "FAILED" | "REJECTED" => "FAILED".to_string(),
        "CANCELLED" | "EXPIRED" => "CANCELLED".to_string(),
        _ => "PENDING".to_string(),
    }
}

/// Get the default tenant ID from environment
fn get_default_tenant_id() -> String {
    std::env::var("DEFAULT_TENANT_ID")
        .unwrap_or_else(|_| "00000000-0000-0000-0000-000000000001".to_string())
}

/// GET /v1/portal/transactions/:id - Get transaction by ID
pub async fn get_transaction(
    State(app_state): State<AppState>,
    Path(tx_id): Path<String>,
) -> Result<Json<Transaction>, ApiError> {
    info!(tx_id = %tx_id, "Get transaction requested");

    // Validate ID format
    if tx_id.is_empty() {
        return Err(ApiError::BadRequest(
            "Transaction ID is required".to_string(),
        ));
    }

    // TODO: Extract user from PortalUser middleware extractor
    // For now, use placeholder values - in production this comes from JWT claims
    let tenant_id = TenantId::new(&get_default_tenant_id());
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
            // TODO: Verify intent belongs to user (from PortalUser extractor)

            let tx_type = match row.intent_type.as_str() {
                "PAY_IN" => "DEPOSIT".to_string(),
                "PAY_OUT" => "WITHDRAW".to_string(),
                _ => row.intent_type.clone(),
            };
            let status = map_intent_state_to_status(&row.state);

            let transaction = Transaction {
                id: row.id.clone(),
                tx_type,
                status,
                amount: row.amount.to_string(),
                currency: row.currency.clone(),
                fee: None, // TODO: Calculate fee from metadata
                reference: row
                    .reference_code
                    .unwrap_or_else(|| format!("REF{}", &row.id[..8])),
                details: row
                    .metadata
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                tx_hash: row.tx_hash,
                created_at: row.created_at.to_rfc3339(),
                updated_at: row.updated_at.to_rfc3339(),
            };

            Ok(Json(transaction))
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

        let json = serde_json::to_string(&tx).unwrap();
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

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":100"));
        assert!(json.contains("\"totalPages\":5"));
    }
}
