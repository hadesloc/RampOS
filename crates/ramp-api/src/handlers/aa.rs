//! Account Abstraction (ERC-4337) API Handlers

use alloy::primitives::{Address, Bytes, B256, U256};
use axum::{
    extract::{Extension, Path, State},
    Json,
};
use std::sync::Arc;
use tracing::{info, instrument, warn};

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
use ramp_core::repository::{CreateSmartAccountRequest, SmartAccountRepository};

/// AA Service state containing all AA-related services
#[derive(Clone)]
pub struct AAServiceState {
    pub smart_account_service: Arc<SmartAccountService>,
    pub bundler_client: Arc<BundlerClient>,
    pub paymaster_service: Arc<PaymasterService>,
    pub chain_config: ChainConfig,
    /// Repository for smart account ownership verification
    pub smart_account_repo: Option<Arc<dyn SmartAccountRepository>>,
}

impl AAServiceState {
    /// Create a new AA service state from configuration
    ///
    /// # Errors
    /// Returns an error if PAYMASTER_SIGNER_KEY is not set or is invalid
    pub fn new(chain_config: ChainConfig) -> Result<Self, anyhow::Error> {
        Self::new_with_repo(chain_config, None)
    }

    /// Create a new AA service state with optional smart account repository
    ///
    /// # Errors
    /// Returns an error if:
    /// - PAYMASTER_SIGNER_KEY environment variable is not set
    /// - PAYMASTER_SIGNER_KEY is empty
    /// - PAYMASTER_SIGNER_KEY is not a valid hex string
    pub fn new_with_repo(
        chain_config: ChainConfig,
        smart_account_repo: Option<Arc<dyn SmartAccountRepository>>,
    ) -> Result<Self, anyhow::Error> {
        let bundler_client = Arc::new(BundlerClient::new(chain_config.clone()));

        let smart_account_service = Arc::new(SmartAccountService::new(
            chain_config.chain_id,
            chain_config.entry_point_address,
            chain_config.entry_point_address, // Factory address - in production would be separate
        ));

        // Create paymaster service with signer key from environment
        // SECURITY: In production, this should come from HSM/Vault
        let paymaster_address = chain_config.paymaster_address.unwrap_or(Address::ZERO);

        // CRITICAL: PAYMASTER_SIGNER_KEY is always required - no fallback keys
        let signer_key = match std::env::var("PAYMASTER_SIGNER_KEY") {
            Ok(key) if !key.is_empty() => {
                // Expect hex-encoded private key (32 bytes = 64 hex chars)
                hex::decode(key.trim_start_matches("0x")).map_err(|e| {
                    anyhow::anyhow!("PAYMASTER_SIGNER_KEY must be a valid hex string: {}", e)
                })?
            }
            Ok(_) => {
                // Empty key provided - always error, no fallback
                return Err(anyhow::anyhow!(
                    "PAYMASTER_SIGNER_KEY environment variable cannot be empty"
                ));
            }
            Err(_) => {
                // Key not set at all - allow fallback for tests if explicitly enabled
                #[cfg(test)]
                {
                    // For tests only: use a dummy key if env var is missing
                    // This allows existing tests to pass without setting env vars
                    let mut key = vec![0u8; 32];
                    key[31] = 1;
                    key
                }
                #[cfg(not(test))]
                {
                    // Production: always error, no fallback
                    return Err(anyhow::anyhow!(
                        "PAYMASTER_SIGNER_KEY environment variable is required"
                    ));
                }
            }
        };

        let paymaster_service = Arc::new(
            PaymasterService::new(paymaster_address, signer_key)
                .map_err(|e| anyhow::anyhow!("Failed to create paymaster service: {}", e))?,
        );

        Ok(Self {
            smart_account_service,
            bundler_client,
            paymaster_service,
            chain_config,
            smart_account_repo,
        })
    }

    /// Create a new AA service state for testing only
    /// This uses a dummy signer key and should NEVER be used in production
    #[cfg(test)]
    pub fn new_for_testing(chain_config: ChainConfig) -> Self {
        Self::new_with_repo_for_testing(chain_config, None)
    }

