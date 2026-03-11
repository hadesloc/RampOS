//! Mock eKYC Provider for testing
//!
//! This adapter provides a fully functional mock implementation for testing
//! eKYC workflows without real provider integration.

use async_trait::async_trait;
use chrono::Utc;
use ramp_common::Result;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::{
    AddressVerification, AddressVerificationRequest, EkycProvider, EkycProviderConfig, FaceMatch,
    FaceMatchConfidence, FaceMatchRequest, IdDocumentType, IdVerification, IdVerificationRequest,
    LivenessCheckType, LivenessRequest, LivenessResult, SpoofingType,
};

/// Mock eKYC behavior configuration
#[derive(Debug, Clone)]
pub struct MockEkycBehavior {
    /// Delay before returning responses (simulates network latency)
    pub response_delay_ms: u64,
    /// Whether to simulate failures
    pub simulate_failures: bool,
    /// Failure rate (0.0 to 1.0) when simulate_failures is true
    pub failure_rate: f64,
    /// Default ID verification success rate
    pub id_verification_success_rate: f64,
    /// Default face match similarity score
    pub default_face_similarity: f64,
    /// Default liveness score
    pub default_liveness_score: f64,
    /// ID numbers that should always fail
    pub blocked_ids: Vec<String>,
    /// ID numbers that should always succeed
    pub approved_ids: Vec<String>,
}

impl Default for MockEkycBehavior {
    fn default() -> Self {
        Self {
            response_delay_ms: 100,
            simulate_failures: false,
            failure_rate: 0.0,
            id_verification_success_rate: 0.95,
            default_face_similarity: 0.92,
            default_liveness_score: 0.95,
            blocked_ids: vec!["000000000000".to_string()],
            approved_ids: vec!["123456789012".to_string()],
        }
    }
}

/// Mock eKYC Provider for testing
pub struct MockEkycProvider {
    config: EkycProviderConfig,
    behavior: MockEkycBehavior,
    /// Store verification results for later retrieval
    id_verifications: Arc<RwLock<HashMap<String, IdVerification>>>,
    face_matches: Arc<RwLock<HashMap<String, FaceMatch>>>,
    liveness_results: Arc<RwLock<HashMap<String, LivenessResult>>>,
}

