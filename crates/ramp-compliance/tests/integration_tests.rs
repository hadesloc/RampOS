use ramp_common::types::{IntentId, TenantId, UserId};
use ramp_compliance::{
    case::{CaseManager, NoteType},
    store::postgres::PostgresCaseStore,
    types::{CaseSeverity, CaseStatus, CaseType},
};
use sqlx::PgPool;
use std::sync::Arc;

// Note: This test requires a running PostgreSQL instance and will be skipped if DATABASE_URL is not set
#[tokio::test]
#[ignore]
async fn test_case_store_integration() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Initialize store and manager
    let store = Arc::new(PostgresCaseStore::new(pool.clone()));
    let manager = CaseManager::new(store);

    let tenant_id = TenantId::new("test_tenant");
    let user_id = UserId::new("test_user");
    let intent_id = IntentId::new_payin();

    // 1. Create a case
    let case_id = manager
        .create_case(
            &tenant_id,
            Some(&user_id),
            Some(&intent_id),
            CaseType::LargeTransaction,
            CaseSeverity::High,
            serde_json::json!({"amount": 1000000000}),
        )
        .await
        .expect("Failed to create case");

    // 2. Verify case was created
    let cases = manager
        .get_user_cases(&tenant_id, &user_id)
        .await
        .expect("Failed to get user cases");
    assert!(!cases.is_empty());
    let case = cases
        .iter()
        .find(|c| c.id == case_id)
        .expect("Case not found");
    assert_eq!(case.status, CaseStatus::Open);
    assert_eq!(case.case_type, CaseType::LargeTransaction);

    // 3. Add a note
    let _note = manager
        .note_manager
        .add_note(
            &tenant_id,
            &case_id,
            Some("analyst_1".to_string()),
            "Investigating large transaction".to_string(),
            NoteType::Comment,
            true,
        )
        .await
        .expect("Failed to add note");

    // 4. Update status
    manager
        .update_status(
            &tenant_id,
            &case_id,
            CaseStatus::Review,
            Some("analyst_1".to_string()),
        )
        .await
        .expect("Failed to update status");

    // 5. Verify status update and auto-note
    let updated_cases = manager
        .get_user_cases(&tenant_id, &user_id)
        .await
        .expect("Failed to get user cases");
    let updated_case = updated_cases
        .iter()
        .find(|c| c.id == case_id)
        .expect("Case not found");
    assert_eq!(updated_case.status, CaseStatus::Review);

    let notes = manager
        .note_manager
        .get_notes(&tenant_id, &case_id)
        .await
        .expect("Failed to get notes");
    // Should have: initial comment + status change note
    assert!(notes.len() >= 2);

    // 6. Resolve case
    manager
        .resolve_case(
            &tenant_id,
            &case_id,
            "False positive, user provided documentation",
            CaseStatus::Closed,
            Some("analyst_1".to_string()),
        )
        .await
        .expect("Failed to resolve case");

    // 7. Verify resolution
    let final_cases = manager
        .get_user_cases(&tenant_id, &user_id)
        .await
        .expect("Failed to get user cases");
    let final_case = final_cases
        .iter()
        .find(|c| c.id == case_id)
        .expect("Case not found");
    assert_eq!(final_case.status, CaseStatus::Closed);
    assert!(final_case.resolved_at.is_some());
    assert_eq!(
        final_case.resolution.as_deref(),
        Some("False positive, user provided documentation")
    );
}
