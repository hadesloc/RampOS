//! Vietnam eKYC Provider Adapters
//!
//! This module provides adapters for integrating with Vietnam's eKYC providers:
//!
//! - **VNPay eKYC** - VNPay's identity verification service
//! - **FPT.AI** - FPT's AI-powered eKYC solution
//! - **Mock** - Testing adapter with configurable behavior
//!
//! ## Features
//!
//! - ID card OCR (CCCD, CMND, Passport)
//! - Face matching (selfie vs ID photo)
//! - Liveness detection (anti-spoofing)
//! - Address verification

pub mod fpt_ai;
pub mod mock;
pub mod vnpay;

pub use fpt_ai::FptAiEkycProvider;
pub use mock::MockEkycProvider;
pub use vnpay::VnpayEkycProvider;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::Result;
use serde::{Deserialize, Serialize};

/// Supported ID document types for Vietnam
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IdDocumentType {
    /// Căn cước công dân (Citizen Identity Card - new format)
    Cccd,
    /// Chứng minh nhân dân (People's Identity Card - old format)
    Cmnd,
    /// Hộ chiếu (Passport)
    Passport,
    /// Giấy phép lái xe (Driver's License)
    DriverLicense,
}

impl IdDocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IdDocumentType::Cccd => "CCCD",
            IdDocumentType::Cmnd => "CMND",
            IdDocumentType::Passport => "PASSPORT",
            IdDocumentType::DriverLicense => "DRIVER_LICENSE",
        }
    }
}

/// ID verification request
#[derive(Debug, Clone)]
pub struct IdVerificationRequest {
    /// Base64-encoded image of the ID front
    pub id_front_image: Vec<u8>,
    /// Base64-encoded image of the ID back (optional for passport)
    pub id_back_image: Option<Vec<u8>>,
    /// Expected document type
    pub document_type: IdDocumentType,
    /// Request reference for tracking
    pub request_id: String,
}

/// ID verification result from OCR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdVerification {
    /// Unique verification ID from provider
    pub verification_id: String,
    /// Whether verification was successful
    pub success: bool,
    /// Extracted full name
    pub full_name: Option<String>,
    /// Extracted ID number
    pub id_number: Option<String>,
    /// Detected document type
    pub document_type: IdDocumentType,
    /// Date of birth (YYYY-MM-DD)
    pub date_of_birth: Option<String>,
    /// Gender
    pub gender: Option<String>,
    /// Nationality
    pub nationality: Option<String>,
    /// Place of origin (quê quán)
    pub place_of_origin: Option<String>,
    /// Place of residence (nơi thường trú)
    pub place_of_residence: Option<String>,
    /// Expiry date of document
    pub expiry_date: Option<String>,
    /// Issue date of document
    pub issue_date: Option<String>,
    /// Issuing authority
    pub issuing_authority: Option<String>,
    /// Overall confidence score (0.0 - 1.0)
    pub confidence_score: f64,
    /// Individual field confidence scores
    pub field_confidences: std::collections::HashMap<String, f64>,
    /// Error message if verification failed
    pub error_message: Option<String>,
    /// Raw response from provider
    pub raw_response: Option<serde_json::Value>,
    /// Timestamp of verification
    pub verified_at: DateTime<Utc>,
}

/// Face matching request
#[derive(Debug, Clone)]
pub struct FaceMatchRequest {
    /// Base64-encoded selfie image
    pub selfie_image: Vec<u8>,
    /// Base64-encoded ID photo (extracted from ID or separate)
    pub id_photo_image: Vec<u8>,
    /// Request reference for tracking
    pub request_id: String,
}

/// Face matching result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceMatch {
    /// Unique match ID from provider
    pub match_id: String,
    /// Whether faces match
    pub is_match: bool,
    /// Similarity score (0.0 - 1.0)
    pub similarity_score: f64,
    /// Confidence level of the match
    pub confidence: FaceMatchConfidence,
    /// Error message if matching failed
    pub error_message: Option<String>,
    /// Raw response from provider
    pub raw_response: Option<serde_json::Value>,
    /// Timestamp of matching
    pub matched_at: DateTime<Utc>,
}

