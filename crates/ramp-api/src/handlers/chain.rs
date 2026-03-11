//! Chain Abstraction Layer API Handlers
//!
//! Endpoints for multi-chain operations:
//! - List supported chains
//! - Get chain details/status
//! - Get cross-chain bridging quote
//! - Initiate bridge transaction

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::TenantContext;
use crate::router::AppState;
use ramp_core::chain::{ChainId, ChainInfo, ChainType};

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Query parameters for chain listing with optional filtering
#[derive(Debug, Clone, Deserialize, Default, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ChainListQuery {
    /// Filter by chain type (evm, solana, ton)
    pub chain_type: Option<String>,
    /// Filter by testnet status
    pub testnet: Option<bool>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChainListResponse {
    #[schema(value_type = Vec<Object>)]
    pub chains: Vec<ChainInfo>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChainDetailResponse {
    #[serde(flatten)]
    #[schema(value_type = Object)]
    pub info: ChainInfo,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BridgeQuoteRequest {
    pub source_chain_id: u64,
    pub destination_chain_id: u64,
    pub token_address: Option<String>,
    pub amount: String,
    pub sender_address: String,
    pub recipient_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BridgeQuoteResponse {
    pub source_chain_id: u64,
    pub destination_chain_id: u64,
    pub input_amount: String,
    pub output_amount: String,
    pub fee: String,
    pub fee_percentage: String,
    pub fee_breakdown: FeeBreakdown,
    pub estimated_time_seconds: u64,
    pub expires_at: String,
    pub quote_id: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeeBreakdown {
    pub bridge_fee: String,
    pub gas_fee: String,
    pub protocol_fee: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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

/// GET /v1/chains - List all supported chains (with optional filtering)
#[utoipa::path(
    get,
    path = "/v1/chains",
    tag = "chains",
    params(ChainListQuery),
    responses(
        (status = 200, description = "List of supported chains", body = ChainListResponse,
         example = json!({
             "chains": [
                 {"chainId": 1, "name": "Ethereum", "chainType": "Evm", "nativeSymbol": "ETH", "isTestnet": false, "explorerUrl": "https://etherscan.io"},
                 {"chainId": 42161, "name": "Arbitrum One", "chainType": "Evm", "nativeSymbol": "ETH", "isTestnet": false, "explorerUrl": "https://arbiscan.io"},
                 {"chainId": 56, "name": "BNB Smart Chain", "chainType": "Evm", "nativeSymbol": "BNB", "isTestnet": false, "explorerUrl": "https://bscscan.com"}
             ],
             "total": 3
         })),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_chains(
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
    Query(query): Query<ChainListQuery>,
) -> Result<Json<ChainListResponse>, ApiError> {
    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing supported chains"
    );

    let mut chains = ChainRegistryConfig::default_chain_infos();

    // Filter by chain type if specified
    if let Some(ref chain_type_str) = query.chain_type {
        let filter_type = match chain_type_str.to_lowercase().as_str() {
            "evm" => Some(ChainType::Evm),
            "solana" => Some(ChainType::Solana),
            "ton" => Some(ChainType::Ton),
            _ => {
                return Err(ApiError::BadRequest(format!(
                    "Invalid chain type '{}'. Must be one of: evm, solana, ton",
                    chain_type_str
                )));
            }
        };
        if let Some(ct) = filter_type {
            chains.retain(|c| c.chain_type == ct);
        }
    }

    // Filter by testnet status if specified
    if let Some(is_testnet) = query.testnet {
        chains.retain(|c| c.is_testnet == is_testnet);
    }

    let total = chains.len();

    Ok(Json(ChainListResponse { chains, total }))
}

/// GET /v1/chains/:chain_id - Get chain details/status
#[utoipa::path(
    get,
    path = "/v1/chains/{chain_id}",
    tag = "chains",
    params(
        ("chain_id" = u64, Path, description = "Chain ID")
    ),
    responses(
        (status = 200, description = "Chain details", body = ChainDetailResponse,
         example = json!({
             "chainId": 1, "name": "Ethereum", "chainType": "Evm",
             "nativeSymbol": "ETH", "isTestnet": false,
             "explorerUrl": "https://etherscan.io", "status": "active"
         })),
        (status = 404, description = "Chain not found", body = ErrorResponse,
         example = json!({"error": {"code": "NOT_FOUND", "message": "Chain 99999 not found"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
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

    let info = ChainRegistryConfig::get_chain_info(chain_id)
        .ok_or_else(|| ApiError::NotFound(format!("Chain {} not found", chain_id)))?;

    Ok(Json(ChainDetailResponse {
        info,
        status: "active".to_string(),
    }))
}

/// POST /v1/chains/:chain_id/quote - Get cross-chain bridging quote
#[utoipa::path(
    post,
    path = "/v1/chains/{chain_id}/quote",
    tag = "chains",
    params(
        ("chain_id" = u64, Path, description = "Source chain ID")
    ),
    request_body = BridgeQuoteRequest,
    responses(
        (status = 200, description = "Bridge quote", body = BridgeQuoteResponse,
         example = json!({
             "sourceChainId": 1,
             "destinationChainId": 42161,
             "inputAmount": "1000000000000000000",
             "outputAmount": "998950000000000000",
             "fee": "1050000000000000",
             "feePercentage": "0.15",
             "feeBreakdown": {"bridgeFee": "1000000000000000", "gasFee": "50000", "protocolFee": "500000000000"},
             "estimatedTimeSeconds": 300,
             "expiresAt": "2026-01-15T08:35:00Z",
             "quoteId": "quote_a1b2c3d4-e5f6-7890-abcd-ef1234567890"
         })),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Chain not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
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

    // Validate chains exist via registry
    let source_info =
        ChainRegistryConfig::get_chain_info(request.source_chain_id).ok_or_else(|| {
            ApiError::NotFound(format!(
                "Source chain {} not supported",
                request.source_chain_id
            ))
        })?;
    let dest_info =
        ChainRegistryConfig::get_chain_info(request.destination_chain_id).ok_or_else(|| {
            ApiError::NotFound(format!(
                "Destination chain {} not supported",
                request.destination_chain_id
            ))
        })?;

    if request.source_chain_id == request.destination_chain_id {
        return Err(ApiError::BadRequest(
            "Source and destination chains must be different".to_string(),
        ));
    }

    // Validate amount is a positive number
    let input_amount: f64 = request
        .amount
        .parse()
        .map_err(|_| ApiError::BadRequest("amount must be a valid number".to_string()))?;
    if input_amount <= 0.0 {
        return Err(ApiError::BadRequest(
            "amount must be greater than zero".to_string(),
        ));
    }

    // Validate sender address is not empty
    if request.sender_address.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "sender_address cannot be empty".to_string(),
        ));
    }

    // Calculate fees based on chain types
    let (bridge_fee_bps, gas_estimate, estimated_time) =
        ChainRegistryConfig::estimate_bridge_params(&source_info, &dest_info);

    let protocol_fee_bps: u64 = 5; // 0.05% protocol fee
    let total_fee_bps = bridge_fee_bps + protocol_fee_bps;

    let bridge_fee = input_amount * (bridge_fee_bps as f64 / 10000.0);
    let protocol_fee = input_amount * (protocol_fee_bps as f64 / 10000.0);
    let total_fee = bridge_fee + protocol_fee + gas_estimate;
    let output_amount = input_amount - total_fee;

    let fee_percentage = format!("{:.2}", total_fee_bps as f64 / 100.0);

    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::minutes(5);

    Ok(Json(BridgeQuoteResponse {
        source_chain_id: request.source_chain_id,
        destination_chain_id: request.destination_chain_id,
        input_amount: request.amount.clone(),
        output_amount: format!("{:.0}", output_amount.max(0.0)),
        fee: format!("{:.0}", total_fee),
        fee_percentage,
        fee_breakdown: FeeBreakdown {
            bridge_fee: format!("{:.0}", bridge_fee),
            gas_fee: format!("{:.0}", gas_estimate),
            protocol_fee: format!("{:.0}", protocol_fee),
        },
        estimated_time_seconds: estimated_time,
        expires_at: expires_at.to_rfc3339(),
        quote_id: format!("quote_{}", uuid::Uuid::new_v4()),
    }))
}

/// POST /v1/chains/bridge - Initiate bridge transaction
#[utoipa::path(
    post,
    path = "/v1/chains/bridge",
    tag = "chains",
    request_body = BridgeRequest,
    responses(
        (status = 200, description = "Bridge transaction initiated", body = BridgeResponse,
         example = json!({
             "bridgeId": "bridge_a1b2c3d4-e5f6-7890-abcd-ef1234567890",
             "status": "pending",
             "sourceChainId": 1,
             "destinationChainId": 42161,
             "amount": "1000000000000000000",
             "sourceTxHash": null,
             "destinationTxHash": null,
             "estimatedCompletion": "2026-01-15T08:40:00Z",
             "createdAt": "2026-01-15T08:30:00Z"
         })),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Chain not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
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

    // Validate quote_id format
    if request.quote_id.trim().is_empty() {
        return Err(ApiError::BadRequest("quote_id cannot be empty".to_string()));
    }
    if !request.quote_id.starts_with("quote_") {
        return Err(ApiError::BadRequest(
            "Invalid quote_id format. Must start with 'quote_'".to_string(),
        ));
    }

    // Validate chains exist via registry
    let source_info =
        ChainRegistryConfig::get_chain_info(request.source_chain_id).ok_or_else(|| {
            ApiError::NotFound(format!(
                "Source chain {} not supported",
                request.source_chain_id
            ))
        })?;
    let _dest_info =
        ChainRegistryConfig::get_chain_info(request.destination_chain_id).ok_or_else(|| {
            ApiError::NotFound(format!(
                "Destination chain {} not supported",
                request.destination_chain_id
            ))
        })?;

    if request.source_chain_id == request.destination_chain_id {
        return Err(ApiError::BadRequest(
            "Source and destination chains must be different".to_string(),
        ));
    }

    // Validate amount
    let amount: f64 = request
        .amount
        .parse()
        .map_err(|_| ApiError::BadRequest("amount must be a valid number".to_string()))?;
    if amount <= 0.0 {
        return Err(ApiError::BadRequest(
            "amount must be greater than zero".to_string(),
        ));
    }

    // Validate addresses are not empty
    if request.sender_address.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "sender_address cannot be empty".to_string(),
        ));
    }
    if request.recipient_address.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "recipient_address cannot be empty".to_string(),
        ));
    }

    // Estimate completion time based on chain types
    let estimated_minutes = match source_info.chain_type {
        ChainType::Evm => 10,
        ChainType::Solana => 5,
        ChainType::Ton => 8,
    };

    let now = chrono::Utc::now();
    let estimated_completion = now + chrono::Duration::minutes(estimated_minutes);

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
// Config-backed Chain Registry
// ============================================================================

/// Configuration-backed chain registry that provides chain metadata
/// without requiring live RPC connections. This replaces the old hardcoded
/// `get_default_chain_infos()` function with a centralized config source.
pub struct ChainRegistryConfig;

impl ChainRegistryConfig {
    /// Returns the full set of supported chain infos from configuration.
    pub fn default_chain_infos() -> Vec<ChainInfo> {
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
            // Testnets
            ChainInfo {
                chain_id: ChainId::SEPOLIA.0,
                name: "Sepolia".to_string(),
                chain_type: ChainType::Evm,
                native_symbol: "ETH".to_string(),
                is_testnet: true,
                explorer_url: "https://sepolia.etherscan.io".to_string(),
            },
            ChainInfo {
                chain_id: ChainId::SOLANA_DEVNET.0,
                name: "Solana Devnet".to_string(),
                chain_type: ChainType::Solana,
                native_symbol: "SOL".to_string(),
                is_testnet: true,
                explorer_url: "https://explorer.solana.com?cluster=devnet".to_string(),
            },
        ]
    }

    /// Look up a single chain by its numeric ID.
    pub fn get_chain_info(chain_id: u64) -> Option<ChainInfo> {
        Self::default_chain_infos()
            .into_iter()
            .find(|c| c.chain_id == chain_id)
    }

    /// Estimate bridge parameters based on source/destination chain types.
    /// Returns (bridge_fee_bps, gas_estimate, estimated_time_seconds).
    pub fn estimate_bridge_params(source: &ChainInfo, dest: &ChainInfo) -> (u64, f64, u64) {
        // Bridge fee in basis points varies by route
        let bridge_fee_bps = match (&source.chain_type, &dest.chain_type) {
            // EVM <-> EVM: cheapest, well-established bridges
            (ChainType::Evm, ChainType::Evm) => 10, // 0.10%
            // EVM <-> Solana: moderate, needs wormhole/allbridge
            (ChainType::Evm, ChainType::Solana) | (ChainType::Solana, ChainType::Evm) => 25, // 0.25%
            // EVM <-> TON: highest, less liquidity
            (ChainType::Evm, ChainType::Ton) | (ChainType::Ton, ChainType::Evm) => 30, // 0.30%
            // Solana <-> TON: cross-chain exotic
            (ChainType::Solana, ChainType::Ton) | (ChainType::Ton, ChainType::Solana) => 35, // 0.35%
            // Same type but different chains
            _ => 15, // 0.15%
        };

        // Estimated gas cost (in source chain's smallest unit)
        let gas_estimate = match source.chain_type {
            ChainType::Evm => 50000.0,    // ~50k gas units
            ChainType::Solana => 5000.0,  // ~5000 lamports
            ChainType::Ton => 10000000.0, // ~0.01 TON in nanotons
        };

        // Estimated time in seconds
        let estimated_time = match (&source.chain_type, &dest.chain_type) {
            (ChainType::Evm, ChainType::Evm) => 300, // 5 min
            (ChainType::Evm, ChainType::Solana) | (ChainType::Solana, ChainType::Evm) => 600, // 10 min
            (ChainType::Evm, ChainType::Ton) | (ChainType::Ton, ChainType::Evm) => 900, // 15 min
            (ChainType::Solana, ChainType::Solana) => 60,                               // 1 min
            (ChainType::Ton, ChainType::Ton) => 120,                                    // 2 min
            _ => 600, // 10 min default
        };

        (bridge_fee_bps, gas_estimate, estimated_time)
    }
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
    fn test_registry_config_default_chains() {
        let chains = ChainRegistryConfig::default_chain_infos();
        assert!(
            chains.len() >= 8,
            "Should have at least 8 chains (6 EVM + Solana + TON)"
        );

        // Verify Ethereum is included
        let eth = chains.iter().find(|c| c.chain_id == 1).unwrap();
        assert_eq!(eth.name, "Ethereum");
        assert_eq!(eth.native_symbol, "ETH");
        assert!(!eth.is_testnet);

        // Verify Solana is included
        let sol = chains
            .iter()
            .find(|c| c.chain_id == ChainId::SOLANA_MAINNET.0)
            .unwrap();
        assert_eq!(sol.name, "Solana");
        assert_eq!(sol.native_symbol, "SOL");

        // Verify TON is included
        let ton = chains
            .iter()
            .find(|c| c.chain_id == ChainId::TON_MAINNET.0)
            .unwrap();
        assert_eq!(ton.name, "TON");
    }

    #[test]
    fn test_registry_config_get_chain_info() {
        let eth = ChainRegistryConfig::get_chain_info(1);
        assert!(eth.is_some());
        assert_eq!(eth.unwrap().name, "Ethereum");

        let unknown = ChainRegistryConfig::get_chain_info(99999);
        assert!(unknown.is_none());
    }

    #[test]
    fn test_registry_config_testnets_included() {
        let chains = ChainRegistryConfig::default_chain_infos();
        let testnets: Vec<_> = chains.iter().filter(|c| c.is_testnet).collect();
        assert!(
            testnets.len() >= 2,
            "Should have at least 2 testnets (Sepolia + Solana Devnet)"
        );
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

    #[test]
    fn test_bridge_fee_estimation_evm_to_evm() {
        let source = ChainInfo {
            chain_id: 1,
            name: "Ethereum".to_string(),
            chain_type: ChainType::Evm,
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://etherscan.io".to_string(),
        };
        let dest = ChainInfo {
            chain_id: 42161,
            name: "Arbitrum One".to_string(),
            chain_type: ChainType::Evm,
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://arbiscan.io".to_string(),
        };
        let (fee_bps, gas, time) = ChainRegistryConfig::estimate_bridge_params(&source, &dest);
        assert_eq!(fee_bps, 10); // 0.10%
        assert!(gas > 0.0);
        assert_eq!(time, 300); // 5 minutes
    }

    #[test]
    fn test_bridge_fee_estimation_evm_to_solana() {
        let source = ChainInfo {
            chain_id: 1,
            name: "Ethereum".to_string(),
            chain_type: ChainType::Evm,
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "".to_string(),
        };
        let dest = ChainInfo {
            chain_id: ChainId::SOLANA_MAINNET.0,
            name: "Solana".to_string(),
            chain_type: ChainType::Solana,
            native_symbol: "SOL".to_string(),
            is_testnet: false,
            explorer_url: "".to_string(),
        };
        let (fee_bps, _, time) = ChainRegistryConfig::estimate_bridge_params(&source, &dest);
        assert_eq!(fee_bps, 25); // higher for cross-type
        assert_eq!(time, 600); // 10 minutes
    }

    #[test]
    fn test_fee_breakdown_serialization() {
        let fb = FeeBreakdown {
            bridge_fee: "100".to_string(),
            gas_fee: "50".to_string(),
            protocol_fee: "25".to_string(),
        };
        let json = serde_json::to_string(&fb).unwrap();
        assert!(json.contains("\"bridgeFee\":\"100\""));
        assert!(json.contains("\"gasFee\":\"50\""));
        assert!(json.contains("\"protocolFee\":\"25\""));
    }

    #[test]
    fn test_chain_list_query_deserialize() {
        let json = r#"{"chainType":"evm","testnet":false}"#;
        let query: ChainListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.chain_type.as_deref(), Some("evm"));
        assert_eq!(query.testnet, Some(false));

        // Empty query
        let empty: ChainListQuery = serde_json::from_str("{}").unwrap();
        assert!(empty.chain_type.is_none());
        assert!(empty.testnet.is_none());
    }
}