impl MockEkycProvider {
    pub fn new(config: EkycProviderConfig) -> Self {
        Self {
            config,
            behavior: MockEkycBehavior::default(),
            id_verifications: Arc::new(RwLock::new(HashMap::new())),
            face_matches: Arc::new(RwLock::new(HashMap::new())),
            liveness_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_behavior(config: EkycProviderConfig, behavior: MockEkycBehavior) -> Self {
        Self {
            config,
            behavior,
            id_verifications: Arc::new(RwLock::new(HashMap::new())),
            face_matches: Arc::new(RwLock::new(HashMap::new())),
            liveness_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a mock provider with default configuration
    pub fn with_defaults() -> Self {
        Self::new(EkycProviderConfig {
            provider_code: "mock_ekyc".to_string(),
            sandbox_mode: true,
            ..Default::default()
        })
    }

    /// Apply simulated delay if configured
    async fn maybe_delay(&self) {
        if self.behavior.response_delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(
                self.behavior.response_delay_ms,
            ))
            .await;
        }
    }

    /// Check if this request should fail
    fn should_fail(&self) -> bool {
        if !self.behavior.simulate_failures {
            return false;
        }
        rand::thread_rng().gen::<f64>() < self.behavior.failure_rate
    }

    /// Generate mock OCR data for ID
    fn generate_mock_ocr_data(&self, doc_type: IdDocumentType) -> HashMap<String, String> {
        let mut data = HashMap::new();

        match doc_type {
            IdDocumentType::Cccd | IdDocumentType::Cmnd => {
                data.insert("full_name".to_string(), "NGUYEN VAN A".to_string());
                data.insert("id_number".to_string(), "001234567890".to_string());
                data.insert("date_of_birth".to_string(), "01/01/1990".to_string());
                data.insert("gender".to_string(), "Nam".to_string());
                data.insert("nationality".to_string(), "Việt Nam".to_string());
                data.insert("place_of_origin".to_string(), "Hà Nội".to_string());
                data.insert(
                    "place_of_residence".to_string(),
                    "123 Đường ABC, Quận Hoàn Kiếm, Hà Nội".to_string(),
                );
                data.insert("expiry_date".to_string(), "01/01/2035".to_string());
                data.insert("issue_date".to_string(), "01/01/2020".to_string());
                data.insert(
                    "issuing_authority".to_string(),
                    "Cục Cảnh sát ĐKQL cư trú và DLQG về dân cư".to_string(),
                );
            }
            IdDocumentType::Passport => {
                data.insert("full_name".to_string(), "NGUYEN VAN A".to_string());
                data.insert("id_number".to_string(), "B12345678".to_string());
                data.insert("date_of_birth".to_string(), "01/01/1990".to_string());
                data.insert("gender".to_string(), "M".to_string());
                data.insert("nationality".to_string(), "VNM".to_string());
                data.insert("expiry_date".to_string(), "01/01/2030".to_string());
                data.insert("issue_date".to_string(), "01/01/2020".to_string());
                data.insert(
                    "issuing_authority".to_string(),
                    "Immigration Department".to_string(),
                );
            }
            IdDocumentType::DriverLicense => {
                data.insert("full_name".to_string(), "NGUYEN VAN A".to_string());
                data.insert("id_number".to_string(), "123456789012".to_string());
                data.insert("date_of_birth".to_string(), "01/01/1990".to_string());
                data.insert("license_class".to_string(), "B2".to_string());
                data.insert("expiry_date".to_string(), "01/01/2030".to_string());
                data.insert("issue_date".to_string(), "01/01/2020".to_string());
            }
        }

        data
    }

    /// Get stored ID verification result
    pub async fn get_id_verification(&self, verification_id: &str) -> Option<IdVerification> {
        self.id_verifications
            .read()
            .await
            .get(verification_id)
            .cloned()
    }

    /// Get stored face match result
    pub async fn get_face_match(&self, match_id: &str) -> Option<FaceMatch> {
        self.face_matches.read().await.get(match_id).cloned()
    }

    /// Get stored liveness result
    pub async fn get_liveness_result(&self, liveness_id: &str) -> Option<LivenessResult> {
        self.liveness_results.read().await.get(liveness_id).cloned()
    }

    /// Set custom ID verification result (for testing)
    pub async fn set_id_verification(&self, verification_id: String, result: IdVerification) {
        self.id_verifications
            .write()
            .await
            .insert(verification_id, result);
    }

    /// Set custom face match result (for testing)
    pub async fn set_face_match(&self, match_id: String, result: FaceMatch) {
        self.face_matches.write().await.insert(match_id, result);
    }

    /// Set custom liveness result (for testing)
    pub async fn set_liveness_result(&self, liveness_id: String, result: LivenessResult) {
        self.liveness_results
            .write()
            .await
            .insert(liveness_id, result);
    }
}

#[async_trait]
impl EkycProvider for MockEkycProvider {
    fn provider_code(&self) -> &str {
        &self.config.provider_code
    }

    fn provider_name(&self) -> &str {
        "Mock eKYC Provider"
    }

    fn is_sandbox_mode(&self) -> bool {
        true
    }

    async fn verify_id(&self, request: IdVerificationRequest) -> Result<IdVerification> {
        self.maybe_delay().await;

        if self.should_fail() {
            return Err(ramp_common::Error::ExternalService {
                service: "MockEkyc".to_string(),
                message: "Simulated mock failure".to_string(),
            });
        }

        debug!(
            request_id = %request.request_id,
            doc_type = ?request.document_type,
            "Mock eKYC: Verifying ID"
        );

        let verification_id = format!("mock_id_{}", uuid::Uuid::now_v7());
        let ocr_data = self.generate_mock_ocr_data(request.document_type);

        // Check if ID should be blocked
        let id_number = ocr_data.get("id_number").cloned().unwrap_or_default();
        let (success, error_message) = if self.behavior.blocked_ids.contains(&id_number) {
            (false, Some("ID number is on blocked list".to_string()))
        } else if self.behavior.approved_ids.contains(&id_number) {
            (true, None)
        } else {
            let success =
                rand::thread_rng().gen::<f64>() < self.behavior.id_verification_success_rate;
            if success {
                (true, None)
            } else {
                (false, Some("Document verification failed".to_string()))
            }
        };

        let confidence_score = if success { 0.95 } else { 0.3 };
        let mut field_confidences = HashMap::new();
        for key in ocr_data.keys() {
            field_confidences.insert(
                key.clone(),
                if success {
                    0.90 + rand::thread_rng().gen::<f64>() * 0.1
                } else {
                    0.5
                },
            );
        }

        let result = IdVerification {
            verification_id: verification_id.clone(),
            success,
            full_name: ocr_data.get("full_name").cloned(),
            id_number: ocr_data.get("id_number").cloned(),
            document_type: request.document_type,
            date_of_birth: ocr_data.get("date_of_birth").cloned(),
            gender: ocr_data.get("gender").cloned(),
            nationality: ocr_data.get("nationality").cloned(),
            place_of_origin: ocr_data.get("place_of_origin").cloned(),
            place_of_residence: ocr_data.get("place_of_residence").cloned(),
            expiry_date: ocr_data.get("expiry_date").cloned(),
            issue_date: ocr_data.get("issue_date").cloned(),
            issuing_authority: ocr_data.get("issuing_authority").cloned(),
            confidence_score,
            field_confidences,
            error_message,
            raw_response: Some(serde_json::json!({
                "mock": true,
                "ocr_data": ocr_data,
            })),
            verified_at: Utc::now(),
        };

        // Store for later retrieval
        self.id_verifications
            .write()
            .await
            .insert(verification_id.clone(), result.clone());

        info!(
            verification_id = %verification_id,
            success = %success,
            "Mock eKYC: ID verification completed"
        );

        Ok(result)
    }

    async fn match_face(&self, request: FaceMatchRequest) -> Result<FaceMatch> {
        self.maybe_delay().await;

        if self.should_fail() {
            return Err(ramp_common::Error::ExternalService {
                service: "MockEkyc".to_string(),
                message: "Simulated mock failure".to_string(),
            });
        }

        debug!(
            request_id = %request.request_id,
            "Mock eKYC: Matching faces"
        );

        let match_id = format!("mock_face_{}", uuid::Uuid::now_v7());

        // Generate realistic similarity score with some variance
        let base_similarity = self.behavior.default_face_similarity;
        let variance = rand::thread_rng().gen_range(-0.05..0.05);
        let similarity_score = (base_similarity + variance).clamp(0.0, 1.0);

        let is_match = similarity_score >= 0.75;
        let confidence = FaceMatchConfidence::from_score(similarity_score);

        let result = FaceMatch {
            match_id: match_id.clone(),
            is_match,
            similarity_score,
            confidence,
            error_message: if is_match {
                None
            } else {
                Some("Face similarity below threshold".to_string())
            },
            raw_response: Some(serde_json::json!({
                "mock": true,
                "similarity": similarity_score,
            })),
            matched_at: Utc::now(),
        };

        // Store for later retrieval
        self.face_matches
            .write()
            .await
            .insert(match_id.clone(), result.clone());

        info!(
            match_id = %match_id,
            similarity = %similarity_score,
            is_match = %is_match,
            "Mock eKYC: Face matching completed"
        );

        Ok(result)
    }

    async fn check_liveness(&self, request: LivenessRequest) -> Result<LivenessResult> {
        self.maybe_delay().await;

        if self.should_fail() {
            return Err(ramp_common::Error::ExternalService {
                service: "MockEkyc".to_string(),
                message: "Simulated mock failure".to_string(),
            });
        }

        debug!(
            request_id = %request.request_id,
            check_type = ?request.check_type,
            "Mock eKYC: Checking liveness"
        );

        let liveness_id = format!("mock_live_{}", uuid::Uuid::now_v7());

        // Generate liveness score based on check type
        let base_score = self.behavior.default_liveness_score;
        let type_bonus = match request.check_type {
            LivenessCheckType::Passive => 0.0,
            LivenessCheckType::Active => 0.02,
            LivenessCheckType::Video => 0.03,
        };
        let variance = rand::thread_rng().gen_range(-0.05..0.05);
        let liveness_score = (base_score + type_bonus + variance).clamp(0.0, 1.0);

        let is_live = liveness_score >= 0.80;
        let spoofing_types = if is_live {
            vec![]
        } else {
            // Randomly select a spoofing type for failed checks
            let types = [
                SpoofingType::PrintedPhoto,
                SpoofingType::ScreenReplay,
                SpoofingType::VideoReplay,
            ];
            vec![types[rand::thread_rng().gen_range(0..types.len())]]
        };

        let result = LivenessResult {
            liveness_id: liveness_id.clone(),
            is_live,
            liveness_score,
            spoofing_types: spoofing_types.clone(),
            error_message: if is_live {
                None
            } else {
                Some(format!("Liveness check failed: {:?}", spoofing_types))
            },
            raw_response: Some(serde_json::json!({
                "mock": true,
                "liveness_score": liveness_score,
                "check_type": format!("{:?}", request.check_type),
            })),
            checked_at: Utc::now(),
        };

        // Store for later retrieval
        self.liveness_results
            .write()
            .await
            .insert(liveness_id.clone(), result.clone());

        info!(
            liveness_id = %liveness_id,
            is_live = %is_live,
            score = %liveness_score,
            "Mock eKYC: Liveness check completed"
        );

        Ok(result)
    }

    async fn verify_address(
        &self,
        request: AddressVerificationRequest,
    ) -> Result<super::AddressVerification> {
        self.maybe_delay().await;

        if self.should_fail() {
            return Err(ramp_common::Error::ExternalService {
                service: "MockEkyc".to_string(),
                message: "Simulated mock failure".to_string(),
            });
        }

        debug!(
            request_id = %request.request_id,
            address = %request.address,
            "Mock eKYC: Verifying address"
        );

        let verification_id = format!("mock_addr_{}", uuid::Uuid::now_v7());

        // Mock address normalization
        let is_valid = !request.address.is_empty() && request.address.len() > 10;
        let confidence_score = if is_valid { 0.90 } else { 0.3 };

        let result = AddressVerification {
            verification_id: verification_id.clone(),
            is_valid,
            normalized_address: if is_valid {
                Some(request.address.to_uppercase())
            } else {
                None
            },
            province_code: request.province.as_ref().map(|_| "01".to_string()),
            district_code: request.district.as_ref().map(|_| "001".to_string()),
            ward_code: request.ward.as_ref().map(|_| "00001".to_string()),
            confidence_score,
            error_message: if is_valid {
                None
            } else {
                Some("Invalid address format".to_string())
            },
            raw_response: Some(serde_json::json!({
                "mock": true,
                "original_address": request.address,
            })),
            verified_at: Utc::now(),
        };

        info!(
            verification_id = %verification_id,
            is_valid = %is_valid,
            "Mock eKYC: Address verification completed"
        );

        Ok(result)
    }

    async fn health_check(&self) -> Result<bool> {
        self.maybe_delay().await;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_id_verification() {
        let provider = MockEkycProvider::with_defaults();

        let request = IdVerificationRequest {
            id_front_image: vec![0u8; 100],
            id_back_image: Some(vec![0u8; 100]),
            document_type: IdDocumentType::Cccd,
            request_id: "test_001".to_string(),
        };

        let result = provider.verify_id(request).await.unwrap();
        assert!(!result.verification_id.is_empty());
        assert!(result.full_name.is_some());
        assert!(result.id_number.is_some());
    }

    #[tokio::test]
    async fn test_mock_face_matching() {
        let provider = MockEkycProvider::with_defaults();

        let request = FaceMatchRequest {
            selfie_image: vec![0u8; 100],
            id_photo_image: vec![0u8; 100],
            request_id: "test_002".to_string(),
        };

        let result = provider.match_face(request).await.unwrap();
        assert!(!result.match_id.is_empty());
        assert!(result.similarity_score >= 0.0 && result.similarity_score <= 1.0);
    }

    #[tokio::test]
    async fn test_mock_liveness() {
        let provider = MockEkycProvider::with_defaults();

        let request = LivenessRequest {
            video_data: vec![0u8; 100],
            request_id: "test_003".to_string(),
            check_type: LivenessCheckType::Active,
        };

        let result = provider.check_liveness(request).await.unwrap();
        assert!(!result.liveness_id.is_empty());
        assert!(result.liveness_score >= 0.0 && result.liveness_score <= 1.0);
    }

    #[tokio::test]
    async fn test_mock_address_verification() {
        let provider = MockEkycProvider::with_defaults();

        let request = AddressVerificationRequest {
            address: "123 Đường ABC, Quận Hoàn Kiếm, Thành phố Hà Nội".to_string(),
            province: Some("Hà Nội".to_string()),
            district: Some("Hoàn Kiếm".to_string()),
            ward: Some("Phúc Tân".to_string()),
            request_id: "test_004".to_string(),
        };

        let result = provider.verify_address(request).await.unwrap();
        assert!(result.is_valid);
        assert!(result.normalized_address.is_some());
    }

    #[tokio::test]
    async fn test_mock_blocked_id() {
        let behavior = MockEkycBehavior {
            blocked_ids: vec!["001234567890".to_string()],
            ..Default::default()
        };
        let provider = MockEkycProvider::with_behavior(EkycProviderConfig::default(), behavior);

        let request = IdVerificationRequest {
            id_front_image: vec![0u8; 100],
            id_back_image: Some(vec![0u8; 100]),
            document_type: IdDocumentType::Cccd,
            request_id: "test_blocked".to_string(),
        };

        let result = provider.verify_id(request).await.unwrap();
        // The mock generates "001234567890" for CCCD, which should be blocked
        assert!(!result.success);
        assert!(result.error_message.is_some());
    }
}
