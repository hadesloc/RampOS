//! LP (Liquidity Provider) RFQ Handler
//!
//! LP-facing endpoint for submitting bids on open RFQs:
//! - POST /v1/lp/rfq/:id/bid → submit a price quote for an open RFQ
//!
//! Authentication: X-LP-Key header validated against registered LP keys.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use ramp_common::types::TenantId;
use ramp_core::repository::{set_rls_context, PgRfqRepository, RfqRepository};
use ramp_core::service::rfq::{RfqService, SubmitBidRequest};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use std::sync::Arc;
use tracing::info;
use validator::Validate;

use crate::error::ApiError;
use crate::router::AppState;

// ============================================================================
// Auth Helper
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedLpKey {
    lp_id: String,
    tenant_id: String,
    secret: String,
}

#[derive(Debug, Clone, FromRow)]
struct RegisteredLpKeyRow {
    lp_id: String,
    lp_name: Option<String>,
    key_hash: String,
    can_bid_offramp: bool,
    can_bid_onramp: bool,
    max_bid_amount: Option<Decimal>,
    is_active: bool,
    expires_at: Option<DateTime<Utc>>,
}

fn extract_lp_key(headers: &HeaderMap) -> Result<String, ApiError> {
    headers
        .get("X-LP-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::Unauthorized("X-LP-Key header is required".to_string()))
}

/// Parse LP key in format "lp_id:tenant_id:secret".
fn parse_lp_key(lp_key: &str) -> Result<ParsedLpKey, ApiError> {
    let parts: Vec<&str> = lp_key.splitn(3, ':').collect();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(ApiError::Unauthorized(
            "X-LP-Key must be in format 'lp_id:tenant_id:secret'".to_string(),
        ));
    }

    Ok(ParsedLpKey {
        lp_id: parts[0].to_string(),
        tenant_id: parts[1].to_string(),
        secret: parts[2].to_string(),
    })
}

fn hash_lp_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

fn matches_key_hash(stored_hash: &str, candidate_secret: &str) -> bool {
    let normalized = stored_hash.trim().trim_start_matches("\\x").to_ascii_lowercase();
    normalized == hash_lp_secret(candidate_secret)
}

fn ensure_lp_key_active(record: &RegisteredLpKeyRow) -> Result<(), ApiError> {
    if !record.is_active {
        return Err(ApiError::Forbidden("LP key is inactive".to_string()));
    }

    if let Some(expires_at) = record.expires_at {
        if Utc::now() >= expires_at {
            return Err(ApiError::Unauthorized("LP key has expired".to_string()));
        }
    }

    Ok(())
}

fn ensure_lp_permissions(
    record: &RegisteredLpKeyRow,
    direction: &str,
    vnd_amount: Decimal,
) -> Result<(), ApiError> {
    match direction {
        "OFFRAMP" if !record.can_bid_offramp => {
            return Err(ApiError::Forbidden(
                "LP key is not allowed to bid on OFFRAMP RFQs".to_string(),
            ));
        }
        "ONRAMP" if !record.can_bid_onramp => {
            return Err(ApiError::Forbidden(
                "LP key is not allowed to bid on ONRAMP RFQs".to_string(),
            ));
        }
        _ => {}
    }

    if let Some(max_bid_amount) = record.max_bid_amount {
        if vnd_amount > max_bid_amount {
            return Err(ApiError::Forbidden(format!(
                "Bid amount exceeds LP key cap of {}",
                max_bid_amount
            )));
        }
    }

    Ok(())
}

