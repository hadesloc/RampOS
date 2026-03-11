//! Admin Bridge API Handlers
//!
//! Endpoints for cross-chain bridge operations:
//! - GET /v1/admin/bridge/routes - List supported routes
//! - GET /v1/admin/bridge/quote - Get bridge quote
//! - POST /v1/admin/bridge/transfer - Initiate bridge transfer
//! - GET /v1/admin/bridge/transfer/:txHash - Get transfer status
//! - GET /v1/admin/bridge/tokens - List supported tokens per chain

use alloy::primitives::{Address, U256};
use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_core::bridge::{
    BridgeConfig, BridgeQuote, BridgeRegistry, BridgeStatus, BridgeToken, SupportedChain,
};

/// State for bridge handlers
#[derive(Clone)]
pub struct BridgeState {
    pub registry: Arc<BridgeRegistry>,
}

impl BridgeState {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            registry: Arc::new(BridgeRegistry::new(config)),
        }
    }

    pub fn default() -> Self {
        Self {
            registry: Arc::new(BridgeRegistry::default()),
        }
    }
}

/// Supported chain info
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainInfo {
    pub chain_id: u64,
    pub name: String,
    pub tokens: Vec<TokenInfo>,
}

/// Token info on a chain
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
}

/// Bridge route info
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteInfo {
    pub from_chain_id: u64,
    pub from_chain_name: String,
    pub to_chain_id: u64,
    pub to_chain_name: String,
    pub token: String,
    pub bridges: Vec<String>,
}

/// Quote request parameters
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    pub from_chain_id: u64,
    pub to_chain_id: u64,
    pub token: String,
    pub amount: String,
    pub recipient: String,
}

/// Quote response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub quote_id: String,
    pub bridge_name: String,
    pub from_chain_id: u64,
    pub to_chain_id: u64,
    pub token: String,
    pub amount: String,
    pub amount_out: String,
    pub bridge_fee: String,
    pub gas_fee: String,
    pub total_fee: String,
    pub estimated_time_seconds: u64,
    pub expires_at: String,
}

impl From<BridgeQuote> for QuoteResponse {
    fn from(q: BridgeQuote) -> Self {
        let total_fee = q.total_fee().to_string();
        Self {
            quote_id: q.id,
            bridge_name: q.bridge_name,
            from_chain_id: q.from_chain,
            to_chain_id: q.to_chain,
            token: q.token.symbol().to_string(),
            amount: q.amount.to_string(),
            amount_out: q.amount_out.to_string(),
            bridge_fee: q.bridge_fee.to_string(),
            gas_fee: q.gas_fee.to_string(),
            total_fee,
            estimated_time_seconds: q.estimated_time_seconds,
            expires_at: q.expires_at.to_rfc3339(),
        }
    }
}

/// Transfer request
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRequest {
    pub quote_id: String,
    pub bridge_name: String,
    pub from_chain_id: u64,
    pub to_chain_id: u64,
    pub token: String,
    pub amount: String,
    pub recipient: String,
}

/// Transfer response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferResponse {
    pub tx_hash: String,
    pub status: String,
    pub bridge_name: String,
    pub from_chain_id: u64,
    pub to_chain_id: u64,
    pub estimated_time_seconds: u64,
}

/// Transfer status response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferStatusResponse {
    pub tx_hash: String,
    pub status: String,
    pub is_final: bool,
}

/// GET /v1/admin/bridge/chains
/// List supported chains with their tokens
pub async fn list_chains(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<BridgeState>,
) -> Result<Json<Vec<ChainInfo>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing supported bridge chains"
    );

    let mut chains = Vec::new();

    for chain in SupportedChain::all() {
        let mut tokens = Vec::new();

        for token in [BridgeToken::USDT, BridgeToken::USDC] {
            if let Some(address) = state.registry.get_token_address(chain.chain_id(), token) {
                tokens.push(TokenInfo {
                    symbol: token.symbol().to_string(),
                    address: format!("{:?}", address),
                    decimals: token.decimals(),
                });
            }
        }

        if !tokens.is_empty() {
            chains.push(ChainInfo {
                chain_id: chain.chain_id(),
                name: chain.name().to_string(),
                tokens,
            });
        }
    }

    Ok(Json(chains))
}