/// Face match confidence levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaceMatchConfidence {
    /// Very high confidence match
    VeryHigh,
    /// High confidence match
    High,
    /// Medium confidence - may need manual review
    Medium,
    /// Low confidence - likely not a match
    Low,
    /// Very low confidence - not a match
    VeryLow,
}

impl FaceMatchConfidence {
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 0.95 => FaceMatchConfidence::VeryHigh,
            s if s >= 0.85 => FaceMatchConfidence::High,
            s if s >= 0.70 => FaceMatchConfidence::Medium,
            s if s >= 0.50 => FaceMatchConfidence::Low,
            _ => FaceMatchConfidence::VeryLow,
        }
    }
}

/// Liveness detection request
#[derive(Debug, Clone)]
pub struct LivenessRequest {
    /// Video data for liveness check (or multiple frame images)
    pub video_data: Vec<u8>,
    /// Request reference for tracking
    pub request_id: String,
    /// Type of liveness check
    pub check_type: LivenessCheckType,
}

/// Types of liveness checks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LivenessCheckType {
    /// Passive liveness (single image)
    Passive,
    /// Active liveness (requires user actions like blinking)
    Active,
    /// Video-based liveness
    Video,
}

/// Liveness detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessResult {
    /// Unique liveness check ID from provider
    pub liveness_id: String,
    /// Whether liveness check passed
    pub is_live: bool,
    /// Liveness confidence score (0.0 - 1.0)
    pub liveness_score: f64,
    /// Types of spoofing detected (if any)
    pub spoofing_types: Vec<SpoofingType>,
    /// Error message if check failed
    pub error_message: Option<String>,
    /// Raw response from provider
    pub raw_response: Option<serde_json::Value>,
    /// Timestamp of check
    pub checked_at: DateTime<Utc>,
}

/// Types of spoofing attacks detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpoofingType {
    /// Printed photo attack
    PrintedPhoto,
    /// Screen replay attack (phone/tablet showing photo)
    ScreenReplay,
    /// 3D mask attack
    Mask3d,
    /// Video replay attack
    VideoReplay,
    /// Deepfake detected
    Deepfake,
    /// Unknown spoofing type
    Unknown,
}

/// Address verification request
#[derive(Debug, Clone)]
pub struct AddressVerificationRequest {
    /// Full address to verify
    pub address: String,
    /// Province/City
    pub province: Option<String>,
    /// District
    pub district: Option<String>,
    /// Ward/Commune
    pub ward: Option<String>,
    /// Request reference for tracking
    pub request_id: String,
}

/// Address verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressVerification {
    /// Unique verification ID from provider
    pub verification_id: String,
    /// Whether address is valid
    pub is_valid: bool,
    /// Normalized/standardized address
    pub normalized_address: Option<String>,
    /// Province code
    pub province_code: Option<String>,
    /// District code
    pub district_code: Option<String>,
    /// Ward code
    pub ward_code: Option<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence_score: f64,
    /// Error message if verification failed
    pub error_message: Option<String>,
    /// Raw response from provider
    pub raw_response: Option<serde_json::Value>,
    /// Timestamp of verification
    pub verified_at: DateTime<Utc>,
}

/// eKYC provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EkycProviderConfig {
    /// Provider code (vnpay, fpt_ai, etc.)
    pub provider_code: String,
    /// API base URL
    pub api_base_url: String,
    /// API key/client ID
    pub api_key: String,
    /// API secret/client secret
    pub api_secret: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Enable sandbox/test mode
    pub sandbox_mode: bool,
    /// Additional provider-specific configuration
    pub extra: serde_json::Value,
}

