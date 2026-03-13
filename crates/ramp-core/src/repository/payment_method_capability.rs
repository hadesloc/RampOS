use async_trait::async_trait;
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodCapabilityRecord {
    pub payment_method_capability_id: String,
    pub corridor_pack_id: String,
    pub partner_capability_id: Option<String>,
    pub method_family: String,
    pub funding_source: Option<String>,
    pub settlement_direction: String,
    pub presentment_model: Option<String>,
    pub card_funding_enabled: bool,
    pub policy_flags: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPaymentMethodCapabilityRequest {
    pub payment_method_capability_id: String,
    pub corridor_pack_id: String,
    pub partner_capability_id: Option<String>,
    pub method_family: String,
    pub funding_source: Option<String>,
    pub settlement_direction: String,
    pub presentment_model: Option<String>,
    pub card_funding_enabled: bool,
    pub policy_flags: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[async_trait]
pub trait PaymentMethodCapabilityRepository: Send + Sync {
    async fn upsert_payment_method_capability(
        &self,
        request: &UpsertPaymentMethodCapabilityRequest,
    ) -> Result<()>;

    async fn list_payment_method_capabilities(
        &self,
        corridor_pack_id: Option<&str>,
        partner_capability_id: Option<&str>,
    ) -> Result<Vec<PaymentMethodCapabilityRecord>>;
}

pub struct PgPaymentMethodCapabilityRepository {
    pool: PgPool,
}

impl PgPaymentMethodCapabilityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone, FromRow)]
struct PaymentMethodCapabilityRow {
    id: String,
    corridor_pack_id: String,
    partner_capability_id: Option<String>,
    method_family: String,
    funding_source: Option<String>,
    settlement_direction: String,
    presentment_model: Option<String>,
    card_funding_enabled: bool,
    policy_flags: serde_json::Value,
    metadata: serde_json::Value,
}

#[async_trait]
impl PaymentMethodCapabilityRepository for PgPaymentMethodCapabilityRepository {
    async fn upsert_payment_method_capability(
        &self,
        request: &UpsertPaymentMethodCapabilityRequest,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO payment_method_capabilities (
                id,
                corridor_pack_id,
                partner_capability_id,
                method_family,
                funding_source,
                settlement_direction,
                presentment_model,
                card_funding_enabled,
                policy_flags,
                metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT (id) DO UPDATE SET
                corridor_pack_id = EXCLUDED.corridor_pack_id,
                partner_capability_id = EXCLUDED.partner_capability_id,
                method_family = EXCLUDED.method_family,
                funding_source = EXCLUDED.funding_source,
                settlement_direction = EXCLUDED.settlement_direction,
                presentment_model = EXCLUDED.presentment_model,
                card_funding_enabled = EXCLUDED.card_funding_enabled,
                policy_flags = EXCLUDED.policy_flags,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.payment_method_capability_id)
        .bind(&request.corridor_pack_id)
        .bind(&request.partner_capability_id)
        .bind(&request.method_family)
        .bind(&request.funding_source)
        .bind(&request.settlement_direction)
        .bind(&request.presentment_model)
        .bind(request.card_funding_enabled)
        .bind(&request.policy_flags)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
        Ok(())
    }

    async fn list_payment_method_capabilities(
        &self,
        corridor_pack_id: Option<&str>,
        partner_capability_id: Option<&str>,
    ) -> Result<Vec<PaymentMethodCapabilityRecord>> {
        let rows = match (corridor_pack_id, partner_capability_id) {
            (Some(corridor_pack_id), Some(partner_capability_id)) => {
                sqlx::query_as::<_, PaymentMethodCapabilityRow>(
                    r#"
                    SELECT id, corridor_pack_id, partner_capability_id, method_family, funding_source,
                           settlement_direction, presentment_model, card_funding_enabled, policy_flags, metadata
                    FROM payment_method_capabilities
                    WHERE corridor_pack_id = $1 AND partner_capability_id = $2
                    ORDER BY method_family ASC, id ASC
                    "#,
                )
                .bind(corridor_pack_id)
                .bind(partner_capability_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|error| ramp_common::Error::Database(error.to_string()))?
            }
            (Some(corridor_pack_id), None) => {
                sqlx::query_as::<_, PaymentMethodCapabilityRow>(
                    r#"
                    SELECT id, corridor_pack_id, partner_capability_id, method_family, funding_source,
                           settlement_direction, presentment_model, card_funding_enabled, policy_flags, metadata
                    FROM payment_method_capabilities
                    WHERE corridor_pack_id = $1
                    ORDER BY method_family ASC, id ASC
                    "#,
                )
                .bind(corridor_pack_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|error| ramp_common::Error::Database(error.to_string()))?
            }
            (None, Some(partner_capability_id)) => {
                sqlx::query_as::<_, PaymentMethodCapabilityRow>(
                    r#"
                    SELECT id, corridor_pack_id, partner_capability_id, method_family, funding_source,
                           settlement_direction, presentment_model, card_funding_enabled, policy_flags, metadata
                    FROM payment_method_capabilities
                    WHERE partner_capability_id = $1
                    ORDER BY method_family ASC, id ASC
                    "#,
                )
                .bind(partner_capability_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|error| ramp_common::Error::Database(error.to_string()))?
            }
            (None, None) => {
                sqlx::query_as::<_, PaymentMethodCapabilityRow>(
                    r#"
                    SELECT id, corridor_pack_id, partner_capability_id, method_family, funding_source,
                           settlement_direction, presentment_model, card_funding_enabled, policy_flags, metadata
                    FROM payment_method_capabilities
                    ORDER BY method_family ASC, id ASC
                    "#,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|error| ramp_common::Error::Database(error.to_string()))?
            }
        };

        Ok(rows
            .into_iter()
            .map(|row| PaymentMethodCapabilityRecord {
                payment_method_capability_id: row.id,
                corridor_pack_id: row.corridor_pack_id,
                partner_capability_id: row.partner_capability_id,
                method_family: row.method_family,
                funding_source: row.funding_source,
                settlement_direction: row.settlement_direction,
                presentment_model: row.presentment_model,
                card_funding_enabled: row.card_funding_enabled,
                policy_flags: row.policy_flags,
                metadata: row.metadata,
            })
            .collect())
    }
}
