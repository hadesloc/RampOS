use ramp_api::handlers::portal::auth::identity::{
    create_password_identity, find_identity_by_email, find_identity_by_wallet,
    resolve_or_create_wallet_identity,
};
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn portal_identity_helpers_preserve_tenant_and_financial_ownership() {
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

    let tenant_a = Uuid::new_v4().to_string();
    let tenant_b = Uuid::new_v4().to_string();
    seed_tenant(&pool, &tenant_a).await;
    seed_tenant(&pool, &tenant_b).await;

    let created = create_password_identity(
        &pool,
        &tenant_a,
        "  User@Example.COM ",
        "$argon2id$test-hash",
        Some("Test User"),
    )
    .await
    .expect("password identity should be created");

    assert_eq!(created.portal_user_id, created.financial_user_id);
    assert_eq!(created.tenant_id, tenant_a);
    assert_eq!(created.email.as_deref(), Some("user@example.com"));
    assert_eq!(created.auth_methods, vec!["password"]);

    let loaded = find_identity_by_email(&pool, &tenant_a, "USER@example.com")
        .await
        .expect("email lookup should succeed")
        .expect("identity should exist");
    assert_eq!(loaded.portal_user_id, created.portal_user_id);

    assert!(
        find_identity_by_email(&pool, &tenant_b, "user@example.com")
            .await
            .expect("cross-tenant lookup should succeed")
            .is_none(),
        "identity must not leak across tenants"
    );

    let duplicate = create_password_identity(
        &pool,
        &tenant_a,
        "user@example.com",
        "$argon2id$another-hash",
        None,
    )
    .await;
    assert!(duplicate.is_err(), "normalized duplicate email must fail");
    let financial_email_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE tenant_id = $1 AND email = $2")
            .bind(&tenant_a)
            .bind("user@example.com")
            .fetch_one(&pool)
            .await
            .expect("financial email count should load");
    assert_eq!(
        financial_email_rows, 1,
        "failed portal insert must roll back its financial user"
    );

    let wallet = "0x1563915e194d8cfba1943570603f7606a3115508";
    let wallet_identity = resolve_or_create_wallet_identity(&pool, &tenant_a, wallet)
        .await
        .expect("wallet identity should be created");
    assert_eq!(wallet_identity.wallet_address.as_deref(), Some(wallet));
    assert_eq!(wallet_identity.auth_methods, vec!["wallet"]);

    let loaded_wallet = find_identity_by_wallet(&pool, &tenant_a, &wallet.to_uppercase())
        .await
        .expect("wallet lookup should succeed")
        .expect("wallet identity should exist");
    assert_eq!(loaded_wallet.portal_user_id, wallet_identity.portal_user_id);

    let legacy_financial_id = Uuid::new_v4().to_string();
    let legacy_wallet = "0x70997970c51812dc3a010c7d01b50e0d17dc79c8";
    sqlx::query(
        r#"
        INSERT INTO users (
            id, tenant_id, kyc_tier, kyc_status, status, risk_flags,
            created_at, updated_at, wallet_address, auth_method
        ) VALUES (
            $1, $2, 2, 'VERIFIED', 'ACTIVE', '[]'::jsonb,
            NOW(), NOW(), $3, 'wallet'
        )
        "#,
    )
    .bind(&legacy_financial_id)
    .bind(&tenant_a)
    .bind(legacy_wallet)
    .execute(&pool)
    .await
    .expect("legacy financial wallet should be seeded");

    let reconciled = resolve_or_create_wallet_identity(&pool, &tenant_a, legacy_wallet)
        .await
        .expect("legacy wallet should reconcile");
    assert_eq!(reconciled.financial_user_id, legacy_financial_id);
    assert_eq!(reconciled.portal_user_id, legacy_financial_id);
    assert_eq!(reconciled.kyc_status, "VERIFIED");
    assert_eq!(reconciled.kyc_tier, 2);

    cleanup_tenant(&pool, &tenant_a).await;
    cleanup_tenant(&pool, &tenant_b).await;
}

async fn seed_tenant(pool: &PgPool, tenant_id: &str) {
    sqlx::query(
        r#"
        INSERT INTO tenants (
            id, name, status, api_key_hash, webhook_secret_hash, config
        ) VALUES ($1, $2, 'ACTIVE', $3, $4, '{}'::jsonb)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("Identity test {tenant_id}"))
    .bind(format!("api-{tenant_id}"))
    .bind(format!("webhook-{tenant_id}"))
    .execute(pool)
    .await
    .expect("test tenant should be created");
}

async fn cleanup_tenant(pool: &PgPool, tenant_id: &str) {
    sqlx::query("DELETE FROM refresh_tokens WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("refresh tokens should clean up");
    sqlx::query("DELETE FROM portal_users WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("portal users should clean up");
    sqlx::query("DELETE FROM users WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("financial users should clean up");
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("tenant should clean up");
}