impl Default for EkycProviderConfig {
    fn default() -> Self {
        Self {
            provider_code: "mock".to_string(),
            api_base_url: String::new(),
            api_key: String::new(),
            api_secret: String::new(),
            timeout_secs: 30,
            max_retries: 3,
            sandbox_mode: true,
            extra: serde_json::json!({}),
        }
    }
}

/// eKYC Provider trait - implement this for each eKYC provider
#[async_trait]
pub trait EkycProvider: Send + Sync {
    /// Get provider code/identifier
    fn provider_code(&self) -> &str;

    /// Get provider name
    fn provider_name(&self) -> &str;

    /// Check if provider is in sandbox/test mode
    fn is_sandbox_mode(&self) -> bool {
        false
    }

    /// Verify ID document using OCR
    async fn verify_id(&self, request: IdVerificationRequest) -> Result<IdVerification>;

    /// Match face between selfie and ID photo
    async fn match_face(&self, request: FaceMatchRequest) -> Result<FaceMatch>;

    /// Check liveness (anti-spoofing)
    async fn check_liveness(&self, request: LivenessRequest) -> Result<LivenessResult>;

    /// Verify address (optional - not all providers support this)
    async fn verify_address(
        &self,
        _request: AddressVerificationRequest,
    ) -> Result<AddressVerification> {
        // Default implementation returns not supported
        Err(ramp_common::Error::Internal(
            "Address verification not supported by this provider".to_string(),
        ))
    }

    /// Health check - verify provider connectivity
    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

/// Combined eKYC verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullEkycResult {
    /// ID verification result
    pub id_verification: IdVerification,
    /// Face match result
    pub face_match: FaceMatch,
    /// Liveness result
    pub liveness: LivenessResult,
    /// Address verification result (optional)
    pub address_verification: Option<AddressVerification>,
    /// Overall eKYC passed
    pub passed: bool,
    /// Overall confidence score
    pub overall_score: f64,
    /// Reasons for failure (if any)
    pub failure_reasons: Vec<String>,
    /// Provider reference ID
    pub provider_reference: String,
    /// Timestamp
    pub completed_at: DateTime<Utc>,
}

impl FullEkycResult {
    /// Calculate overall result from individual components
    pub fn calculate(
        id_verification: IdVerification,
        face_match: FaceMatch,
        liveness: LivenessResult,
        address_verification: Option<AddressVerification>,
    ) -> Self {
        let mut failure_reasons = Vec::new();
        let mut passed = true;

        // Check ID verification
        if !id_verification.success {
            passed = false;
            if let Some(ref msg) = id_verification.error_message {
                failure_reasons.push(format!("ID verification failed: {}", msg));
            } else {
                failure_reasons.push("ID verification failed".to_string());
            }
        }

        // Check face match
        if !face_match.is_match {
            passed = false;
            failure_reasons.push(format!(
                "Face match failed: similarity {}%",
                (face_match.similarity_score * 100.0) as u32
            ));
        }

        // Check liveness
        if !liveness.is_live {
            passed = false;
            let spoofing_types: Vec<_> = liveness
                .spoofing_types
                .iter()
                .map(|t| format!("{:?}", t))
                .collect();
            if spoofing_types.is_empty() {
                failure_reasons.push("Liveness check failed".to_string());
            } else {
                failure_reasons.push(format!(
                    "Liveness check failed: detected {}",
                    spoofing_types.join(", ")
                ));
            }
        }

        // Calculate overall score (weighted average)
        let overall_score = (id_verification.confidence_score * 0.3
            + face_match.similarity_score * 0.4
            + liveness.liveness_score * 0.3)
            .min(1.0);

        let provider_reference = format!(
            "ekyc_{}_{}",
            id_verification.verification_id, face_match.match_id
        );

        Self {
            id_verification,
            face_match,
            liveness,
            address_verification,
            passed,
            overall_score,
            failure_reasons,
            provider_reference,
            completed_at: Utc::now(),
        }
    }
}