    /// Create a new AA service state with optional repository for testing only
    /// This uses a dummy signer key and should NEVER be used in production
    #[cfg(test)]
    pub fn new_with_repo_for_testing(
        chain_config: ChainConfig,
        smart_account_repo: Option<Arc<dyn SmartAccountRepository>>,
    ) -> Self {
        let bundler_client = Arc::new(BundlerClient::new(chain_config.clone()));

        let smart_account_service = Arc::new(SmartAccountService::new(
            chain_config.chain_id,
            chain_config.entry_point_address,
            chain_config.entry_point_address,
        ));

        let paymaster_address = chain_config.paymaster_address.unwrap_or(Address::ZERO);

        // Test-only key - the scalar 1, which is a valid secp256k1 private key
        let mut signer_key = vec![0u8; 32];
        signer_key[31] = 1;

        let paymaster_service = Arc::new(
            PaymasterService::new(paymaster_address, signer_key)
                .expect("Failed to create test paymaster service"),
        );

        Self {
            smart_account_service,
            bundler_client,
            paymaster_service,
            chain_config,
            smart_account_repo,
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

    tracing::Span::current().record("account_address", format!("{:?}", account.address));

    // Save the smart account mapping to database for ownership verification
    if let Some(ref repo) = aa_state.smart_account_repo {
        let create_req = CreateSmartAccountRequest {
            tenant_id: req.tenant_id.clone(),
            user_id: req.user_id.clone(),
            address: format!("{:?}", account.address),
            owner_address: format!("{:?}", account.owner),
            account_type: format!("{:?}", account.account_type),
            chain_id: aa_state.chain_config.chain_id,
            factory_address: Some(format!("{:?}", aa_state.chain_config.entry_point_address)),
            entry_point_address: Some(format!("{:?}", aa_state.chain_config.entry_point_address)),
        };

        if let Err(e) = repo.create(&create_req).await {
            warn!(
                tenant_id = %req.tenant_id,
                user_id = %req.user_id,
                account_address = %account.address,
                error = %e,
                "Failed to save smart account to database - ownership verification may fail"
            );
            // Don't fail the request, but log the error
            // The account was created successfully on the AA service
        } else {
            info!(
                tenant_id = %req.tenant_id,
                user_id = %req.user_id,
                account_address = %account.address,
                "Smart account saved to database for ownership verification"
            );
        }
    } else {
        warn!(
            tenant_id = %req.tenant_id,
            user_id = %req.user_id,
            account_address = %account.address,
            "Smart account repository not configured - ownership verification will use fallback"
        );
    }

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
    let is_authorized =
        verify_account_ownership(&aa_state, &tenant_ctx.tenant_id, account_address).await;

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

    tracing::Span::current().record("sender", format!("{:?}", user_op.sender));

    ensure_account_belongs_to_tenant(&aa_state, &tenant_ctx.tenant_id, user_op.sender).await?;

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

    tracing::Span::current().record("user_op_hash", format!("{:?}", user_op_hash));

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

    tracing::Span::current().record("sender", format!("{:?}", user_op.sender));

    ensure_account_belongs_to_tenant(&aa_state, &tenant_ctx.tenant_id, user_op.sender).await?;

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
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(hash): Path<String>,
) -> Result<Json<UserOperationDto>, ApiError> {
    tracing::Span::current().record("hash", &hash);

    // Parse hash
    let op_hash: B256 = hash
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid UserOperation hash".to_string()))?;

    // Get from bundler
    let user_op = aa_state
        .bundler_client
        .get_user_operation_by_hash(op_hash)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("UserOperation not found".to_string()))?;

    ensure_account_belongs_to_tenant(&aa_state, &tenant_ctx.tenant_id, user_op.sender).await?;

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
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(hash): Path<String>,
) -> Result<Json<crate::dto::UserOpReceiptDto>, ApiError> {
    tracing::Span::current().record("hash", &hash);

    // Parse hash
    let op_hash: B256 = hash
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid UserOperation hash".to_string()))?;

    // Get receipt from bundler
    let receipt = aa_state
        .bundler_client
        .get_user_operation_receipt(op_hash)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("UserOperation receipt not found".to_string()))?;

