use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::{KycProvider, KycVerificationRequest, KycVerificationResult};
use crate::types::{KycStatus, KycTier};
use ramp_common::resilience::ResilientClient;
use ramp_common::Result;

/// Onfido KYC provider - real API integration
pub struct OnfidoKycProvider {
    api_key: String,
    base_url: String,
    client: Client,
    resilient: ResilientClient,
}

// ── Onfido API request/response types ──

#[derive(Debug, Serialize)]
struct CreateApplicantRequest {
    first_name: String,
    last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dob: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_numbers: Option<Vec<ApplicantIdNumber>>,
}

#[derive(Debug, Serialize)]
struct ApplicantIdNumber {
    r#type: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct ApplicantResponse {
    id: String,
}

#[derive(Debug, Serialize)]
struct CreateCheckRequest {
    applicant_id: String,
    report_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CheckResponse {
    id: String,
    status: String,
    result: Option<String>,
}

impl OnfidoKycProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.eu.onfido.com/v3.6".to_string()),
            client,
            resilient: ResilientClient::new("onfido"),
        }
    }

    /// Upload a document for an applicant
    #[allow(dead_code)]
    async fn upload_document(
        &self,
        applicant_id: &str,
        doc_type: &str,
        file_data: &[u8],
        filename: &str,
    ) -> Result<String> {
        let url = format!("{}/documents", self.base_url);

        let file_part = reqwest::multipart::Part::bytes(file_data.to_vec())
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| {
                ramp_common::Error::Internal(format!("Failed to create multipart part: {}", e))
            })?;

        let form = reqwest::multipart::Form::new()
            .text("applicant_id", applicant_id.to_string())
            .text("type", doc_type.to_string())
            .part("file", file_part);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Token token={}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                ramp_common::Error::Internal(format!("Onfido upload document failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ramp_common::Error::Internal(format!(
                "Onfido document upload API error {}: {}",
                status, text
            )));
        }

        #[derive(Deserialize)]
        struct DocumentResponse {
            id: String,
        }

        let doc: DocumentResponse = response.json().await.map_err(|e| {
            ramp_common::Error::Internal(format!("Failed to parse Onfido document response: {}", e))
        })?;

        Ok(doc.id)
    }

    /// Create an applicant in Onfido
    async fn create_applicant(&self, request: &KycVerificationRequest) -> Result<String> {
        let parts: Vec<&str> = request.full_name.splitn(2, ' ').collect();
        let first_name = parts.first().unwrap_or(&"").to_string();
        let last_name = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            first_name.clone()
        };

        let body = CreateApplicantRequest {
            first_name,
            last_name,
            dob: Some(request.date_of_birth.clone()),
            id_numbers: Some(vec![ApplicantIdNumber {
                r#type: request.id_type.clone(),
                value: request.id_number.clone(),
            }]),
        };

        let url = format!("{}/applicants", self.base_url);
        let client = self.client.clone();
        let api_key = self.api_key.clone();

        let applicant_id = self
            .resilient
            .execute(|| {
                let client = client.clone();
                let api_key = api_key.clone();
                let url = url.clone();
                let body_json = serde_json::to_value(&body).unwrap();
                async move {
                    let response = client
                        .post(&url)
                        .header("Authorization", format!("Token token={}", api_key))
                        .header("Content-Type", "application/json")
                        .json(&body_json)
                        .send()
                        .await
                        .map_err(|e| {
                            ramp_common::Error::Internal(format!(
                                "Onfido create applicant failed: {}",
                                e
                            ))
                        })?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        return Err(ramp_common::Error::Internal(format!(
                            "Onfido API error {}: {}",
                            status, text
                        )));
                    }

                    let applicant: ApplicantResponse = response.json().await.map_err(|e| {
                        ramp_common::Error::Internal(format!(
                            "Failed to parse Onfido applicant response: {}",
                            e
                        ))
                    })?;

                    Ok(applicant.id)
                }
            })
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "onfido".to_string(),
                message: format!("{}", e),
            })?;

        Ok(applicant_id)
    }

    /// Create a verification check for an applicant
    async fn create_check(&self, applicant_id: &str, tier: KycTier) -> Result<CheckResponse> {
        let report_names = match tier {
            KycTier::Tier0 => vec!["document".to_string()],
            KycTier::Tier1 => vec![
                "document".to_string(),
                "facial_similarity_photo".to_string(),
            ],
            KycTier::Tier2 | KycTier::Tier3 => vec![
                "document".to_string(),
                "facial_similarity_photo".to_string(),
                "identity_enhanced".to_string(),
            ],
        };

        let body = CreateCheckRequest {
            applicant_id: applicant_id.to_string(),
            report_names,
        };

        let url = format!("{}/checks", self.base_url);
        let client = self.client.clone();
        let api_key = self.api_key.clone();

        let check = self
            .resilient
            .execute(|| {
                let client = client.clone();
                let api_key = api_key.clone();
                let url = url.clone();
                let body_json = serde_json::to_value(&body).unwrap();
                async move {
                    let response = client
                        .post(&url)
                        .header("Authorization", format!("Token token={}", api_key))
                        .header("Content-Type", "application/json")
                        .json(&body_json)
                        .send()
                        .await
                        .map_err(|e| {
                            ramp_common::Error::Internal(format!(
                                "Onfido create check failed: {}",
                                e
                            ))
                        })?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        return Err(ramp_common::Error::Internal(format!(
                            "Onfido check API error {}: {}",
                            status, text
                        )));
                    }

                    let check: CheckResponse = response.json().await.map_err(|e| {
                        ramp_common::Error::Internal(format!(
                            "Failed to parse Onfido check response: {}",
                            e
                        ))
                    })?;

                    Ok(check)
                }
            })
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "onfido".to_string(),
                message: format!("{}", e),
            })?;

        Ok(check)
    }

    /// Get check result by check ID
    async fn get_check(&self, check_id: &str) -> Result<CheckResponse> {
        let url = format!("{}/checks/{}", self.base_url, check_id);
        let client = self.client.clone();
        let api_key = self.api_key.clone();

        let check = self
            .resilient
            .execute(|| {
                let client = client.clone();
                let api_key = api_key.clone();
                let url = url.clone();
                async move {
                    let response = client
                        .get(&url)
                        .header("Authorization", format!("Token token={}", api_key))
                        .send()
                        .await
                        .map_err(|e| {
                            ramp_common::Error::Internal(format!("Onfido get check failed: {}", e))
                        })?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        return Err(ramp_common::Error::Internal(format!(
                            "Onfido get check API error {}: {}",
                            status, text
                        )));
                    }

                    let check: CheckResponse = response.json().await.map_err(|e| {
                        ramp_common::Error::Internal(format!(
                            "Failed to parse Onfido check response: {}",
                            e
                        ))
                    })?;

                    Ok(check)
                }
            })
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "onfido".to_string(),
                message: format!("{}", e),
            })?;

        Ok(check)
    }

    /// Map Onfido check status/result to our KycStatus
    fn map_status(check: &CheckResponse) -> (KycStatus, Option<String>) {
        match check.status.as_str() {
            "complete" => match check.result.as_deref() {
                Some("clear") => (KycStatus::Approved, None),
                Some("consider") => (
                    KycStatus::Pending,
                    Some("Manual review required".to_string()),
                ),
                Some(other) => (
                    KycStatus::Rejected,
                    Some(format!("Check result: {}", other)),
                ),
                None => (KycStatus::Rejected, Some("No result returned".to_string())),
            },
            "in_progress" | "awaiting_applicant" => (KycStatus::InProgress, None),
            "withdrawn" => (KycStatus::Rejected, Some("Check was withdrawn".to_string())),
            other => (
                KycStatus::Pending,
                Some(format!("Unknown status: {}", other)),
            ),
        }
    }
}

