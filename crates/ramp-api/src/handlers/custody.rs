use std::sync::Arc;

use axum::{
    extract::{Extension, Query},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use validator::Validate;

use ramp_aa::{CustodySigner, UserOperation};
use ramp_core::custody::{
    policy::TransactionRequest, CustodyPolicy, MpcKeyService, MpcSigningService, PolicyDecision,
    PolicyEngine,
};

use crate::error::ApiError;
use crate::extract::ValidatedJson;
use crate::middleware::TenantContext;

static MPC_KEY_SERVICE: Lazy<Arc<MpcKeyService>> = Lazy::new(|| Arc::new(MpcKeyService::new()));
static POLICY_ENGINE: Lazy<Arc<PolicyEngine>> = Lazy::new(|| Arc::new(PolicyEngine::new()));
static CUSTODY_SIGNER: Lazy<Arc<CustodySigner>> =
    Lazy::new(|| Arc::new(CustodySigner::new(Arc::new(MpcSigningService::new()))));

#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenerateCustodyKeyRequest {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenerateCustodyKeyResponse {
    pub user_id: String,
    pub public_key: String,
    pub generation: u64,
    pub share_count: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustodySignRequest {
    pub user_id: String,
    pub user_operation: crate::dto::UserOperationDto,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustodySignResponse {
    pub user_id: String,
    pub signature: String,
    pub algorithm: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustodyPolicyRequest {
    pub user_id: String,
    pub whitelist_addresses: Vec<String>,
    pub daily_limit: String,
    pub require_multi_approval_above: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustodyPolicyResponse {
    pub whitelist_addresses: Vec<String>,
    pub daily_limit: String,
    pub require_multi_approval_above: String,
    pub enabled: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustodyPolicyCheckRequest {
    pub user_id: String,
    pub to_address: String,
    pub amount: String,
    pub currency: String,
    pub chain_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustodyPolicyCheckResponse {
    pub decision: String,
    pub reason: Option<String>,
}

fn check_operator(headers: &HeaderMap) -> Result<(), ApiError> {
    crate::handlers::admin::tier::check_admin_key_operator(headers)?;
    Ok(())
}

fn convert_dto_to_user_op(dto: &crate::dto::UserOperationDto) -> Result<UserOperation, ApiError> {
    let sender = dto
        .sender
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid sender address".to_string()))?;
    let nonce = dto
        .nonce
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid nonce".to_string()))?;

    let parse_bytes = |input: Option<&String>| -> Result<alloy::primitives::Bytes, ApiError> {
        let raw = input.map(|s| s.as_str()).unwrap_or("0x");
        let trimmed = raw.strip_prefix("0x").unwrap_or(raw);
        let bytes = if trimmed.is_empty() {
            vec![]
        } else {
            hex::decode(trimmed)
                .map_err(|_| ApiError::BadRequest("Invalid hex field in userOperation".to_string()))?
        };
        Ok(alloy::primitives::Bytes::from(bytes))
    };

    let call_data = {
        let trimmed = dto.call_data.strip_prefix("0x").unwrap_or(&dto.call_data);
        let bytes = if trimmed.is_empty() {
            vec![]
        } else {
            hex::decode(trimmed)
                .map_err(|_| ApiError::BadRequest("Invalid callData hex".to_string()))?
        };
        alloy::primitives::Bytes::from(bytes)
    };

    Ok(UserOperation {
        sender,
        nonce,
        init_code: parse_bytes(dto.init_code.as_ref())?,
        call_data,
        call_gas_limit: dto
            .call_gas_limit
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid callGasLimit".to_string()))?,
        verification_gas_limit: dto
            .verification_gas_limit
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid verificationGasLimit".to_string()))?,
        pre_verification_gas: dto
            .pre_verification_gas
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid preVerificationGas".to_string()))?,
        max_fee_per_gas: dto
            .max_fee_per_gas
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid maxFeePerGas".to_string()))?,
        max_priority_fee_per_gas: dto
            .max_priority_fee_per_gas
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid maxPriorityFeePerGas".to_string()))?,
        paymaster_and_data: parse_bytes(dto.paymaster_and_data.as_ref())?,
        signature: parse_bytes(dto.signature.as_ref())?,
    })
}

/// POST /v1/custody/keys/generate
pub async fn generate_custody_keys(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    ValidatedJson(req): ValidatedJson<GenerateCustodyKeyRequest>,
) -> Result<Json<GenerateCustodyKeyResponse>, ApiError> {
    check_operator(&headers)?;

    let result = MPC_KEY_SERVICE.generate_key_shares();
    for share in result.shares.iter() {
        MPC_KEY_SERVICE
            .store_key_share(&req.user_id, share.party_id, share.clone())
            .map_err(ApiError::from)?;
    }

    tracing::info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %req.user_id,
        generation = result.generation,
        "Generated custody MPC key shares"
    );

    Ok(Json(GenerateCustodyKeyResponse {
        user_id: req.user_id,
        public_key: format!("0x{}", hex::encode(result.public_key)),
        generation: result.generation,
        share_count: result.shares.len(),
        created_at: Utc::now().to_rfc3339(),
    }))
}

/// POST /v1/custody/sign
pub async fn sign_with_custody(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    ValidatedJson(req): ValidatedJson<CustodySignRequest>,
) -> Result<Json<CustodySignResponse>, ApiError> {
    check_operator(&headers)?;

    let op = convert_dto_to_user_op(&req.user_operation)?;
    let sig = CUSTODY_SIGNER
        .sign_user_operation(&req.user_id, &op, alloy::primitives::Address::ZERO, 1)
        .map_err(ApiError::from)?;

    tracing::info!(
        tenant = %tenant_ctx.tenant_id.0,
        user_id = %req.user_id,
        "Signed UserOperation via MPC custody"
    );

    Ok(Json(CustodySignResponse {
        user_id: req.user_id,
        signature: format!("0x{}", hex::encode(sig)),
        algorithm: "mpc-threshold-2of3-simulated".to_string(),
    }))
}

/// GET /v1/custody/policies?userId=<id>
pub async fn get_custody_policy(
    headers: HeaderMap,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<CustodyPolicyResponse>, ApiError> {
    check_operator(&headers)?;

    let user_id = query
        .get("userId")
        .or_else(|| query.get("user_id"))
        .ok_or_else(|| ApiError::BadRequest("Missing userId query parameter".to_string()))?;

    let policy = POLICY_ENGINE
        .get_policy(user_id)
        .unwrap_or_else(CustodyPolicy::permissive);

    Ok(Json(CustodyPolicyResponse {
        whitelist_addresses: policy.whitelist_addresses,
        daily_limit: policy.daily_limit.to_string(),
        require_multi_approval_above: policy.require_multi_approval_above.to_string(),
        enabled: policy.enabled,
        updated_at: policy.updated_at.to_rfc3339(),
    }))
}

/// PUT /v1/custody/policies
pub async fn update_custody_policy(
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<CustodyPolicyRequest>,
) -> Result<Json<CustodyPolicyResponse>, ApiError> {
    check_operator(&headers)?;

    let daily_limit = req
        .daily_limit
        .parse()
        .map_err(|_| ApiError::Validation("Invalid dailyLimit".to_string()))?;
    let multi = req
        .require_multi_approval_above
        .parse()
        .map_err(|_| ApiError::Validation("Invalid requireMultiApprovalAbove".to_string()))?;

    let now = Utc::now();
    let policy = CustodyPolicy {
        whitelist_addresses: req.whitelist_addresses.clone(),
        daily_limit,
        require_multi_approval_above: multi,
        time_restrictions: None,
        enabled: req.enabled,
        created_at: now,
        updated_at: now,
    };

    POLICY_ENGINE.update_policy(&req.user_id, policy.clone());

    Ok(Json(CustodyPolicyResponse {
        whitelist_addresses: policy.whitelist_addresses,
        daily_limit: policy.daily_limit.to_string(),
        require_multi_approval_above: policy.require_multi_approval_above.to_string(),
        enabled: policy.enabled,
        updated_at: policy.updated_at.to_rfc3339(),
    }))
}

/// POST /v1/custody/policies/check
pub async fn check_custody_policy(
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<CustodyPolicyCheckRequest>,
) -> Result<Json<CustodyPolicyCheckResponse>, ApiError> {
    check_operator(&headers)?;

    let amount = req
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("Invalid amount".to_string()))?;

    let tx = TransactionRequest {
        to_address: req.to_address,
        amount,
        currency: req.currency,
        chain_id: req.chain_id,
    };

    let decision = POLICY_ENGINE.check_policy(&req.user_id, &tx);

    let (decision_text, reason) = match decision {
        PolicyDecision::Allow => ("allow".to_string(), None),
        PolicyDecision::Deny(msg) => ("deny".to_string(), Some(msg)),
        PolicyDecision::RequireApproval(msg) => ("require_approval".to_string(), Some(msg)),
    };

    Ok(Json(CustodyPolicyCheckResponse {
        decision: decision_text,
        reason,
    }))
}
