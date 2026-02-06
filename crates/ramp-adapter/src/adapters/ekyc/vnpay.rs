//! VNPay eKYC Provider
//!
//! Integration with VNPay's eKYC service for Vietnam identity verification.
//!
//! ## Features
//! - ID card OCR (CCCD, CMND, Passport)
//! - Face matching (selfie vs ID photo)
//! - Liveness detection
//!
//! ## API Documentation
//! https://developers.vnpay.vn/docs/ekyc

use async_trait::async_trait;
use chrono::Utc;
use ramp_common::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use super::{
    EkycProvider, EkycProviderConfig, FaceMatch, FaceMatchConfidence, FaceMatchRequest,
    IdDocumentType, IdVerification, IdVerificationRequest, LivenessCheckType, LivenessRequest,
    LivenessResult, SpoofingType,
};

/// VNPay-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnpayEkycConfig {
    /// Base configuration
    #[serde(flatten)]
    pub base: EkycProviderConfig,
    /// VNPay merchant ID
    pub merchant_id: String,
    /// VNPay terminal ID
    pub terminal_id: String,
    /// Secret key for signing requests
    pub secret_key: String,
}

impl Default for VnpayEkycConfig {
    fn default() -> Self {
        Self {
            base: EkycProviderConfig {
                provider_code: "vnpay_ekyc".to_string(),
                api_base_url: "https://sandbox.vnpay.vn/ekyc/api/v1".to_string(),
                timeout_secs: 30,
                max_retries: 3,
                sandbox_mode: true,
                ..Default::default()
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            secret_key: String::new(),
        }
    }
}

/// VNPay eKYC Provider
pub struct VnpayEkycProvider {
    config: VnpayEkycConfig,
    http_client: Client,
}

impl VnpayEkycProvider {
    pub fn new(config: VnpayEkycConfig) -> Result<Self> {
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

    /// Generate request signature for VNPay API
    fn generate_signature(&self, data: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.config.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(data.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// Make authenticated request to VNPay API
    async fn make_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        body: &T,
    ) -> Result<R> {
        let url = format!("{}{}", self.config.base.api_base_url, endpoint);
        let body_json = serde_json::to_string(body)
            .map_err(|e| Error::Internal(format!("Failed to serialize request: {}", e)))?;

        let signature = self.generate_signature(&body_json);
        let timestamp = Utc::now().timestamp_millis().to_string();

        let mut retries = 0;
        let max_retries = self.config.base.max_retries;

        loop {
            let response = self
                .http_client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-VNPAY-Merchant-ID", &self.config.merchant_id)
                .header("X-VNPAY-Terminal-ID", &self.config.terminal_id)
                .header("X-VNPAY-Signature", &signature)
                .header("X-VNPAY-Timestamp", &timestamp)
                .body(body_json.clone())
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let body_text = resp.text().await.unwrap_or_default();

                    if status.is_success() {
                        return serde_json::from_str(&body_text).map_err(|e| {
                            Error::ExternalService {
                                service: "VNPay".to_string(),
                                message: format!("Failed to parse response: {} - {}", e, body_text),
                            }
                        });
                    }

                    // Handle rate limiting
                    if status.as_u16() == 429 && retries < max_retries {
                        retries += 1;
                        let delay = Duration::from_millis(1000 * 2u64.pow(retries));
                        warn!(
                            retries = %retries,
                            delay_ms = %delay.as_millis(),
                            "VNPay rate limited, retrying..."
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    // Handle other errors
                    error!(
                        status = %status,
                        body = %body_text,
                        "VNPay API error"
                    );

                    return Err(Error::ExternalService {
                        service: "VNPay".to_string(),
                        message: format!("API error {}: {}", status, body_text),
                    });
                }
                Err(e) => {
                    if retries < max_retries && e.is_timeout() {
                        retries += 1;
                        let delay = Duration::from_millis(1000 * 2u64.pow(retries));
                        warn!(
                            retries = %retries,
                            error = %e,
                            "VNPay request timeout, retrying..."
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    return Err(Error::ExternalService {
                        service: "VNPay".to_string(),
                        message: format!("Request failed: {}", e),
                    });
                }
            }
        }
    }

    /// Convert internal doc type to VNPay format
    fn doc_type_to_vnpay(doc_type: IdDocumentType) -> &'static str {
        match doc_type {
            IdDocumentType::Cccd => "CCCD",
            IdDocumentType::Cmnd => "CMND",
            IdDocumentType::Passport => "PASSPORT",
            IdDocumentType::DriverLicense => "GPLX",
        }
    }

    /// Convert VNPay doc type to internal format
    fn vnpay_to_doc_type(vnpay_type: &str) -> IdDocumentType {
        match vnpay_type.to_uppercase().as_str() {
            "CCCD" => IdDocumentType::Cccd,
            "CMND" => IdDocumentType::Cmnd,
            "PASSPORT" => IdDocumentType::Passport,
            "GPLX" | "DRIVER_LICENSE" => IdDocumentType::DriverLicense,
            _ => IdDocumentType::Cccd, // Default fallback
        }
    }
}

// VNPay API request/response types
#[derive(Debug, Serialize)]
struct VnpayIdVerifyRequest {
    request_id: String,
    document_type: String,
    front_image: String, // Base64
    back_image: Option<String>, // Base64
}

#[derive(Debug, Deserialize)]
struct VnpayIdVerifyResponse {
    code: String,
    message: String,
    data: Option<VnpayIdData>,
}

#[derive(Debug, Deserialize)]
struct VnpayIdData {
    verification_id: String,
    ocr_result: VnpayOcrResult,
    confidence_score: f64,
}

#[derive(Debug, Deserialize)]
struct VnpayOcrResult {
    full_name: Option<String>,
    id_number: Option<String>,
    date_of_birth: Option<String>,
    gender: Option<String>,
    nationality: Option<String>,
    place_of_origin: Option<String>,
    place_of_residence: Option<String>,
    expiry_date: Option<String>,
    issue_date: Option<String>,
    issuing_authority: Option<String>,
    document_type: Option<String>,
    field_confidences: Option<HashMap<String, f64>>,
}

#[derive(Debug, Serialize)]
struct VnpayFaceMatchRequest {
    request_id: String,
    selfie_image: String, // Base64
    id_photo_image: String, // Base64
}

#[derive(Debug, Deserialize)]
struct VnpayFaceMatchResponse {
    code: String,
    message: String,
    data: Option<VnpayFaceMatchData>,
}

#[derive(Debug, Deserialize)]
struct VnpayFaceMatchData {
    match_id: String,
    is_match: bool,
    similarity_score: f64,
}

#[derive(Debug, Serialize)]
struct VnpayLivenessRequest {
    request_id: String,
    video_data: String, // Base64
    check_type: String,
}

#[derive(Debug, Deserialize)]
struct VnpayLivenessResponse {
    code: String,
    message: String,
    data: Option<VnpayLivenessData>,
}

#[derive(Debug, Deserialize)]
struct VnpayLivenessData {
    liveness_id: String,
    is_live: bool,
    liveness_score: f64,
    spoofing_types: Option<Vec<String>>,
}

#[async_trait]
impl EkycProvider for VnpayEkycProvider {
    fn provider_code(&self) -> &str {
        &self.config.base.provider_code
    }

    fn provider_name(&self) -> &str {
        "VNPay eKYC"
    }

    fn is_sandbox_mode(&self) -> bool {
        self.config.base.sandbox_mode
    }

    async fn verify_id(&self, request: IdVerificationRequest) -> Result<IdVerification> {
        debug!(
            request_id = %request.request_id,
            doc_type = ?request.document_type,
            "VNPay eKYC: Verifying ID"
        );

        let vnpay_request = VnpayIdVerifyRequest {
            request_id: request.request_id.clone(),
            document_type: Self::doc_type_to_vnpay(request.document_type).to_string(),
            front_image: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &request.id_front_image,
            ),
            back_image: request.id_back_image.map(|img| {
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &img)
            }),
        };

        let response: VnpayIdVerifyResponse = self
            .make_request("/ocr/verify", &vnpay_request)
            .await?;

        if response.code != "00" && response.code != "0" && response.code != "200" {
            return Ok(IdVerification {
                verification_id: format!("vnpay_err_{}", uuid::Uuid::now_v7()),
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
                error_message: Some(response.message),
                raw_response: None,
                verified_at: Utc::now(),
            });
        }

        let data = response.data.ok_or_else(|| Error::ExternalService {
            service: "VNPay".to_string(),
            message: "Missing data in response".to_string(),
        })?;

        let detected_doc_type = data
            .ocr_result
            .document_type
            .as_ref()
            .map(|t| Self::vnpay_to_doc_type(t))
            .unwrap_or(request.document_type);

        info!(
            verification_id = %data.verification_id,
            confidence = %data.confidence_score,
            "VNPay eKYC: ID verification completed"
        );

        Ok(IdVerification {
            verification_id: data.verification_id,
            success: data.confidence_score >= 0.7,
            full_name: data.ocr_result.full_name,
            id_number: data.ocr_result.id_number,
            document_type: detected_doc_type,
            date_of_birth: data.ocr_result.date_of_birth,
            gender: data.ocr_result.gender,
            nationality: data.ocr_result.nationality,
            place_of_origin: data.ocr_result.place_of_origin,
            place_of_residence: data.ocr_result.place_of_residence,
            expiry_date: data.ocr_result.expiry_date,
            issue_date: data.ocr_result.issue_date,
            issuing_authority: data.ocr_result.issuing_authority,
            confidence_score: data.confidence_score,
            field_confidences: data.ocr_result.field_confidences.unwrap_or_default(),
            error_message: None,
            raw_response: None,
            verified_at: Utc::now(),
        })
    }

    async fn match_face(&self, request: FaceMatchRequest) -> Result<FaceMatch> {
        debug!(
            request_id = %request.request_id,
            "VNPay eKYC: Matching faces"
        );

        let vnpay_request = VnpayFaceMatchRequest {
            request_id: request.request_id.clone(),
            selfie_image: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &request.selfie_image,
            ),
            id_photo_image: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &request.id_photo_image,
            ),
        };