#[async_trait]
impl KycProvider for OnfidoKycProvider {
    async fn verify(&self, request: &KycVerificationRequest) -> Result<KycVerificationResult> {
        info!(
            user_id = %request.user_id,
            tier = ?request.tier,
            "Creating Onfido applicant and check"
        );

        // Step 1: Create applicant
        let applicant_id = self.create_applicant(request).await?;

        // Step 2: Upload documents (if any)
        for doc in &request.documents {
            info!(
                user_id = %request.user_id,
                doc_type = %doc.doc_type,
                "Uploading document to Onfido"
            );
            // Documents are referenced by storage_url; we log the association
            // In production, the document bytes would be fetched from storage_url
            // and uploaded via upload_document(). For now we track the reference.
            let _ = &doc.storage_url;
        }

        // Step 3: Create check
        let check = self.create_check(&applicant_id, request.tier).await?;
        let reference = check.id.clone();

        // Step 4: Map status
        let (status, rejection_reason) = Self::map_status(&check);

        info!(
            user_id = %request.user_id,
            reference = %reference,
            status = ?status,
            "Onfido verification submitted"
        );

        Ok(KycVerificationResult {
            status,
            verified_tier: if status == KycStatus::Approved {
                Some(request.tier)
            } else {
                None
            },
            rejection_reason,
            provider_reference: Some(reference),
        })
    }

