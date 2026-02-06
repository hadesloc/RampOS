//! Vietnam Licensing Requirements Types
//!
//! Types for tracking Vietnam regulatory licensing requirements and compliance deadlines.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::TenantId;

/// Unique identifier for a license requirement
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LicenseRequirementId(pub String);

impl LicenseRequirementId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn generate() -> Self {
        Self(format!("lic_{}", Uuid::now_v7()))
    }
}

impl std::fmt::Display for LicenseRequirementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a license submission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LicenseSubmissionId(pub String);

impl LicenseSubmissionId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn generate() -> Self {
        Self(format!("sub_{}", Uuid::now_v7()))
    }
}

impl std::fmt::Display for LicenseSubmissionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// License requirement status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseStatus {
    /// Requirement is pending - no submission yet
    Pending,
    /// Documents have been submitted, awaiting review
    Submitted,
    /// License has been approved by regulator
    Approved,
    /// License has expired and needs renewal
    Expired,
    /// License was rejected, may need resubmission
    Rejected,
    /// License is under review by regulator
    UnderReview,
}

impl LicenseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LicenseStatus::Pending => "PENDING",
            LicenseStatus::Submitted => "SUBMITTED",
            LicenseStatus::Approved => "APPROVED",
            LicenseStatus::Expired => "EXPIRED",
            LicenseStatus::Rejected => "REJECTED",
            LicenseStatus::UnderReview => "UNDER_REVIEW",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "PENDING" => Some(LicenseStatus::Pending),
            "SUBMITTED" => Some(LicenseStatus::Submitted),
            "APPROVED" => Some(LicenseStatus::Approved),
            "EXPIRED" => Some(LicenseStatus::Expired),
            "REJECTED" => Some(LicenseStatus::Rejected),
            "UNDER_REVIEW" => Some(LicenseStatus::UnderReview),
            _ => None,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, LicenseStatus::Approved)
    }

    pub fn requires_action(&self) -> bool {
        matches!(
            self,
            LicenseStatus::Pending | LicenseStatus::Expired | LicenseStatus::Rejected
        )
    }
}

impl std::fmt::Display for LicenseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Type of license requirement (Vietnam-specific)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseType {
    /// State Bank of Vietnam license for payment services
    SbvPaymentLicense,
    /// Anti-money laundering registration
    AmlRegistration,
    /// Data protection registration with Ministry of Public Security
    DataProtection,
    /// Business registration certificate
    BusinessRegistration,
    /// Crypto asset service provider license (if applicable)
    CryptoLicense,
    /// Foreign exchange license
    ForexLicense,
}

impl LicenseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LicenseType::SbvPaymentLicense => "SBV_PAYMENT_LICENSE",
            LicenseType::AmlRegistration => "AML_REGISTRATION",
            LicenseType::DataProtection => "DATA_PROTECTION",
            LicenseType::BusinessRegistration => "BUSINESS_REGISTRATION",
            LicenseType::CryptoLicense => "CRYPTO_LICENSE",
            LicenseType::ForexLicense => "FOREX_LICENSE",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "SBV_PAYMENT_LICENSE" => Some(LicenseType::SbvPaymentLicense),
            "AML_REGISTRATION" => Some(LicenseType::AmlRegistration),
            "DATA_PROTECTION" => Some(LicenseType::DataProtection),
            "BUSINESS_REGISTRATION" => Some(LicenseType::BusinessRegistration),
            "CRYPTO_LICENSE" => Some(LicenseType::CryptoLicense),
            "FOREX_LICENSE" => Some(LicenseType::ForexLicense),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            LicenseType::SbvPaymentLicense => "State Bank of Vietnam Payment Service License",
            LicenseType::AmlRegistration => "Anti-Money Laundering Registration",
            LicenseType::DataProtection => "Data Protection Registration (MPS)",
            LicenseType::BusinessRegistration => "Business Registration Certificate",
            LicenseType::CryptoLicense => "Crypto Asset Service Provider License",
            LicenseType::ForexLicense => "Foreign Exchange License",
        }
    }
}

