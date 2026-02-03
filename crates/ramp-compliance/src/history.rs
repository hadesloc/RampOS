use chrono::{DateTime, Duration, Utc};
use ramp_common::{
    types::{IntentId, TenantId, UserId},
    Result,
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreHistory {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub intent_id: Option<IntentId>,
    pub score: Decimal,
    pub triggered_rules: serde_json::Value,
    pub action_taken: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreTrend {
    Improving,
    Stable,
    Declining,
    InsufficientData,
}

pub struct ScoreHistoryManager {
    pool: PgPool,
}

impl ScoreHistoryManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn record(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        intent_id: Option<&IntentId>,
        score: Decimal,
        rules: Vec<String>,
        action: Option<String>,
    ) -> Result<()> {
        let rules_json = serde_json::to_value(rules).unwrap_or(serde_json::json!([]));
        let tenant_id_str = tenant_id.to_string();
        let user_id_str = user_id.to_string();
        let intent_id_str = intent_id.map(|id| id.to_string());
        let id = Uuid::now_v7();

        sqlx::query(
            r#"
            INSERT INTO risk_score_history (
                id, tenant_id, user_id, intent_id, score, triggered_rules, action_taken
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#
        )
        .bind(id)
        .bind(tenant_id_str)
        .bind(user_id_str)
        .bind(intent_id_str)
        .bind(score)
        .bind(rules_json)
        .bind(action)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn get_user_history(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        limit: i64,
    ) -> Result<Vec<ScoreHistory>> {
        let tenant_id_str = tenant_id.to_string();
        let user_id_str = user_id.to_string();

        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, user_id, intent_id, score, triggered_rules, action_taken, created_at
            FROM risk_score_history
            WHERE tenant_id = $1 AND user_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(tenant_id_str)
        .bind(user_id_str)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let history = rows
            .into_iter()
            .map(|row| {
                Ok(ScoreHistory {
                    id: row.try_get("id")?,
                    tenant_id: TenantId::new(row.try_get::<String, _>("tenant_id")?),
                    user_id: UserId::new(row.try_get::<String, _>("user_id")?),
                    intent_id: row
                        .try_get::<Option<String>, _>("intent_id")?
                        .map(IntentId::new),
                    score: row.try_get("score")?,
                    triggered_rules: row.try_get("triggered_rules")?,
                    action_taken: row.try_get("action_taken")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect::<std::result::Result<Vec<_>, sqlx::Error>>()
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(history)
    }

    pub async fn get_average_score(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        days: i64,
    ) -> Result<f64> {
        let tenant_id_str = tenant_id.to_string();
        let user_id_str = user_id.to_string();
        let since = Utc::now() - Duration::days(days);

        let row = sqlx::query(
            r#"
            SELECT AVG(score) as avg_score
            FROM risk_score_history
            WHERE tenant_id = $1 AND user_id = $2 AND created_at >= $3
            "#,
        )
        .bind(tenant_id_str)
        .bind(user_id_str)
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let avg_score: Option<Decimal> = row.try_get("avg_score").unwrap_or(None);

        Ok(avg_score.and_then(|v| v.to_f64()).unwrap_or(0.0))
    }

    pub async fn detect_trend(&self, tenant_id: &TenantId, user_id: &UserId) -> Result<ScoreTrend> {
        // Get last 10 scores
        let history = self.get_user_history(tenant_id, user_id, 10).await?;
        Ok(Self::calculate_trend_internal(&history))
    }

    fn calculate_trend_internal(history: &[ScoreHistory]) -> ScoreTrend {
        if history.len() < 3 {
            return ScoreTrend::InsufficientData;
        }

        let mid = history.len() / 2;
        let (recent, older) = history.split_at(mid);

        let recent_sum: Decimal = recent.iter().map(|h| h.score).sum();
        let older_sum: Decimal = older.iter().map(|h| h.score).sum();

        let recent_avg = if !recent.is_empty() {
            recent_sum / Decimal::from(recent.len())
        } else {
            Decimal::ZERO
        };
        let older_avg = if !older.is_empty() {
            older_sum / Decimal::from(older.len())
        } else {
            Decimal::ZERO
        };

        let diff = recent_avg - older_avg;

        if diff < Decimal::from(-5) {
            ScoreTrend::Improving // Score went down (risk reduced)
        } else if diff > Decimal::from(5) {
            ScoreTrend::Declining // Score went up (risk increased)
        } else {
            ScoreTrend::Stable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use rust_decimal::dec; // Unused

    #[test]
    fn test_trend_calculation() {
        let now = Utc::now();
        let tenant_id = TenantId::new("tenant1");
        let user_id = UserId::new("user1");

        let create_record = |score: i64, hours_ago: i64| ScoreHistory {
            id: Uuid::new_v4(),
            tenant_id: tenant_id.clone(),
            user_id: user_id.clone(),
            intent_id: None,
            score: Decimal::from(score),
            triggered_rules: serde_json::Value::Null,
            action_taken: None,
            created_at: now - Duration::hours(hours_ago),
        };

        // Case 1: Improving (Score going down)
        // Recent (newer timestamp): 10, 20. Older: 80, 90.
        // History is sorted by created_at DESC (newest first).
        let history = vec![
            create_record(10, 1),
            create_record(20, 2),
            create_record(80, 3),
            create_record(90, 4),
        ];

        assert_eq!(
            ScoreHistoryManager::calculate_trend_internal(&history),
            ScoreTrend::Improving
        );

        // Case 2: Declining (Score going up)
        // Recent: 90, 80. Older: 20, 10.
        let history = vec![
            create_record(90, 1),
            create_record(80, 2),
            create_record(20, 3),
            create_record(10, 4),
        ];

        assert_eq!(
            ScoreHistoryManager::calculate_trend_internal(&history),
            ScoreTrend::Declining
        );

        // Case 3: Stable
        let history = vec![
            create_record(52, 1),
            create_record(50, 2),
            create_record(50, 3),
            create_record(48, 4),
        ];

        assert_eq!(
            ScoreHistoryManager::calculate_trend_internal(&history),
            ScoreTrend::Stable
        );

        // Case 4: Insufficient data
        let history = vec![create_record(50, 1), create_record(50, 2)];

        assert_eq!(
            ScoreHistoryManager::calculate_trend_internal(&history),
            ScoreTrend::InsufficientData
        );
    }
}