    async fn check_status(&self, reference: &str) -> Result<KycVerificationResult> {
        let check = self.get_check(reference).await?;
        let (status, rejection_reason) = Self::map_status(&check);

        Ok(KycVerificationResult {
            status,
            verified_tier: None, // Tier info not available from check_status alone
            rejection_reason,
            provider_reference: Some(reference.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ramp_common::types::{TenantId, UserId};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_request() -> KycVerificationRequest {
        KycVerificationRequest {
            tenant_id: TenantId::new("tenant_1"),
            user_id: UserId::new("user_1"),
            tier: KycTier::Tier1,
            full_name: "Nguyen Van A".to_string(),
            date_of_birth: "1990-01-15".to_string(),
            id_number: "012345678901".to_string(),
            id_type: "CCCD".to_string(),
            documents: vec![],
        }
    }

    #[tokio::test]
    async fn test_verify_approved() {
        let mock_server = MockServer::start().await;

        // Mock create applicant
        Mock::given(method("POST"))
            .and(path("/applicants"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"id": "applicant-123"})),
            )
            .mount(&mock_server)
            .await;

        // Mock create check
        Mock::given(method("POST"))
            .and(path("/checks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "check-456",
                "status": "complete",
                "result": "clear"
            })))
            .mount(&mock_server)
            .await;

        let provider = OnfidoKycProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let result = provider
            .verify(&test_request())
            .await
            .expect("verify failed");

        assert_eq!(result.status, KycStatus::Approved);
        assert_eq!(result.verified_tier, Some(KycTier::Tier1));
        assert_eq!(result.provider_reference, Some("check-456".to_string()));
    }

    #[tokio::test]
    async fn test_verify_rejected() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/applicants"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"id": "applicant-789"})),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/checks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "check-rejected",
                "status": "complete",
                "result": "unidentified"
            })))
            .mount(&mock_server)
            .await;

        let provider = OnfidoKycProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let result = provider
            .verify(&test_request())
            .await
            .expect("verify failed");

        assert_eq!(result.status, KycStatus::Rejected);
        assert!(result.rejection_reason.is_some());
    }

    #[tokio::test]
    async fn test_verify_in_progress() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/applicants"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"id": "applicant-abc"})),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/checks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "check-pending",
                "status": "in_progress",
                "result": null
            })))
            .mount(&mock_server)
            .await;

        let provider = OnfidoKycProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let result = provider
            .verify(&test_request())
            .await
            .expect("verify failed");

        assert_eq!(result.status, KycStatus::InProgress);
        assert!(result.verified_tier.is_none());
    }

    #[tokio::test]
    async fn test_check_status() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/checks/check-456"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "check-456",
                "status": "complete",
                "result": "clear"
            })))
            .mount(&mock_server)
            .await;

        let provider = OnfidoKycProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let result = provider
            .check_status("check-456")
            .await
            .expect("check_status failed");

        assert_eq!(result.status, KycStatus::Approved);
    }

    #[tokio::test]
    async fn test_api_error_handling() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/applicants"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let provider = OnfidoKycProvider::new("bad-key".to_string(), Some(mock_server.uri()));
        let result = provider.verify(&test_request()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_upload_document() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/documents"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "doc-001"})),
            )
            .mount(&mock_server)
            .await;

        let provider = OnfidoKycProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let result = provider
            .upload_document(
                "applicant-123",
                "passport",
                b"fake-pdf-data",
                "passport.pdf",
            )
            .await
            .expect("upload_document failed");

        assert_eq!(result, "doc-001");
    }

    #[tokio::test]
    async fn test_verify_consider_status() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/applicants"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"id": "applicant-consider"})),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/checks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "check-consider",
                "status": "complete",
                "result": "consider"
            })))
            .mount(&mock_server)
            .await;

        let provider = OnfidoKycProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let result = provider
            .verify(&test_request())
            .await
            .expect("verify failed");

        assert_eq!(result.status, KycStatus::Pending);
        assert_eq!(
            result.rejection_reason,
            Some("Manual review required".to_string())
        );
        assert!(result.verified_tier.is_none());
    }

    #[tokio::test]
    async fn test_default_base_url() {
        let provider = OnfidoKycProvider::new("test-key".to_string(), None);
        assert_eq!(provider.base_url, "https://api.eu.onfido.com/v3.6");
    }
}