impl std::fmt::Display for LicenseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A licensing requirement that tenants must fulfill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseRequirement {
    pub id: LicenseRequirementId,
    pub name: String,
    pub description: String,
    pub license_type: LicenseType,
    pub regulatory_body: String,
    pub deadline: Option<DateTime<Utc>>,
    pub renewal_period_days: Option<i32>,
    pub required_documents: Vec<String>,
    pub is_mandatory: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Tenant's license status for a specific requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantLicenseStatus {
    pub tenant_id: TenantId,
    pub requirement_id: LicenseRequirementId,
    pub status: LicenseStatus,
    pub license_number: Option<String>,
    pub issue_date: Option<DateTime<Utc>>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub last_submission_id: Option<LicenseSubmissionId>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A document submission for a license requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseSubmission {
    pub id: LicenseSubmissionId,
    pub tenant_id: TenantId,
    pub requirement_id: LicenseRequirementId,
    pub documents: Vec<SubmittedDocument>,
    pub status: SubmissionStatus,
    pub submitted_by: String,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewer_notes: Option<String>,
}

/// A document included in a submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmittedDocument {
    pub name: String,
    pub file_url: String,
    pub file_hash: String,
    pub file_size_bytes: i64,
    pub uploaded_at: DateTime<Utc>,
}

/// Status of a license submission
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubmissionStatus {
    Draft,
    Submitted,
    UnderReview,
    Approved,
    Rejected,
    Cancelled,
}

impl SubmissionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubmissionStatus::Draft => "DRAFT",
            SubmissionStatus::Submitted => "SUBMITTED",
            SubmissionStatus::UnderReview => "UNDER_REVIEW",
            SubmissionStatus::Approved => "APPROVED",
            SubmissionStatus::Rejected => "REJECTED",
            SubmissionStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "DRAFT" => Some(SubmissionStatus::Draft),
            "SUBMITTED" => Some(SubmissionStatus::Submitted),
            "UNDER_REVIEW" => Some(SubmissionStatus::UnderReview),
            "APPROVED" => Some(SubmissionStatus::Approved),
            "REJECTED" => Some(SubmissionStatus::Rejected),
            "CANCELLED" => Some(SubmissionStatus::Cancelled),
            _ => None,
        }
    }
}

impl std::fmt::Display for SubmissionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Upcoming deadline information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseDeadline {
    pub requirement_id: LicenseRequirementId,
    pub requirement_name: String,
    pub license_type: LicenseType,
    pub deadline: DateTime<Utc>,
    pub days_remaining: i64,
    pub status: LicenseStatus,
    pub is_overdue: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_status_from_str() {
        assert_eq!(
            LicenseStatus::from_str("PENDING"),
            Some(LicenseStatus::Pending)
        );
        assert_eq!(
            LicenseStatus::from_str("approved"),
            Some(LicenseStatus::Approved)
        );
        assert_eq!(LicenseStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_license_status_properties() {
        assert!(LicenseStatus::Approved.is_active());
        assert!(!LicenseStatus::Pending.is_active());
        assert!(LicenseStatus::Pending.requires_action());
        assert!(LicenseStatus::Expired.requires_action());
        assert!(!LicenseStatus::Approved.requires_action());
    }

    #[test]
    fn test_license_type_from_str() {
        assert_eq!(
            LicenseType::from_str("SBV_PAYMENT_LICENSE"),
            Some(LicenseType::SbvPaymentLicense)
        );
        assert_eq!(
            LicenseType::from_str("aml_registration"),
            Some(LicenseType::AmlRegistration)
        );
        assert_eq!(LicenseType::from_str("invalid"), None);
    }

    #[test]
    fn test_license_requirement_id_generation() {
        let id = LicenseRequirementId::generate();
        assert!(id.0.starts_with("lic_"));
    }

    #[test]
    fn test_license_submission_id_generation() {
        let id = LicenseSubmissionId::generate();
        assert!(id.0.starts_with("sub_"));
    }
}
