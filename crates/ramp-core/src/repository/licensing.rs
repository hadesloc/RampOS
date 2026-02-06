//! Licensing Repository - Database access for licensing requirements
//!
//! Provides CRUD operations for Vietnam licensing requirements and tenant compliance status.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::licensing::{
    LicenseRequirementId, LicenseStatus, LicenseSubmissionId, LicenseType, SubmissionStatus,
};
use ramp_common::types::TenantId;
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

/// Database row for license requirements
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LicenseRequirementRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub license_type: String,
    pub regulatory_body: String,
    pub deadline: Option<DateTime<Utc>>,
    pub renewal_period_days: Option<i32>,
    pub required_documents: serde_json::Value,
    pub is_mandatory: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database row for tenant license status
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TenantLicenseStatusRow {
    pub id: String,
    pub tenant_id: String,
    pub requirement_id: String,
    pub status: String,
    pub license_number: Option<String>,
    pub issue_date: Option<DateTime<Utc>>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub last_submission_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database row for license submissions
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LicenseSubmissionRow {
    pub id: String,
    pub tenant_id: String,
    pub requirement_id: String,
    pub documents: serde_json::Value,
    pub status: String,
    pub submitted_by: String,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewer_notes: Option<String>,
}

/// Request to create a new license requirement
#[derive(Debug, Clone)]
pub struct CreateLicenseRequirementRequest {
    pub name: String,
    pub description: String,
    pub license_type: LicenseType,
    pub regulatory_body: String,
    pub deadline: Option<DateTime<Utc>>,
    pub renewal_period_days: Option<i32>,
    pub required_documents: Vec<String>,
    pub is_mandatory: bool,
}

/// Request to create a license submission
#[derive(Debug, Clone)]
pub struct CreateLicenseSubmissionRequest {
    pub tenant_id: TenantId,
    pub requirement_id: LicenseRequirementId,
    pub documents: serde_json::Value,
    pub submitted_by: String,
}

/// Licensing repository trait
#[async_trait]
pub trait LicensingRepository: Send + Sync {
    /// List all license requirements
    async fn list_requirements(&self, limit: i64, offset: i64)
        -> Result<Vec<LicenseRequirementRow>>;

    /// Get a specific requirement by ID
    async fn get_requirement(
        &self,
        id: &LicenseRequirementId,
    ) -> Result<Option<LicenseRequirementRow>>;

    /// Create a new license requirement
    async fn create_requirement(
        &self,
        req: &CreateLicenseRequirementRequest,
    ) -> Result<LicenseRequirementRow>;

    /// Get tenant's license status for all requirements
    async fn get_tenant_license_statuses(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<TenantLicenseStatusRow>>;

    /// Get tenant's license status for a specific requirement
    async fn get_tenant_license_status(
        &self,
        tenant_id: &TenantId,
        requirement_id: &LicenseRequirementId,
    ) -> Result<Option<TenantLicenseStatusRow>>;

    /// Update or create tenant license status
    async fn upsert_tenant_license_status(
        &self,
        tenant_id: &TenantId,
        requirement_id: &LicenseRequirementId,
        status: LicenseStatus,
        license_number: Option<&str>,
        expiry_date: Option<DateTime<Utc>>,
    ) -> Result<TenantLicenseStatusRow>;

    /// Create a license submission
    async fn create_submission(
        &self,
        req: &CreateLicenseSubmissionRequest,
    ) -> Result<LicenseSubmissionRow>;

    /// Get submissions for a tenant
    async fn list_submissions(
        &self,
        tenant_id: &TenantId,
        requirement_id: Option<&LicenseRequirementId>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LicenseSubmissionRow>>;

    /// Update submission status
    async fn update_submission_status(
        &self,
        submission_id: &LicenseSubmissionId,
        status: SubmissionStatus,
        reviewer_notes: Option<&str>,
    ) -> Result<()>;

    /// Get upcoming deadlines for a tenant
    async fn get_upcoming_deadlines(
        &self,
        tenant_id: &TenantId,
        days_ahead: i32,
    ) -> Result<Vec<(LicenseRequirementRow, Option<TenantLicenseStatusRow>)>>;

    /// Count total requirements
    async fn count_requirements(&self) -> Result<i64>;
}

/// PostgreSQL implementation of LicensingRepository
pub struct PgLicensingRepository {
    pool: PgPool,
}

impl PgLicensingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LicensingRepository for PgLicensingRepository {
    async fn list_requirements(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LicenseRequirementRow>> {
        let rows = sqlx::query_as::<_, LicenseRequirementRow>(
            r#"
            SELECT id, name, description, license_type, regulatory_body, deadline,
                   renewal_period_days, required_documents, is_mandatory, created_at, updated_at
            FROM license_requirements
            ORDER BY deadline ASC NULLS LAST, name ASC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn get_requirement(
        &self,
        id: &LicenseRequirementId,
    ) -> Result<Option<LicenseRequirementRow>> {
        let row = sqlx::query_as::<_, LicenseRequirementRow>(
            r#"
            SELECT id, name, description, license_type, regulatory_body, deadline,
                   renewal_period_days, required_documents, is_mandatory, created_at, updated_at
            FROM license_requirements
            WHERE id = $1
            "#,
        )
        .bind(&id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn create_requirement(
        &self,
        req: &CreateLicenseRequirementRequest,
    ) -> Result<LicenseRequirementRow> {
        let id = LicenseRequirementId::generate();
        let now = Utc::now();
        let documents_json = serde_json::to_value(&req.required_documents)
            .map_err(|e| ramp_common::Error::Serialization(e.to_string()))?;

        let row = sqlx::query_as::<_, LicenseRequirementRow>(
            r#"
            INSERT INTO license_requirements (
                id, name, description, license_type, regulatory_body, deadline,
                renewal_period_days, required_documents, is_mandatory, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, name, description, license_type, regulatory_body, deadline,
                      renewal_period_days, required_documents, is_mandatory, created_at, updated_at
            "#,
        )
        .bind(&id.0)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.license_type.as_str())
        .bind(&req.regulatory_body)
        .bind(req.deadline)
        .bind(req.renewal_period_days)
        .bind(&documents_json)
        .bind(req.is_mandatory)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_tenant_license_statuses(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<TenantLicenseStatusRow>> {
        let rows = sqlx::query_as::<_, TenantLicenseStatusRow>(
            r#"
            SELECT id, tenant_id, requirement_id, status, license_number, issue_date,
                   expiry_date, last_submission_id, notes, created_at, updated_at
            FROM tenant_license_status
            WHERE tenant_id = $1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(&tenant_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn get_tenant_license_status(
        &self,
        tenant_id: &TenantId,
        requirement_id: &LicenseRequirementId,
    ) -> Result<Option<TenantLicenseStatusRow>> {
        let row = sqlx::query_as::<_, TenantLicenseStatusRow>(
            r#"
            SELECT id, tenant_id, requirement_id, status, license_number, issue_date,
                   expiry_date, last_submission_id, notes, created_at, updated_at
            FROM tenant_license_status
            WHERE tenant_id = $1 AND requirement_id = $2
            "#,
        )
        .bind(&tenant_id.0)
        .bind(&requirement_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn upsert_tenant_license_status(
        &self,
        tenant_id: &TenantId,
        requirement_id: &LicenseRequirementId,
        status: LicenseStatus,
        license_number: Option<&str>,
        expiry_date: Option<DateTime<Utc>>,
    ) -> Result<TenantLicenseStatusRow> {
        let now = Utc::now();
        let id = format!("tls_{}", uuid::Uuid::now_v7());

        let row = sqlx::query_as::<_, TenantLicenseStatusRow>(
            r#"
            INSERT INTO tenant_license_status (
                id, tenant_id, requirement_id, status, license_number, expiry_date,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (tenant_id, requirement_id) DO UPDATE SET
                status = EXCLUDED.status,
                license_number = COALESCE(EXCLUDED.license_number, tenant_license_status.license_number),
                expiry_date = COALESCE(EXCLUDED.expiry_date, tenant_license_status.expiry_date),
                updated_at = EXCLUDED.updated_at
            RETURNING id, tenant_id, requirement_id, status, license_number, issue_date,
                      expiry_date, last_submission_id, notes, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&tenant_id.0)
        .bind(&requirement_id.0)
        .bind(status.as_str())
        .bind(license_number)
        .bind(expiry_date)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn create_submission(
        &self,
        req: &CreateLicenseSubmissionRequest,
    ) -> Result<LicenseSubmissionRow> {
        let id = LicenseSubmissionId::generate();
        let now = Utc::now();

        let row = sqlx::query_as::<_, LicenseSubmissionRow>(
            r#"
            INSERT INTO license_submissions (
                id, tenant_id, requirement_id, documents, status, submitted_by, submitted_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, tenant_id, requirement_id, documents, status, submitted_by,
                      submitted_at, reviewed_at, reviewer_notes
            "#,
        )
        .bind(&id.0)
        .bind(&req.tenant_id.0)
        .bind(&req.requirement_id.0)
        .bind(&req.documents)
        .bind(SubmissionStatus::Submitted.as_str())
        .bind(&req.submitted_by)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        // Update tenant license status to SUBMITTED
        self.upsert_tenant_license_status(
            &req.tenant_id,
            &req.requirement_id,
            LicenseStatus::Submitted,
            None,
            None,
        )
        .await?;

        // Update last_submission_id
        sqlx::query(
            r#"
            UPDATE tenant_license_status
            SET last_submission_id = $1, updated_at = NOW()
            WHERE tenant_id = $2 AND requirement_id = $3
            "#,
        )
        .bind(&id.0)
        .bind(&req.tenant_id.0)
        .bind(&req.requirement_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn list_submissions(
        &self,
        tenant_id: &TenantId,
        requirement_id: Option<&LicenseRequirementId>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LicenseSubmissionRow>> {
        let rows = if let Some(req_id) = requirement_id {
            sqlx::query_as::<_, LicenseSubmissionRow>(
                r#"
                SELECT id, tenant_id, requirement_id, documents, status, submitted_by,
                       submitted_at, reviewed_at, reviewer_notes
                FROM license_submissions
                WHERE tenant_id = $1 AND requirement_id = $2
                ORDER BY submitted_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(&tenant_id.0)
            .bind(&req_id.0)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, LicenseSubmissionRow>(
                r#"
                SELECT id, tenant_id, requirement_id, documents, status, submitted_by,
                       submitted_at, reviewed_at, reviewer_notes
                FROM license_submissions
                WHERE tenant_id = $1
                ORDER BY submitted_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&tenant_id.0)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn update_submission_status(
        &self,
        submission_id: &LicenseSubmissionId,
        status: SubmissionStatus,
        reviewer_notes: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE license_submissions
            SET status = $1, reviewed_at = $2, reviewer_notes = $3
            WHERE id = $4
            "#,
        )
        .bind(status.as_str())
        .bind(now)
        .bind(reviewer_notes)
        .bind(&submission_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_upcoming_deadlines(
        &self,
        tenant_id: &TenantId,
        days_ahead: i32,
    ) -> Result<Vec<(LicenseRequirementRow, Option<TenantLicenseStatusRow>)>> {
        // First get requirements with deadlines within the specified range
        let requirements = sqlx::query_as::<_, LicenseRequirementRow>(
            r#"
            SELECT id, name, description, license_type, regulatory_body, deadline,
                   renewal_period_days, required_documents, is_mandatory, created_at, updated_at
            FROM license_requirements
            WHERE deadline IS NOT NULL
              AND deadline <= NOW() + ($1 || ' days')::INTERVAL
            ORDER BY deadline ASC
            "#,
        )
        .bind(days_ahead)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        // Get tenant statuses for these requirements
        let mut results = Vec::new();
        for req in requirements {
            let status = self
                .get_tenant_license_status(tenant_id, &LicenseRequirementId::new(&req.id))
                .await?;
            results.push((req, status));
        }

        Ok(results)
    }

    async fn count_requirements(&self) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM license_requirements")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(count)
    }
}
