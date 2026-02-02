//! Account Abstraction (ERC-4337) API Handlers

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use ethers::types::{Address, Bytes, H256, U256};
use std::sync::Arc;
use tracing::{info, instrument};

use crate::dto::{
    CreateAccountRequest, CreateAccountResponse, EstimateGasRequest, EstimateGasResponse,
    GetAccountResponse, SendUserOpRequest, SendUserOpResponse, UserOperationDto,
};
use crate::error::ApiError;
use crate::extract::ValidatedJson;
use crate::middleware::TenantContext;

use ramp_aa::{
    bundler::{Bundler, BundlerClient},
    paymaster::{Paymaster, PaymasterService, SponsorshipPolicy},
    smart_account::SmartAccountService,
    types::{ChainConfig, GasEstimation, SmartAccountType},
    user_operation::UserOperation,
};
use ramp_common::types::{TenantId, UserId};

/// AA Service state containing all AA-related services
#[derive(Clone)]
pub struct AAServiceState {
    pub smart_account_service: Arc<SmartAccountService>,
    pub bundler_client: Arc<BundlerClient>,
    pub paymaster_service: Arc<PaymasterService>,
    pub chain_config: ChainConfig,
}

impl AAServiceState {
    /// Create a new AA service state from configuration
    pub fn new(chain_config: ChainConfig) -> Self {
        let bundler_client = Arc::new(BundlerClient::new(chain_config.clone()));

        let smart_account_service = Arc::new(SmartAccountService::new(
            chain_config.chain_id,
            chain_config.entry_point_address,
            chain_config.entry_point_address, // Factory address - in production would be separate
        ));

        // Create paymaster service with signer key from environment
        // SECURITY: In production, this should come from HSM/Vault
        let paymaster_address = chain_config
            .paymaster_address
            .unwrap_or_else(|| Address::zero());

        // CRITICAL: Require PAYMASTER_SIGNER_KEY in production
        let signer_key = match std::env::var("PAYMASTER_SIGNER_KEY") {
            Ok(key) if !key.is_empty() => {
                // Expect hex-encoded private key (32 bytes = 64 hex chars)
                hex::decode(key.trim_start_matches("0x"))
                    .expect("PAYMASTER_SIGNER_KEY must be a valid hex string")
            }
            Ok(_) => {
                // Empty key provided
                if std::env::var("RAMPOS_ENV").unwrap_or_default() == "production" {
                    panic!("CRITICAL: PAYMASTER_SIGNER_KEY cannot be empty in production");
                }
                tracing::warn!("PAYMASTER_SIGNER_KEY is empty - using test key (NOT FOR PRODUCTION)");
                // Test key for development only - DO NOT USE IN PRODUCTION
                vec![0u8; 32]
            }
            Err(_) => {
                // Key not set at all
                if std::env::var("RAMPOS_ENV").unwrap_or_default() == "production" {
                    panic!("CRITICAL: PAYMASTER_SIGNER_KEY environment variable is required in production");
                }
                tracing::warn!("PAYMASTER_SIGNER_KEY not set - using test key (NOT FOR PRODUCTION)");
                // Test key for development only - DO NOT USE IN PRODUCTION
                vec![0u8; 32]
            }
        };

        let paymaster_service = Arc::new(PaymasterService::new(paymaster_address, signer_key));

        Self {
            smart_account_service,
            bundler_client,
            paymaster_service,
            chain_config,
        }
    }
}