/// GET /v1/admin/bridge/routes
/// List supported bridge routes
pub async fn list_routes(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<BridgeState>,
) -> Result<Json<Vec<RouteInfo>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing supported bridge routes"
    );

    let routes = state.registry.get_supported_routes();
    let bridges = state.registry.all_bridges();

    let route_infos: Vec<RouteInfo> = routes
        .into_iter()
        .map(|(from, to, token)| {
            let available_bridges: Vec<String> = bridges
                .iter()
                .filter(|b| b.supports_route(from, to, token))
                .map(|b| b.name().to_string())
                .collect();

            RouteInfo {
                from_chain_id: from,
                from_chain_name: SupportedChain::from_chain_id(from)
                    .map(|c| c.name().to_string())
                    .unwrap_or_else(|| format!("Chain {}", from)),
                to_chain_id: to,
                to_chain_name: SupportedChain::from_chain_id(to)
                    .map(|c| c.name().to_string())
                    .unwrap_or_else(|| format!("Chain {}", to)),
                token: token.symbol().to_string(),
                bridges: available_bridges,
            }
        })
        .collect();

    Ok(Json(route_infos))
}

/// GET /v1/admin/bridge/quote
/// Get quote for a bridge transfer
pub async fn get_quote(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<BridgeState>,
    Query(request): Query<QuoteRequest>,
) -> Result<Json<Vec<QuoteResponse>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        from_chain = request.from_chain_id,
        to_chain = request.to_chain_id,
        token = %request.token,
        "Getting bridge quotes"
    );

    // Parse token
    let token = BridgeToken::from_symbol(&request.token)
        .ok_or_else(|| ApiError::BadRequest(format!("Unsupported token: {}", request.token)))?;

    // Get token address
    let token_address = state
        .registry
        .get_token_address(request.from_chain_id, token)
        .ok_or_else(|| {
            ApiError::BadRequest(format!(
                "Token {} not available on chain {}",
                request.token, request.from_chain_id
            ))
        })?;

    // Parse amount
    let amount = request
        .amount
        .parse::<U256>()
        .map_err(|_| ApiError::BadRequest("Invalid amount".to_string()))?;

    // Parse recipient
    let recipient: Address = request
        .recipient
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid recipient address".to_string()))?;

    // Get all quotes
    let quotes = state
        .registry
        .get_all_quotes(
            request.from_chain_id,
            request.to_chain_id,
            token_address,
            amount,
            recipient,
        )
        .await;

    if quotes.is_empty() {
        return Err(ApiError::BadRequest(
            "No bridge available for this route".to_string(),
        ));
    }

    let responses: Vec<QuoteResponse> = quotes.into_iter().map(QuoteResponse::from).collect();

    Ok(Json(responses))
}

