//! Portal Off-Ramp Handlers
//!
//! User-facing endpoints for off-ramp (crypto -> VND) operations:
//! - Get quote
//! - Create off-ramp intent
//! - Check status
//! - Confirm off-ramp

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use ramp_common::types::{ChainId, CryptoSymbol, TenantId, TxHash, WalletAddress};
use ramp_core::repository::{
    OfframpIntentRepository, OfframpIntentRow, PgOfframpIntentRepository,
    PgOnchainObservationRepository,
};
use ramp_core::service::exchange_rate::ExchangeRateService;
use ramp_core::service::offramp_fees::OffRampFeeCalculator;
use ramp_core::service::{
    ConfigBundleService, ConfirmOfframpObservationRequest, OfframpDepositAddressAllocator,
    OfframpDepositAddressRequest, OfframpObservationService, PartnerRegistryService,
    RecordOfframpObservationRequest,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use validator::Validate;

use crate::error::ApiError;
use crate::middleware::PortalUser;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OfframpQuoteRequest {
    #[validate(length(min = 1, message = "Crypto asset is required"))]
    pub crypto_asset: String,

    #[validate(length(min = 1, message = "Amount is required"))]
    pub amount: String,

    #[validate(length(min = 1, message = "Bank code is required"))]
    pub bank_code: String,

    #[validate(length(min = 1, message = "Account number is required"))]
    pub account_number: String,

    #[validate(length(min = 1, message = "Account name is required"))]
    pub account_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfframpQuoteResponse {
    pub quote_id: String,
    pub crypto_asset: String,
    pub crypto_amount: String,
    pub exchange_rate: String,
    pub gross_vnd_amount: String,
    pub net_vnd_amount: String,
    pub fee_total: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OfframpCreateRequest {
    #[validate(length(min = 1, message = "Quote ID is required"))]
    pub quote_id: String,

    #[validate(range(min = 1, message = "Chain ID must be positive"))]
    pub chain_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OfframpCryptoReceivedRequest {
    #[validate(length(min = 1, message = "Transaction hash is required"))]
    pub tx_hash: String,

    #[validate(range(min = 1, message = "Chain ID must be positive"))]
    pub chain_id: i64,

    #[validate(length(min = 1, message = "Sender address is required"))]
    pub from_address: String,

    #[validate(length(min = 1, message = "Recipient address is required"))]
    pub to_address: String,

    pub block_number: Option<i64>,
    pub confirmations: Option<i32>,
    pub raw_payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfframpIntentResponse {
    pub id: String,
    pub state: String,
    pub crypto_asset: String,
    pub crypto_amount: String,
    pub exchange_rate: String,
    pub net_vnd_amount: String,
    pub gross_vnd_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_reference: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/quote", post(create_quote))
        .route("/create", post(create_offramp))
        .route("/:id/status", get(get_offramp_status))
        .route("/:id/confirm", post(confirm_offramp))
        .route("/:id/crypto-received", post(mark_offramp_crypto_received))
}

// ============================================================================
// Internal helpers
// ============================================================================

fn parse_crypto_symbol(asset: &str) -> Result<CryptoSymbol, ApiError> {
    let upper = asset.to_uppercase();
    let symbol = match upper.as_str() {
        "USDT" => CryptoSymbol::USDT,
        "USDC" => CryptoSymbol::USDC,
        "ETH" => CryptoSymbol::ETH,
        "BNB" => CryptoSymbol::BNB,
        "MATIC" => CryptoSymbol::MATIC,
        "SOL" => CryptoSymbol::SOL,
        "BTC" => CryptoSymbol::BTC,
        _ => {
            return Err(ApiError::Validation(
                "Invalid crypto asset. Must be one of: USDT, USDC, ETH, BNB, MATIC, SOL, BTC"
                    .to_string(),
            ))
        }
    };
    Ok(symbol)
}

fn map_intent_response(intent: &OfframpIntentRow) -> OfframpIntentResponse {
    OfframpIntentResponse {
        id: intent.id.clone(),
        state: intent.state.clone(),
        crypto_asset: intent.crypto_asset.clone(),
        crypto_amount: intent.crypto_amount.to_string(),
        exchange_rate: intent.exchange_rate.to_string(),
        net_vnd_amount: intent.net_vnd_amount.to_string(),
        gross_vnd_amount: intent.gross_vnd_amount.to_string(),
        deposit_address: intent.deposit_address.clone(),
        chain_id: intent.chain_id,
        tx_hash: intent.tx_hash.clone(),
        bank_reference: intent.bank_reference.clone(),
        created_at: intent.created_at.to_rfc3339(),
        updated_at: intent.updated_at.to_rfc3339(),
    }
}

fn append_state_transition(
    mut history: serde_json::Value,
    from: &str,
    to: &str,
    reason: Option<&str>,
) -> serde_json::Value {
    let Some(arr) = history.as_array_mut() else {
        return json!([
            {
                "from": from,
                "to": to,
                "timestamp": Utc::now().to_rfc3339(),
                "reason": reason,
            }
        ]);
    };

    arr.push(json!({
        "from": from,
        "to": to,
        "timestamp": Utc::now().to_rfc3339(),
        "reason": reason,
    }));

    history
}

fn ensure_runtime_pool(state: &AppState) -> Result<&sqlx::PgPool, ApiError> {
    state.db_pool.as_ref().ok_or_else(|| {
        ApiError::Internal("Off-ramp runtime is unavailable: database not configured".to_string())
    })
}

async fn issue_offramp_deposit_address(
    pool: &sqlx::PgPool,
    request: &OfframpDepositAddressRequest,
) -> Result<String, ApiError> {
    if supports_governed_offramp_lookup(request.chain_id) {
        let registry_snapshot = PartnerRegistryService::with_pool(pool.clone())
            .list_approved_partners(Some(&request.tenant_id))
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;

        if let Some(address) = OfframpDepositAddressAllocator::configured_address_from_registry(
            request,
            &registry_snapshot,
        )? {
            return Ok(address);
        }
    }

    let bundle_service = ConfigBundleService::with_pool(pool.clone());

    if let Some(bundle) = bundle_service
        .get_strict_registry_bundle(&request.tenant_id)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?
    {
        if let Some(address) =
            OfframpDepositAddressAllocator::configured_address_from_bundle(request, &bundle)?
        {
            return Ok(address);
        }
    }

    OfframpDepositAddressAllocator::new()
        .issue(request)
        .map_err(ApiError::from)
}

fn supports_governed_offramp_lookup(chain_id: Option<i64>) -> bool {
    matches!(chain_id, Some(1 | 56 | 101 | 137 | 43114))
}

fn validate_chain_facts(req: &OfframpCryptoReceivedRequest) -> Result<(), ApiError> {
    if req.confirmations.unwrap_or(0) < 0 {
        return Err(ApiError::Validation(
            "Confirmations cannot be negative".to_string(),
        ));
    }

    if req.block_number.unwrap_or(0) < 0 {
        return Err(ApiError::Validation(
            "Block number cannot be negative".to_string(),
        ));
    }

    if req.confirmations.is_some() && req.block_number.is_none() {
        return Err(ApiError::Validation(
            "Block number is required when confirmations are provided".to_string(),
        ));
    }

    if req.block_number.is_some() && req.confirmations.is_none() {
        return Err(ApiError::Validation(
            "Confirmations are required when block number is provided".to_string(),
        ));
    }

    Ok(())
}

fn parse_observation_chain_id(chain_id: i64) -> Result<ChainId, ApiError> {
    match chain_id {
        1 => Ok(ChainId::Ethereum),
        137 => Ok(ChainId::Polygon),
        56 => Ok(ChainId::BnbChain),
        42161 => Ok(ChainId::Arbitrum),
        10 => Ok(ChainId::Optimism),
        8453 => Ok(ChainId::Base),
        43114 => Ok(ChainId::Avalanche),
        101 => Ok(ChainId::Solana),
        other => Err(ApiError::Validation(format!(
            "Unsupported chain ID '{}' for off-ramp observation",
            other
        ))),
    }
}

fn validate_chain_asset_scope(chain_id: Option<i64>, symbol: CryptoSymbol) -> Result<(), ApiError> {
    if chain_id.is_none()
        && matches!(
            symbol,
            CryptoSymbol::SOL | CryptoSymbol::BNB | CryptoSymbol::MATIC | CryptoSymbol::BTC
        )
    {
        return Err(ApiError::Validation(
            "chainId is required for this asset".to_string(),
        ));
    }

    if symbol == CryptoSymbol::SOL && chain_id != Some(101) {
        return Err(ApiError::Validation(
            "SOL off-ramp requires chainId=101".to_string(),
        ));
    }

    if chain_id == Some(43114) && !matches!(symbol, CryptoSymbol::USDT | CryptoSymbol::USDC) {
        return Err(ApiError::Validation(
            "Avalanche off-ramp currently supports only USDT/USDC in this monitor lane".to_string(),
        ));
    }
    Ok(())
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/portal/offramp/quote - Get quote for VND off-ramp
pub async fn create_quote(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<OfframpQuoteRequest>,
) -> Result<Json<OfframpQuoteResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let pool = ensure_runtime_pool(&app_state)?;

    let amount: Decimal = req
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("Amount must be a valid number".to_string()))?;

    if amount <= Decimal::ZERO {
        return Err(ApiError::Validation("Amount must be positive".to_string()));
    }

    let symbol = parse_crypto_symbol(&req.crypto_asset)?;
    let exchange_rate_service = ExchangeRateService::new();
    let fee_calculator = OffRampFeeCalculator::new();

    let rate = exchange_rate_service.get_rate(symbol, "VND")?.sell_price;
    let gross_vnd = amount * rate;
    let fees = fee_calculator.calculate_fees(gross_vnd, symbol, "domestic");
    let quote_id = format!("ofr_{}", uuid::Uuid::now_v7());
    let now = Utc::now();
    let expires_at = now + chrono::Duration::minutes(5);

    let repo = PgOfframpIntentRepository::new(pool.clone());
    let intent = OfframpIntentRow {
        id: quote_id.clone(),
        tenant_id: portal_user.tenant_id.to_string(),
        user_id: portal_user.user_id.to_string(),
        chain_id: None,
        crypto_asset: symbol.to_string(),
        crypto_amount: amount,
        exchange_rate: rate,
        locked_rate_id: None,
        fees: serde_json::to_value(&fees)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize fees: {}", e)))?,
        net_vnd_amount: fees.net_amount_vnd,
        gross_vnd_amount: fees.gross_amount_vnd,
        bank_account: json!({
            "bank_code": req.bank_code,
            "account_number": req.account_number,
            "account_name": req.account_name,
        }),
        deposit_address: None,
        tx_hash: None,
        bank_reference: None,
        state: "QUOTE_CREATED".to_string(),
        state_history: json!([
            {
                "from": "NONE",
                "to": "QUOTE_CREATED",
                "timestamp": now.to_rfc3339(),
                "reason": "Quote created",
            }
        ]),
        created_at: now,
        updated_at: now,
        quote_expires_at: expires_at,
    };

    repo.create_intent(&intent).await?;

    info!(
        user_id = %portal_user.user_id,
        quote_id = %quote_id,
        crypto_asset = %symbol,
        amount = %amount,
        "Off-ramp quote created"
    );

    Ok(Json(OfframpQuoteResponse {
        quote_id,
        crypto_asset: symbol.to_string(),
        crypto_amount: amount.to_string(),
        exchange_rate: rate.to_string(),
        gross_vnd_amount: fees.gross_amount_vnd.to_string(),
        net_vnd_amount: fees.net_amount_vnd.to_string(),
        fee_total: fees.total_fee.to_string(),
        expires_at: expires_at.to_rfc3339(),
    }))
}

/// POST /v1/portal/offramp/create - Create off-ramp intent from quote
pub async fn create_offramp(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<OfframpCreateRequest>,
) -> Result<Json<OfframpIntentResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let pool = ensure_runtime_pool(&app_state)?;
    let repo = PgOfframpIntentRepository::new(pool.clone());

    let tenant_id = TenantId(portal_user.tenant_id.to_string());
    let mut intent = repo
        .get_intent(&tenant_id, &req.quote_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Off-ramp quote not found".to_string()))?;

    if intent.user_id != portal_user.user_id.to_string() {
        return Err(ApiError::NotFound("Off-ramp quote not found".to_string()));
    }

    if Utc::now() >= intent.quote_expires_at {
        intent.state_history = append_state_transition(
            intent.state_history.clone(),
            &intent.state,
            "EXPIRED",
            Some("Quote expired before create"),
        );
        intent.state = "EXPIRED".to_string();
        intent.updated_at = Utc::now();
        repo.update_intent(&intent).await?;
        return Err(ApiError::Gone("Off-ramp quote has expired".to_string()));
    }

    if intent.state != "QUOTE_CREATED" {
        return Err(ApiError::Conflict(format!(
            "Off-ramp quote is not creatable from state {}",
            intent.state
        )));
    }

    let symbol = parse_crypto_symbol(&intent.crypto_asset)?;
    let locked_rate = ExchangeRateService::new().lock_rate(symbol, "VND", 60)?;
    if let Some(chain_id) = req.chain_id {
        intent.chain_id = Some(chain_id);
    }
    validate_chain_asset_scope(intent.chain_id, symbol)?;

    intent.locked_rate_id = Some(locked_rate.id);
    intent.deposit_address = Some(
        issue_offramp_deposit_address(
            pool,
            &OfframpDepositAddressRequest {
                tenant_id: intent.tenant_id.clone(),
                user_id: intent.user_id.clone(),
                chain_id: intent.chain_id,
            },
        )
        .await?,
    );
    intent.state_history = append_state_transition(
        intent.state_history.clone(),
        "QUOTE_CREATED",
        "CRYPTO_PENDING",
        Some("Quote confirmed and awaiting crypto deposit"),
    );
    intent.state = "CRYPTO_PENDING".to_string();
    intent.updated_at = Utc::now();

    repo.update_intent(&intent).await?;

    info!(
        user_id = %portal_user.user_id,
        intent_id = %intent.id,
        "Off-ramp intent moved to CRYPTO_PENDING"
    );

    Ok(Json(map_intent_response(&intent)))
}

/// GET /v1/portal/offramp/:id/status - Check off-ramp intent status
pub async fn get_offramp_status(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(id): Path<String>,
) -> Result<Json<OfframpIntentResponse>, ApiError> {
    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "Off-ramp intent ID is required".to_string(),
        ));
    }

    let pool = ensure_runtime_pool(&app_state)?;
    let repo = PgOfframpIntentRepository::new(pool.clone());
    let tenant_id = TenantId(portal_user.tenant_id.to_string());

    let intent = repo
        .get_intent(&tenant_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Off-ramp intent not found".to_string()))?;

    if intent.user_id != portal_user.user_id.to_string() {
        return Err(ApiError::NotFound("Off-ramp intent not found".to_string()));
    }

    info!(
        user_id = %portal_user.user_id,
        intent_id = %id,
        state = %intent.state,
        "Off-ramp status requested"
    );

    Ok(Json(map_intent_response(&intent)))
}

/// POST /v1/portal/offramp/:id/confirm - Confirm off-ramp (user confirms bank details)
pub async fn confirm_offramp(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(id): Path<String>,
) -> Result<Json<OfframpIntentResponse>, ApiError> {
    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "Off-ramp intent ID is required".to_string(),
        ));
    }

    let pool = ensure_runtime_pool(&app_state)?;
    let repo = PgOfframpIntentRepository::new(pool.clone());
    let tenant_id = TenantId(portal_user.tenant_id.to_string());

    let mut intent = repo
        .get_intent(&tenant_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Off-ramp intent not found".to_string()))?;

    if intent.user_id != portal_user.user_id.to_string() {
        return Err(ApiError::NotFound("Off-ramp intent not found".to_string()));
    }

    if intent.state == "QUOTE_CREATED" {
        let symbol = parse_crypto_symbol(&intent.crypto_asset)?;
        let locked_rate = ExchangeRateService::new().lock_rate(symbol, "VND", 60)?;
        validate_chain_asset_scope(intent.chain_id, symbol)?;
        intent.locked_rate_id = Some(locked_rate.id);
        if intent.deposit_address.is_none() {
            intent.deposit_address = Some(
                issue_offramp_deposit_address(
                    pool,
                    &OfframpDepositAddressRequest {
                        tenant_id: intent.tenant_id.clone(),
                        user_id: intent.user_id.clone(),
                        chain_id: intent.chain_id,
                    },
                )
                .await?,
            );
        }
        intent.state_history = append_state_transition(
            intent.state_history.clone(),
            "QUOTE_CREATED",
            "CRYPTO_PENDING",
            Some("User confirmed off-ramp details"),
        );
        intent.state = "CRYPTO_PENDING".to_string();
        intent.updated_at = Utc::now();
        repo.update_intent(&intent).await?;
    } else if intent.state != "CRYPTO_PENDING" {
        return Err(ApiError::Conflict(format!(
            "Off-ramp cannot be confirmed from state {}",
            intent.state
        )));
    }

    info!(
        user_id = %portal_user.user_id,
        intent_id = %id,
        state = %intent.state,
        "Off-ramp confirm requested"
    );

    Ok(Json(map_intent_response(&intent)))
}

