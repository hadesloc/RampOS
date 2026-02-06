//! License repository - Database access for license management

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

// ============================================================================
// Simple License Row for deadline checker
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseRow {
    pub id: String,
    pub tenant_id: String,
    pub license_type: String,
    pub license_number: String,
    pub issued_by: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Row Types
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LicenseTypeRow {
    pub id: String,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub jurisdiction: String,
    pub regulatory_body: Option<String>,
    pub is_active: bool,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LicenseRequirementRow {
    pub id: String,
    pub license_type_id: String,
    pub requirement_name: String,
    pub requirement_code: String,
    pub description: Option<String>,
    pub is_mandatory: bool,
    pub document_type: Option<String>,
    pub validation_rules: Option<serde_json::Value>,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TenantLicenseRow {
    pub id: String,
    pub tenant_id: String,
    pub license_type_id: String,
    pub status: String,
    pub license_number: Option<String>,
    pub issued_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub compliance_percentage: Decimal,
    pub last_compliance_check: Option<DateTime<Utc>>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_notes: Option<String>,
    pub rejection_reason: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TenantLicenseDocumentRow {
    pub id: String,
    pub tenant_id: String,
    pub tenant_license_id: String,
    pub requirement_id: String,
    pub document_name: String,
    pub document_url: String,
    pub document_hash: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_notes: Option<String>,
    pub rejection_reason: Option<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub uploaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Create Request Types
// ============================================================================

#[derive(Debug, Clone)]
pub struct CreateTenantLicenseRequest {
    pub id: String,
    pub tenant_id: String,
    pub license_type_id: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct CreateLicenseDocumentRequest {
    pub id: String,
    pub tenant_id: String,
    pub tenant_license_id: String,
    pub requirement_id: String,
    pub document_name: String,
    pub document_url: String,
    pub document_hash: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// License Status Enum
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseStatus {
    Draft,
    Submitted,
    UnderReview,
    Approved,
    Rejected,
    Active,
    Expired,
    Suspended,
    Revoked,
}

impl LicenseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LicenseStatus::Draft => "DRAFT",
            LicenseStatus::Submitted => "SUBMITTED",
            LicenseStatus::UnderReview => "UNDER_REVIEW",
            LicenseStatus::Approved => "APPROVED",
            LicenseStatus::Rejected => "REJECTED",
            LicenseStatus::Active => "ACTIVE",
            LicenseStatus::Expired => "EXPIRED",
            LicenseStatus::Suspended => "SUSPENDED",
            LicenseStatus::Revoked => "REVOKED",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "DRAFT" => Some(LicenseStatus::Draft),
            "SUBMITTED" => Some(LicenseStatus::Submitted),
            "UNDER_REVIEW" => Some(LicenseStatus::UnderReview),
            "APPROVED" => Some(LicenseStatus::Approved),
            "REJECTED" => Some(LicenseStatus::Rejected),
            "ACTIVE" => Some(LicenseStatus::Active),
            "EXPIRED" => Some(LicenseStatus::Expired),
            "SUSPENDED" => Some(LicenseStatus::Suspended),
            "REVOKED" => Some(LicenseStatus::Revoked),
            _ => None,
        }
    }

    pub fn can_transition_to(&self, target: LicenseStatus) -> bool {
        match self {
            LicenseStatus::Draft => matches!(target, LicenseStatus::Submitted),
            LicenseStatus::Submitted => {
                matches!(target, LicenseStatus::UnderReview | LicenseStatus::Draft)
            }
            LicenseStatus::UnderReview => {
                matches!(target, LicenseStatus::Approved | LicenseStatus::Rejected)
            }
            LicenseStatus::Approved => matches!(target, LicenseStatus::Active),
            LicenseStatus::Rejected => matches!(target, LicenseStatus::Draft),
            LicenseStatus::Active => matches!(
                target,
                LicenseStatus::Expired | LicenseStatus::Suspended | LicenseStatus::Revoked
            ),
            LicenseStatus::Expired => matches!(target, LicenseStatus::Draft),
            LicenseStatus::Suspended => {
                matches!(target, LicenseStatus::Active | LicenseStatus::Revoked)
            }
            LicenseStatus::Revoked => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentStatus::Pending => "PENDING",
            DocumentStatus::Approved => "APPROVED",
            DocumentStatus::Rejected => "REJECTED",
            DocumentStatus::Expired => "EXPIRED",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "PENDING" => Some(DocumentStatus::Pending),
            "APPROVED" => Some(DocumentStatus::Approved),
            "REJECTED" => Some(DocumentStatus::Rejected),
            "EXPIRED" => Some(DocumentStatus::Expired),
            _ => None,
        }
    }
}

// ============================================================================
// Repository Trait
// ============================================================================

#[async_trait]
pub trait LicenseRepository: Send + Sync {
    // License Types
    async fn get_license_types(&self) -> Result<Vec<LicenseTypeRow>>;
    async fn get_license_type_by_id(&self, id: &str) -> Result<Option<LicenseTypeRow>>;
    async fn get_license_type_by_code(&self, code: &str) -> Result<Option<LicenseTypeRow>>;

    // License Requirements
    async fn get_requirements_by_license_type(
        &self,
        license_type_id: &str,
    ) -> Result<Vec<LicenseRequirementRow>>;
    async fn get_requirement_by_id(&self, id: &str) -> Result<Option<LicenseRequirementRow>>;
    async fn get_mandatory_requirements(
        &self,
        license_type_id: &str,
    ) -> Result<Vec<LicenseRequirementRow>>;

    // Tenant Licenses
    async fn get_tenant_licenses(&self, tenant_id: &TenantId) -> Result<Vec<TenantLicenseRow>>;
    async fn get_tenant_license_by_id(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
    ) -> Result<Option<TenantLicenseRow>>;
    async fn get_tenant_license_by_type(
        &self,
        tenant_id: &TenantId,
        license_type_id: &str,
    ) -> Result<Option<TenantLicenseRow>>;
    async fn create_tenant_license(
        &self,
        request: &CreateTenantLicenseRequest,
    ) -> Result<TenantLicenseRow>;
    async fn update_license_status(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
        status: LicenseStatus,
        reviewed_by: Option<&str>,
        review_notes: Option<&str>,
        rejection_reason: Option<&str>,
    ) -> Result<()>;
    async fn update_compliance_percentage(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
        percentage: Decimal,
    ) -> Result<()>;
    async fn set_license_number(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
        license_number: &str,
        issued_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()>;
    async fn get_expiring_licenses(&self, days_until_expiry: i32) -> Result<Vec<TenantLicenseRow>>;

    // Deadline checker support
    async fn find_expiring_before(&self, before: DateTime<Utc>) -> Result<Vec<LicenseRow>>;
    async fn get_by_id(&self, tenant_id: &TenantId, id: &str) -> Result<Option<LicenseRow>>;
    async fn list_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<LicenseRow>>;
    async fn create(&self, license: &LicenseRow) -> Result<()>;
    async fn update(&self, license: &LicenseRow) -> Result<()>;

    // License Documents
    async fn get_license_documents(
        &self,
        tenant_id: &TenantId,
        tenant_license_id: &str,
    ) -> Result<Vec<TenantLicenseDocumentRow>>;
    async fn get_document_by_id(
        &self,
        tenant_id: &TenantId,
        document_id: &str,
    ) -> Result<Option<TenantLicenseDocumentRow>>;
    async fn get_document_by_requirement(
        &self,
        tenant_id: &TenantId,
        tenant_license_id: &str,
        requirement_id: &str,
    ) -> Result<Option<TenantLicenseDocumentRow>>;
    async fn create_document(
        &self,
        request: &CreateLicenseDocumentRequest,
    ) -> Result<TenantLicenseDocumentRow>;
    async fn update_document_status(
        &self,
        tenant_id: &TenantId,
        document_id: &str,
        status: DocumentStatus,
        reviewed_by: Option<&str>,
        review_notes: Option<&str>,
        rejection_reason: Option<&str>,
    ) -> Result<()>;
    async fn count_approved_documents(
        &self,
        tenant_id: &TenantId,
        tenant_license_id: &str,
    ) -> Result<i64>;
}

// ============================================================================
// PostgreSQL Implementation
// ============================================================================

pub struct PgLicenseRepository {
    pool: PgPool,
}

impl PgLicenseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LicenseRepository for PgLicenseRepository {
    // License Types
    async fn get_license_types(&self) -> Result<Vec<LicenseTypeRow>> {
        let rows = sqlx::query_as::<_, LicenseTypeRow>(
            "SELECT * FROM license_types WHERE is_active = true ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn get_license_type_by_id(&self, id: &str) -> Result<Option<LicenseTypeRow>> {
        let row = sqlx::query_as::<_, LicenseTypeRow>("SELECT * FROM license_types WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_license_type_by_code(&self, code: &str) -> Result<Option<LicenseTypeRow>> {
        let row =
            sqlx::query_as::<_, LicenseTypeRow>("SELECT * FROM license_types WHERE code = $1")
                .bind(code)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    // License Requirements
    async fn get_requirements_by_license_type(
        &self,
        license_type_id: &str,
    ) -> Result<Vec<LicenseRequirementRow>> {
        let rows = sqlx::query_as::<_, LicenseRequirementRow>(
            "SELECT * FROM license_requirements WHERE license_type_id = $1 ORDER BY display_order",
        )
        .bind(license_type_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn get_requirement_by_id(&self, id: &str) -> Result<Option<LicenseRequirementRow>> {
        let row = sqlx::query_as::<_, LicenseRequirementRow>(
            "SELECT * FROM license_requirements WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_mandatory_requirements(
        &self,
        license_type_id: &str,
    ) -> Result<Vec<LicenseRequirementRow>> {
        let rows = sqlx::query_as::<_, LicenseRequirementRow>(
            "SELECT * FROM license_requirements WHERE license_type_id = $1 AND is_mandatory = true ORDER BY display_order"
        )
        .bind(license_type_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    // Tenant Licenses
    async fn get_tenant_licenses(&self, tenant_id: &TenantId) -> Result<Vec<TenantLicenseRow>> {
        let rows = sqlx::query_as::<_, TenantLicenseRow>(
            "SELECT * FROM tenant_licenses WHERE tenant_id = $1 ORDER BY created_at DESC",
        )
        .bind(&tenant_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn get_tenant_license_by_id(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
    ) -> Result<Option<TenantLicenseRow>> {
        let row = sqlx::query_as::<_, TenantLicenseRow>(
            "SELECT * FROM tenant_licenses WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&tenant_id.0)
        .bind(license_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_tenant_license_by_type(
        &self,
        tenant_id: &TenantId,
        license_type_id: &str,
    ) -> Result<Option<TenantLicenseRow>> {
        let row = sqlx::query_as::<_, TenantLicenseRow>(
            "SELECT * FROM tenant_licenses WHERE tenant_id = $1 AND license_type_id = $2",
        )
        .bind(&tenant_id.0)
        .bind(license_type_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn create_tenant_license(
        &self,
        request: &CreateTenantLicenseRequest,
    ) -> Result<TenantLicenseRow> {
        let row = sqlx::query_as::<_, TenantLicenseRow>(
            r#"
            INSERT INTO tenant_licenses (id, tenant_id, license_type_id, metadata)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&request.id)
        .bind(&request.tenant_id)
        .bind(&request.license_type_id)
        .bind(request.metadata.clone().unwrap_or(serde_json::json!({})))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn update_license_status(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
        status: LicenseStatus,
        reviewed_by: Option<&str>,
        review_notes: Option<&str>,
        rejection_reason: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();
        let submitted_at = if status == LicenseStatus::Submitted {
            Some(now)
        } else {
            None
        };
        let reviewed_at = if reviewed_by.is_some() {
            Some(now)
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE tenant_licenses
            SET status = $1,
                reviewed_by = COALESCE($2, reviewed_by),
                reviewed_at = COALESCE($3, reviewed_at),
                review_notes = COALESCE($4, review_notes),
                rejection_reason = $5,
                submitted_at = COALESCE($6, submitted_at)
            WHERE tenant_id = $7 AND id = $8
            "#,
        )
        .bind(status.as_str())
        .bind(reviewed_by)
        .bind(reviewed_at)
        .bind(review_notes)
        .bind(rejection_reason)
        .bind(submitted_at)
        .bind(&tenant_id.0)
        .bind(license_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_compliance_percentage(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
        percentage: Decimal,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tenant_licenses
            SET compliance_percentage = $1, last_compliance_check = NOW()
            WHERE tenant_id = $2 AND id = $3
            "#,
        )
        .bind(percentage)
        .bind(&tenant_id.0)
        .bind(license_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn set_license_number(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
        license_number: &str,
        issued_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tenant_licenses
            SET license_number = $1, issued_at = $2, expires_at = $3
            WHERE tenant_id = $4 AND id = $5
            "#,
        )
        .bind(license_number)
        .bind(issued_at)
        .bind(expires_at)
        .bind(&tenant_id.0)
        .bind(license_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_expiring_licenses(&self, days_until_expiry: i32) -> Result<Vec<TenantLicenseRow>> {
        let rows = sqlx::query_as::<_, TenantLicenseRow>(
            r#"
            SELECT * FROM tenant_licenses
            WHERE status = 'ACTIVE'
              AND expires_at IS NOT NULL
              AND expires_at <= NOW() + ($1 || ' days')::INTERVAL
            ORDER BY expires_at ASC
            "#,
        )
        .bind(days_until_expiry)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    // License Documents
    async fn get_license_documents(
        &self,
        tenant_id: &TenantId,
        tenant_license_id: &str,
    ) -> Result<Vec<TenantLicenseDocumentRow>> {
        let rows = sqlx::query_as::<_, TenantLicenseDocumentRow>(
            r#"
            SELECT * FROM tenant_license_documents
            WHERE tenant_id = $1 AND tenant_license_id = $2
            ORDER BY uploaded_at DESC
            "#,
        )
        .bind(&tenant_id.0)
        .bind(tenant_license_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn get_document_by_id(
        &self,
        tenant_id: &TenantId,
        document_id: &str,
    ) -> Result<Option<TenantLicenseDocumentRow>> {
        let row = sqlx::query_as::<_, TenantLicenseDocumentRow>(
            "SELECT * FROM tenant_license_documents WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&tenant_id.0)
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn get_document_by_requirement(
        &self,
        tenant_id: &TenantId,
        tenant_license_id: &str,
        requirement_id: &str,
    ) -> Result<Option<TenantLicenseDocumentRow>> {
        let row = sqlx::query_as::<_, TenantLicenseDocumentRow>(
            r#"
            SELECT * FROM tenant_license_documents
            WHERE tenant_id = $1 AND tenant_license_id = $2 AND requirement_id = $3
            ORDER BY uploaded_at DESC
            LIMIT 1
            "#,
        )
        .bind(&tenant_id.0)
        .bind(tenant_license_id)
        .bind(requirement_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn create_document(
        &self,
        request: &CreateLicenseDocumentRequest,
    ) -> Result<TenantLicenseDocumentRow> {
        let row = sqlx::query_as::<_, TenantLicenseDocumentRow>(
            r#"
            INSERT INTO tenant_license_documents (
                id, tenant_id, tenant_license_id, requirement_id,
                document_name, document_url, document_hash, file_size, mime_type,
                valid_from, valid_until, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(&request.id)
        .bind(&request.tenant_id)
        .bind(&request.tenant_license_id)
        .bind(&request.requirement_id)
        .bind(&request.document_name)
        .bind(&request.document_url)
        .bind(&request.document_hash)
        .bind(request.file_size)
        .bind(&request.mime_type)
        .bind(request.valid_from)
        .bind(request.valid_until)
        .bind(request.metadata.clone().unwrap_or(serde_json::json!({})))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(row)
    }

    async fn update_document_status(
        &self,
        tenant_id: &TenantId,
        document_id: &str,
        status: DocumentStatus,
        reviewed_by: Option<&str>,
        review_notes: Option<&str>,
        rejection_reason: Option<&str>,
    ) -> Result<()> {
        let reviewed_at = if reviewed_by.is_some() {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE tenant_license_documents
            SET status = $1,
                reviewed_by = COALESCE($2, reviewed_by),
                reviewed_at = COALESCE($3, reviewed_at),
                review_notes = COALESCE($4, review_notes),
                rejection_reason = $5
            WHERE tenant_id = $6 AND id = $7
            "#,
        )
        .bind(status.as_str())
        .bind(reviewed_by)
        .bind(reviewed_at)
        .bind(review_notes)
        .bind(rejection_reason)
        .bind(&tenant_id.0)
        .bind(document_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn count_approved_documents(
        &self,
        tenant_id: &TenantId,
        tenant_license_id: &str,
    ) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT requirement_id)
            FROM tenant_license_documents
            WHERE tenant_id = $1 AND tenant_license_id = $2 AND status = 'APPROVED'
            "#,
        )
        .bind(&tenant_id.0)
        .bind(tenant_license_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(count.0)
    }

    async fn find_expiring_before(&self, before: DateTime<Utc>) -> Result<Vec<LicenseRow>> {
        let rows = sqlx::query_as::<_, TenantLicenseRow>(
            r#"
            SELECT * FROM tenant_licenses
            WHERE status = 'ACTIVE'
              AND expires_at IS NOT NULL
              AND expires_at <= $1
              AND expires_at > NOW()
            ORDER BY expires_at ASC
            "#,
        )
        .bind(before)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .filter_map(|r| {
                Some(LicenseRow {
                    id: r.id,
                    tenant_id: r.tenant_id,
                    license_type: r.license_type_id,
                    license_number: r.license_number?,
                    issued_by: "SBV".to_string(),
                    issued_at: r.issued_at?,
                    expires_at: r.expires_at?,
                    status: r.status,
                    metadata: r.metadata,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                })
            })
            .collect())
    }

    async fn get_by_id(&self, tenant_id: &TenantId, id: &str) -> Result<Option<LicenseRow>> {
        let row = self.get_tenant_license_by_id(tenant_id, id).await?;
        Ok(row.and_then(|r| {
            Some(LicenseRow {
                id: r.id,
                tenant_id: r.tenant_id,
                license_type: r.license_type_id,
                license_number: r.license_number?,
                issued_by: "SBV".to_string(),
                issued_at: r.issued_at?,
                expires_at: r.expires_at?,
                status: r.status,
                metadata: r.metadata,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        }))
    }

    async fn list_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<LicenseRow>> {
        let rows = self.get_tenant_licenses(tenant_id).await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                Some(LicenseRow {
                    id: r.id,
                    tenant_id: r.tenant_id,
                    license_type: r.license_type_id,
                    license_number: r.license_number?,
                    issued_by: "SBV".to_string(),
                    issued_at: r.issued_at?,
                    expires_at: r.expires_at?,
                    status: r.status,
                    metadata: r.metadata,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                })
            })
            .collect())
    }

    async fn create(&self, _license: &LicenseRow) -> Result<()> {
        Ok(())
    }

    async fn update(&self, _license: &LicenseRow) -> Result<()> {
        Ok(())
    }
}
