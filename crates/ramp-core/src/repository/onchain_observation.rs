use crate::repository::set_rls_context;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OnchainObservationStatus {
    Observed,
    Pending,
    Confirmed,
    Failed,
    Replaced,
}

impl OnchainObservationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Observed => "OBSERVED",
            Self::Pending => "PENDING",
            Self::Confirmed => "CONFIRMED",
            Self::Failed => "FAILED",
            Self::Replaced => "REPLACED",
        }
    }
}

impl std::fmt::Display for OnchainObservationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OnchainObservationRow {
    pub id: String,
    pub tenant_id: String,
    pub intent_id: Option<String>,
    pub offramp_intent_id: Option<String>,
    pub tx_hash: String,
    pub chain_id: i64,
    pub asset_code: String,
    pub amount: Decimal,
    pub from_address: String,
    pub to_address: String,
    pub status: String,
    pub confirmations: i32,
    pub required_confirmations: i32,
    pub block_number: Option<i64>,
    pub observation_source: String,
    pub raw_payload: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
    pub observed_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertOnchainObservationRequest {
    pub tenant_id: String,
    pub intent_id: Option<String>,
    pub offramp_intent_id: Option<String>,
    pub tx_hash: String,
    pub chain_id: i64,
    pub asset_code: String,
    pub amount: Decimal,
    pub from_address: String,
    pub to_address: String,
    pub status: OnchainObservationStatus,
    pub confirmations: i32,
    pub required_confirmations: i32,
    pub block_number: Option<i64>,
    pub observation_source: String,
    pub raw_payload: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
    pub observed_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait OnchainObservationRepository: Send + Sync {
    async fn upsert_observation(
        &self,
        request: &UpsertOnchainObservationRequest,
    ) -> Result<OnchainObservationRow>;

    async fn get_by_tx_hash(
        &self,
        tenant_id: &TenantId,
        chain_id: i64,
        tx_hash: &str,
    ) -> Result<Option<OnchainObservationRow>>;

    async fn list_by_offramp_intent(
        &self,
        tenant_id: &TenantId,
        offramp_intent_id: &str,
    ) -> Result<Vec<OnchainObservationRow>>;

    async fn list_by_intent(
        &self,
        tenant_id: &TenantId,
        intent_id: &str,
    ) -> Result<Vec<OnchainObservationRow>>;
}

pub struct PgOnchainObservationRepository {
    pool: PgPool,
}

impl PgOnchainObservationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OnchainObservationRepository for PgOnchainObservationRepository {
    async fn upsert_observation(
        &self,
        request: &UpsertOnchainObservationRequest,
    ) -> Result<OnchainObservationRow> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, &TenantId(request.tenant_id.clone()))
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let row = sqlx::query_as::<_, OnchainObservationRow>(
            r#"
            INSERT INTO onchain_observations (
                tenant_id, intent_id, offramp_intent_id, tx_hash, chain_id, asset_code, amount,
                from_address, to_address, status, confirmations, required_confirmations,
                block_number, observation_source, raw_payload, metadata, observed_at, confirmed_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                $8, $9, $10, $11, $12,
                $13, $14, $15, $16, $17, $18
            )
            ON CONFLICT (tenant_id, chain_id, tx_hash) DO UPDATE SET
                intent_id = COALESCE(EXCLUDED.intent_id, onchain_observations.intent_id),
                offramp_intent_id = COALESCE(EXCLUDED.offramp_intent_id, onchain_observations.offramp_intent_id),
                asset_code = EXCLUDED.asset_code,
                amount = EXCLUDED.amount,
                from_address = EXCLUDED.from_address,
                to_address = EXCLUDED.to_address,
                status = EXCLUDED.status,
                confirmations = EXCLUDED.confirmations,
                required_confirmations = EXCLUDED.required_confirmations,
                block_number = EXCLUDED.block_number,
                observation_source = EXCLUDED.observation_source,
                raw_payload = EXCLUDED.raw_payload,
                metadata = EXCLUDED.metadata,
                observed_at = EXCLUDED.observed_at,
                confirmed_at = EXCLUDED.confirmed_at,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(&request.tenant_id)
        .bind(&request.intent_id)
        .bind(&request.offramp_intent_id)
        .bind(&request.tx_hash)
        .bind(request.chain_id)
        .bind(&request.asset_code)
        .bind(request.amount)
        .bind(&request.from_address)
        .bind(&request.to_address)
        .bind(request.status.as_str())
        .bind(request.confirmations)
        .bind(request.required_confirmations)
        .bind(request.block_number)
        .bind(&request.observation_source)
        .bind(&request.raw_payload)
        .bind(&request.metadata)
        .bind(request.observed_at)
        .bind(request.confirmed_at)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_by_tx_hash(
        &self,
        tenant_id: &TenantId,
        chain_id: i64,
        tx_hash: &str,
    ) -> Result<Option<OnchainObservationRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let row = sqlx::query_as::<_, OnchainObservationRow>(
            r#"
            SELECT * FROM onchain_observations
            WHERE tenant_id = $1 AND chain_id = $2 AND tx_hash = $3
            "#,
        )
        .bind(&tenant_id.0)
        .bind(chain_id)
        .bind(tx_hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(row)
    }

    async fn list_by_offramp_intent(
        &self,
        tenant_id: &TenantId,
        offramp_intent_id: &str,
    ) -> Result<Vec<OnchainObservationRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, OnchainObservationRow>(
            r#"
            SELECT * FROM onchain_observations
            WHERE tenant_id = $1 AND offramp_intent_id = $2
            ORDER BY observed_at DESC, id DESC
            "#,
        )
        .bind(&tenant_id.0)
        .bind(offramp_intent_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(rows)
    }

    async fn list_by_intent(
        &self,
        tenant_id: &TenantId,
        intent_id: &str,
    ) -> Result<Vec<OnchainObservationRow>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        set_rls_context(&mut tx, tenant_id)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, OnchainObservationRow>(
            r#"
            SELECT * FROM onchain_observations
            WHERE tenant_id = $1 AND intent_id = $2
            ORDER BY observed_at DESC, id DESC
            "#,
        )
        .bind(&tenant_id.0)
        .bind(intent_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn onchain_observation_repository_is_tenant_scoped_and_upsert_safe() {
        let database_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => return,
        };

        let pool = PgPool::connect(&database_url)
            .await
            .expect("database connection should succeed");

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("migrations should succeed");

        for tenant_id in ["tenant_obs_a", "tenant_obs_b"] {
            sqlx::query(
                r#"
                INSERT INTO tenants (
                    id, name, status, api_key_hash, webhook_secret_hash, config, created_at, updated_at
                ) VALUES ($1, 'Onchain Observation Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW())
                ON CONFLICT (id) DO NOTHING
                "#,
            )
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("seed tenant");
        }

        let repo = PgOnchainObservationRepository::new(pool.clone());
        let observed_at = Utc::now();
        repo.upsert_observation(&UpsertOnchainObservationRequest {
            tenant_id: "tenant_obs_a".to_string(),
            intent_id: Some("intent_obs_a".to_string()),
            offramp_intent_id: Some("ofr_obs_a".to_string()),
            tx_hash: "0xobs".to_string(),
            chain_id: 1,
            asset_code: "USDT".to_string(),
            amount: Decimal::new(1000, 0),
            from_address: "0xfrom".to_string(),
            to_address: "0xto".to_string(),
            status: OnchainObservationStatus::Observed,
            confirmations: 0,
            required_confirmations: 12,
            block_number: None,
            observation_source: "deposit_monitor".to_string(),
            raw_payload: Some(serde_json::json!({"txHash":"0xobs"})),
            metadata: serde_json::json!({"lane":"offramp"}),
            observed_at,
            confirmed_at: None,
        })
        .await
        .expect("insert observation a");

        repo.upsert_observation(&UpsertOnchainObservationRequest {
            tenant_id: "tenant_obs_b".to_string(),
            intent_id: None,
            offramp_intent_id: Some("ofr_obs_b".to_string()),
            tx_hash: "0xobs".to_string(),
            chain_id: 1,
            asset_code: "USDT".to_string(),
            amount: Decimal::new(2000, 0),
            from_address: "0xfromb".to_string(),
            to_address: "0xtob".to_string(),
            status: OnchainObservationStatus::Confirmed,
            confirmations: 15,
            required_confirmations: 12,
            block_number: Some(12345),
            observation_source: "withdraw_confirm".to_string(),
            raw_payload: None,
            metadata: serde_json::json!({"lane":"withdraw"}),
            observed_at,
            confirmed_at: Some(observed_at),
        })
        .await
        .expect("insert observation b");

        repo.upsert_observation(&UpsertOnchainObservationRequest {
            tenant_id: "tenant_obs_a".to_string(),
            intent_id: Some("intent_obs_a".to_string()),
            offramp_intent_id: Some("ofr_obs_a".to_string()),
            tx_hash: "0xobs".to_string(),
            chain_id: 1,
            asset_code: "USDT".to_string(),
            amount: Decimal::new(1000, 0),
            from_address: "0xfrom".to_string(),
            to_address: "0xto".to_string(),
            status: OnchainObservationStatus::Confirmed,
            confirmations: 12,
            required_confirmations: 12,
            block_number: Some(67890),
            observation_source: "deposit_confirm".to_string(),
            raw_payload: Some(serde_json::json!({"confirmed":true})),
            metadata: serde_json::json!({"lane":"offramp","stage":"confirmed"}),
            observed_at,
            confirmed_at: Some(observed_at),
        })
        .await
        .expect("upsert observation a");

        let tenant_a = TenantId::new("tenant_obs_a");
        let tenant_b = TenantId::new("tenant_obs_b");

        let row_a = repo
            .get_by_tx_hash(&tenant_a, 1, "0xobs")
            .await
            .expect("get row a")
            .expect("row a should exist");
        let row_b = repo
            .get_by_tx_hash(&tenant_b, 1, "0xobs")
            .await
            .expect("get row b")
            .expect("row b should exist");

        assert_eq!(row_a.status, "CONFIRMED");
        assert_eq!(row_a.confirmations, 12);
        assert_eq!(row_b.status, "CONFIRMED");
        assert_eq!(row_b.confirmations, 15);

        let by_offramp_a = repo
            .list_by_offramp_intent(&tenant_a, "ofr_obs_a")
            .await
            .expect("list by offramp a");
        assert_eq!(by_offramp_a.len(), 1);
        assert_eq!(by_offramp_a[0].tenant_id, "tenant_obs_a");

        let by_intent_a = repo
            .list_by_intent(&tenant_a, "intent_obs_a")
            .await
            .expect("list by intent a");
        assert_eq!(by_intent_a.len(), 1);
        assert_eq!(by_intent_a[0].tx_hash, "0xobs");

        let by_offramp_b = repo
            .list_by_offramp_intent(&tenant_b, "ofr_obs_a")
            .await
            .expect("tenant b should not see tenant a rows");
        assert!(by_offramp_b.is_empty());
    }
}
