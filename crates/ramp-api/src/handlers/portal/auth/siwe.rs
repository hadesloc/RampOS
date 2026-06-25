use axum::{extract::State, Json};
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Duration, Utc};
use ramp_aa::recover_personal_sign;
use tracing::{info, warn};
use uuid::Uuid;

use super::identity::link_wallet_to_identity;
use super::{
    generate_base62_nonce, AuthUser, WalletNonceRequest, WalletNonceResponse, WalletVerifyRequest,
};
use crate::error::ApiError;
use crate::middleware::PortalUser;
use crate::router::AppState;

#[derive(Debug, Clone)]
pub struct SiweValidationConfig {
    pub domain: String,
    pub uri: String,
    pub chain_id: u64,
    pub max_age: Duration,
    pub clock_skew: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSiweMessage {
    pub domain: String,
    pub address: String,
    pub uri: String,
    pub chain_id: u64,
    pub nonce: String,
    pub issued_at: DateTime<Utc>,
}

pub fn parse_and_validate_message(
    message: &str,
    config: &SiweValidationConfig,
    now: DateTime<Utc>,
) -> Result<ValidatedSiweMessage, String> {
    let lines: Vec<&str> = message.lines().collect();
    let first_line = lines
        .first()
        .map(|line| line.trim())
        .ok_or_else(|| "SIWE message is empty".to_string())?;
    let domain = first_line
        .strip_suffix(" wants you to sign in with your Ethereum account:")
        .filter(|domain| !domain.is_empty())
        .ok_or_else(|| "Invalid SIWE domain line".to_string())?;
    if domain != config.domain {
        return Err("SIWE domain mismatch".to_string());
    }

    let address = lines
        .get(1)
        .map(|line| line.trim().to_lowercase())
        .filter(|address| {
            address.len() == 42
                && address.starts_with("0x")
                && address[2..]
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
        })
        .ok_or_else(|| "Invalid SIWE Ethereum address".to_string())?;

    let uri = field(&lines, "URI: ")?;
    if uri != config.uri {
        return Err("SIWE URI mismatch".to_string());
    }

    let version = field(&lines, "Version: ")?;
    if version != "1" {
        return Err("Unsupported SIWE version".to_string());
    }

    let chain_id = field(&lines, "Chain ID: ")?
        .parse::<u64>()
        .map_err(|_| "Invalid SIWE chain ID".to_string())?;
    if chain_id != config.chain_id {
        return Err("SIWE chain ID mismatch".to_string());
    }

    let nonce = field(&lines, "Nonce: ")?;
    if nonce.len() < 8
        || !nonce
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return Err("Invalid SIWE nonce".to_string());
    }

    let issued_at = DateTime::parse_from_rfc3339(&field(&lines, "Issued At: ")?)
        .map_err(|_| "Invalid SIWE issued-at timestamp".to_string())?
        .with_timezone(&Utc);
    if issued_at > now + config.clock_skew {
        return Err("SIWE message was issued in the future".to_string());
    }
    if issued_at < now - config.max_age - config.clock_skew {
        return Err("SIWE message has expired".to_string());
    }

    Ok(ValidatedSiweMessage {
        domain: domain.to_string(),
        address,
        uri,
        chain_id,
        nonce,
        issued_at,
    })
}

fn field(lines: &[&str], prefix: &str) -> Result<String, String> {
    let matches: Vec<String> = lines
        .iter()
        .filter_map(|line| line.trim().strip_prefix(prefix))
        .map(|value| value.trim().to_string())
        .collect();

    match matches.as_slice() {
        [value] if !value.is_empty() => Ok(value.clone()),
        [] => Err(format!("Missing SIWE field {}", prefix.trim())),
        _ => Err(format!("Duplicate SIWE field {}", prefix.trim())),
    }
}

pub async fn wallet_link_nonce(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Json(request): Json<WalletNonceRequest>,
) -> Result<Json<WalletNonceResponse>, ApiError> {
    let address = validate_wallet_address(&request.address)?;
    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;
    let config = validation_config(&app_state);
    let nonce = generate_base62_nonce(24);
    let now = Utc::now();
    let expires_at = now + Duration::minutes(10);
    let issued_at = now.format("%Y-%m-%dT%H:%M:%SZ");
    let message = format!(
        "{} wants you to sign in with your Ethereum account:\n{}\n\n\
         Link this wallet to your RampOS Portal account.\n\n\
         URI: {}\nVersion: 1\nChain ID: {}\nNonce: {}\nIssued At: {}",
        config.domain, address, config.uri, config.chain_id, nonce, issued_at
    );

    sqlx::query(
        r#"
        DELETE FROM portal_auth_nonces
        WHERE portal_user_id = $1
          AND purpose = 'link'
          AND used_at IS NULL
        "#,
    )
    .bind(portal_user.user_id.to_string())
    .execute(pool)
    .await
    .map_err(|error| ApiError::Internal(format!("Failed to clear wallet link nonce: {error}")))?;

    sqlx::query(
        r#"
        INSERT INTO portal_auth_nonces (
            address, nonce, domain, issued_at, expires_at, purpose, portal_user_id
        ) VALUES ($1, $2, $3, $4, $5, 'link', $6)
        "#,
    )
    .bind(&address)
    .bind(&nonce)
    .bind(&config.domain)
    .bind(now)
    .bind(expires_at)
    .bind(portal_user.user_id.to_string())
    .execute(pool)
    .await
    .map_err(|error| ApiError::Internal(format!("Failed to store wallet link nonce: {error}")))?;

    Ok(Json(WalletNonceResponse {
        nonce,
        message,
        expires_at: expires_at.timestamp(),
    }))
}