async fn load_registered_lp_key(
    pool: &sqlx::PgPool,
    tenant_id: &TenantId,
    lp_id: &str,
) -> Result<Option<RegisteredLpKeyRow>, ApiError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to start LP key lookup: {}", e)))?;
    set_rls_context(&mut tx, tenant_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to set LP key tenant context: {}", e)))?;

    let row = sqlx::query_as::<_, RegisteredLpKeyRow>(
        r#"
        SELECT
            lp_id,
            lp_name,
            key_hash,
            can_bid_offramp,
            can_bid_onramp,
            max_bid_amount,
            is_active,
            expires_at
        FROM registered_lp_keys
        WHERE tenant_id = $1 AND lp_id = $2
        "#,
    )
    .bind(&tenant_id.0)
    .bind(lp_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to load LP key: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to commit LP key lookup: {}", e)))?;

    Ok(row)
}

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SubmitBidRequest_ {
    /// Exchange rate (VND per 1 unit of crypto)
    #[validate(length(min = 1))]
    pub exchange_rate: String,

    /// Total VND amount in the deal
    #[validate(length(min = 1))]
    pub vnd_amount: String,

    /// Optional LP name for display
    pub lp_name: Option<String>,

    /// How long this bid is valid (minutes, 1-30), default 5
    pub valid_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BidResponse {
    pub id: String,
    pub rfq_id: String,
    pub lp_id: String,
    pub exchange_rate: String,
    pub vnd_amount: String,
    pub valid_until: String,
    pub state: String,
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new().route("/rfq/:rfq_id/bid", post(submit_bid))
}

// ============================================================================
// Handler
// ============================================================================

/// POST /v1/lp/rfq/:rfq_id/bid - LP submits a bid for an open RFQ
pub async fn submit_bid(
    State(app_state): State<AppState>,
    headers: HeaderMap,
    Path(rfq_id): Path<String>,
    Json(req): Json<SubmitBidRequest_>,
) -> Result<Json<BidResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let parsed_key = parse_lp_key(&extract_lp_key(&headers)?)?;
    let tenant_id = TenantId(parsed_key.tenant_id.clone());

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("RFQ service unavailable".to_string()))?
        .clone();

    let exchange_rate: Decimal = req
        .exchange_rate
        .parse()
        .map_err(|_| ApiError::Validation("exchange_rate must be a valid number".to_string()))?;

    let vnd_amount: Decimal = req
        .vnd_amount
        .parse()
        .map_err(|_| ApiError::Validation("vnd_amount must be a valid number".to_string()))?;

    if exchange_rate <= Decimal::ZERO {
        return Err(ApiError::Validation(
            "exchange_rate must be positive".to_string(),
        ));
    }
    if vnd_amount <= Decimal::ZERO {
        return Err(ApiError::Validation(
            "vnd_amount must be positive".to_string(),
        ));
    }

    let lp_record = load_registered_lp_key(&pool, &tenant_id, &parsed_key.lp_id)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("LP key is not registered".to_string()))?;
    ensure_lp_key_active(&lp_record)?;
    if !matches_key_hash(&lp_record.key_hash, &parsed_key.secret) {
        return Err(ApiError::Unauthorized("Invalid LP key secret".to_string()));
    }

    let repo = PgRfqRepository::new(pool.clone());
    let rfq = repo
        .get_request(&tenant_id, &rfq_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound("RFQ not found".to_string()))?;
    ensure_lp_permissions(&lp_record, &rfq.direction, vnd_amount)?;

    let svc = RfqService::new(
        Arc::new(PgRfqRepository::new(pool)),
        app_state.event_publisher.clone(),
    );

    let bid = svc
        .submit_bid(SubmitBidRequest {
            tenant_id: tenant_id.clone(),
            rfq_id: rfq_id.clone(),
            lp_id: parsed_key.lp_id.clone(),
            lp_name: req.lp_name.or(lp_record.lp_name.clone()),
            exchange_rate,
            vnd_amount,
            valid_minutes: req.valid_minutes.unwrap_or(5).clamp(1, 30),
        })
        .await
        .map_err(ApiError::from)?;

    info!(
        bid_id = %bid.id,
        rfq_id = %rfq_id,
        lp_id = %parsed_key.lp_id,
        rate = %exchange_rate,
        "LP: bid submitted"
    );

    Ok(Json(BidResponse {
        id: bid.id,
        rfq_id: bid.rfq_id,
        lp_id: bid.lp_id,
        exchange_rate: bid.exchange_rate.to_string(),
        vnd_amount: bid.vnd_amount.to_string(),
        valid_until: bid.valid_until.to_rfc3339(),
        state: bid.state,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use rust_decimal_macros::dec;

    #[test]
    fn parse_lp_key_requires_three_non_empty_segments() {
        assert!(parse_lp_key("lp:tenant:secret").is_ok());
        assert!(parse_lp_key("lp:tenant").is_err());
        assert!(parse_lp_key("lp::secret").is_err());
        assert!(parse_lp_key(":tenant:secret").is_err());
    }

    #[test]
    fn matches_key_hash_accepts_hex_and_bytea_prefix() {
        let secret = "s3cr3t";
        let hash = hash_lp_secret(secret);
        assert!(matches_key_hash(&hash, secret));
        assert!(matches_key_hash(&format!("\\x{}", hash), secret));
        assert!(!matches_key_hash(&hash, "wrong"));
    }

    #[test]
    fn ensure_lp_permissions_rejects_direction_and_cap_violations() {
        let row = RegisteredLpKeyRow {
            lp_id: "lp_1".to_string(),
            lp_name: None,
            key_hash: "hash".to_string(),
            can_bid_offramp: true,
            can_bid_onramp: false,
            max_bid_amount: Some(dec!(1000000)),
            is_active: true,
            expires_at: None,
        };

        assert!(ensure_lp_permissions(&row, "OFFRAMP", dec!(999999)).is_ok());
        assert!(matches!(
            ensure_lp_permissions(&row, "ONRAMP", dec!(1)),
            Err(ApiError::Forbidden(_))
        ));
        assert!(matches!(
            ensure_lp_permissions(&row, "OFFRAMP", dec!(1000001)),
            Err(ApiError::Forbidden(_))
        ));
    }

    #[test]
    fn ensure_lp_key_active_rejects_inactive_or_expired_keys() {
        let expired = RegisteredLpKeyRow {
            lp_id: "lp_2".to_string(),
            lp_name: None,
            key_hash: "hash".to_string(),
            can_bid_offramp: true,
            can_bid_onramp: true,
            max_bid_amount: None,
            is_active: true,
            expires_at: Some(Utc::now() - Duration::minutes(1)),
        };
        let inactive = RegisteredLpKeyRow {
            is_active: false,
            expires_at: None,
            ..expired.clone()
        };

        assert!(matches!(
            ensure_lp_key_active(&expired),
            Err(ApiError::Unauthorized(_))
        ));
        assert!(matches!(
            ensure_lp_key_active(&inactive),
            Err(ApiError::Forbidden(_))
        ));
    }
}
