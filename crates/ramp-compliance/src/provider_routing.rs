use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFamily {
    Kyc,
    Kyb,
    Kyt,
    Sanctions,
    AdverseMedia,
    TravelRule,
}

impl ProviderFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Kyc => "kyc",
            Self::Kyb => "kyb",
            Self::Kyt => "kyt",
            Self::Sanctions => "sanctions",
            Self::AdverseMedia => "adverse_media",
            Self::TravelRule => "travel_rule",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRoutingPolicy {
    pub policy_id: String,
    pub tenant_id: Option<String>,
    pub provider_family: ProviderFamily,
    pub policy_name: String,
    pub corridor_code: Option<String>,
    pub entity_type: Option<String>,
    pub risk_tier: Option<String>,
    pub partner_key: Option<String>,
    pub asset_code: Option<String>,
    pub amount_min: Option<Decimal>,
    pub amount_max: Option<Decimal>,
    pub fallback_order: Vec<String>,
    pub scorecard: serde_json::Value,
    pub provider_weights: serde_json::Value,
    pub lifecycle_state: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertProviderRoutingPolicyRequest {
    pub policy_id: String,
    pub tenant_id: Option<String>,
    pub provider_family: ProviderFamily,
    pub policy_name: String,
    pub corridor_code: Option<String>,
    pub entity_type: Option<String>,
    pub risk_tier: Option<String>,
    pub partner_key: Option<String>,
    pub asset_code: Option<String>,
    pub amount_min: Option<Decimal>,
    pub amount_max: Option<Decimal>,
    pub fallback_order: Vec<String>,
    pub scorecard: serde_json::Value,
    pub provider_weights: serde_json::Value,
    pub lifecycle_state: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRoutingQuery {
    pub provider_family: ProviderFamily,
    pub corridor_code: Option<String>,
    pub entity_type: Option<String>,
    pub risk_tier: Option<String>,
    pub partner_key: Option<String>,
    pub asset_code: Option<String>,
    pub amount: Option<Decimal>,
}

#[derive(Debug, Clone, FromRow)]
struct ProviderRoutingPolicyRow {
    id: String,
    tenant_id: Option<String>,
    provider_family: String,
    policy_name: String,
    corridor_code: Option<String>,
    entity_type: Option<String>,
    risk_tier: Option<String>,
    partner_key: Option<String>,
    asset_code: Option<String>,
    amount_min: Option<Decimal>,
    amount_max: Option<Decimal>,
    fallback_order: serde_json::Value,
    scorecard: serde_json::Value,
    provider_weights: serde_json::Value,
    lifecycle_state: String,
    metadata: serde_json::Value,
}

#[derive(Clone)]
pub struct ProviderRoutingPolicyStore {
    pool: PgPool,
}

impl ProviderRoutingPolicyStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_policy(
        &self,
        request: &UpsertProviderRoutingPolicyRequest,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO provider_routing_policies (
                id, tenant_id, provider_family, policy_name, corridor_code, entity_type,
                risk_tier, partner_key, asset_code, amount_min, amount_max, fallback_order,
                scorecard, provider_weights, lifecycle_state, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                provider_family = EXCLUDED.provider_family,
                policy_name = EXCLUDED.policy_name,
                corridor_code = EXCLUDED.corridor_code,
                entity_type = EXCLUDED.entity_type,
                risk_tier = EXCLUDED.risk_tier,
                partner_key = EXCLUDED.partner_key,
                asset_code = EXCLUDED.asset_code,
                amount_min = EXCLUDED.amount_min,
                amount_max = EXCLUDED.amount_max,
                fallback_order = EXCLUDED.fallback_order,
                scorecard = EXCLUDED.scorecard,
                provider_weights = EXCLUDED.provider_weights,
                lifecycle_state = EXCLUDED.lifecycle_state,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&request.policy_id)
        .bind(&request.tenant_id)
        .bind(request.provider_family.as_str())
        .bind(&request.policy_name)
        .bind(&request.corridor_code)
        .bind(&request.entity_type)
        .bind(&request.risk_tier)
        .bind(&request.partner_key)
        .bind(&request.asset_code)
        .bind(request.amount_min)
        .bind(request.amount_max)
        .bind(serde_json::json!(request.fallback_order))
        .bind(&request.scorecard)
        .bind(&request.provider_weights)
        .bind(&request.lifecycle_state)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_policies(
        &self,
        tenant_id: Option<&str>,
        provider_family: ProviderFamily,
    ) -> Result<Vec<ProviderRoutingPolicy>, sqlx::Error> {
        let rows = if let Some(tenant_id) = tenant_id {
            sqlx::query_as::<_, ProviderRoutingPolicyRow>(
                r#"
                SELECT id, tenant_id, provider_family, policy_name, corridor_code, entity_type,
                       risk_tier, partner_key, asset_code, amount_min, amount_max, fallback_order,
                       scorecard, provider_weights, lifecycle_state, metadata
                FROM provider_routing_policies
                WHERE provider_family = $1
                  AND (tenant_id = $2 OR tenant_id IS NULL)
                ORDER BY tenant_id DESC NULLS LAST, policy_name ASC
                "#,
            )
            .bind(provider_family.as_str())
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ProviderRoutingPolicyRow>(
                r#"
                SELECT id, tenant_id, provider_family, policy_name, corridor_code, entity_type,
                       risk_tier, partner_key, asset_code, amount_min, amount_max, fallback_order,
                       scorecard, provider_weights, lifecycle_state, metadata
                FROM provider_routing_policies
                WHERE provider_family = $1
                ORDER BY policy_name ASC
                "#,
            )
            .bind(provider_family.as_str())
            .fetch_all(&self.pool)
            .await?
        };

        rows.into_iter().map(row_to_policy).collect()
    }

    pub async fn select_policy(
        &self,
        tenant_id: Option<&str>,
        query: &ProviderRoutingQuery,
    ) -> Result<Option<ProviderRoutingPolicy>, sqlx::Error> {
        let policies = self
            .list_policies(tenant_id, query.provider_family.clone())
            .await?;

        Ok(policies
            .into_iter()
            .filter(|policy| policy.lifecycle_state.eq_ignore_ascii_case("active"))
            .max_by_key(|policy| evaluate_policy_match(policy, query)))
    }
}

fn row_to_policy(row: ProviderRoutingPolicyRow) -> Result<ProviderRoutingPolicy, sqlx::Error> {
    Ok(ProviderRoutingPolicy {
        policy_id: row.id,
        tenant_id: row.tenant_id,
        provider_family: parse_provider_family(&row.provider_family).map_err(sqlx::Error::Decode)?,
        policy_name: row.policy_name,
        corridor_code: row.corridor_code,
        entity_type: row.entity_type,
        risk_tier: row.risk_tier,
        partner_key: row.partner_key,
        asset_code: row.asset_code,
        amount_min: row.amount_min,
        amount_max: row.amount_max,
        fallback_order: row
            .fallback_order
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        scorecard: row.scorecard,
        provider_weights: row.provider_weights,
        lifecycle_state: row.lifecycle_state,
        metadata: row.metadata,
    })
}

fn parse_provider_family(
    value: &str,
) -> Result<ProviderFamily, Box<dyn std::error::Error + Send + Sync>> {
    match value.trim().to_ascii_lowercase().as_str() {
        "kyc" => Ok(ProviderFamily::Kyc),
        "kyb" => Ok(ProviderFamily::Kyb),
        "kyt" => Ok(ProviderFamily::Kyt),
        "sanctions" => Ok(ProviderFamily::Sanctions),
        "adverse_media" => Ok(ProviderFamily::AdverseMedia),
        "travel_rule" => Ok(ProviderFamily::TravelRule),
        other => Err(format!("unsupported provider family '${other}'").into()),
    }
}

fn evaluate_policy_match(policy: &ProviderRoutingPolicy, query: &ProviderRoutingQuery) -> i32 {
    let mut score = 0;
    score += string_match_score(policy.corridor_code.as_deref(), query.corridor_code.as_deref(), 8);
    score += string_match_score(policy.entity_type.as_deref(), query.entity_type.as_deref(), 5);
    score += string_match_score(policy.risk_tier.as_deref(), query.risk_tier.as_deref(), 5);
    score += string_match_score(policy.partner_key.as_deref(), query.partner_key.as_deref(), 6);
    score += string_match_score(policy.asset_code.as_deref(), query.asset_code.as_deref(), 4);

    if matches_amount(policy.amount_min, policy.amount_max, query.amount) {
        score += 4;
    } else if policy.amount_min.is_some() || policy.amount_max.is_some() {
        score -= 100;
    }

    score
}

fn string_match_score(policy_value: Option<&str>, query_value: Option<&str>, exact_score: i32) -> i32 {
    match (policy_value, query_value) {
        (Some(policy_value), Some(query_value))
            if policy_value.eq_ignore_ascii_case(query_value) => exact_score,
        (Some(_), Some(_)) => -100,
        (None, _) => 1,
        (Some(_), None) => 0,
    }
}

fn matches_amount(
    amount_min: Option<Decimal>,
    amount_max: Option<Decimal>,
    amount: Option<Decimal>,
) -> bool {
    match amount {
        Some(amount) => {
            if let Some(amount_min) = amount_min {
                if amount < amount_min {
                    return false;
                }
            }
            if let Some(amount_max) = amount_max {
                if amount > amount_max {
                    return false;
                }
            }
            true
        }
        None => amount_min.is_none() && amount_max.is_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_corridor_and_partner_match_beats_generic_policy() {
        let generic = ProviderRoutingPolicy {
            policy_id: "policy_generic".to_string(),
            tenant_id: Some("tenant".to_string()),
            provider_family: ProviderFamily::Sanctions,
            policy_name: "generic".to_string(),
            corridor_code: None,
            entity_type: None,
            risk_tier: None,
            partner_key: None,
            asset_code: None,
            amount_min: None,
            amount_max: None,
            fallback_order: vec!["provider_a".to_string()],
            scorecard: serde_json::json!({}),
            provider_weights: serde_json::json!({}),
            lifecycle_state: "active".to_string(),
            metadata: serde_json::json!({}),
        };

        let specific = ProviderRoutingPolicy {
            policy_id: "policy_specific".to_string(),
            tenant_id: Some("tenant".to_string()),
            provider_family: ProviderFamily::Sanctions,
            policy_name: "specific".to_string(),
            corridor_code: Some("VN_SG_PAYOUT".to_string()),
            entity_type: Some("business".to_string()),
            risk_tier: Some("high".to_string()),
            partner_key: Some("partner_scb".to_string()),
            asset_code: Some("USDT".to_string()),
            amount_min: Some(Decimal::new(100, 0)),
            amount_max: Some(Decimal::new(5000, 0)),
            fallback_order: vec!["provider_b".to_string()],
            scorecard: serde_json::json!({}),
            provider_weights: serde_json::json!({}),
            lifecycle_state: "active".to_string(),
            metadata: serde_json::json!({}),
        };

        let query = ProviderRoutingQuery {
            provider_family: ProviderFamily::Sanctions,
            corridor_code: Some("VN_SG_PAYOUT".to_string()),
            entity_type: Some("business".to_string()),
            risk_tier: Some("high".to_string()),
            partner_key: Some("partner_scb".to_string()),
            asset_code: Some("USDT".to_string()),
            amount: Some(Decimal::new(1000, 0)),
        };

        assert!(evaluate_policy_match(&specific, &query) > evaluate_policy_match(&generic, &query));
    }

    #[test]
    fn amount_out_of_range_penalizes_policy() {
        let policy = ProviderRoutingPolicy {
            policy_id: "policy_amount".to_string(),
            tenant_id: Some("tenant".to_string()),
            provider_family: ProviderFamily::Kyt,
            policy_name: "amount-bounded".to_string(),
            corridor_code: None,
            entity_type: None,
            risk_tier: None,
            partner_key: None,
            asset_code: None,
            amount_min: Some(Decimal::new(1000, 0)),
            amount_max: Some(Decimal::new(2000, 0)),
            fallback_order: vec!["provider_a".to_string()],
            scorecard: serde_json::json!({}),
            provider_weights: serde_json::json!({}),
            lifecycle_state: "active".to_string(),
            metadata: serde_json::json!({}),
        };

        let query = ProviderRoutingQuery {
            provider_family: ProviderFamily::Kyt,
            corridor_code: None,
            entity_type: None,
            risk_tier: None,
            partner_key: None,
            asset_code: None,
            amount: Some(Decimal::new(2500, 0)),
        };

        assert!(evaluate_policy_match(&policy, &query) < 0);
    }

    #[tokio::test]
    async fn db_gated_upsert_and_select_policy() {
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

        let store = ProviderRoutingPolicyStore::new(pool);
        store
            .upsert_policy(&UpsertProviderRoutingPolicyRequest {
                policy_id: "provider_policy_vn_sg".to_string(),
                tenant_id: Some("tenant_provider_policy".to_string()),
                provider_family: ProviderFamily::TravelRule,
                policy_name: "travel-rule-vn-sg".to_string(),
                corridor_code: Some("VN_SG_PAYOUT".to_string()),
                entity_type: Some("business".to_string()),
                risk_tier: Some("high".to_string()),
                partner_key: Some("partner_scb".to_string()),
                asset_code: Some("USDT".to_string()),
                amount_min: Some(Decimal::new(100, 0)),
                amount_max: Some(Decimal::new(5000, 0)),
                fallback_order: vec!["notabene".to_string(), "trisa".to_string()],
                scorecard: serde_json::json!({"latencyWeight": 0.6}),
                provider_weights: serde_json::json!({"notabene": 10, "trisa": 7}),
                lifecycle_state: "active".to_string(),
                metadata: serde_json::json!({"phase": "m3"}),
            })
            .await
            .expect("policy upsert should succeed");

        let selected = store
            .select_policy(
                Some("tenant_provider_policy"),
                &ProviderRoutingQuery {
                    provider_family: ProviderFamily::TravelRule,
                    corridor_code: Some("VN_SG_PAYOUT".to_string()),
                    entity_type: Some("business".to_string()),
                    risk_tier: Some("high".to_string()),
                    partner_key: Some("partner_scb".to_string()),
                    asset_code: Some("USDT".to_string()),
                    amount: Some(Decimal::new(1000, 0)),
                },
            )
            .await
            .expect("selection should succeed")
            .expect("policy should match");

        assert_eq!(selected.policy_id, "provider_policy_vn_sg");
        assert_eq!(selected.fallback_order, vec!["notabene".to_string(), "trisa".to_string()]);
    }
}