/// POST /v1/admin/bridge/transfer
/// Initiate a bridge transfer
pub async fn initiate_transfer(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<BridgeState>,
    Json(request): Json<TransferRequest>,
) -> Result<Json<TransferResponse>, ApiError> {
    // Require operator role for transfers
    let auth = super::tier::check_admin_key_operator(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        admin_user = ?auth.user_id,
        bridge = %request.bridge_name,
        from_chain = request.from_chain_id,
        to_chain = request.to_chain_id,
        "Initiating bridge transfer"
    );

    // Get bridge
    let bridge = state
        .registry
        .get_bridge(&request.bridge_name)
        .ok_or_else(|| ApiError::BadRequest(format!("Unknown bridge: {}", request.bridge_name)))?;

    // Parse token
    let token = BridgeToken::from_symbol(&request.token)
        .ok_or_else(|| ApiError::BadRequest(format!("Unsupported token: {}", request.token)))?;

    // Get token address
    let token_address = state
        .registry
        .get_token_address(request.from_chain_id, token)
        .ok_or_else(|| {
            ApiError::BadRequest(format!(
                "Token {} not available on chain {}",
                request.token, request.from_chain_id
            ))
        })?;

    // Parse amount
    let amount = request
        .amount
        .parse::<U256>()
        .map_err(|_| ApiError::BadRequest("Invalid amount".to_string()))?;

    // Parse recipient
    let recipient: Address = request
        .recipient
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid recipient address".to_string()))?;

    // Get fresh quote
    let quote = bridge
        .quote(
            request.from_chain_id,
            request.to_chain_id,
            token_address,
            amount,
            recipient,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get quote: {}", e)))?;

    // Execute bridge transfer
    let tx_hash = bridge
        .bridge(quote.clone())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to execute bridge: {}", e)))?;

    Ok(Json(TransferResponse {
        tx_hash: format!("{:?}", tx_hash),
        status: "Pending".to_string(),
        bridge_name: request.bridge_name,
        from_chain_id: request.from_chain_id,
        to_chain_id: request.to_chain_id,
        estimated_time_seconds: quote.estimated_time_seconds,
    }))
}

/// GET /v1/admin/bridge/transfer/:txHash
/// Get transfer status
pub async fn get_transfer_status(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<BridgeState>,
    Path((bridge_name, tx_hash)): Path<(String, String)>,
) -> Result<Json<TransferStatusResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        bridge = %bridge_name,
        tx_hash = %tx_hash,
        "Getting bridge transfer status"
    );

    // Get bridge
    let bridge = state
        .registry
        .get_bridge(&bridge_name)
        .ok_or_else(|| ApiError::BadRequest(format!("Unknown bridge: {}", bridge_name)))?;

    // Parse tx hash
    let hash: alloy::primitives::B256 = tx_hash
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid transaction hash".to_string()))?;

    // Get status
    let status = bridge
        .status(hash)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get status: {}", e)))?;

    let status_str = match &status {
        BridgeStatus::Pending => "Pending",
        BridgeStatus::SourceConfirmed => "SourceConfirmed",
        BridgeStatus::InProgress => "InProgress",
        BridgeStatus::Completed => "Completed",
        BridgeStatus::Failed(msg) => msg.as_str(),
        BridgeStatus::Refunded => "Refunded",
    };

    Ok(Json(TransferStatusResponse {
        tx_hash,
        status: status_str.to_string(),
        is_final: status.is_final(),
    }))
}

/// GET /v1/admin/bridge/tokens
/// List supported tokens across all chains
pub async fn list_tokens(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(state): State<BridgeState>,
) -> Result<Json<Vec<TokenSummary>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing bridge tokens"
    );

    let tokens = vec![
        TokenSummary {
            symbol: "USDT".to_string(),
            name: "Tether USD".to_string(),
            decimals: 6,
            chains: get_token_chains(&state, BridgeToken::USDT),
        },
        TokenSummary {
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            decimals: 6,
            chains: get_token_chains(&state, BridgeToken::USDC),
        },
    ];

    Ok(Json(tokens))
}

/// Token summary across chains
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenSummary {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub chains: Vec<TokenChainInfo>,
}

/// Token info per chain
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenChainInfo {
    pub chain_id: u64,
    pub chain_name: String,
    pub address: String,
}

fn get_token_chains(state: &BridgeState, token: BridgeToken) -> Vec<TokenChainInfo> {
    SupportedChain::all()
        .into_iter()
        .filter_map(|chain| {
            state
                .registry
                .get_token_address(chain.chain_id(), token)
                .map(|addr| TokenChainInfo {
                    chain_id: chain.chain_id(),
                    chain_name: chain.name().to_string(),
                    address: format!("{:?}", addr),
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_response_from() {
        // Basic structure test
        let state = BridgeState::default();
        assert!(state.registry.get_bridge("Stargate").is_some());
    }
}