        let response: VnpayFaceMatchResponse = self
            .make_request("/face/match", &vnpay_request)
            .await?;

        if response.code != "00" && response.code != "0" && response.code != "200" {
            return Ok(FaceMatch {
                match_id: format!("vnpay_err_{}", uuid::Uuid::now_v7()),
                is_match: false,
                similarity_score: 0.0,
                confidence: FaceMatchConfidence::VeryLow,
                error_message: Some(response.message),
                raw_response: None,
                matched_at: Utc::now(),
            });
        }

        let data = response.data.ok_or_else(|| Error::ExternalService {
            service: "VNPay".to_string(),
            message: "Missing data in response".to_string(),
        })?;

        info!(
            match_id = %data.match_id,
            similarity = %data.similarity_score,
            is_match = %data.is_match,
            "VNPay eKYC: Face matching completed"
        );

        Ok(FaceMatch {
            match_id: data.match_id,
            is_match: data.is_match,
            similarity_score: data.similarity_score,
            confidence: FaceMatchConfidence::from_score(data.similarity_score),
            error_message: None,
            raw_response: None,
            matched_at: Utc::now(),
        })
    }

    async fn check_liveness(&self, request: LivenessRequest) -> Result<LivenessResult> {
        debug!(
            request_id = %request.request_id,
            check_type = ?request.check_type,
            "VNPay eKYC: Checking liveness"
        );

        let check_type_str = match request.check_type {
            LivenessCheckType::Passive => "PASSIVE",
            LivenessCheckType::Active => "ACTIVE",
            LivenessCheckType::Video => "VIDEO",
        };

        let vnpay_request = VnpayLivenessRequest {
            request_id: request.request_id.clone(),
            video_data: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &request.video_data,
            ),
            check_type: check_type_str.to_string(),
        };

