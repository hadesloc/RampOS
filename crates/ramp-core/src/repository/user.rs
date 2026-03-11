use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{
    types::{TenantId, UserId},
    Result,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, QueryBuilder, Row};

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
    async fn list_due_for_rescreening(
        &self,
        tenant_id: &TenantId,
        due_before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<UserRow>>;
    async fn update_kyc_tier(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        tier: i16,
    ) -> Result<()>;
    async fn update_risk_score(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        score: Decimal,
    ) -> Result<()>;
    async fn update_status(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        status: &str,
    ) -> Result<()>;
    async fn update_risk_flags(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        risk_flags: serde_json::Value,
    ) -> Result<()>;
    async fn update_limits(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        daily_payin_limit_vnd: Option<Decimal>,
        daily_payout_limit_vnd: Option<Decimal>,
    ) -> Result<()>;
    async fn list_users(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
        kyc_tier: Option<i16>,
        status: Option<&str>,
        search: Option<&str>,
    ) -> Result<Vec<UserRow>>;
    async fn count_users(
        &self,
        tenant_id: &TenantId,
        kyc_tier: Option<i16>,
        status: Option<&str>,
        search: Option<&str>,
    ) -> Result<i64>;
    async fn count_users_by_kyc_status(
        &self,
        tenant_id: &TenantId,
        kyc_status: &str,
    ) -> Result<i64>;
    async fn count_users_created_since(
        &self,
        tenant_id: &TenantId,
        since: DateTime<Utc>,
    ) -> Result<i64>;
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
        let row =
            sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE tenant_id = $1 AND id = $2")
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

    async fn list_due_for_rescreening(
        &self,
        tenant_id: &TenantId,
        due_before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<UserRow>> {
        let rows = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT *
            FROM users
            WHERE tenant_id = $1
              AND status = 'ACTIVE'
              AND COALESCE(
                    NULLIF(risk_flags #>> '{rescreening,nextRunAt}', '')::timestamptz,
                    COALESCE(kyc_verified_at, created_at) + INTERVAL '180 days'
                  ) <= $2
            ORDER BY COALESCE(
                    NULLIF(risk_flags #>> '{rescreening,nextRunAt}', '')::timestamptz,
                    COALESCE(kyc_verified_at, created_at) + INTERVAL '180 days'
                ) ASC,
                id ASC
            LIMIT $3
            "#,
        )
        .bind(&tenant_id.0)
        .bind(due_before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
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
        sqlx::query("UPDATE users SET risk_score = $1 WHERE tenant_id = $2 AND id = $3")
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
        sqlx::query("UPDATE users SET status = $1 WHERE tenant_id = $2 AND id = $3")
            .bind(status)
            .bind(&tenant_id.0)
            .bind(&user_id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_risk_flags(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        risk_flags: serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET risk_flags = $1,
                updated_at = NOW()
            WHERE tenant_id = $2 AND id = $3
            "#,
        )
        .bind(risk_flags)
        .bind(&tenant_id.0)
        .bind(&user_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_limits(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        daily_payin_limit_vnd: Option<Decimal>,
        daily_payout_limit_vnd: Option<Decimal>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET daily_payin_limit_vnd = COALESCE($1, daily_payin_limit_vnd),
                daily_payout_limit_vnd = COALESCE($2, daily_payout_limit_vnd),
                updated_at = NOW()
            WHERE tenant_id = $3 AND id = $4
            "#,
        )
        .bind(daily_payin_limit_vnd)
        .bind(daily_payout_limit_vnd)
        .bind(&tenant_id.0)
        .bind(&user_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn list_users(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
        kyc_tier: Option<i16>,
        status: Option<&str>,
        search: Option<&str>,
    ) -> Result<Vec<UserRow>> {
        let mut builder = QueryBuilder::new("SELECT * FROM users WHERE tenant_id = ");
        builder.push_bind(&tenant_id.0);

        if let Some(tier) = kyc_tier {
            builder.push(" AND kyc_tier = ").push_bind(tier);
        }

        if let Some(status) = status {
            builder.push(" AND status = ").push_bind(status);
        }

        if let Some(search) = search {
            let pattern = format!("%{}%", search);
            builder.push(" AND id ILIKE ").push_bind(pattern);
        }

        builder
            .push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(limit)
            .push(" OFFSET ")
            .push_bind(offset);

        let query = builder.build_query_as::<UserRow>();
        query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))
    }

    async fn count_users(
        &self,
        tenant_id: &TenantId,
        kyc_tier: Option<i16>,
        status: Option<&str>,
        search: Option<&str>,
    ) -> Result<i64> {
        let mut builder =
            QueryBuilder::new("SELECT COUNT(*) as count FROM users WHERE tenant_id = ");
        builder.push_bind(&tenant_id.0);

        if let Some(tier) = kyc_tier {
            builder.push(" AND kyc_tier = ").push_bind(tier);
        }

        if let Some(status) = status {
            builder.push(" AND status = ").push_bind(status);
        }

        if let Some(search) = search {
            let pattern = format!("%{}%", search);
            builder.push(" AND id ILIKE ").push_bind(pattern);
        }

        let row = builder
            .build()
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        let count: i64 = row.try_get("count").unwrap_or(0);
        Ok(count)
    }

    async fn count_users_by_kyc_status(
        &self,
        tenant_id: &TenantId,
        kyc_status: &str,
    ) -> Result<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM users WHERE tenant_id = $1 AND kyc_status = $2",
        )
        .bind(&tenant_id.0)
        .bind(kyc_status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        let count: i64 = row.try_get("count").unwrap_or(0);
        Ok(count)
    }

    async fn count_users_created_since(
        &self,
        tenant_id: &TenantId,
        since: DateTime<Utc>,
    ) -> Result<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM users WHERE tenant_id = $1 AND created_at >= $2",
        )
        .bind(&tenant_id.0)
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        let count: i64 = row.try_get("count").unwrap_or(0);
        Ok(count)
    }
}
