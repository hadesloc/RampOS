use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct PortalIdentity {
    pub portal_user_id: String,
    pub financial_user_id: String,
    pub tenant_id: String,
    pub email: Option<String>,
    pub wallet_address: Option<String>,
    pub auth_methods: Vec<String>,
    pub kyc_status: String,
    pub kyc_tier: i16,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn normalize_wallet(wallet_address: &str) -> String {
    wallet_address.trim().to_lowercase()
}

pub async fn find_identity_by_email(
    pool: &PgPool,
    tenant_id: &str,
    email: &str,
) -> Result<Option<PortalIdentity>, sqlx::Error> {
    let normalized = normalize_email(email);
    sqlx::query_as::<_, PortalIdentity>(
        r#"
        SELECT
            portal.id AS portal_user_id,
            portal.financial_user_id,
            portal.tenant_id,
            portal.email,
            portal.wallet_address,
            portal.auth_methods,
            financial.kyc_status,
            financial.kyc_tier,
            financial.status,
            portal.created_at
        FROM portal_users portal
        JOIN users financial
          ON financial.tenant_id = portal.tenant_id
         AND financial.id = portal.financial_user_id
        WHERE portal.tenant_id = $1
          AND portal.email_normalized = $2
        "#,
    )
    .bind(tenant_id)
    .bind(normalized)
    .fetch_optional(pool)
    .await
}

pub async fn find_identity_by_wallet(
    pool: &PgPool,
    tenant_id: &str,
    wallet_address: &str,
) -> Result<Option<PortalIdentity>, sqlx::Error> {
    let normalized = normalize_wallet(wallet_address);
    sqlx::query_as::<_, PortalIdentity>(
        r#"
        SELECT
            portal.id AS portal_user_id,
            portal.financial_user_id,
            portal.tenant_id,
            portal.email,
            portal.wallet_address,
            portal.auth_methods,
            financial.kyc_status,
            financial.kyc_tier,
            financial.status,
            portal.created_at
        FROM portal_users portal
        JOIN users financial
          ON financial.tenant_id = portal.tenant_id
         AND financial.id = portal.financial_user_id
        WHERE portal.tenant_id = $1
          AND lower(portal.wallet_address) = $2
        "#,
    )
    .bind(tenant_id)
    .bind(normalized)
    .fetch_optional(pool)
    .await
}

pub async fn create_password_identity(
    pool: &PgPool,
    tenant_id: &str,
    email: &str,
    password_hash: &str,
    full_name: Option<&str>,
) -> Result<PortalIdentity, sqlx::Error> {
    let normalized_email = normalize_email(email);
    let identity_id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO users (
            id, tenant_id, kyc_tier, kyc_status, status, risk_flags,
            created_at, updated_at, email, auth_method
        ) VALUES (
            $1, $2, 0, 'PENDING', 'ACTIVE', '[]'::jsonb,
            NOW(), NOW(), $3, 'password'
        )
        "#,
    )
    .bind(&identity_id)
    .bind(tenant_id)
    .bind(&normalized_email)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO portal_users (
            id, email, tenant_id, kyc_status, kyc_tier, status,
            created_at, updated_at, financial_user_id, email_normalized,
            password_hash, auth_methods, full_name, last_password_change
        ) VALUES (
            $1, $2, $3, 'PENDING', 0, 'ACTIVE',
            NOW(), NOW(), $1, $2, $4, ARRAY['password']::TEXT[], $5, NOW()
        )
        "#,
    )
    .bind(&identity_id)
    .bind(&normalized_email)
    .bind(tenant_id)
    .bind(password_hash)
    .bind(full_name)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    find_identity_by_email(pool, tenant_id, &normalized_email)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

