#[cfg(test)]
mod tests {
    use super::super::licensing::*;
    use crate::error::ApiError;
    use crate::middleware::tenant::{TenantContext, TenantTier};
    use async_trait::async_trait;
    use axum::{extract::{Path, State}, http::HeaderMap, Extension, Json};
    use chrono::Utc;
    use ramp_common::{
        licensing::{LicenseRequirementId, LicenseStatus, LicenseSubmissionId, LicenseType, SubmissionStatus},
        types::TenantId,
        Result,
    };
    use ramp_core::repository::licensing::{
        CreateLicenseRequirementRequest, CreateLicenseSubmissionRequest, LicenseRequirementRow,
        LicenseSubmissionRow, LicensingRepository, TenantLicenseStatusRow,
    };
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockLicensingRepository {
        requirements: Mutex<Vec<LicenseRequirementRow>>,
        statuses: Mutex<Vec<TenantLicenseStatusRow>>,
        submissions: Mutex<Vec<LicenseSubmissionRow>>,
    }

    #[async_trait]
    impl LicensingRepository for MockLicensingRepository {
        async fn list_requirements(&self, _limit: i64, _offset: i64) -> Result<Vec<LicenseRequirementRow>> {
            Ok(self.requirements.lock().unwrap().clone())
        }

        async fn get_requirement(&self, id: &LicenseRequirementId) -> Result<Option<LicenseRequirementRow>> {
            Ok(self.requirements.lock().unwrap().iter().find(|row| row.id == id.0).cloned())
        }

        async fn create_requirement(&self, _req: &CreateLicenseRequirementRequest) -> Result<LicenseRequirementRow> {
            unimplemented!("not needed for tests")
        }

        async fn get_tenant_license_statuses(&self, tenant_id: &TenantId) -> Result<Vec<TenantLicenseStatusRow>> {
            Ok(self.statuses.lock().unwrap().iter().filter(|row| row.tenant_id == tenant_id.0).cloned().collect())
        }

        async fn get_tenant_license_status(&self, tenant_id: &TenantId, requirement_id: &LicenseRequirementId) -> Result<Option<TenantLicenseStatusRow>> {
            Ok(self.statuses.lock().unwrap().iter().find(|row| row.tenant_id == tenant_id.0 && row.requirement_id == requirement_id.0).cloned())
        }

        async fn upsert_tenant_license_status(&self, _tenant_id: &TenantId, _requirement_id: &LicenseRequirementId, _status: LicenseStatus, _license_number: Option<&str>, _expiry_date: Option<chrono::DateTime<Utc>>) -> Result<TenantLicenseStatusRow> {
            unimplemented!("not needed for tests")
        }

        async fn create_submission(&self, req: &CreateLicenseSubmissionRequest) -> Result<LicenseSubmissionRow> {
            let submission = LicenseSubmissionRow {
                id: "sub_001".to_string(),
                tenant_id: req.tenant_id.0.clone(),
                requirement_id: req.requirement_id.0.clone(),
                documents: req.documents.clone(),
                status: "PENDING".to_string(),
                submitted_by: req.submitted_by.clone(),
                submitted_at: Utc::now(),
                reviewed_at: None,
                reviewer_notes: None,
            };
            self.submissions.lock().unwrap().push(submission.clone());
            Ok(submission)
        }

        async fn list_submissions(&self, tenant_id: &TenantId, requirement_id: Option<&LicenseRequirementId>, _limit: i64, _offset: i64) -> Result<Vec<LicenseSubmissionRow>> {
            Ok(self.submissions.lock().unwrap().iter().filter(|row| {
                row.tenant_id == tenant_id.0 && requirement_id.is_none_or(|id| row.requirement_id == id.0)
            }).cloned().collect())
        }

        async fn update_submission_status(&self, _submission_id: &LicenseSubmissionId, _status: SubmissionStatus, _reviewer_notes: Option<&str>) -> Result<()> {
            Ok(())
        }

        async fn get_upcoming_deadlines(&self, _tenant_id: &TenantId, _days_ahead: i32) -> Result<Vec<(LicenseRequirementRow, Option<TenantLicenseStatusRow>)>> {
            Ok(vec![])
        }

        async fn count_requirements(&self) -> Result<i64> {
            Ok(self.requirements.lock().unwrap().len() as i64)
        }
    }

    fn seeded_repo() -> Arc<dyn LicensingRepository> {
        let repo = Arc::new(MockLicensingRepository::default());
        let now = Utc::now();
        repo.requirements.lock().unwrap().push(LicenseRequirementRow {
            id: "req_vasp".to_string(),
            name: "VASP License".to_string(),
            description: "Required".to_string(),
            license_type: LicenseType::SbvPaymentLicense.as_str().to_string(),
            regulatory_body: "SBV".to_string(),
            deadline: None,
            renewal_period_days: None,
            required_documents: serde_json::json!(["form-a"]),
            is_mandatory: true,
            created_at: now,
            updated_at: now,
        });
        repo.statuses.lock().unwrap().push(TenantLicenseStatusRow {
            id: "status_other".to_string(),
            tenant_id: "tenant_other".to_string(),
            requirement_id: "req_vasp".to_string(),
            status: "APPROVED".to_string(),
            license_number: Some("LIC-001".to_string()),
            issue_date: Some(now),
            expiry_date: None,
            last_submission_id: None,
            notes: None,
            created_at: now,
            updated_at: now,
        });
        repo
    }

    fn tenant_ctx() -> TenantContext {
        TenantContext {
            tenant_id: TenantId::new("tenant_self"),
            name: "Tenant".to_string(),
            tier: TenantTier::Standard,
        }
    }

    #[tokio::test]
    async fn get_tenant_status_rejects_cross_tenant_reads() {
        std::env::set_var("RAMPOS_ADMIN_KEY", "admin-secret-key");
        let repo = seeded_repo();
        let mut headers = HeaderMap::new();
        headers.insert("X-Admin-Key", "admin-secret-key".parse().unwrap());

        let err = get_tenant_status(
            headers,
            Extension(tenant_ctx()),
            State(repo),
            Path("tenant_other".to_string()),
        )
        .await
        .unwrap_err();

        match err {
            ApiError::NotFound(message) => assert!(message.contains("tenant_other")),
            other => panic!("expected not found error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn submit_license_rejects_viewer_role() {
        std::env::set_var("RAMPOS_ADMIN_KEY", "admin-secret-key");
        std::env::set_var("RAMPOS_ADMIN_ROLE", "viewer");
        let repo = seeded_repo();
        let mut headers = HeaderMap::new();
        headers.insert("X-Admin-Key", "admin-secret-key".parse().unwrap());

        let request = SubmitLicenseRequest {
            requirement_id: "req_vasp".to_string(),
            documents: vec![DocumentSubmission {
                name: "form-a".to_string(),
                file_url: "https://files.example/doc.pdf".to_string(),
                file_hash: "abc123".to_string(),
                file_size_bytes: 10,
            }],
            submitted_by: Some("viewer".to_string()),
        };

        let err = submit_license(
            headers,
            Extension(tenant_ctx()),
            State(repo),
            Json(request),
        )
        .await
        .unwrap_err();

        match err {
            ApiError::Forbidden(message) => assert!(message.contains("Insufficient permissions")),
            other => panic!("expected forbidden error, got {other:?}"),
        }

        std::env::remove_var("RAMPOS_ADMIN_ROLE");
    }
}
