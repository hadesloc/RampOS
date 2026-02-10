//! Chain Abstraction Layer API Handlers
//!
//! Endpoints for multi-chain operations:
//! - List supported chains
//! - Get chain details/status
//! - Get cross-chain bridging quote
//! - Initiate bridge transaction

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ApiError;
use crate::middleware::TenantContext;
use crate::router::AppState;
use ramp_core::chain::{ChainId, ChainInfo, ChainType};

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainListResponse {
    pub chains: Vec<ChainInfo>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainDetailResponse {
    #[serde(flatten)]
    pub info: ChainInfo,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeQuoteRequest {
    pub source_chain_id: u64,
    pub destination_chain_id: u64,
    pub token_address: Option<String>,
    pub amount: String,
    pub sender_address: String,
    pub recipient_address: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeQuoteResponse {
    pub source_chain_id: u64,
    pub destination_chain_id: u64,
    pub input_amount: String,
    pub output_amount: String,
    pub fee: String,
    pub fee_percentage: String,
    pub estimated_time_seconds: u64,
    pub expires_at: String,
    pub quote_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeRequest {
    pub quote_id: String,
    pub source_chain_id: u64,
    pub destination_chain_id: u64,
    pub token_address: Option<String>,
    pub amount: String,
    pub sender_address: String,
    pub recipient_address: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeResponse {
    pub bridge_id: String,
    pub status: String,
    pub source_chain_id: u64,
    pub destination_chain_id: u64,
    pub amount: String,
    pub source_tx_hash: Option<String>,
    pub destination_tx_hash: Option<String>,
    pub estimated_completion: String,
    pub created_at: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/chains - List all supported chains
pub async fn list_chains(
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
) -> Result<Json<ChainListResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing supported chains"
    );

    // Build chain info list from known chain IDs
    // In production, this would come from ChainAbstractionLayer service
    let chains = get_default_chain_infos();
    let total = chains.len();

    Ok(Json(ChainListResponse { chains, total }))
}

/// GET /v1/chains/:chain_id - Get chain details/status
pub async fn get_chain_detail(
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
    Path(chain_id): Path<u64>,
) -> Result<Json<ChainDetailResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        chain_id = chain_id,
        "Fetching chain details"
    );

    let chains = get_default_chain_infos();
    let info = chains
        .into_iter()
        .find(|c| c.chain_id == chain_id)
        .ok_or_else(|| ApiError::NotFound(format!("Chain {} not found", chain_id)))?;

    Ok(Json(ChainDetailResponse {
        info,
        status: "active".to_string(),
    }))
}

/// POST /v1/chains/:chain_id/quote - Get cross-chain bridging quote
pub async fn get_bridge_quote(
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
    Path(chain_id): Path<u64>,
    Json(request): Json<BridgeQuoteRequest>,
) -> Result<Json<BridgeQuoteResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        source_chain = request.source_chain_id,
        dest_chain = request.destination_chain_id,
        amount = %request.amount,
        "Getting bridge quote"
    );

    // Validate source chain matches path
    if request.source_chain_id != chain_id {
        return Err(ApiError::BadRequest(
            "source_chain_id must match the chain_id in the URL path".to_string(),
        ));
    }

    // Validate chains exist
    let chains = get_default_chain_infos();
    let source_exists = chains.iter().any(|c| c.chain_id == request.source_chain_id);
    let dest_exists = chains.iter().any(|c| c.chain_id == request.destination_chain_id);

    if !source_exists {
        return Err(ApiError::NotFound(format!(
            "Source chain {} not found",
            request.source_chain_id
        )));
    }
    if !dest_exists {
        return Err(ApiError::NotFound(format!(
            "Destination chain {} not found",
            request.destination_chain_id
        )));
    }

    if request.source_chain_id == request.destination_chain_id {
        return Err(ApiError::BadRequest(
            "Source and destination chains must be different".to_string(),
        ));
    }

    // Placeholder quote - real implementation would query bridge protocols
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::minutes(5);

    Ok(Json(BridgeQuoteResponse {
        source_chain_id: request.source_chain_id,
        destination_chain_id: request.destination_chain_id,
        input_amount: request.amount.clone(),
        output_amount: request.amount.clone(), // 1:1 for placeholder
        fee: "0.001".to_string(),
        fee_percentage: "0.1".to_string(),
        estimated_time_seconds: 300,
        expires_at: expires_at.to_rfc3339(),
        quote_id: format!("quote_{}", uuid::Uuid::new_v4()),
    }))
}

