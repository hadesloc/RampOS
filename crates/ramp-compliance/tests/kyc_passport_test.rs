use ramp_compliance::{passport_summary_from_flags, PassportService};

#[test]
fn passport_summary_is_built_from_risk_flags() {
    let summary = passport_summary_from_flags(&serde_json::json!({
        "passport": {
            "packageId": "pkg_passport_001",
            "sourceTenantId": "tenant_origin",
            "status": "available",
            "consentStatus": "granted",
            "destinationTenantId": "tenant_partner",
            "fieldsShared": ["identity", "sanctions"],
            "expiresAt": "2026-04-01T00:00:00Z",
            "reuseAllowed": true
        }
    }))
    .expect("passport summary");

    assert_eq!(summary.package_id, "pkg_passport_001");
    assert_eq!(summary.consent_status, "granted");
    assert!(summary.reuse_allowed);
}

#[test]
fn passport_service_lists_review_queue() {
    let queue = PassportService::new().list_queue(None);

    assert!(!queue.is_empty());
    assert!(queue.iter().any(|item| item.review_status == "pending_review"));
}