/// POST /v1/portal/offramp/:id/crypto-received - Persist chain facts for a submitted off-ramp
pub async fn mark_offramp_crypto_received(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Path(id): Path<String>,
    Json(req): Json<OfframpCryptoReceivedRequest>,
) -> Result<Json<OfframpIntentResponse>, ApiError> {
    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "Off-ramp intent ID is required".to_string(),
        ));
    }

    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;
    validate_chain_facts(&req)?;

    let pool = ensure_runtime_pool(&app_state)?;
    let repo = PgOfframpIntentRepository::new(pool.clone());
    let tenant_id = TenantId(portal_user.tenant_id.to_string());

    let mut intent = repo
        .get_intent(&tenant_id, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Off-ramp intent not found".to_string()))?;

    if intent.user_id != portal_user.user_id.to_string() {
        return Err(ApiError::NotFound("Off-ramp intent not found".to_string()));
    }

    let intent_symbol = parse_crypto_symbol(&intent.crypto_asset)?;
    validate_chain_asset_scope(Some(req.chain_id), intent_symbol)?;
    let chain_id = parse_observation_chain_id(req.chain_id)?;
    let observation_service = OfframpObservationService::new(
        Arc::new(repo),
        Arc::new(PgOnchainObservationRepository::new(pool.clone())),
    );
    intent = observation_service
        .record_detected(RecordOfframpObservationRequest {
            tenant_id: tenant_id.clone(),
            offramp_intent_id: id.clone(),
            chain_id,
            tx_hash: TxHash::new(req.tx_hash.clone()),
            from_address: WalletAddress::new(req.from_address.clone()),
            to_address: WalletAddress::new(req.to_address.clone()),
            amount: intent.crypto_amount,
            symbol: intent_symbol,
            metadata: json!({
                "source_kind": "portal_runtime",
                "rawPayload": req.raw_payload,
            }),
        })
        .await
        .map_err(ApiError::from)?;

    if let (Some(confirmations), Some(block_number)) = (req.confirmations, req.block_number) {
        observation_service
            .confirm_detected(ConfirmOfframpObservationRequest {
                tenant_id,
                offramp_intent_id: id.clone(),
                chain_id,
                tx_hash: TxHash::new(req.tx_hash.clone()),
                confirmations: confirmations as u32,
                block_number: block_number as u64,
                metadata: json!({
                    "source_kind": "portal_runtime",
                }),
            })
            .await
            .map_err(ApiError::from)?;
    }

    info!(
        user_id = %portal_user.user_id,
        intent_id = %id,
        tx_hash = %req.tx_hash,
        chain_id = req.chain_id,
        state = %intent.state,
        "Off-ramp crypto receipt recorded"
    );

    Ok(Json(map_intent_response(&intent)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_request_validation() {
        let valid = OfframpQuoteRequest {
            crypto_asset: "USDT".to_string(),
            amount: "100".to_string(),
            bank_code: "VCB".to_string(),
            account_number: "1234567890".to_string(),
            account_name: "Nguyen Van A".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid = OfframpQuoteRequest {
            crypto_asset: "".to_string(),
            amount: "100".to_string(),
            bank_code: "VCB".to_string(),
            account_number: "123".to_string(),
            account_name: "Test".to_string(),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_request_validation() {
        let valid = OfframpCreateRequest {
            quote_id: "ofr_123".to_string(),
            chain_id: Some(137),
        };
        assert!(valid.validate().is_ok());

        let invalid = OfframpCreateRequest {
            quote_id: "".to_string(),
            chain_id: Some(0),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_parse_crypto_symbol_supports_supported_native_assets() {
        assert_eq!(parse_crypto_symbol("BNB").unwrap(), CryptoSymbol::BNB);
        assert_eq!(parse_crypto_symbol("bnb").unwrap(), CryptoSymbol::BNB);
        assert_eq!(parse_crypto_symbol("MATIC").unwrap(), CryptoSymbol::MATIC);
        assert_eq!(parse_crypto_symbol("matic").unwrap(), CryptoSymbol::MATIC);
        assert_eq!(parse_crypto_symbol("SOL").unwrap(), CryptoSymbol::SOL);
        assert_eq!(parse_crypto_symbol("sol").unwrap(), CryptoSymbol::SOL);
    }

    #[test]
    fn test_issue_portal_deposit_address_is_chain_aware() {
        let allocator = OfframpDepositAddressAllocator::new();
        let evm_address = allocator
            .issue(&OfframpDepositAddressRequest {
                tenant_id: "tenant".to_string(),
                user_id: "user".to_string(),
                chain_id: Some(137),
            })
            .unwrap();
        assert!(evm_address.starts_with("0x"));
        assert_eq!(evm_address.len(), 42);

        let default_address = allocator
            .issue(&OfframpDepositAddressRequest {
                tenant_id: "tenant".to_string(),
                user_id: "user".to_string(),
                chain_id: None,
            })
            .unwrap();
        assert!(default_address.starts_with("0x"));
        assert_eq!(default_address.len(), 42);

        let solana = allocator
            .issue(&OfframpDepositAddressRequest {
                tenant_id: "tenant".to_string(),
                user_id: "user".to_string(),
                chain_id: Some(101),
            })
            .unwrap_err();
        match solana {
            ramp_common::Error::Conflict(message) => {
                assert!(message.contains("Solana deposit address issuance is unavailable"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn test_crypto_received_request_validation() {
        let valid = OfframpCryptoReceivedRequest {
            tx_hash: "0xtx".to_string(),
            chain_id: 1,
            from_address: "0xfrom".to_string(),
            to_address: "0xto".to_string(),
            block_number: Some(123),
            confirmations: Some(3),
            raw_payload: Some(json!({"txHash":"0xtx"})),
        };
        assert!(valid.validate().is_ok());
        assert!(validate_chain_facts(&valid).is_ok());

        let invalid = OfframpCryptoReceivedRequest {
            tx_hash: "".to_string(),
            chain_id: 0,
            from_address: "".to_string(),
            to_address: "".to_string(),
            block_number: Some(-1),
            confirmations: Some(-1),
            raw_payload: None,
        };
        assert!(invalid.validate().is_err());
        assert!(validate_chain_facts(&invalid).is_err());
    }

    #[test]
    fn test_derive_observation_status() {
        assert_eq!(parse_observation_chain_id(1).unwrap(), ChainId::Ethereum);
        assert_eq!(parse_observation_chain_id(137).unwrap(), ChainId::Polygon);
        assert_eq!(
            parse_observation_chain_id(43114).unwrap(),
            ChainId::Avalanche
        );
        assert!(parse_observation_chain_id(999999).is_err());
    }

    #[test]
    fn test_validate_chain_asset_scope_allows_avalanche_stablecoins() {
        assert!(validate_chain_asset_scope(Some(43114), CryptoSymbol::USDT).is_ok());
        assert!(validate_chain_asset_scope(Some(43114), CryptoSymbol::USDC).is_ok());
    }

    #[test]
    fn test_validate_chain_asset_scope_allows_native_bnb_and_matic_with_chain_id() {
        assert!(validate_chain_asset_scope(Some(56), CryptoSymbol::BNB).is_ok());
        assert!(validate_chain_asset_scope(Some(137), CryptoSymbol::MATIC).is_ok());
    }

    #[test]
    fn test_supports_governed_offramp_lookup_for_next_live_detect_lanes() {
        assert!(supports_governed_offramp_lookup(Some(1)));
        assert!(supports_governed_offramp_lookup(Some(56)));
        assert!(supports_governed_offramp_lookup(Some(101)));
        assert!(supports_governed_offramp_lookup(Some(137)));
        assert!(supports_governed_offramp_lookup(Some(43114)));
        assert!(!supports_governed_offramp_lookup(Some(10)));
        assert!(!supports_governed_offramp_lookup(None));
    }

    #[test]
    fn test_validate_chain_asset_scope_rejects_avalanche_non_stablecoins() {
        let error = validate_chain_asset_scope(Some(43114), CryptoSymbol::BTC).unwrap_err();
        match error {
            ApiError::Validation(message) => {
                assert!(message.contains("Avalanche off-ramp currently supports only USDT/USDC"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn test_validate_chain_asset_scope_rejects_sol_without_solana_chain() {
        let error = validate_chain_asset_scope(None, CryptoSymbol::SOL).unwrap_err();
        match error {
            ApiError::Validation(message) => {
                assert!(message.contains("chainId is required for this asset"));
            }
            other => panic!("unexpected error: {other:?}"),
        }

        assert!(validate_chain_asset_scope(Some(101), CryptoSymbol::SOL).is_ok());
    }

    #[test]
    fn test_validate_chain_asset_scope_rejects_non_default_assets_without_chain_id() {
        for symbol in [CryptoSymbol::BNB, CryptoSymbol::MATIC, CryptoSymbol::BTC] {
            let error = validate_chain_asset_scope(None, symbol).unwrap_err();
            match error {
                ApiError::Validation(message) => {
                    assert!(message.contains("chainId is required for this asset"));
                }
                other => panic!("unexpected error: {other:?}"),
            }
        }
    }

    #[test]
    fn test_offramp_response_serialization() {
        let resp = OfframpIntentResponse {
            id: "ofr_123".to_string(),
            state: "QUOTE_CREATED".to_string(),
            crypto_asset: "USDT".to_string(),
            crypto_amount: "100".to_string(),
            exchange_rate: "25000".to_string(),
            net_vnd_amount: "2475000".to_string(),
            gross_vnd_amount: "2500000".to_string(),
            deposit_address: None,
            chain_id: None,
            tx_hash: None,
            bank_reference: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("\"state\":\"QUOTE_CREATED\""));
        // None fields should be skipped
        assert!(!json.contains("\"depositAddress\""));
        assert!(!json.contains("\"txHash\""));
    }
}