/// POST /v1/chains/bridge - Initiate bridge transaction
pub async fn initiate_bridge(
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
    Json(request): Json<BridgeRequest>,
) -> Result<Json<BridgeResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        source_chain = request.source_chain_id,
        dest_chain = request.destination_chain_id,
        amount = %request.amount,
        quote_id = %request.quote_id,
        "Initiating bridge transaction"
    );

    // Validate chains exist
    let chains = get_default_chain_infos();
    let source_exists = chains.iter().any(|c| c.chain_id == request.source_chain_id);
    let dest_exists = chains.iter().any(|c| c.chain_id == request.destination_chain_id);

    if !source_exists {
        return Err(ApiError::NotFound(format!(
            "Source chain {} not found",
            request.source_chain_id
        )));
    }
    if !dest_exists {
        return Err(ApiError::NotFound(format!(
            "Destination chain {} not found",
            request.destination_chain_id
        )));
    }

    // Placeholder response - real implementation would submit to bridge protocol
    let now = chrono::Utc::now();
    let estimated_completion = now + chrono::Duration::minutes(10);

    Ok(Json(BridgeResponse {
        bridge_id: format!("bridge_{}", uuid::Uuid::new_v4()),
        status: "pending".to_string(),
        source_chain_id: request.source_chain_id,
        destination_chain_id: request.destination_chain_id,
        amount: request.amount,
        source_tx_hash: None,
        destination_tx_hash: None,
        estimated_completion: estimated_completion.to_rfc3339(),
        created_at: now.to_rfc3339(),
    }))
}

// ============================================================================
// Helpers
// ============================================================================

/// Returns the default set of supported chain infos.
/// In production, this would be populated from ChainAbstractionLayer service.
fn get_default_chain_infos() -> Vec<ChainInfo> {
    vec![
        ChainInfo {
            chain_id: ChainId::ETHEREUM.0,
            name: "Ethereum".to_string(),
            chain_type: ChainType::Evm,
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://etherscan.io".to_string(),
        },
        ChainInfo {
            chain_id: ChainId::ARBITRUM.0,
            name: "Arbitrum One".to_string(),
            chain_type: ChainType::Evm,
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://arbiscan.io".to_string(),
        },
        ChainInfo {
            chain_id: ChainId::BASE.0,
            name: "Base".to_string(),
            chain_type: ChainType::Evm,
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://basescan.org".to_string(),
        },
        ChainInfo {
            chain_id: ChainId::OPTIMISM.0,
            name: "Optimism".to_string(),
            chain_type: ChainType::Evm,
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://optimistic.etherscan.io".to_string(),
        },
        ChainInfo {
            chain_id: ChainId::POLYGON.0,
            name: "Polygon".to_string(),
            chain_type: ChainType::Evm,
            native_symbol: "MATIC".to_string(),
            is_testnet: false,
            explorer_url: "https://polygonscan.com".to_string(),
        },
        ChainInfo {
            chain_id: ChainId::BSC.0,
            name: "BNB Smart Chain".to_string(),
            chain_type: ChainType::Evm,
            native_symbol: "BNB".to_string(),
            is_testnet: false,
            explorer_url: "https://bscscan.com".to_string(),
        },
        ChainInfo {
            chain_id: ChainId::SOLANA_MAINNET.0,
            name: "Solana".to_string(),
            chain_type: ChainType::Solana,
            native_symbol: "SOL".to_string(),
            is_testnet: false,
            explorer_url: "https://solscan.io".to_string(),
        },
        ChainInfo {
            chain_id: ChainId::TON_MAINNET.0,
            name: "TON".to_string(),
            chain_type: ChainType::Ton,
            native_symbol: "TON".to_string(),
            is_testnet: false,
            explorer_url: "https://tonscan.org".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_quote_request_deserialize() {
        let json = r#"{
            "sourceChainId": 1,
            "destinationChainId": 42161,
            "amount": "1000000000000000000",
            "senderAddress": "0x1234567890123456789012345678901234567890"
        }"#;
        let request: BridgeQuoteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.source_chain_id, 1);
        assert_eq!(request.destination_chain_id, 42161);
        assert_eq!(request.amount, "1000000000000000000");
        assert!(request.token_address.is_none());
        assert!(request.recipient_address.is_none());
    }

    #[test]
    fn test_bridge_request_deserialize() {
        let json = r#"{
            "quoteId": "quote_123",
            "sourceChainId": 1,
            "destinationChainId": 42161,
            "amount": "1000000000000000000",
            "senderAddress": "0x1234567890123456789012345678901234567890",
            "recipientAddress": "0x0987654321098765432109876543210987654321"
        }"#;
        let request: BridgeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.quote_id, "quote_123");
        assert_eq!(request.source_chain_id, 1);
        assert_eq!(request.destination_chain_id, 42161);
    }

    #[test]
    fn test_default_chain_infos() {
        let chains = get_default_chain_infos();
        assert!(chains.len() >= 7);

        // Verify Ethereum is included
        let eth = chains.iter().find(|c| c.chain_id == 1).unwrap();
        assert_eq!(eth.name, "Ethereum");
        assert_eq!(eth.native_symbol, "ETH");
        assert!(!eth.is_testnet);

        // Verify Solana is included
        let sol = chains.iter().find(|c| c.chain_id == ChainId::SOLANA_MAINNET.0).unwrap();
        assert_eq!(sol.name, "Solana");
        assert_eq!(sol.native_symbol, "SOL");
    }

    #[test]
    fn test_chain_list_response_serialize() {
        let response = ChainListResponse {
            chains: vec![ChainInfo {
                chain_id: 1,
                name: "Ethereum".to_string(),
                chain_type: ChainType::Evm,
                native_symbol: "ETH".to_string(),
                is_testnet: false,
                explorer_url: "https://etherscan.io".to_string(),
            }],
            total: 1,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"chainId\":1"));
        assert!(json.contains("\"total\":1"));
    }
}
