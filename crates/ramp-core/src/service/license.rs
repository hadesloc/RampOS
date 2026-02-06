//! License service - Business logic for license management

use ramp_common::{types::TenantId, Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::repository::license::{
    CreateLicenseDocumentRequest, CreateTenantLicenseRequest, DocumentStatus, LicenseRepository,
    LicenseRequirementRow, LicenseStatus, LicenseTypeRow, TenantLicenseDocumentRow,
    TenantLicenseRow,
};

// ============================================================================
// Service Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseWithDetails {
    pub license: TenantLicenseRow,
    pub license_type: LicenseTypeRow,
    pub requirements: Vec<RequirementWithDocument>,
    pub missing_requirements: Vec<LicenseRequirementRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementWithDocument {
    pub requirement: LicenseRequirementRow,
    pub document: Option<TenantLicenseDocumentRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub license_id: String,
    pub total_requirements: i32,
    pub mandatory_requirements: i32,
    pub fulfilled_requirements: i32,
    pub fulfilled_mandatory: i32,
    pub compliance_percentage: Decimal,
    pub is_compliant: bool,
    pub missing_mandatory: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CreateLicenseRequest {
    pub license_type_code: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct UploadDocumentRequest {
    pub license_id: String,
    pub requirement_id: String,
    pub document_name: String,
    pub document_url: String,
    pub document_hash: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
}

// ============================================================================
// License Service
// ============================================================================

pub struct LicenseService {
    repo: Arc<dyn LicenseRepository>,
}

impl LicenseService {
    pub fn new(repo: Arc<dyn LicenseRepository>) -> Self {
        Self { repo }
    }

    // ========================================================================
    // License Types
    // ========================================================================

    pub async fn get_available_license_types(&self) -> Result<Vec<LicenseTypeRow>> {
        self.repo.get_license_types().await
    }

    pub async fn get_license_type(&self, code: &str) -> Result<LicenseTypeRow> {
        self.repo
            .get_license_type_by_code(code)
            .await?
            .ok_or_else(|| Error::NotFound(format!("License type not found: {}", code)))
    }

    pub async fn get_requirements_for_license_type(
        &self,
        license_type_id: &str,
    ) -> Result<Vec<LicenseRequirementRow>> {
        self.repo
            .get_requirements_by_license_type(license_type_id)
            .await
    }

    // ========================================================================
    // Tenant Licenses
    // ========================================================================

    pub async fn get_tenant_licenses(&self, tenant_id: &TenantId) -> Result<Vec<TenantLicenseRow>> {
        self.repo.get_tenant_licenses(tenant_id).await
    }

    pub async fn get_license_with_details(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
    ) -> Result<LicenseWithDetails> {
        let license = self
            .repo
            .get_tenant_license_by_id(tenant_id, license_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("License not found: {}", license_id)))?;

        let license_type = self
            .repo
            .get_license_type_by_id(&license.license_type_id)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "License type not found: {}",
                    license.license_type_id
                ))
            })?;

        let requirements = self
            .repo
            .get_requirements_by_license_type(&license.license_type_id)
            .await?;

        let documents = self
            .repo
            .get_license_documents(tenant_id, license_id)
            .await?;

        let mut requirements_with_docs = Vec::new();
        let mut missing_requirements = Vec::new();

        for req in requirements {
            let doc = documents
                .iter()
                .find(|d| d.requirement_id == req.id)
                .cloned();

            if doc.is_none() && req.is_mandatory {
                missing_requirements.push(req.clone());
            }

            requirements_with_docs.push(RequirementWithDocument {
                requirement: req,
                document: doc,
            });
        }

        Ok(LicenseWithDetails {
            license,
            license_type,
            requirements: requirements_with_docs,
            missing_requirements,
        })
    }

    pub async fn create_license(
        &self,
        tenant_id: &TenantId,
        request: CreateLicenseRequest,
    ) -> Result<TenantLicenseRow> {
        let license_type = self
            .repo
            .get_license_type_by_code(&request.license_type_code)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "License type not found: {}",
                    request.license_type_code
                ))
            })?;

        // Check if tenant already has this license type
        if let Some(existing) = self
            .repo
            .get_tenant_license_by_type(tenant_id, &license_type.id)
            .await?
        {
            return Err(Error::Validation(format!(
                "Tenant already has a {} license (ID: {})",
                request.license_type_code, existing.id
            )));
        }

        let license_id = format!("lic_{}", Uuid::now_v7());

        let create_request = CreateTenantLicenseRequest {
            id: license_id.clone(),
            tenant_id: tenant_id.0.clone(),
            license_type_id: license_type.id,
            metadata: request.metadata,
        };

        let license = self.repo.create_tenant_license(&create_request).await?;

        info!(
            tenant_id = %tenant_id,
            license_id = %license_id,
            license_type = %request.license_type_code,
            "Created new license application"
        );

        Ok(license)
    }

    pub async fn submit_license(&self, tenant_id: &TenantId, license_id: &str) -> Result<()> {
        let license = self
            .repo
            .get_tenant_license_by_id(tenant_id, license_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("License not found: {}", license_id)))?;

        let current_status = LicenseStatus::from_str(&license.status).ok_or_else(|| {
            Error::Internal(format!("Invalid license status: {}", license.status))
        })?;

        if !current_status.can_transition_to(LicenseStatus::Submitted) {
            return Err(Error::Validation(format!(
                "Cannot submit license from status: {}",
                license.status
            )));
        }

        // Check compliance before submission
        let compliance = self.check_compliance(tenant_id, license_id).await?;
        if !compliance.is_compliant {
            return Err(Error::Validation(format!(
                "Cannot submit license: missing {} mandatory requirements",
                compliance.missing_mandatory.len()
            )));
        }

        self.repo
            .update_license_status(
                tenant_id,
                license_id,
                LicenseStatus::Submitted,
                None,
                None,
                None,
            )
            .await?;

        info!(
            tenant_id = %tenant_id,
            license_id = %license_id,
            "License submitted for review"
        );

        Ok(())
    }

    pub async fn update_license_status(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
        new_status: LicenseStatus,
        reviewed_by: Option<&str>,
        review_notes: Option<&str>,
        rejection_reason: Option<&str>,
    ) -> Result<()> {
        let license = self
            .repo
            .get_tenant_license_by_id(tenant_id, license_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("License not found: {}", license_id)))?;

        let current_status = LicenseStatus::from_str(&license.status).ok_or_else(|| {
            Error::Internal(format!("Invalid license status: {}", license.status))
        })?;

        if !current_status.can_transition_to(new_status) {
            return Err(Error::Validation(format!(
                "Cannot transition from {} to {}",
                license.status,
                new_status.as_str()
            )));
        }

        self.repo
            .update_license_status(
                tenant_id,
                license_id,
                new_status,
                reviewed_by,
                review_notes,
                rejection_reason,
            )
            .await?;

        info!(
            tenant_id = %tenant_id,
            license_id = %license_id,
            from_status = %license.status,
            to_status = %new_status.as_str(),
            "License status updated"
        );

        Ok(())
    }

    pub async fn activate_license(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
        license_number: &str,
        expires_in_days: Option<i32>,
    ) -> Result<()> {
        let license = self
            .repo
            .get_tenant_license_by_id(tenant_id, license_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("License not found: {}", license_id)))?;

        if license.status != "APPROVED" {
            return Err(Error::Validation(format!(
                "Can only activate APPROVED licenses, current status: {}",
                license.status
            )));
        }

        let now = chrono::Utc::now();
        let expires_at = expires_in_days.map(|days| now + chrono::Duration::days(days.into()));

        self.repo
            .set_license_number(tenant_id, license_id, license_number, now, expires_at)
            .await?;

        self.repo
            .update_license_status(
                tenant_id,
                license_id,
                LicenseStatus::Active,
                None,
                Some("License activated"),
                None,
            )
            .await?;

        info!(
            tenant_id = %tenant_id,
            license_id = %license_id,
            license_number = %license_number,
            expires_at = ?expires_at,
            "License activated"
        );

        Ok(())
    }

    // ========================================================================
    // Documents
    // ========================================================================

    pub async fn upload_document(
        &self,
        tenant_id: &TenantId,
        request: UploadDocumentRequest,
    ) -> Result<TenantLicenseDocumentRow> {
        // Verify license exists
        let license = self
            .repo
            .get_tenant_license_by_id(tenant_id, &request.license_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("License not found: {}", request.license_id)))?;

        // Verify requirement exists and belongs to license type
        let requirement = self
            .repo
            .get_requirement_by_id(&request.requirement_id)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!("Requirement not found: {}", request.requirement_id))
            })?;

        if requirement.license_type_id != license.license_type_id {
            return Err(Error::Validation(
                "Requirement does not belong to this license type".to_string(),
            ));
        }

        // Check if document already exists for this requirement
        if let Some(existing) = self
            .repo
            .get_document_by_requirement(tenant_id, &request.license_id, &request.requirement_id)
            .await?
        {
            if existing.status == "APPROVED" {
                return Err(Error::Validation(format!(
                    "An approved document already exists for requirement: {}",
                    requirement.requirement_name
                )));
            }
        }

        let doc_id = format!("doc_{}", Uuid::now_v7());

        let create_request = CreateLicenseDocumentRequest {
            id: doc_id.clone(),
            tenant_id: tenant_id.0.clone(),
            tenant_license_id: request.license_id.clone(),
            requirement_id: request.requirement_id,
            document_name: request.document_name,
            document_url: request.document_url,
            document_hash: request.document_hash,
            file_size: request.file_size,
            mime_type: request.mime_type,
            valid_from: None,
            valid_until: None,
            metadata: None,
        };

        let document = self.repo.create_document(&create_request).await?;

        // Recalculate compliance
        self.recalculate_compliance(tenant_id, &request.license_id)
            .await?;

        info!(
            tenant_id = %tenant_id,
            license_id = %request.license_id,
            document_id = %doc_id,
            requirement = %requirement.requirement_name,
            "Document uploaded"
        );

        Ok(document)
    }

    pub async fn review_document(
        &self,
        tenant_id: &TenantId,
        document_id: &str,
        approved: bool,
        reviewed_by: &str,
        review_notes: Option<&str>,
        rejection_reason: Option<&str>,
    ) -> Result<()> {
        let document = self
            .repo
            .get_document_by_id(tenant_id, document_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Document not found: {}", document_id)))?;

        let new_status = if approved {
            DocumentStatus::Approved
        } else {
            DocumentStatus::Rejected
        };

        self.repo
            .update_document_status(
                tenant_id,
                document_id,
                new_status,
                Some(reviewed_by),
                review_notes,
                rejection_reason,
            )
            .await?;

        // Recalculate compliance after document review
        self.recalculate_compliance(tenant_id, &document.tenant_license_id)
            .await?;

        info!(
            tenant_id = %tenant_id,
            document_id = %document_id,
            status = %new_status.as_str(),
            reviewed_by = %reviewed_by,
            "Document reviewed"
        );

        Ok(())
    }

    // ========================================================================
    // Compliance
    // ========================================================================

    pub async fn check_compliance(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
    ) -> Result<ComplianceStatus> {
        let license = self
            .repo
            .get_tenant_license_by_id(tenant_id, license_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("License not found: {}", license_id)))?;

        let all_requirements = self
            .repo
            .get_requirements_by_license_type(&license.license_type_id)
            .await?;

        let mandatory_requirements = self
            .repo
            .get_mandatory_requirements(&license.license_type_id)
            .await?;

        let documents = self
            .repo
            .get_license_documents(tenant_id, license_id)
            .await?;

        let approved_docs: Vec<_> = documents
            .iter()
            .filter(|d| d.status == "APPROVED")
            .collect();

        let fulfilled_mandatory: Vec<_> = mandatory_requirements
            .iter()
            .filter(|req| approved_docs.iter().any(|doc| doc.requirement_id == req.id))
            .collect();

        let missing_mandatory: Vec<String> = mandatory_requirements
            .iter()
            .filter(|req| !approved_docs.iter().any(|doc| doc.requirement_id == req.id))
            .map(|req| req.requirement_name.clone())
            .collect();

        let total_requirements = all_requirements.len() as i32;
        let mandatory_count = mandatory_requirements.len() as i32;
        let fulfilled_count = approved_docs.len() as i32;
        let fulfilled_mandatory_count = fulfilled_mandatory.len() as i32;

        let compliance_percentage = if mandatory_count > 0 {
            Decimal::from(fulfilled_mandatory_count * 100) / Decimal::from(mandatory_count)
        } else {
            Decimal::from(100)
        };

        let is_compliant = missing_mandatory.is_empty();

        Ok(ComplianceStatus {
            license_id: license_id.to_string(),
            total_requirements,
            mandatory_requirements: mandatory_count,
            fulfilled_requirements: fulfilled_count,
            fulfilled_mandatory: fulfilled_mandatory_count,
            compliance_percentage,
            is_compliant,
            missing_mandatory,
        })
    }

    async fn recalculate_compliance(&self, tenant_id: &TenantId, license_id: &str) -> Result<()> {
        let compliance = self.check_compliance(tenant_id, license_id).await?;

        self.repo
            .update_compliance_percentage(tenant_id, license_id, compliance.compliance_percentage)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Expiry Management
    // ========================================================================

    pub async fn get_expiring_licenses(&self, days: i32) -> Result<Vec<TenantLicenseRow>> {
        self.repo.get_expiring_licenses(days).await
    }

    pub async fn check_and_expire_licenses(&self) -> Result<i32> {
        let expired = self.repo.get_expiring_licenses(0).await?;
        let mut count = 0;

        for license in expired {
            if let Some(expires_at) = license.expires_at {
                if expires_at <= chrono::Utc::now() {
                    let tenant_id = TenantId::new(&license.tenant_id);

                    if let Err(e) = self
                        .repo
                        .update_license_status(
                            &tenant_id,
                            &license.id,
                            LicenseStatus::Expired,
                            None,
                            Some("Automatically expired"),
                            None,
                        )
                        .await
                    {
                        warn!(
                            license_id = %license.id,
                            error = %e,
                            "Failed to expire license"
                        );
                    } else {
                        count += 1;
                        info!(
                            tenant_id = %license.tenant_id,
                            license_id = %license.id,
                            "License expired"
                        );
                    }
                }
            }
        }

        Ok(count)
    }
}
