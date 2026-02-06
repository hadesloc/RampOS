//! FPT.AI eKYC Provider
//!
//! Integration with FPT.AI's eKYC service for Vietnam identity verification.
//!
//! ## Features
//! - ID card OCR (CCCD, CMND, Passport)
//! - Face matching (selfie vs ID photo)
//! - Liveness detection (passive and active)
//!
//! ## API Documentation
//! https://fpt.ai/ekyc-api

use async_trait::async_trait;
use chrono::Utc;
use ramp_common::{Error, Result};
use reqwest::{multipart, Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use super::{
    EkycProvider, EkycProviderConfig, FaceMatch, FaceMatchConfidence, FaceMatchRequest,
    IdDocumentType, IdVerification, IdVerificationRequest, LivenessCheckType, LivenessRequest,
    LivenessResult, SpoofingType,
};

/// FPT.AI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FptAiEkycConfig {
    /// Base configuration
    #[serde(flatten)]
    pub base: EkycProviderConfig,
    /// FPT.AI API token
    pub api_token: String,
    /// Whether to enable anti-fraud checks
    pub enable_anti_fraud: bool,
    /// Minimum confidence threshold (0.0 - 1.0)
    pub min_confidence_threshold: f64,
}

impl Default for FptAiEkycConfig {
    fn default() -> Self {
        Self {
            base: EkycProviderConfig {
                provider_code: "fpt_ai_ekyc".to_string(),
                api_base_url: "https://api.fpt.ai/vision/idr/vnm".to_string(),
                timeout_secs: 30,
                max_retries: 3,
                sandbox_mode: true,
                ..Default::default()
            },
            api_token: String::new(),
            enable_anti_fraud: true,
            min_confidence_threshold: 0.7,
        }
    }
}

/// FPT.AI eKYC Provider
pub struct FptAiEkycProvider {
    config: FptAiEkycConfig,
    http_client: Client,
}

impl FptAiEkycProvider {
    pub fn new(config: FptAiEkycConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.base.timeout_secs))
            .user_agent("RampOS-eKYC/1.0")
            .build()
            .map_err(|e| Error::Internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Make authenticated request to FPT.AI API with multipart form data
    /// Note: multipart::Form is not cloneable in reqwest 0.11, so no retry for these requests
    async fn make_multipart_request<R: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        form: multipart::Form,
    ) -> Result<R> {
        let url = format!("{}{}", self.config.base.api_base_url, endpoint);

        let response = self
            .http_client
            .post(&url)
            .header("api-key", &self.config.api_token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::ExternalService {
                service: "FPT.AI".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();

        if status.is_success() {
            return serde_json::from_str(&body_text).map_err(|e| Error::ExternalService {
                service: "FPT.AI".to_string(),
                message: format!("Failed to parse response: {} - {}", e, body_text),
            });
        }

        // Handle rate limiting
        if status.as_u16() == 429 {
            warn!("FPT.AI rate limited");
            return Err(Error::ExternalService {
                service: "FPT.AI".to_string(),
                message: "Rate limited, please retry later".to_string(),
            });
        }

        error!(
            status = %status,
            body = %body_text,
            "FPT.AI API error"
        );

        Err(Error::ExternalService {
            service: "FPT.AI".to_string(),
            message: format!("API error {}: {}", status, body_text),
        })
    }

    /// Get FPT.AI endpoint for document type
    fn get_ocr_endpoint(doc_type: IdDocumentType) -> &'static str {
        match doc_type {
            IdDocumentType::Cccd => "",
            IdDocumentType::Cmnd => "",
            IdDocumentType::Passport => "/passport",
            IdDocumentType::DriverLicense => "/driver-license",
        }
    }

    /// Parse FPT.AI document type
    fn parse_document_type(fpt_type: &str) -> IdDocumentType {
        match fpt_type.to_lowercase().as_str() {
            "chip_front" | "chip_back" | "new_front" | "new_back" => IdDocumentType::Cccd,
            "old_front" | "old_back" => IdDocumentType::Cmnd,
            "passport" => IdDocumentType::Passport,
            "driver_license" | "driving_license" => IdDocumentType::DriverLicense,
            _ => IdDocumentType::Cccd,
        }
    }
}

