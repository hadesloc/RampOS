use ramp_common::types::TenantId;
use ramp_core::repository::set_rls_context;
use serde_json::Value;
use sqlx::PgPool;
use tracing::warn;

pub async fn record(
    pool: &PgPool,
    tenant_id: &str,
    actor_id: Option<&str>,
    action: &str,
    details: Value,
) {
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => {
            warn!(error = %error, action, "Failed to start portal auth audit transaction");
            return;
        }
    };
    let tenant = TenantId(tenant_id.to_string());
    if let Err(error) = set_rls_context(&mut transaction, &tenant).await {
        warn!(error = %error, action, "Failed to set portal auth audit tenant");
        return;
    }

    let result = sqlx::query(
        r#"
        INSERT INTO audit_log (
            tenant_id,
            actor_type,
            actor_id,
            action,
            resource_type,
            resource_id,
            details,
            entry_hash
        ) VALUES ($1, $2, $3, $4, 'PORTAL_AUTH', $3, $5, $6)
        "#,
    )
    .bind(tenant_id)
    .bind(if actor_id.is_some() { "USER" } else { "SYSTEM" })
    .bind(actor_id)
    .bind(action)
    .bind(details)
    .bind(format!("portal-auth-{}", uuid::Uuid::new_v4()))
    .execute(&mut *transaction)
    .await;

    if let Err(error) = result {
        warn!(error = %error, action, "Failed to persist portal auth audit event");
        return;
    }
    if let Err(error) = transaction.commit().await {
        warn!(error = %error, action, "Failed to commit portal auth audit event");
    }
}