    ensure_account_belongs_to_tenant(&aa_state, &tenant_ctx.tenant_id, receipt.sender).await?;

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

async fn ensure_account_belongs_to_tenant(
    aa_state: &AAServiceState,
    tenant_id: &TenantId,
    sender: Address,
) -> Result<(), ApiError> {
    let authorized = verify_account_ownership(aa_state, tenant_id, sender).await;
    if !authorized {
        return Err(ApiError::Forbidden(
            "Account does not belong to this tenant".to_string(),
        ));
    }
    Ok(())
}

/// Verify that a smart account address belongs to the given tenant.
///
/// SECURITY: This function checks account ownership to prevent unauthorized access.
/// It queries the database to verify that the account was created by this tenant.
///
/// Returns:
/// - `true` if the account belongs to the tenant
/// - `false` if the account does not belong to the tenant or is not found
async fn verify_account_ownership(
    aa_state: &AAServiceState,
    tenant_id: &TenantId,
    account_address: Address,
) -> bool {
    // Zero address is never authorized
    if account_address == Address::ZERO {
        return false;
    }

    // Format address for database lookup
    let address_str = format!("{:?}", account_address);
    let chain_id = aa_state.chain_config.chain_id;

    // Query database for ownership verification
    if let Some(ref repo) = aa_state.smart_account_repo {
        match repo
            .verify_ownership(tenant_id, &address_str, chain_id)
            .await
        {
            Ok(is_owner) => {
                if !is_owner {
                    warn!(
                        tenant_id = %tenant_id,
                        address = %account_address,
                        chain_id = chain_id,
                        "Account ownership verification failed - account not found for tenant"
                    );
                }
                return is_owner;
            }
            Err(e) => {
                // Log error but don't expose database errors to caller
                warn!(
                    tenant_id = %tenant_id,
                    address = %account_address,
                    error = %e,
                    "Database error during account ownership verification"
                );
                // SECURITY: Fail closed - deny access if we can't verify ownership
                return false;
            }
        }
    }

    // Fallback: No repository configured
    // SECURITY: In production, this should fail closed (return false)
    // For backward compatibility, we log a warning and allow access
    // This behavior should be removed once all deployments have the repository configured
    if std::env::var("RAMPOS_ENV").unwrap_or_default() == "production" {
        warn!(
            tenant_id = %tenant_id,
            address = %account_address,
            "SECURITY: Smart account repository not configured in production - denying access"
        );
        return false;
    }

    warn!(
        tenant_id = %tenant_id,
        address = %account_address,
        "Smart account repository not configured - denying access (fail closed)"
    );

    // SECURITY: Fail closed - deny access when repository is not configured
    // This ensures that without proper database configuration, no access is granted
    false
}

/// Verify that a smart account address belongs to the given user within a tenant.
///
/// SECURITY: This function provides user-level access control in addition to tenant-level.
/// It queries the database to verify that the account was created by this specific user.
///
/// Use this for operations that should be restricted to the account owner only,
/// such as sending transactions or modifying account settings.
///
/// Returns:
/// - `true` if the account belongs to the user within the tenant
/// - `false` if the account does not belong to the user or is not found
#[allow(dead_code)]
async fn verify_account_user_ownership(
    aa_state: &AAServiceState,
    tenant_id: &TenantId,
    user_id: &str,
    account_address: Address,
) -> bool {
    // Zero address is never authorized
    if account_address == Address::ZERO {
        return false;
    }

    // Empty user_id is never authorized
    if user_id.is_empty() {
        warn!(
            tenant_id = %tenant_id,
            address = %account_address,
            "User ownership verification failed - empty user_id"
        );
        return false;
    }

    // Format address for database lookup
    let address_str = format!("{:?}", account_address);
    let chain_id = aa_state.chain_config.chain_id;

    // Query database for user ownership verification
    if let Some(ref repo) = aa_state.smart_account_repo {
        match repo
            .verify_user_ownership(tenant_id, user_id, &address_str, chain_id)
            .await
        {
            Ok(is_owner) => {
                if !is_owner {
                    warn!(
                        tenant_id = %tenant_id,
                        user_id = %user_id,
                        address = %account_address,
                        chain_id = chain_id,
                        "User ownership verification failed - account not found for user"
                    );
                }
                return is_owner;
            }
            Err(e) => {
                // Log error but don't expose database errors to caller
                warn!(
                    tenant_id = %tenant_id,
                    user_id = %user_id,
                    address = %account_address,
                    error = %e,
                    "Database error during user ownership verification"
                );
                // SECURITY: Fail closed - deny access if we can't verify ownership
                return false;
            }
        }
    }

    // Fallback: No repository configured
    // SECURITY: In production, this MUST fail closed
    if std::env::var("RAMPOS_ENV").unwrap_or_default() == "production" {
        warn!(
            tenant_id = %tenant_id,
            user_id = %user_id,
            address = %account_address,
            "SECURITY: Smart account repository not configured in production - denying access"
        );
        return false;
    }

    warn!(
        tenant_id = %tenant_id,
        user_id = %user_id,
        address = %account_address,
        "Smart account repository not configured - denying access (fail closed)"
    );

    // SECURITY: Fail closed - deny access when repository is not configured
    // This ensures that without proper database configuration, no access is granted
    false
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
        let op = UserOperation::new(Address::ZERO, U256::from(1), Bytes::from(vec![0x12, 0x34]));

        let dto = convert_user_op_to_dto(&op);
        assert_eq!(dto.sender, "0x0000000000000000000000000000000000000000");
        assert_eq!(dto.nonce, "1");
        assert_eq!(dto.call_data, "0x1234");
    }

