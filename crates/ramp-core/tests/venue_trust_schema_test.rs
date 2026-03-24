use sqlx::{PgPool, Row};

#[tokio::test]
async fn venue_trust_schema_migrations_install_generic_foundation() {
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

    let installed_versions: Vec<i64> = sqlx::query_scalar(
        r#"
        SELECT version::bigint
        FROM _sqlx_migrations
        WHERE version BETWEEN 59 AND 63
        ORDER BY version
        "#,
    )
    .fetch_all(&pool)
    .await
    .expect("migration versions should load");
    assert_eq!(installed_versions, vec![59, 60, 61, 62, 63]);

    assert_table_columns(
        &pool,
        "venue_connections",
        &[
            "id",
            "tenant_id",
            "subject_type",
            "subject_id",
            "user_id",
            "venue_key",
            "connection_mode",
            "status",
            "metadata",
            "last_verified_at",
            "created_at",
            "updated_at",
        ],
    )
    .await;

    assert_table_columns(
        &pool,
        "venue_accounts",
        &[
            "id",
            "tenant_id",
            "venue_connection_id",
            "venue_key",
            "account_label",
            "account_ref",
            "wallet_address",
            "subaccount_ref",
            "api_scope_summary",
            "status",
            "metadata",
            "last_verified_at",
            "created_at",
            "updated_at",
        ],
    )
    .await;

    assert_table_columns(
        &pool,
        "beneficiary_profiles",
        &[
            "id",
            "tenant_id",
            "subject_type",
            "subject_id",
            "user_id",
            "destination_type",
            "destination_ref",
            "display_name",
            "asset_symbol",
            "network",
            "verification_status",
            "cooldown_ends_at",
            "metadata",
            "last_verified_at",
            "created_at",
            "updated_at",
        ],
    )
    .await;

    assert_table_columns(
        &pool,
        "venue_transfers",
        &[
            "id",
            "tenant_id",
            "user_id",
            "beneficiary_profile_id",
            "wallet_attestation_id",
            "venue_connection_id",
            "venue_account_id",
            "transfer_direction",
            "asset_symbol",
            "network",
            "amount",
            "origin_intent_id",
            "rfq_id",
            "status",
            "wallet_tx_hash",
            "venue_credit_ref",
            "failure_code",
            "metadata",
            "submitted_at",
            "completed_at",
            "created_at",
            "updated_at",
        ],
    )
    .await;

    assert_table_columns(
        &pool,
        "source_of_funds_packages",
        &[
            "id",
            "tenant_id",
            "subject_type",
            "subject_id",
            "wallet_attestation_id",
            "venue_account_id",
            "venue_transfer_id",
            "review_status",
            "package_uri",
            "metadata",
            "reviewed_at",
            "created_at",
            "updated_at",
        ],
    )
    .await;

    for table_name in [
        "venue_connections",
        "venue_accounts",
        "beneficiary_profiles",
        "venue_transfers",
        "source_of_funds_packages",
    ] {
        assert_rls_enabled(&pool, table_name).await;
    }

    for index_name in [
        "idx_venue_connections_tenant_subject",
        "idx_venue_accounts_connection_status",
        "idx_beneficiary_profiles_tenant_subject",
        "idx_venue_transfers_tenant_status",
        "idx_source_of_funds_packages_tenant_subject",
    ] {
        assert_index_exists(&pool, index_name).await;
    }

    for (table_name, constraint_name) in [
        ("venue_connections", "venue_connections_subject_check"),
        ("venue_accounts", "venue_accounts_identity_check"),
        ("beneficiary_profiles", "beneficiary_profiles_subject_check"),
        ("venue_transfers", "venue_transfers_amount_positive"),
        (
            "source_of_funds_packages",
            "source_of_funds_packages_review_status_check",
        ),
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

    assert!(
        !actual_columns.is_empty(),
        "expected table {table_name} to exist"
    );

    for expected_column in expected_columns {
        assert!(
            actual_columns.iter().any(|column| column == expected_column),
            "expected table {table_name} to contain column {expected_column}, found {actual_columns:?}"
        );
    }
}

async fn assert_rls_enabled(pool: &PgPool, table_name: &str) {
    let rowsecurity = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT rowsecurity
        FROM pg_tables
        WHERE schemaname = 'public' AND tablename = $1
        "#,
    )
    .bind(table_name)
    .fetch_optional(pool)
    .await
    .expect("rls state should load")
    .unwrap_or(false);

    assert!(rowsecurity, "expected RLS to be enabled on {table_name}");
}

async fn assert_index_exists(pool: &PgPool, index_name: &str) {
    let installed_index = sqlx::query_scalar::<_, String>(
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

    assert_eq!(
        installed_index.as_deref(),
        Some(index_name),
        "expected index {index_name} to exist"
    );
}

async fn assert_constraint_exists(pool: &PgPool, table_name: &str, constraint_name: &str) {
    let installed_constraint = sqlx::query_scalar::<_, String>(
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

    assert_eq!(
        installed_constraint.as_deref(),
        Some(constraint_name),
        "expected constraint {constraint_name} on table {table_name}"
    );
}