pub async fn resolve_or_create_wallet_identity(
    pool: &PgPool,
    tenant_id: &str,
    wallet_address: &str,
) -> Result<PortalIdentity, sqlx::Error> {
    let normalized_wallet = normalize_wallet(wallet_address);
    if let Some(identity) = find_identity_by_wallet(pool, tenant_id, &normalized_wallet).await? {
        return Ok(identity);
    }

    let mut tx = pool.begin().await?;

    #[derive(FromRow)]
    struct FinancialIdentity {
        id: String,
    }

    let financial = sqlx::query_as::<_, FinancialIdentity>(
        r#"
        SELECT id
        FROM users
        WHERE tenant_id = $1
          AND lower(wallet_address) = $2
        FOR UPDATE
        "#,
    )
    .bind(tenant_id)
    .bind(&normalized_wallet)
    .fetch_optional(&mut *tx)
    .await?;

    let identity_id = if let Some(financial) = financial {
        financial.id
    } else {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO users (
                id, tenant_id, kyc_tier, kyc_status, status, risk_flags,
                created_at, updated_at, wallet_address, auth_method
            ) VALUES (
                $1, $2, 0, 'PENDING', 'ACTIVE', '[]'::jsonb,
                NOW(), NOW(), $3, 'wallet'
            )
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&normalized_wallet)
        .execute(&mut *tx)
        .await?;
        id
    };

    sqlx::query(
        r#"
        INSERT INTO portal_users (
            id, email, tenant_id, kyc_status, kyc_tier, status,
            created_at, updated_at, financial_user_id, wallet_address,
            auth_methods
        )
        SELECT
            financial.id,
            financial.email,
            financial.tenant_id,
            financial.kyc_status,
            financial.kyc_tier,
            financial.status,
            financial.created_at,
            financial.updated_at,
            financial.id,
            $3,
            ARRAY['wallet']::TEXT[]
        FROM users financial
        WHERE financial.tenant_id = $1
          AND financial.id = $2
        ON CONFLICT (id) DO UPDATE
        SET
            wallet_address = EXCLUDED.wallet_address,
            auth_methods = (
                SELECT ARRAY(
                    SELECT DISTINCT method
                    FROM unnest(
                        portal_users.auth_methods || ARRAY['wallet']::TEXT[]
                    ) AS method
                    ORDER BY method
                )
            ),
            updated_at = NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(&identity_id)
    .bind(&normalized_wallet)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    find_identity_by_wallet(pool, tenant_id, &normalized_wallet)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

pub async fn link_wallet_to_identity(
    pool: &PgPool,
    tenant_id: &str,
    portal_user_id: &str,
    wallet_address: &str,
) -> Result<PortalIdentity, sqlx::Error> {
    let normalized_wallet = normalize_wallet(wallet_address);
    let mut tx = pool.begin().await?;

    let financial_user_id: String = sqlx::query_scalar(
        r#"
        SELECT financial_user_id
        FROM portal_users
        WHERE tenant_id = $1 AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(tenant_id)
    .bind(portal_user_id)
    .fetch_one(&mut *tx)
    .await?;

    let owned_elsewhere: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM portal_users
            WHERE tenant_id = $1
              AND lower(wallet_address) = $2
              AND id <> $3
        ) OR EXISTS (
            SELECT 1
            FROM users
            WHERE tenant_id = $1
              AND lower(wallet_address) = $2
              AND id <> $4
        )
        "#,
    )
    .bind(tenant_id)
    .bind(&normalized_wallet)
    .bind(portal_user_id)
    .bind(&financial_user_id)
    .fetch_one(&mut *tx)
    .await?;
    if owned_elsewhere {
        return Err(sqlx::Error::Protocol(
            "wallet address is already linked".to_string(),
        ));
    }

    sqlx::query(
        r#"
        UPDATE portal_users
        SET
            wallet_address = $1,
            auth_methods = (
                SELECT ARRAY(
                    SELECT DISTINCT method
                    FROM unnest(auth_methods || ARRAY['wallet']::TEXT[]) AS method
                    ORDER BY method
                )
            ),
            updated_at = NOW()
        WHERE tenant_id = $2 AND id = $3
        "#,
    )
    .bind(&normalized_wallet)
    .bind(tenant_id)
    .bind(portal_user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE users
        SET
            wallet_address = $1,
            auth_method = CASE
                WHEN auth_method = 'password' THEN 'password+wallet'
                ELSE 'wallet'
            END,
            updated_at = NOW()
        WHERE tenant_id = $2 AND id = $3
        "#,
    )
    .bind(&normalized_wallet)
    .bind(tenant_id)
    .bind(&financial_user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    find_identity_by_wallet(pool, tenant_id, &normalized_wallet)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}