// FPT.AI API response types
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct FptAiOcrResponse {
    errorCode: i32,
    errorMessage: String,
    data: Option<Vec<FptAiOcrData>>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct FptAiOcrData {
    id: Option<String>,
    name: Option<String>,
    dob: Option<String>,
    sex: Option<String>,
    nationality: Option<String>,
    home: Option<String>,
    address: Option<String>,
    doe: Option<String>,
    doi: Option<String>,
    #[serde(rename = "type")]
    doc_type: Option<String>,
    overall_score: Option<f64>,
    type_score: Option<f64>,
    tampering: Option<FptAiTamperingInfo>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct FptAiTamperingInfo {
    is_tampered: Option<bool>,
    #[allow(dead_code)]
    confidence: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct FptAiFaceMatchResponse {
    errorCode: i32,
    errorMessage: String,
    data: Option<FptAiFaceMatchData>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct FptAiFaceMatchData {
    isMatch: bool,
    similarity: f64,
    #[allow(dead_code)]
    isBothImgIDCard: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct FptAiLivenessResponse {
    errorCode: i32,
    errorMessage: String,
    data: Option<FptAiLivenessData>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct FptAiLivenessData {
    isLive: bool,
    score: f64,
    deepfake: Option<FptAiDeepfakeInfo>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct FptAiDeepfakeInfo {
    is_deepfake: bool,
    #[allow(dead_code)]
    confidence: f64,
}

#[async_trait]
impl EkycProvider for FptAiEkycProvider {
    fn provider_code(&self) -> &str {
        &self.config.base.provider_code
    }

    fn provider_name(&self) -> &str {
        "FPT.AI eKYC"
    }

    fn is_sandbox_mode(&self) -> bool {
        self.config.base.sandbox_mode
    }

    async fn verify_id(&self, request: IdVerificationRequest) -> Result<IdVerification> {
        debug!(
            request_id = %request.request_id,
            doc_type = ?request.document_type,
            "FPT.AI eKYC: Verifying ID"
        );

        let endpoint = Self::get_ocr_endpoint(request.document_type);

        // Build multipart form with front image
        let front_part = multipart::Part::bytes(request.id_front_image.clone())
            .file_name("front.jpg")
            .mime_str("image/jpeg")
            .map_err(|e| Error::Internal(format!("Failed to create multipart: {}", e)))?;

        let mut form = multipart::Form::new().part("image", front_part);

        // Add back image if provided
        if let Some(back_image) = &request.id_back_image {
            let back_part = multipart::Part::bytes(back_image.clone())
                .file_name("back.jpg")
                .mime_str("image/jpeg")
                .map_err(|e| Error::Internal(format!("Failed to create multipart: {}", e)))?;
            form = form.part("image_back", back_part);
        }

        let response: FptAiOcrResponse = self.make_multipart_request(endpoint, form).await?;

        if response.errorCode != 0 {
            return Ok(IdVerification {
                verification_id: format!("fpt_err_{}", uuid::Uuid::now_v7()),
                success: false,
                full_name: None,
                id_number: None,
                document_type: request.document_type,
                date_of_birth: None,
                gender: None,
                nationality: None,
                place_of_origin: None,
                place_of_residence: None,
                expiry_date: None,
                issue_date: None,
                issuing_authority: None,
                confidence_score: 0.0,
                field_confidences: HashMap::new(),
                error_message: Some(response.errorMessage),
                raw_response: None,
                verified_at: Utc::now(),
            });
        }

        let data = response.data.and_then(|d| d.into_iter().next());
        let data = match data {
            Some(d) => d,
            None => {
                return Ok(IdVerification {
                    verification_id: format!("fpt_empty_{}", uuid::Uuid::now_v7()),
                    success: false,
                    full_name: None,
                    id_number: None,
                    document_type: request.document_type,
                    date_of_birth: None,
                    gender: None,
                    nationality: None,
                    place_of_origin: None,
                    place_of_residence: None,
                    expiry_date: None,
                    issue_date: None,
                    issuing_authority: None,
                    confidence_score: 0.0,
                    field_confidences: HashMap::new(),
                    error_message: Some("No OCR data returned".to_string()),
                    raw_response: None,
                    verified_at: Utc::now(),
                });
            }
        };

        let verification_id = format!("fpt_{}", uuid::Uuid::now_v7());
        let confidence_score = data.overall_score.unwrap_or(0.0);
        let detected_doc_type = data
            .doc_type
            .as_ref()
            .map(|t| Self::parse_document_type(t))
            .unwrap_or(request.document_type);

        // Check for tampering if anti-fraud is enabled
        let is_tampered = data
            .tampering
            .as_ref()
            .map(|t| t.is_tampered.unwrap_or(false))
            .unwrap_or(false);

        let success = confidence_score >= self.config.min_confidence_threshold && !is_tampered;

        // Build field confidences
        let mut field_confidences = HashMap::new();
        if let Some(type_score) = data.type_score {
            field_confidences.insert("document_type".to_string(), type_score);
        }

        let error_message = if is_tampered {
            Some("Document tampering detected".to_string())
        } else if confidence_score < self.config.min_confidence_threshold {
            Some(format!(
                "Confidence score {} below threshold {}",
                confidence_score, self.config.min_confidence_threshold
            ))
        } else {
            None
        };

        info!(
            verification_id = %verification_id,
            confidence = %confidence_score,
            is_tampered = %is_tampered,
            "FPT.AI eKYC: ID verification completed"
        );

        Ok(IdVerification {
            verification_id,
            success,
            full_name: data.name,
            id_number: data.id,
            document_type: detected_doc_type,
            date_of_birth: data.dob,
            gender: data.sex,
            nationality: data.nationality,
            place_of_origin: data.home,
            place_of_residence: data.address,
            expiry_date: data.doe,
            issue_date: data.doi,
            issuing_authority: None, // FPT.AI doesn't return this
            confidence_score,
            field_confidences,
            error_message,
            raw_response: None,
            verified_at: Utc::now(),
        })
    }

    async fn match_face(&self, request: FaceMatchRequest) -> Result<FaceMatch> {
        debug!(
            request_id = %request.request_id,
            "FPT.AI eKYC: Matching faces"
        );

        // Build multipart form with both images
        let selfie_part = multipart::Part::bytes(request.selfie_image.clone())
            .file_name("selfie.jpg")
            .mime_str("image/jpeg")
            .map_err(|e| Error::Internal(format!("Failed to create multipart: {}", e)))?;

        let id_photo_part = multipart::Part::bytes(request.id_photo_image.clone())
            .file_name("id_photo.jpg")
            .mime_str("image/jpeg")
            .map_err(|e| Error::Internal(format!("Failed to create multipart: {}", e)))?;

        let form = multipart::Form::new()
            .part("file[]", selfie_part)
            .part("file[]", id_photo_part);

        // Use face-matching endpoint
        let response: FptAiFaceMatchResponse =
            self.make_multipart_request("/face-matching", form).await?;

        if response.errorCode != 0 {
            return Ok(FaceMatch {
                match_id: format!("fpt_err_{}", uuid::Uuid::now_v7()),
                is_match: false,
                similarity_score: 0.0,
                confidence: FaceMatchConfidence::VeryLow,
                error_message: Some(response.errorMessage),
                raw_response: None,
                matched_at: Utc::now(),
            });
        }

        let data = response.data.ok_or_else(|| Error::ExternalService {
            service: "FPT.AI".to_string(),
            message: "Missing data in face match response".to_string(),
        })?;

        let match_id = format!("fpt_face_{}", uuid::Uuid::now_v7());

        info!(
            match_id = %match_id,
            similarity = %data.similarity,
            is_match = %data.isMatch,
            "FPT.AI eKYC: Face matching completed"
        );

        Ok(FaceMatch {
            match_id,
            is_match: data.isMatch,
            similarity_score: data.similarity,
            confidence: FaceMatchConfidence::from_score(data.similarity),
            error_message: None,
            raw_response: None,
            matched_at: Utc::now(),
        })
    }

    async fn check_liveness(&self, request: LivenessRequest) -> Result<LivenessResult> {
        debug!(
            request_id = %request.request_id,
            check_type = ?request.check_type,
            "FPT.AI eKYC: Checking liveness"
        );

        // Determine endpoint based on check type
        let endpoint = match request.check_type {
            LivenessCheckType::Passive => "/liveness",
            LivenessCheckType::Active => "/liveness/active",
            LivenessCheckType::Video => "/liveness/video",
        };

        // Build multipart form
        let video_part = multipart::Part::bytes(request.video_data.clone())
            .file_name("liveness.mp4")
            .mime_str("video/mp4")
            .map_err(|e| Error::Internal(format!("Failed to create multipart: {}", e)))?;

        let form = multipart::Form::new().part("file", video_part);

        let response: FptAiLivenessResponse = self.make_multipart_request(endpoint, form).await?;

        if response.errorCode != 0 {
            return Ok(LivenessResult {
                liveness_id: format!("fpt_err_{}", uuid::Uuid::now_v7()),
                is_live: false,
                liveness_score: 0.0,
                spoofing_types: vec![],
                error_message: Some(response.errorMessage),
                raw_response: None,
                checked_at: Utc::now(),
            });
        }

        let data = response.data.ok_or_else(|| Error::ExternalService {
            service: "FPT.AI".to_string(),
            message: "Missing data in liveness response".to_string(),
        })?;

        let liveness_id = format!("fpt_live_{}", uuid::Uuid::now_v7());

        // Check for deepfake
        let mut spoofing_types = Vec::new();
        if let Some(deepfake_info) = &data.deepfake {
            if deepfake_info.is_deepfake {
                spoofing_types.push(SpoofingType::Deepfake);
            }
        }

        // If not live but no deepfake detected, classify as unknown
        if !data.isLive && spoofing_types.is_empty() {
            spoofing_types.push(SpoofingType::Unknown);
        }

        info!(
            liveness_id = %liveness_id,
            is_live = %data.isLive,
            score = %data.score,
            "FPT.AI eKYC: Liveness check completed"
        );

        Ok(LivenessResult {
            liveness_id,
            is_live: data.isLive,
            liveness_score: data.score,
            spoofing_types,
            error_message: if data.isLive {
                None
            } else {
                Some("Liveness check failed".to_string())
            },
            raw_response: None,
            checked_at: Utc::now(),
        })
    }

    async fn health_check(&self) -> Result<bool> {
        // FPT.AI doesn't have a dedicated health endpoint
        // We can use a lightweight request to check connectivity
        let test_image = vec![0u8; 100]; // Minimal test data
        let part = multipart::Part::bytes(test_image)
            .file_name("test.jpg")
            .mime_str("image/jpeg")
            .map_err(|e| Error::Internal(format!("Failed to create multipart: {}", e)))?;

        let form = multipart::Form::new().part("image", part);

        // We expect this to fail (bad image), but if we get a response, the service is up
        match self.make_multipart_request::<FptAiOcrResponse>("", form).await {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's an API error (service is up but request failed)
                if e.to_string().contains("API error") {
                    Ok(true)
                } else {
                    warn!(error = %e, "FPT.AI health check failed");
                    Ok(false)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fpt_ai_config_default() {
        let config = FptAiEkycConfig::default();
        assert_eq!(config.base.provider_code, "fpt_ai_ekyc");
        assert!(config.enable_anti_fraud);
        assert_eq!(config.min_confidence_threshold, 0.7);
    }

    #[test]
    fn test_parse_document_type() {
        assert!(matches!(
            FptAiEkycProvider::parse_document_type("chip_front"),
            IdDocumentType::Cccd
        ));
        assert!(matches!(
            FptAiEkycProvider::parse_document_type("old_front"),
            IdDocumentType::Cmnd
        ));
        assert!(matches!(
            FptAiEkycProvider::parse_document_type("passport"),
            IdDocumentType::Passport
        ));
    }

    #[test]
    fn test_get_ocr_endpoint() {
        assert_eq!(FptAiEkycProvider::get_ocr_endpoint(IdDocumentType::Cccd), "");
        assert_eq!(
            FptAiEkycProvider::get_ocr_endpoint(IdDocumentType::Passport),
            "/passport"
        );
    }
}
