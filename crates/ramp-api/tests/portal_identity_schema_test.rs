use sqlx::{PgPool, Row};

#[tokio::test]
async fn unified_portal_identity_schema_is_installed() {
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

    let installed = sqlx::query_scalar::<_, i64>(
        "SELECT version::bigint FROM _sqlx_migrations WHERE version = 69",
    )
    .fetch_optional(&pool)
    .await
    .expect("migration metadata should load");
    assert_eq!(installed, Some(69), "migration 069 must be installed");

    assert_table_columns(
        &pool,
        "portal_users",
        &[
            "financial_user_id",
            "email_normalized",
            "password_hash",
            "wallet_address",
            "auth_methods",
            "full_name",
            "phone",
            "avatar_url",
            "two_factor_enabled",
            "last_password_change",
            "email_notifications",
            "sms_notifications",
            "push_notifications",
        ],
    )
    .await;

    assert_table_columns(
        &pool,
        "refresh_tokens",
        &[
            "tenant_id",
            "rotated_to_id",
            "revoked_at",
            "revoke_reason",
            "updated_at",
        ],
    )
    .await;

    assert_table_columns(&pool, "portal_auth_nonces", &["purpose", "portal_user_id"]).await;

    for index_name in [
        "idx_portal_users_normalized_email",
        "idx_portal_users_wallet",
        "idx_portal_users_financial_user",
        "idx_refresh_tokens_active_family",
    ] {
        assert_index_exists(&pool, index_name).await;
    }

    for (table_name, constraint_name) in [
        ("portal_users", "portal_users_financial_user_fk"),
        ("portal_users", "portal_users_auth_methods_check"),
        ("refresh_tokens", "refresh_tokens_rotated_to_fk"),
        ("portal_auth_nonces", "portal_auth_nonces_purpose_check"),
        ("portal_auth_nonces", "portal_auth_nonces_portal_user_fk"),
    ] {
        assert_constraint_exists(&pool, table_name, constraint_name).await;
    }
}

async fn assert_table_columns(pool: &PgPool, table_name: &str, expected_columns: &[&str]) {
    let rows = sqlx::query(
        r#"
        SELECT column_name
        FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = $1
        ORDER BY ordinal_position
        "#,
    )
    .bind(table_name)
    .fetch_all(pool)
    .await
    .expect("table columns should load");

    let actual_columns: Vec<String> = rows
        .into_iter()
        .map(|row| row.get::<String, _>("column_name"))
        .collect();

    for expected_column in expected_columns {
        assert!(
            actual_columns
                .iter()
                .any(|column| column == expected_column),
            "expected {table_name}.{expected_column}; found {actual_columns:?}"
        );
    }
}

async fn assert_index_exists(pool: &PgPool, index_name: &str) {
    let installed = sqlx::query_scalar::<_, String>(
        r#"
        SELECT indexname
        FROM pg_indexes
        WHERE schemaname = 'public' AND indexname = $1
        "#,
    )
    .bind(index_name)
    .fetch_optional(pool)
    .await
    .expect("index metadata should load");

    assert_eq!(installed.as_deref(), Some(index_name));
}

async fn assert_constraint_exists(pool: &PgPool, table_name: &str, constraint_name: &str) {
    let installed = sqlx::query_scalar::<_, String>(
        r#"
        SELECT con.conname
        FROM pg_constraint con
        JOIN pg_class rel ON rel.oid = con.conrelid
        JOIN pg_namespace nsp ON nsp.oid = rel.relnamespace
        WHERE nsp.nspname = 'public'
          AND rel.relname = $1
          AND con.conname = $2
        "#,
    )
    .bind(table_name)
    .bind(constraint_name)
    .fetch_optional(pool)
    .await
    .expect("constraint metadata should load");

    assert_eq!(installed.as_deref(), Some(constraint_name));
}