pub async fn wallet_link_verify(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    _jar: CookieJar,
    Json(request): Json<WalletVerifyRequest>,
) -> Result<Json<AuthUser>, ApiError> {
    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;
    let config = validation_config(&app_state);
    let parsed = parse_and_validate_message(&request.message, &config, Utc::now())
        .map_err(|error| ApiError::Unauthorized(format!("Invalid SIWE message: {error}")))?;
    verify_message_signature(&request.message, &request.signature, &parsed.address)?;

    let consumed: Option<Uuid> = sqlx::query_scalar(
        r#"
        UPDATE portal_auth_nonces
        SET used_at = NOW()
        WHERE nonce = $1
          AND lower(address) = $2
          AND domain = $3
          AND purpose = 'link'
          AND portal_user_id = $4
          AND used_at IS NULL
          AND expires_at > NOW()
        RETURNING id
        "#,
    )
    .bind(&parsed.nonce)
    .bind(&parsed.address)
    .bind(&config.domain)
    .bind(portal_user.user_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(format!("Failed to consume wallet link nonce: {error}")))?;
    if consumed.is_none() {
        return Err(ApiError::Unauthorized(
            "Invalid, expired, or already used nonce".to_string(),
        ));
    }

    let identity = link_wallet_to_identity(
        pool,
        &portal_user.tenant_id.to_string(),
        &portal_user.user_id.to_string(),
        &parsed.address,
    )
    .await
    .map_err(|error| {
        if error.to_string().contains("already linked")
            || error
                .as_database_error()
                .and_then(|database_error| database_error.code())
                .as_deref()
                == Some("23505")
        {
            ApiError::Conflict("Wallet is already linked to another account".to_string())
        } else {
            warn!(error = %error, "Wallet link failed");
            ApiError::Internal("Failed to link wallet".to_string())
        }
    })?;

    info!(
        portal_user_id = %identity.portal_user_id,
        financial_user_id = %identity.financial_user_id,
        "Portal wallet linked"
    );

    Ok(Json(AuthUser {
        id: identity.portal_user_id,
        email: identity.email.unwrap_or_default(),
        kyc_status: identity.kyc_status,
        kyc_tier: identity.kyc_tier as i32,
        status: identity.status,
        created_at: identity.created_at.to_rfc3339(),
        wallet_address: identity.wallet_address.unwrap_or_default(),
    }))
}

fn validation_config(app_state: &AppState) -> SiweValidationConfig {
    SiweValidationConfig {
        domain: std::env::var("PORTAL_SIWE_DOMAIN")
            .unwrap_or_else(|_| "localhost:3000".to_string()),
        uri: std::env::var("PORTAL_SIWE_URI")
            .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        chain_id: app_state
            .aa_service
            .as_ref()
            .map(|aa| aa.chain_config.chain_id)
            .unwrap_or(137),
        max_age: Duration::minutes(10),
        clock_skew: Duration::seconds(30),
    }
}

fn validate_wallet_address(address: &str) -> Result<String, ApiError> {
    let normalized = address.trim().to_lowercase();
    if normalized.len() != 42
        || !normalized.starts_with("0x")
        || !normalized[2..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(ApiError::Validation(
            "address must be a 0x-prefixed 40-hex Ethereum address".to_string(),
        ));
    }
    Ok(normalized)
}

fn verify_message_signature(
    message: &str,
    signature: &str,
    claimed_address: &str,
) -> Result<(), ApiError> {
    let signature_hex = signature.trim_start_matches("0x");
    if signature_hex.len() != 130 {
        return Err(ApiError::Unauthorized(
            "Signature must be a 65-byte hex value".to_string(),
        ));
    }
    let signature_bytes = hex::decode(signature_hex)
        .map_err(|_| ApiError::Unauthorized("Invalid signature encoding".to_string()))?;
    let recovered = recover_personal_sign(message.as_bytes(), &signature_bytes)
        .map_err(|_| ApiError::Unauthorized("Invalid wallet signature".to_string()))?;
    if format!("{recovered:?}").to_lowercase() != claimed_address {
        return Err(ApiError::Unauthorized(
            "Signature does not match the claimed address".to_string(),
        ));
    }
    Ok(())
}