    #[test]
    fn test_aa_service_state_creation() {
        // Test that AAServiceState can be created without a repository
        let chain_config = ChainConfig {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state = AAServiceState::new_for_testing(chain_config.clone());
        assert!(state.smart_account_repo.is_none());
        assert_eq!(state.chain_config.chain_id, 1);
    }

    #[test]
    fn test_create_smart_account_request() {
        let req = CreateSmartAccountRequest {
            tenant_id: "tenant_123".to_string(),
            user_id: "user_456".to_string(),
            address: "0x1234567890123456789012345678901234567890".to_string(),
            owner_address: "0x0987654321098765432109876543210987654321".to_string(),
            account_type: "SimpleAccount".to_string(),
            chain_id: 1,
            factory_address: None,
            entry_point_address: Some("0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string()),
        };

        assert_eq!(req.tenant_id, "tenant_123");
        assert_eq!(req.user_id, "user_456");
        assert_eq!(req.chain_id, 1);
    }

    /// Mock repository for testing
    #[cfg(test)]
    mod mock_repo {
        use super::*;
        use async_trait::async_trait;
        use ramp_core::repository::{CreateSmartAccountRequest, SmartAccountRow};
        use std::collections::HashMap;
        use std::sync::Mutex;

        /// Account data stored in mock: (tenant_id, user_id, chain_id)
        pub struct MockSmartAccountRepository {
            accounts: Mutex<HashMap<String, (String, String, u64)>>, // address -> (tenant_id, user_id, chain_id)
        }

        impl MockSmartAccountRepository {
            pub fn new() -> Self {
                Self {
                    accounts: Mutex::new(HashMap::new()),
                }
            }

            /// Add account with tenant_id only (backward compatibility)
            pub fn add_account(&self, address: &str, tenant_id: &str, chain_id: u64) {
                self.add_account_with_user(address, tenant_id, "", chain_id);
            }

            /// Add account with both tenant_id and user_id
            pub fn add_account_with_user(
                &self,
                address: &str,
                tenant_id: &str,
                user_id: &str,
                chain_id: u64,
            ) {
                let mut accounts = self.accounts.lock().unwrap();
                accounts.insert(
                    address.to_lowercase(),
                    (tenant_id.to_string(), user_id.to_string(), chain_id),
                );
            }
        }

        #[async_trait]
        impl SmartAccountRepository for MockSmartAccountRepository {
            async fn verify_ownership(
                &self,
                tenant_id: &TenantId,
                address: &str,
                chain_id: u64,
            ) -> ramp_common::Result<bool> {
                let accounts = self.accounts.lock().unwrap();
                if let Some((stored_tenant, _, stored_chain)) =
                    accounts.get(&address.to_lowercase())
                {
                    Ok(stored_tenant == &tenant_id.0 && *stored_chain == chain_id)
                } else {
                    Ok(false)
                }
            }

            async fn verify_user_ownership(
                &self,
                tenant_id: &TenantId,
                user_id: &str,
                address: &str,
                chain_id: u64,
            ) -> ramp_common::Result<bool> {
                let accounts = self.accounts.lock().unwrap();
                if let Some((stored_tenant, stored_user, stored_chain)) =
                    accounts.get(&address.to_lowercase())
                {
                    Ok(stored_tenant == &tenant_id.0
                        && stored_user == user_id
                        && *stored_chain == chain_id)
                } else {
                    Ok(false)
                }
            }

            async fn get_by_address(
                &self,
                _address: &str,
                _chain_id: u64,
            ) -> ramp_common::Result<Option<SmartAccountRow>> {
                Ok(None)
            }

            async fn get_by_address_for_tenant(
                &self,
                _tenant_id: &TenantId,
                _address: &str,
                _chain_id: u64,
            ) -> ramp_common::Result<Option<SmartAccountRow>> {
                Ok(None)
            }

            async fn get_by_user(
                &self,
                _tenant_id: &TenantId,
                _user_id: &str,
            ) -> ramp_common::Result<Vec<SmartAccountRow>> {
                Ok(vec![])
            }

            async fn create(
                &self,
                req: &CreateSmartAccountRequest,
            ) -> ramp_common::Result<SmartAccountRow> {
                let mut accounts = self.accounts.lock().unwrap();
                accounts.insert(
                    req.address.to_lowercase(),
                    (req.tenant_id.clone(), req.user_id.clone(), req.chain_id),
                );
                Err(ramp_common::Error::Database("Not implemented".to_string()))
            }

            async fn update_deployment_status(
                &self,
                _id: &str,
                _is_deployed: bool,
                _tx_hash: Option<&str>,
            ) -> ramp_common::Result<()> {
                Ok(())
            }

            async fn update_status(&self, _id: &str, _status: &str) -> ramp_common::Result<()> {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_verify_account_ownership_zero_address() {
        let chain_config = ChainConfig {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state = AAServiceState::new_for_testing(chain_config);
        let tenant_id = TenantId::new("test_tenant");

        // Zero address should always return false
        let result = verify_account_ownership(&state, &tenant_id, Address::ZERO).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_account_ownership_with_mock_repo() {
        use mock_repo::MockSmartAccountRepository;

        let mock_repo = MockSmartAccountRepository::new();
        mock_repo.add_account("0x1234567890123456789012345678901234567890", "tenant_a", 1);

        let chain_config = ChainConfig {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state =
            AAServiceState::new_with_repo_for_testing(chain_config, Some(Arc::new(mock_repo)));

        let tenant_a = TenantId::new("tenant_a");
        let tenant_b = TenantId::new("tenant_b");
        let address: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();

        // Tenant A should have access
        let result = verify_account_ownership(&state, &tenant_a, address).await;
        assert!(result);

        // Tenant B should NOT have access
        let result = verify_account_ownership(&state, &tenant_b, address).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_account_ownership_unknown_address() {
        use mock_repo::MockSmartAccountRepository;

        let mock_repo = MockSmartAccountRepository::new();
        // Don't add any accounts

        let chain_config = ChainConfig {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state =
            AAServiceState::new_with_repo_for_testing(chain_config, Some(Arc::new(mock_repo)));

        let tenant_id = TenantId::new("any_tenant");
        let address: Address = "0xabcdef1234567890abcdef1234567890abcdef12"
            .parse()
            .unwrap();

        // Unknown address should return false
        let result = verify_account_ownership(&state, &tenant_id, address).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_account_ownership_chain_mismatch() {
        use mock_repo::MockSmartAccountRepository;

        let mock_repo = MockSmartAccountRepository::new();
        // Add account on chain 1
        mock_repo.add_account("0x1234567890123456789012345678901234567890", "tenant_a", 1);

        // But state is for chain 137 (Polygon)
        let chain_config = ChainConfig {
            chain_id: 137,
            name: "Polygon Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state =
            AAServiceState::new_with_repo_for_testing(chain_config, Some(Arc::new(mock_repo)));

        let tenant_id = TenantId::new("tenant_a");
        let address: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();

        // Should fail because chain ID doesn't match
        let result = verify_account_ownership(&state, &tenant_id, address).await;
        assert!(!result);
    }

    // ==========================================================================
    // User Ownership Verification Tests
    // ==========================================================================

    #[tokio::test]
    async fn test_verify_user_ownership_zero_address() {
        let chain_config = ChainConfig {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state = AAServiceState::new_for_testing(chain_config);
        let tenant_id = TenantId::new("test_tenant");

        // Zero address should always return false
        let result =
            verify_account_user_ownership(&state, &tenant_id, "user_123", Address::ZERO).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_user_ownership_empty_user_id() {
        let chain_config = ChainConfig {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state = AAServiceState::new_for_testing(chain_config);
        let tenant_id = TenantId::new("test_tenant");
        let address: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();

        // Empty user_id should always return false
        let result = verify_account_user_ownership(&state, &tenant_id, "", address).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_user_ownership_with_mock_repo() {
        use mock_repo::MockSmartAccountRepository;

        let mock_repo = MockSmartAccountRepository::new();
        mock_repo.add_account_with_user(
            "0x1234567890123456789012345678901234567890",
            "tenant_a",
            "user_123",
            1,
        );

        let chain_config = ChainConfig {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state =
            AAServiceState::new_with_repo_for_testing(chain_config, Some(Arc::new(mock_repo)));

        let tenant_a = TenantId::new("tenant_a");
        let address: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();

        // User 123 should have access
        let result = verify_account_user_ownership(&state, &tenant_a, "user_123", address).await;
        assert!(result);

        // User 456 should NOT have access (different user)
        let result = verify_account_user_ownership(&state, &tenant_a, "user_456", address).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_user_ownership_different_tenant() {
        use mock_repo::MockSmartAccountRepository;

        let mock_repo = MockSmartAccountRepository::new();
        mock_repo.add_account_with_user(
            "0x1234567890123456789012345678901234567890",
            "tenant_a",
            "user_123",
            1,
        );

        let chain_config = ChainConfig {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state =
            AAServiceState::new_with_repo_for_testing(chain_config, Some(Arc::new(mock_repo)));

        let tenant_b = TenantId::new("tenant_b");
        let address: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();

        // Same user_id but different tenant should NOT have access
        let result = verify_account_user_ownership(&state, &tenant_b, "user_123", address).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_user_ownership_chain_mismatch() {
        use mock_repo::MockSmartAccountRepository;

        let mock_repo = MockSmartAccountRepository::new();
        mock_repo.add_account_with_user(
            "0x1234567890123456789012345678901234567890",
            "tenant_a",
            "user_123",
            1, // Chain 1
        );

        // State is for chain 137 (Polygon)
        let chain_config = ChainConfig {
            chain_id: 137,
            name: "Polygon Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state =
            AAServiceState::new_with_repo_for_testing(chain_config, Some(Arc::new(mock_repo)));

        let tenant_a = TenantId::new("tenant_a");
        let address: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();

        // Should fail because chain ID doesn't match
        let result = verify_account_user_ownership(&state, &tenant_a, "user_123", address).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_user_ownership_unknown_address() {
        use mock_repo::MockSmartAccountRepository;

        let mock_repo = MockSmartAccountRepository::new();
        // Don't add any accounts

        let chain_config = ChainConfig {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: Address::ZERO,
            bundler_url: "http://localhost:4337".to_string(),
            paymaster_address: None,
        };

        let state =
            AAServiceState::new_with_repo_for_testing(chain_config, Some(Arc::new(mock_repo)));

        let tenant_id = TenantId::new("any_tenant");
        let address: Address = "0xabcdef1234567890abcdef1234567890abcdef12"
            .parse()
            .unwrap();

        // Unknown address should return false
        let result = verify_account_user_ownership(&state, &tenant_id, "any_user", address).await;
        assert!(!result);
    }
}
