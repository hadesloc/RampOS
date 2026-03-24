use ramp_core::repository::{
    CommercializationPackRepository, PgCommercializationPackRepository,
    UpsertCommercializationPackRequest,
};
use sqlx::PgPool;

#[tokio::test]
async fn commercialization_pack_repository_persists_reference_record() {
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

    sqlx::query(
        r#"
        INSERT INTO partners (
            id,
            tenant_id,
            partner_class,
            code,
            display_name,
            service_domain,
            lifecycle_state,
            approval_status,
            metadata
        ) VALUES (
            'partner_pack_repo',
            'tenant_pack_repo',
            'issuer',
            'pack-repo',
            'Pack Repo Partner',
            'card_distribution',
            'active',
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("partner seed should succeed");

    sqlx::query(
        r#"
        INSERT INTO partner_capabilities (
            id,
            partner_id,
            capability_family,
            environment,
            supported_rails,
            supported_methods,
            approval_status,
            metadata
        ) VALUES (
            'capability_pack_repo',
            'partner_pack_repo',
            'card_issuing',
            'production',
            '["fps"]'::jsonb,
            '["push_transfer"]'::jsonb,
            'approved',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("partner capability seed should succeed");

    let repository = PgCommercializationPackRepository::new(pool);
    repository
        .upsert_commercialization_pack(&UpsertCommercializationPackRequest {
            commercialization_pack_id: "pack_repo_vn_hk".to_string(),
            tenant_id: Some("tenant_pack_repo".to_string()),
            pack_code: "pilot_vn_hk_repo".to_string(),
            partner_id: "partner_pack_repo".to_string(),
            partner_capability_id: "capability_pack_repo".to_string(),
            commercial_extension_id: "card_payout".to_string(),
            corridor_code: "VN_HK_PAYOUT".to_string(),
            approval_reference: None,
            lifecycle_state: "active".to_string(),
            rollout_state: "approved".to_string(),
            metadata: serde_json::json!({"source":"repo_test"}),
        })
        .await
        .expect("commercialization pack should persist");

    let rows = repository
        .list_commercialization_packs(Some("tenant_pack_repo"))
        .await
        .expect("list should load");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].pack_code, "pilot_vn_hk_repo");
    assert_eq!(rows[0].corridor_code, "VN_HK_PAYOUT");

    let detail = repository
        .get_commercialization_pack(Some("tenant_pack_repo"), "pilot_vn_hk_repo")
        .await
        .expect("detail should load")
        .expect("pack should exist");
    assert_eq!(detail.commercial_extension_id, "card_payout");
}
