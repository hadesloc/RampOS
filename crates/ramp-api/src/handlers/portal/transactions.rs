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
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

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
    State(_app_state): State<AppState>,
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

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Query transactions with filters
    // 3. Apply pagination
    // 4. Return results

    let now = Utc::now();

    // Mock transactions
    let transactions = vec![
        Transaction {
            id: Uuid::new_v4().to_string(),
            tx_type: "DEPOSIT".to_string(),
            status: "COMPLETED".to_string(),
            amount: "10000000".to_string(),
            currency: "VND".to_string(),
            fee: Some("0".to_string()),
            reference: "DEP123456".to_string(),
            details: Some("Bank transfer deposit".to_string()),
            tx_hash: None,
            created_at: (now - chrono::Duration::hours(2)).to_rfc3339(),
            updated_at: (now - chrono::Duration::hours(1)).to_rfc3339(),
        },
        Transaction {
            id: Uuid::new_v4().to_string(),
            tx_type: "TRADE".to_string(),
            status: "COMPLETED".to_string(),
            amount: "0.001".to_string(),
            currency: "BTC".to_string(),
            fee: Some("0.00001".to_string()),
            reference: "TRD789012".to_string(),
            details: Some("Buy BTC/VND".to_string()),
            tx_hash: None,
            created_at: (now - chrono::Duration::hours(1)).to_rfc3339(),
            updated_at: (now - chrono::Duration::minutes(30)).to_rfc3339(),
        },
        Transaction {
            id: Uuid::new_v4().to_string(),
            tx_type: "WITHDRAW".to_string(),
            status: "PENDING".to_string(),
            amount: "5000000".to_string(),
            currency: "VND".to_string(),
            fee: Some("22000".to_string()),
            reference: "WTH345678".to_string(),
            details: Some("Bank withdrawal".to_string()),
            tx_hash: None,
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
        },
    ];

    // Apply mock filtering
    let filtered: Vec<Transaction> = transactions
        .into_iter()
        .filter(|tx| {
            if let Some(ref t) = filters.tx_type {
                if &tx.tx_type != t {
                    return false;
                }
            }
            if let Some(ref s) = filters.status {
                if &tx.status != s {
                    return false;
                }
            }
            true
        })
        .collect();

    let total = filtered.len() as i64;
    let total_pages = ((total as f64) / (filters.per_page as f64)).ceil() as i32;

    // Apply pagination
    let start = ((filters.page - 1) * filters.per_page) as usize;
    let end = (start + filters.per_page as usize).min(filtered.len());
    let data = if start < filtered.len() {
        filtered[start..end].to_vec()
    } else {
        vec![]
    };

    let response = PaginatedResponse {
        data,
        total,
        page: filters.page,
        per_page: filters.per_page,
        total_pages,
    };

    Ok(Json(response))
}

/// GET /v1/portal/transactions/:id - Get transaction by ID
pub async fn get_transaction(
    State(_app_state): State<AppState>,
    Path(tx_id): Path<String>,
) -> Result<Json<Transaction>, ApiError> {
    info!(tx_id = %tx_id, "Get transaction requested");

    // Validate ID format
    if tx_id.is_empty() {
        return Err(ApiError::BadRequest(
            "Transaction ID is required".to_string(),
        ));
    }

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Query transaction by ID
    // 3. Verify transaction belongs to user
    // 4. Return transaction or 404

    // For now, return mock transaction
    let now = Utc::now();

    let transaction = Transaction {
        id: tx_id,
        tx_type: "DEPOSIT".to_string(),
        status: "COMPLETED".to_string(),
        amount: "10000000".to_string(),
        currency: "VND".to_string(),
        fee: Some("0".to_string()),
        reference: "DEP123456".to_string(),
        details: Some("Bank transfer deposit from Vietcombank".to_string()),
        tx_hash: None,
        created_at: (now - chrono::Duration::hours(2)).to_rfc3339(),
        updated_at: (now - chrono::Duration::hours(1)).to_rfc3339(),
    };

    Ok(Json(transaction))
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