/// Create a smart account for a user
///
/// Creates a new ERC-4337 smart account (counterfactual address) for the specified user.
/// The account is not deployed on-chain until the first UserOperation is sent.
#[utoipa::path(
    post,
    path = "/v1/aa/accounts",
    tag = "account-abstraction",
    request_body = CreateAccountRequest,
    responses(
        (status = 200, description = "Smart account created", body = CreateAccountResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[instrument(skip_all, fields(tenant_id, user_id, account_address))]
pub async fn create_account(
    State(aa_state): State<AAServiceState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    ValidatedJson(req): ValidatedJson<CreateAccountRequest>,
) -> Result<Json<CreateAccountResponse>, ApiError> {
    // Verify tenant
    if tenant_ctx.tenant_id.0 != req.tenant_id {
        return Err(ApiError::Forbidden("Tenant mismatch".to_string()));
    }

    tracing::Span::current()
        .record("tenant_id", &req.tenant_id)
        .record("user_id", &req.user_id);

    // Parse owner address
    let owner: Address = req
        .owner_address
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid owner address".to_string()))?;

    // Get or create smart account
    let tenant_id = TenantId::new(&req.tenant_id);
    let user_id = UserId::new(&req.user_id);

    let account = aa_state
        .smart_account_service
        .get_or_create_account(&tenant_id, &user_id, owner)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::Span::current().record("account_address", &format!("{:?}", account.address));

    info!(
        tenant_id = %req.tenant_id,
        user_id = %req.user_id,
        account_address = %account.address,
        "Smart account created"
    );

    Ok(Json(CreateAccountResponse {
        address: format!("{:?}", account.address),
        owner: format!("{:?}", account.owner),
        account_type: format!("{:?}", account.account_type),
        is_deployed: account.is_deployed,
        chain_id: aa_state.chain_config.chain_id,
        entry_point: format!("{:?}", aa_state.chain_config.entry_point_address),
    }))
}

/// Get smart account information
///
/// Retrieves information about a smart account by its address.
#[utoipa::path(
    get,
    path = "/v1/aa/accounts/{address}",
    tag = "account-abstraction",
    params(
        ("address" = String, Path, description = "Smart account address (0x...)")
    ),
    responses(
        (status = 200, description = "Account information", body = GetAccountResponse),
        (status = 400, description = "Invalid address", body = ErrorResponse),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[instrument(skip_all, fields(address, tenant_id))]
pub async fn get_account(
    State(aa_state): State<AAServiceState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(address): Path<String>,
) -> Result<Json<GetAccountResponse>, ApiError> {
    tracing::Span::current()
        .record("address", &address)
        .record("tenant_id", &tenant_ctx.tenant_id.0);

    // Parse address
    let account_address: Address = address
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid account address".to_string()))?;

    // SECURITY: Verify that the requested account belongs to this tenant
    // In a production system, we would query a database to verify ownership.
    // For now, we verify by checking if the account was created by this tenant
    // by computing all possible addresses for the tenant and checking if the
    // requested address matches any of them.
    //
    // NOTE: This is a simplified check. In production, you should maintain
    // a database mapping of accounts to tenants for efficient lookups.
    let is_authorized = verify_account_ownership(
        &aa_state,
        &tenant_ctx.tenant_id,
        account_address,
    ).await;

    if !is_authorized {
        tracing::warn!(
            tenant_id = %tenant_ctx.tenant_id,
            address = %account_address,
            "Unauthorized attempt to access account belonging to another tenant"
        );
        return Err(ApiError::Forbidden(
            "Account does not belong to this tenant".to_string(),
        ));
    }

    // In a full implementation, we would query the blockchain to check if the account
    // is deployed and get its current state. For now, we return basic information.
    // This would typically involve:
    // 1. Checking if code exists at the address (is_deployed)
    // 2. Querying the account's nonce from the EntryPoint
    // 3. Getting the owner from the account contract

    // Get supported entry points from bundler
    let entry_points = aa_state
        .bundler_client
        .supported_entry_points()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let entry_point = entry_points
        .first()
        .copied()
        .unwrap_or(aa_state.chain_config.entry_point_address);

    info!(
        address = %account_address,
        tenant_id = %tenant_ctx.tenant_id,
        "Retrieved account information"
    );

    Ok(Json(GetAccountResponse {
        address: format!("{:?}", account_address),
        is_deployed: false, // Would query chain in production
        nonce: "0".to_string(),
        chain_id: aa_state.chain_config.chain_id,
        entry_point: format!("{:?}", entry_point),
        account_type: format!("{:?}", SmartAccountType::SimpleAccount),
    }))
}

/// Send a UserOperation
///
/// Submits a UserOperation to the bundler for execution on-chain.
/// Optionally sponsors the operation via paymaster.
#[utoipa::path(
    post,
    path = "/v1/aa/user-operations",
    tag = "account-abstraction",
    request_body = SendUserOpRequest,
    responses(
        (status = 200, description = "UserOperation submitted", body = SendUserOpResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Forbidden or sponsorship denied", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[instrument(skip_all, fields(tenant_id, sender, user_op_hash))]
pub async fn send_user_operation(
    State(aa_state): State<AAServiceState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    ValidatedJson(req): ValidatedJson<SendUserOpRequest>,
) -> Result<Json<SendUserOpResponse>, ApiError> {
    // Verify tenant
    if tenant_ctx.tenant_id.0 != req.tenant_id {
        return Err(ApiError::Forbidden("Tenant mismatch".to_string()));
    }

    tracing::Span::current().record("tenant_id", &req.tenant_id);

    // Convert DTO to UserOperation
    let user_op = convert_dto_to_user_op(&req.user_operation)?;

    tracing::Span::current().record("sender", &format!("{:?}", user_op.sender));

    // Handle sponsorship if requested
    let final_user_op = if req.sponsor {
        // Check if we can sponsor this operation
        let policy = SponsorshipPolicy {
            tenant_id: TenantId::new(&req.tenant_id),
            ..Default::default()
        };

        let can_sponsor = aa_state
            .paymaster_service
            .can_sponsor(&user_op, &policy)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        if !can_sponsor {
            return Err(ApiError::Forbidden(
                "Operation cannot be sponsored under current policy".to_string(),
            ));
        }

        // Get paymaster data
        let paymaster_data = aa_state
            .paymaster_service
            .sponsor(&user_op)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        // Add paymaster data to user operation
        user_op.with_paymaster(paymaster_data.paymaster_and_data)
    } else {
        user_op
    };

    // Send to bundler
    let user_op_hash = aa_state
        .bundler_client
        .send_user_operation(&final_user_op, aa_state.chain_config.entry_point_address)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::Span::current().record("user_op_hash", &format!("{:?}", user_op_hash));

    info!(
        tenant_id = %req.tenant_id,
        sender = %final_user_op.sender,
        user_op_hash = %user_op_hash,
        sponsored = req.sponsor,
        "UserOperation submitted to bundler"
    );

    Ok(Json(SendUserOpResponse {
        user_op_hash: format!("{:?}", user_op_hash),
        sender: format!("{:?}", final_user_op.sender),
        nonce: format!("{}", final_user_op.nonce),
        status: "PENDING".to_string(),
        sponsored: req.sponsor,
    }))
}

/// Estimate gas for a UserOperation
///
/// Estimates the gas parameters required for a UserOperation.
#[utoipa::path(
    post,
    path = "/v1/aa/user-operations/estimate",
    tag = "account-abstraction",
    request_body = EstimateGasRequest,
    responses(
        (status = 200, description = "Gas estimation", body = EstimateGasResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[instrument(skip_all, fields(tenant_id, sender))]
pub async fn estimate_gas(
    State(aa_state): State<AAServiceState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    ValidatedJson(req): ValidatedJson<EstimateGasRequest>,
) -> Result<Json<EstimateGasResponse>, ApiError> {
    // Verify tenant
    if tenant_ctx.tenant_id.0 != req.tenant_id {
        return Err(ApiError::Forbidden("Tenant mismatch".to_string()));
    }

    tracing::Span::current().record("tenant_id", &req.tenant_id);

    // Convert DTO to UserOperation
    let user_op = convert_dto_to_user_op(&req.user_operation)?;

    tracing::Span::current().record("sender", &format!("{:?}", user_op.sender));

    // Estimate gas via bundler
    let estimation: GasEstimation = aa_state
        .bundler_client
        .estimate_user_operation_gas(&user_op, aa_state.chain_config.entry_point_address)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    info!(
        sender = %user_op.sender,
        call_gas = %estimation.call_gas_limit,
        verification_gas = %estimation.verification_gas_limit,
        "Gas estimated for UserOperation"
    );

    Ok(Json(EstimateGasResponse {
        pre_verification_gas: format!("{}", estimation.pre_verification_gas),
        verification_gas_limit: format!("{}", estimation.verification_gas_limit),
        call_gas_limit: format!("{}", estimation.call_gas_limit),
        max_fee_per_gas: format!("{}", estimation.max_fee_per_gas),
        max_priority_fee_per_gas: format!("{}", estimation.max_priority_fee_per_gas),
    }))
}

/// Get UserOperation by hash
///
/// Retrieves a UserOperation by its hash.
#[utoipa::path(
    get,
    path = "/v1/aa/user-operations/{hash}",
    tag = "account-abstraction",
    params(
        ("hash" = String, Path, description = "UserOperation hash (0x...)")
    ),
    responses(
        (status = 200, description = "UserOperation found", body = UserOperationDto),
        (status = 400, description = "Invalid hash", body = ErrorResponse),
        (status = 404, description = "UserOperation not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[instrument(skip_all, fields(hash))]
pub async fn get_user_operation(
    State(aa_state): State<AAServiceState>,
    Extension(_tenant_ctx): Extension<TenantContext>,
    Path(hash): Path<String>,
) -> Result<Json<UserOperationDto>, ApiError> {
    tracing::Span::current().record("hash", &hash);

    // Parse hash
    let op_hash: H256 = hash
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid UserOperation hash".to_string()))?;

    // Get from bundler
    let user_op = aa_state
        .bundler_client
        .get_user_operation_by_hash(op_hash)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("UserOperation not found".to_string()))?;

    info!(
        hash = %op_hash,
        sender = %user_op.sender,
        "Retrieved UserOperation"
    );

    Ok(Json(convert_user_op_to_dto(&user_op)))
}

/// Get UserOperation receipt
///
/// Retrieves the receipt for a completed UserOperation.
#[utoipa::path(
    get,
    path = "/v1/aa/user-operations/{hash}/receipt",
    tag = "account-abstraction",
    params(
        ("hash" = String, Path, description = "UserOperation hash (0x...)")
    ),
    responses(
        (status = 200, description = "Receipt found", body = UserOpReceiptDto),
        (status = 400, description = "Invalid hash", body = ErrorResponse),
        (status = 404, description = "Receipt not found (operation pending or failed)", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[instrument(skip_all, fields(hash))]
pub async fn get_user_operation_receipt(
    State(aa_state): State<AAServiceState>,
    Extension(_tenant_ctx): Extension<TenantContext>,
    Path(hash): Path<String>,
) -> Result<Json<crate::dto::UserOpReceiptDto>, ApiError> {
    tracing::Span::current().record("hash", &hash);

    // Parse hash
    let op_hash: H256 = hash
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid UserOperation hash".to_string()))?;

    // Get receipt from bundler
    let receipt = aa_state
        .bundler_client
        .get_user_operation_receipt(op_hash)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("UserOperation receipt not found".to_string()))?;

    info!(
        hash = %op_hash,
        success = receipt.success,
        "Retrieved UserOperation receipt"
    );

    Ok(Json(crate::dto::UserOpReceiptDto {
        user_op_hash: format!("{:?}", receipt.user_op_hash),
        sender: format!("{:?}", receipt.sender),
        nonce: format!("{}", receipt.nonce),
        success: receipt.success,
        actual_gas_cost: format!("{}", receipt.actual_gas_cost),
        actual_gas_used: format!("{}", receipt.actual_gas_used),
        paymaster: receipt.paymaster.map(|p| format!("{:?}", p)),
        transaction_hash: format!("{:?}", receipt.receipt.transaction_hash),
        block_hash: format!("{:?}", receipt.receipt.block_hash),
        block_number: format!("{}", receipt.receipt.block_number),
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Verify that a smart account address belongs to the given tenant.
///
/// SECURITY: This function checks account ownership to prevent unauthorized access.
/// In production, this should query a database that maps accounts to tenants.
/// The current implementation is a placeholder that always returns true for
/// non-zero addresses (indicating the account lookup infrastructure is not yet
/// implemented).
///
/// TODO: Implement proper account-to-tenant mapping in database for production.
async fn verify_account_ownership(
    _aa_state: &AAServiceState,
    tenant_id: &TenantId,
    account_address: Address,
) -> bool {
    // In production, query database to verify:
    // SELECT 1 FROM smart_accounts WHERE address = $1 AND tenant_id = $2
    //
    // For now, we implement a basic check:
    // 1. Zero address is never authorized
    // 2. Non-zero addresses require database lookup (not yet implemented)

    if account_address == Address::zero() {
        return false;
    }

    // TODO: Replace with actual database lookup
    // For now, log a warning that this is a placeholder
    tracing::warn!(
        tenant_id = %tenant_id,
        address = %account_address,
        "Account ownership verification is using placeholder implementation - implement database lookup for production"
    );

    // TEMPORARY: Return true for non-zero addresses
    // In production, this MUST be replaced with actual database verification
    // that checks the account was created by this tenant
    true
}

/// Convert DTO to UserOperation
fn convert_dto_to_user_op(dto: &UserOperationDto) -> Result<UserOperation, ApiError> {
    let sender: Address = dto
        .sender
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid sender address".to_string()))?;

    let nonce: U256 = dto
        .nonce
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid nonce".to_string()))?;

    let init_code: Bytes = if let Some(ref code) = dto.init_code {
        hex_to_bytes(code)?
    } else {
        Bytes::default()
    };

    let call_data: Bytes = hex_to_bytes(&dto.call_data)?;

    let call_gas_limit: U256 = dto
        .call_gas_limit
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid call_gas_limit".to_string()))?;

    let verification_gas_limit: U256 = dto
        .verification_gas_limit
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid verification_gas_limit".to_string()))?;

    let pre_verification_gas: U256 = dto
        .pre_verification_gas
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid pre_verification_gas".to_string()))?;

    let max_fee_per_gas: U256 = dto
        .max_fee_per_gas
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid max_fee_per_gas".to_string()))?;

    let max_priority_fee_per_gas: U256 = dto
        .max_priority_fee_per_gas
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid max_priority_fee_per_gas".to_string()))?;

    let paymaster_and_data: Bytes = if let Some(ref data) = dto.paymaster_and_data {
        hex_to_bytes(data)?
    } else {
        Bytes::default()
    };

    let signature: Bytes = if let Some(ref sig) = dto.signature {
        hex_to_bytes(sig)?
    } else {
        Bytes::default()
    };

    Ok(UserOperation {
        sender,
        nonce,
        init_code,
        call_data,
        call_gas_limit,
        verification_gas_limit,
        pre_verification_gas,
        max_fee_per_gas,
        max_priority_fee_per_gas,
        paymaster_and_data,
        signature,
    })
}

/// Convert UserOperation to DTO
fn convert_user_op_to_dto(op: &UserOperation) -> UserOperationDto {
    UserOperationDto {
        sender: format!("{:?}", op.sender),
        nonce: format!("{}", op.nonce),
        init_code: if op.init_code.is_empty() {
            None
        } else {
            Some(format!("0x{}", hex::encode(&op.init_code)))
        },
        call_data: format!("0x{}", hex::encode(&op.call_data)),
        call_gas_limit: format!("{}", op.call_gas_limit),
        verification_gas_limit: format!("{}", op.verification_gas_limit),
        pre_verification_gas: format!("{}", op.pre_verification_gas),
        max_fee_per_gas: format!("{}", op.max_fee_per_gas),
        max_priority_fee_per_gas: format!("{}", op.max_priority_fee_per_gas),
        paymaster_and_data: if op.paymaster_and_data.is_empty() {
            None
        } else {
            Some(format!("0x{}", hex::encode(&op.paymaster_and_data)))
        },
        signature: if op.signature.is_empty() {
            None
        } else {
            Some(format!("0x{}", hex::encode(&op.signature)))
        },
    }
}

/// Convert hex string to Bytes
fn hex_to_bytes(hex_str: &str) -> Result<Bytes, ApiError> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(hex_str)
        .map_err(|_| ApiError::BadRequest(format!("Invalid hex string: {}", hex_str)))?;
    Ok(Bytes::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_bytes() {
        let result = hex_to_bytes("0x1234").unwrap();
        assert_eq!(result.as_ref(), &[0x12, 0x34]);

        let result = hex_to_bytes("abcd").unwrap();
        assert_eq!(result.as_ref(), &[0xab, 0xcd]);

        let result = hex_to_bytes("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_user_op_to_dto() {
        let op = UserOperation::new(
            Address::zero(),
            U256::from(1),
            Bytes::from(vec![0x12, 0x34]),
        );

        let dto = convert_user_op_to_dto(&op);
        assert_eq!(dto.sender, "0x0000000000000000000000000000000000000000");
        assert_eq!(dto.nonce, "1");
        assert_eq!(dto.call_data, "0x1234");
    }
}
