use axum::{
    extract::Query,
    routing::{get, post},
    Extension,
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::{Mutex, OnceLock}};

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;

static SWAP_HISTORY: OnceLock<Mutex<HashMap<String, Vec<SwapTransactionResponse>>>> = OnceLock::new();

fn history_store() -> &'static Mutex<HashMap<String, Vec<SwapTransactionResponse>>> {
    SWAP_HISTORY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn supported_tokens() -> &'static [&'static str] {
    &["ETH", "USDC", "USDT", "WBTC"]
}

fn mock_rate(from_token: &str, to_token: &str) -> Option<f64> {
    match (from_token, to_token) {
        ("ETH", "USDC") | ("ETH", "USDT") => Some(3500.0),
        ("USDC", "ETH") | ("USDT", "ETH") => Some(1.0 / 3500.0),
        ("WBTC", "USDC") | ("WBTC", "USDT") => Some(65400.0),
        ("USDC", "WBTC") | ("USDT", "WBTC") => Some(1.0 / 65400.0),
        ("USDC", "USDT") | ("USDT", "USDC") => Some(1.0),
        _ if from_token == to_token => None,
        _ => Some(1.0),
    }
}

fn parse_amount(amount: &str) -> Result<f64, ApiError> {
    let parsed = amount
        .parse::<f64>()
        .map_err(|_| ApiError::Validation("amount must be a valid number".to_string()))?;
    if parsed <= 0.0 {
        return Err(ApiError::Validation("amount must be positive".to_string()));
    }
    Ok(parsed)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SwapQuoteQuery {
    pub from_token: String,
    pub to_token: String,
    pub amount: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapQuoteResponse {
    pub quote_id: String,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub to_amount: String,
    pub rate: String,
    pub price_impact: String,
    pub gas_cost: String,
    pub route: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteSwapRequest {
    pub quote_id: String,
    pub from_token: String,
    pub to_token: String,
    pub amount: String,
    pub slippage: f64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SwapTransactionResponse {
    pub tx_hash: String,
    pub status: String,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub to_amount: String,
    pub rate: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub data: Vec<SwapTransactionResponse>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

pub fn router() -> Router {
    Router::new()
        .route("/quote", get(get_quote))
        .route("/execute", post(execute_swap))
        .route("/history", get(get_history))
}

pub async fn get_quote(
    Extension(_tenant_ctx): Extension<TenantContext>,
    Query(query): Query<SwapQuoteQuery>,
) -> Result<Json<SwapQuoteResponse>, ApiError> {
    let from_token = query.from_token.to_uppercase();
    let to_token = query.to_token.to_uppercase();

    if !supported_tokens().contains(&from_token.as_str()) {
        return Err(ApiError::Validation(format!("unsupported from_token {}", from_token)));
    }
    if !supported_tokens().contains(&to_token.as_str()) {
        return Err(ApiError::Validation(format!("unsupported to_token {}", to_token)));
    }
    if from_token == to_token {
        return Err(ApiError::Validation(
            "from_token and to_token must differ".to_string(),
        ));
    }

    let amount = parse_amount(&query.amount)?;
    let rate = mock_rate(&from_token, &to_token)
        .ok_or_else(|| ApiError::Validation("unsupported swap route".to_string()))?;
    let to_amount = amount * rate;

    Ok(Json(SwapQuoteResponse {
        quote_id: format!("quote_{}", uuid::Uuid::now_v7()),
        from_token,
        to_token,
        from_amount: format!("{amount:.6}"),
        to_amount: format!("{to_amount:.6}"),
        rate: format!("{rate:.6}"),
        price_impact: "0.30".to_string(),
        gas_cost: "4.25".to_string(),
        route: "MockRouter".to_string(),
        expires_at: (Utc::now() + Duration::minutes(5)).to_rfc3339(),
    }))
}

pub async fn execute_swap(
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(request): Json<ExecuteSwapRequest>,
) -> Result<Json<SwapTransactionResponse>, ApiError> {
    if request.quote_id.trim().is_empty() {
        return Err(ApiError::Validation("quoteId is required".to_string()));
    }
    if request.slippage < 0.0 {
        return Err(ApiError::Validation("slippage must be non-negative".to_string()));
    }

    let from_token = request.from_token.to_uppercase();
    let to_token = request.to_token.to_uppercase();
    let amount = parse_amount(&request.amount)?;
    let rate = mock_rate(&from_token, &to_token)
        .ok_or_else(|| ApiError::Validation("unsupported swap route".to_string()))?;
    let to_amount = amount * rate;

    let tx = SwapTransactionResponse {
        tx_hash: format!("0x{}", uuid::Uuid::now_v7().simple()),
        status: "success".to_string(),
        from_token,
        to_token,
        from_amount: format!("{amount:.6}"),
        to_amount: format!("{to_amount:.6}"),
        rate: format!("{rate:.6}"),
        timestamp: Utc::now().to_rfc3339(),
    };

    let mut store = history_store()
        .lock()
        .map_err(|_| ApiError::Internal("swap history lock poisoned".to_string()))?;
    let tenant_history = store.entry(tenant_ctx.tenant_id.0.clone()).or_default();
    tenant_history.insert(0, tx.clone());
    if tenant_history.len() > 100 {
        tenant_history.truncate(100);
    }

    Ok(Json(tx))
}

pub async fn get_history(
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).max(1);
    let store = history_store()
        .lock()
        .map_err(|_| ApiError::Internal("swap history lock poisoned".to_string()))?;
    let tenant_history = store.get(&tenant_ctx.tenant_id.0).cloned().unwrap_or_default();
    let total = tenant_history.len();
    let start = (page - 1) * per_page;
    let data = tenant_history
        .iter()
        .skip(start)
        .take(per_page)
        .cloned()
        .collect::<Vec<_>>();

    Ok(Json(HistoryResponse {
        data,
        total,
        page,
        per_page,
        total_pages: total.div_ceil(per_page).max(1),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::tenant::{TenantContext, TenantTier};

    fn tenant_ctx() -> TenantContext {
        TenantContext {
            tenant_id: ramp_common::types::TenantId::new("tenant_swap"),
            name: "Tenant Swap".to_string(),
            tier: TenantTier::Standard,
        }
    }

    #[tokio::test]
    async fn quote_rejects_same_token_pair() {
        let result = get_quote(Extension(tenant_ctx()), Query(SwapQuoteQuery {
            from_token: "USDC".to_string(),
            to_token: "USDC".to_string(),
            amount: "10".to_string(),
        }))
        .await;

        assert!(matches!(result, Err(ApiError::Validation(_))));
    }

    #[tokio::test]
    async fn execute_swap_persists_history() {
        history_store().lock().unwrap().clear();

        let tx = execute_swap(Extension(tenant_ctx()), Json(ExecuteSwapRequest {
            quote_id: "quote_test".to_string(),
            from_token: "ETH".to_string(),
            to_token: "USDC".to_string(),
            amount: "1.5".to_string(),
            slippage: 0.5,
        }))
        .await
        .expect("swap should succeed")
        .0;

        assert_eq!(tx.status, "success");

        let history = get_history(Extension(tenant_ctx()), Query(HistoryQuery {
            page: Some(1),
            per_page: Some(10),
        }))
        .await
        .expect("history should load")
        .0;

        assert_eq!(history.total, 1);
        assert_eq!(history.data[0].tx_hash, tx.tx_hash);
    }
}
