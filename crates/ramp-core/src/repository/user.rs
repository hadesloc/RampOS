use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::{TenantId, UserId}, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserRow {
    pub id: String,
    pub tenant_id: String,
    pub kyc_tier: i16,
    pub kyc_status: String,
    pub kyc_verified_at: Option<DateTime<Utc>>,
    pub risk_score: Option<Decimal>,
    pub risk_flags: serde_json::Value,
    pub daily_payin_limit_vnd: Option<Decimal>,
    pub daily_payout_limit_vnd: Option<Decimal>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn get_by_id(&self, tenant_id: &TenantId, user_id: &UserId) -> Result<Option<UserRow>>;
    async fn create(&self, user: &UserRow) -> Result<()>;
    async fn update_kyc_tier(&self, tenant_id: &TenantId, user_id: &UserId, tier: i16) -> Result<()>;
    async fn update_risk_score(&self, tenant_id: &TenantId, user_id: &UserId, score: Decimal) -> Result<()>;
    async fn update_status(&self, tenant_id: &TenantId, user_id: &UserId, status: &str) -> Result<()>;
}

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn get_by_id(&self, tenant_id: &TenantId, user_id: &UserId) -> Result<Option<UserRow>> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT * FROM users WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&tenant_id.0)
        .bind(&user_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn create(&self, user: &UserRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO users (
                id, tenant_id, kyc_tier, kyc_status, kyc_verified_at,
                risk_score, risk_flags, daily_payin_limit_vnd, daily_payout_limit_vnd,
                status, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(&user.id)
        .bind(&user.tenant_id)
        .bind(user.kyc_tier)
        .bind(&user.kyc_status)
        .bind(user.kyc_verified_at)
        .bind(user.risk_score)
        .bind(&user.risk_flags)
        .bind(user.daily_payin_limit_vnd)
        .bind(user.daily_payout_limit_vnd)
        .bind(&user.status)
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_kyc_tier(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        tier: i16,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE users SET kyc_tier = $1, kyc_verified_at = NOW() WHERE tenant_id = $2 AND id = $3",
        )
        .bind(tier)
        .bind(&tenant_id.0)
        .bind(&user_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_risk_score(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        score: Decimal,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE users SET risk_score = $1 WHERE tenant_id = $2 AND id = $3",
        )
        .bind(score)
        .bind(&tenant_id.0)
        .bind(&user_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_status(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE users SET status = $1 WHERE tenant_id = $2 AND id = $3",
        )
        .bind(status)
        .bind(&tenant_id.0)
        .bind(&user_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }
}