        let response: VnpayLivenessResponse = self
            .make_request("/liveness/check", &vnpay_request)
            .await?;

        if response.code != "00" && response.code != "0" && response.code != "200" {
            return Ok(LivenessResult {
                liveness_id: format!("vnpay_err_{}", uuid::Uuid::now_v7()),
                is_live: false,
                liveness_score: 0.0,
                spoofing_types: vec![],
                error_message: Some(response.message),
                raw_response: None,
                checked_at: Utc::now(),
            });
        }

        let data = response.data.ok_or_else(|| Error::ExternalService {
            service: "VNPay".to_string(),
            message: "Missing data in response".to_string(),
        })?;

        let spoofing_types: Vec<SpoofingType> = data
            .spoofing_types
            .unwrap_or_default()
            .iter()
            .map(|s| match s.to_uppercase().as_str() {
                "PRINTED_PHOTO" => SpoofingType::PrintedPhoto,
                "SCREEN_REPLAY" => SpoofingType::ScreenReplay,
                "MASK_3D" => SpoofingType::Mask3d,
                "VIDEO_REPLAY" => SpoofingType::VideoReplay,
                "DEEPFAKE" => SpoofingType::Deepfake,
                _ => SpoofingType::Unknown,
            })
            .collect();

        info!(
            liveness_id = %data.liveness_id,
            is_live = %data.is_live,
            score = %data.liveness_score,
            "VNPay eKYC: Liveness check completed"
        );

        Ok(LivenessResult {
            liveness_id: data.liveness_id,
            is_live: data.is_live,
            liveness_score: data.liveness_score,
            spoofing_types,
            error_message: None,
            raw_response: None,
            checked_at: Utc::now(),
        })
    }

    async fn health_check(&self) -> Result<bool> {
        #[derive(Serialize)]
        struct HealthRequest {
            check: String,
        }

        #[derive(Deserialize)]
        struct HealthResponse {
            code: String,
        }

        let request = HealthRequest {
            check: "ping".to_string(),
        };

        match self.make_request::<_, HealthResponse>("/health", &request).await {
            Ok(resp) => Ok(resp.code == "00" || resp.code == "0" || resp.code == "200"),
            Err(e) => {
                warn!(error = %e, "VNPay health check failed");
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vnpay_config_default() {
        let config = VnpayEkycConfig::default();
        assert_eq!(config.base.provider_code, "vnpay_ekyc");
        assert!(config.base.sandbox_mode);
    }

    #[test]
    fn test_doc_type_conversion() {
        assert_eq!(VnpayEkycProvider::doc_type_to_vnpay(IdDocumentType::Cccd), "CCCD");
        assert_eq!(VnpayEkycProvider::doc_type_to_vnpay(IdDocumentType::Passport), "PASSPORT");

        assert!(matches!(
            VnpayEkycProvider::vnpay_to_doc_type("CCCD"),
            IdDocumentType::Cccd
        ));
        assert!(matches!(
            VnpayEkycProvider::vnpay_to_doc_type("GPLX"),
            IdDocumentType::DriverLicense
        ));
    }
}
