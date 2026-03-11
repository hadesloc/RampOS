//! License deadline checker job
//! Checks for licenses expiring within 7 or 30 days and publishes webhook events

use crate::event::{EventPublisher, RegulatoryEvent};
use crate::repository::license::LicenseRepository;
use chrono::{Duration, Utc};
use ramp_common::{types::TenantId, Result};
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct LicenseDeadlineChecker<L, E> {
    license_repo: Arc<L>,
    event_publisher: Arc<E>,
}

impl<L: LicenseRepository, E: EventPublisher> LicenseDeadlineChecker<L, E> {
    pub fn new(license_repo: Arc<L>, event_publisher: Arc<E>) -> Self {
        Self {
            license_repo,
            event_publisher,
        }
    }

    pub async fn run(&self) {
        info!("Starting license deadline checker job");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400)); // Daily

        loop {
            interval.tick().await;
            if let Err(e) = self.check_expiring_licenses().await {
                error!(error = %e, "Failed to check expiring licenses");
            }
        }
    }

    pub async fn check_expiring_licenses(&self) -> Result<usize> {
        let now = Utc::now();
        let mut notifications_sent = 0;

        // Check licenses expiring in 7 days
        let seven_days = now + Duration::days(7);
        let expiring_7 = self.license_repo.find_expiring_before(seven_days).await?;

        for license in expiring_7 {
            let days_remaining = (license.expires_at - now).num_days() as i32;
            if days_remaining <= 7 && days_remaining > 0 {
                let event = RegulatoryEvent::LicenseExpiring {
                    license_id: license.id.clone(),
                    license_type: license.license_type.clone(),
                    expires_at: license.expires_at,
                    days_remaining,
                };

                if let Err(e) = self
                    .event_publisher
                    .publish_regulatory_event(&TenantId(license.tenant_id.clone()), event)
                    .await
                {
                    warn!(
                        license_id = %license.id,
                        error = %e,
                        "Failed to publish license expiring event"
                    );
                } else {
                    notifications_sent += 1;
                }
            }
        }

        // Check licenses expiring in 30 days
        let thirty_days = now + Duration::days(30);
        let expiring_30 = self.license_repo.find_expiring_before(thirty_days).await?;

        for license in expiring_30 {
            let days_remaining = (license.expires_at - now).num_days() as i32;
            // Only send 30-day notice if between 28-30 days (avoid duplicate with 7-day)
            if days_remaining >= 28 && days_remaining <= 30 {
                let event = RegulatoryEvent::LicenseExpiring {
                    license_id: license.id.clone(),
                    license_type: license.license_type.clone(),
                    expires_at: license.expires_at,
                    days_remaining,
                };

                if let Err(e) = self
                    .event_publisher
                    .publish_regulatory_event(&TenantId(license.tenant_id.clone()), event)
                    .await
                {
                    warn!(
                        license_id = %license.id,
                        error = %e,
                        "Failed to publish license expiring event (30 day)"
                    );
                } else {
                    notifications_sent += 1;
                }
            }
        }

        info!(
            count = notifications_sent,
            "License deadline check complete"
        );
        Ok(notifications_sent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::InMemoryEventPublisher;
    use crate::repository::license::{
        CreateLicenseDocumentRequest, CreateTenantLicenseRequest, DocumentStatus,
        LicenseRepository, LicenseRequirementRow, LicenseRow, LicenseStatus, LicenseTypeRow,
        TenantLicenseDocumentRow, TenantLicenseRow,
    };
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use rust_decimal::Decimal;
    use std::sync::Mutex;

    struct MockLicenseRepository {
        licenses: Mutex<Vec<LicenseRow>>,
    }

    impl MockLicenseRepository {
        fn new() -> Self {
            Self {
                licenses: Mutex::new(Vec::new()),
            }
        }

        fn add_license(&self, license: LicenseRow) {
            self.licenses.lock().unwrap().push(license);
        }
    }

    #[async_trait]
    impl LicenseRepository for MockLicenseRepository {
        async fn get_license_types(&self) -> Result<Vec<LicenseTypeRow>> {
            Ok(vec![])
        }
        async fn get_license_type_by_id(&self, _id: &str) -> Result<Option<LicenseTypeRow>> {
            Ok(None)
        }
        async fn get_license_type_by_code(&self, _code: &str) -> Result<Option<LicenseTypeRow>> {
            Ok(None)
        }
        async fn get_requirements_by_license_type(
            &self,
            _license_type_id: &str,
        ) -> Result<Vec<LicenseRequirementRow>> {
            Ok(vec![])
        }
        async fn get_requirement_by_id(&self, _id: &str) -> Result<Option<LicenseRequirementRow>> {
            Ok(None)
        }
        async fn get_mandatory_requirements(
            &self,
            _license_type_id: &str,
        ) -> Result<Vec<LicenseRequirementRow>> {
            Ok(vec![])
        }
        async fn get_tenant_licenses(
            &self,
            _tenant_id: &TenantId,
        ) -> Result<Vec<TenantLicenseRow>> {
            Ok(vec![])
        }
        async fn get_tenant_license_by_id(
            &self,
            _tenant_id: &TenantId,
            _license_id: &str,
        ) -> Result<Option<TenantLicenseRow>> {
            Ok(None)
        }
        async fn get_tenant_license_by_type(
            &self,
            _tenant_id: &TenantId,
            _license_type_id: &str,
        ) -> Result<Option<TenantLicenseRow>> {
            Ok(None)
        }
        async fn create_tenant_license(
            &self,
            _request: &CreateTenantLicenseRequest,
        ) -> Result<TenantLicenseRow> {
            Err(ramp_common::Error::NotFound("not implemented".to_string()))
        }
        async fn update_license_status(
            &self,
            _tenant_id: &TenantId,
            _license_id: &str,
            _status: LicenseStatus,
            _reviewed_by: Option<&str>,
            _review_notes: Option<&str>,
            _rejection_reason: Option<&str>,
        ) -> Result<()> {
            Ok(())
        }
        async fn update_compliance_percentage(
            &self,
            _tenant_id: &TenantId,
            _license_id: &str,
            _percentage: Decimal,
        ) -> Result<()> {
            Ok(())
        }
        async fn set_license_number(
            &self,
            _tenant_id: &TenantId,
            _license_id: &str,
            _license_number: &str,
            _issued_at: DateTime<Utc>,
            _expires_at: Option<DateTime<Utc>>,
        ) -> Result<()> {
            Ok(())
        }
        async fn get_expiring_licenses(
            &self,
            _days_until_expiry: i32,
        ) -> Result<Vec<TenantLicenseRow>> {
            Ok(vec![])
        }
        async fn get_license_documents(
            &self,
            _tenant_id: &TenantId,
            _tenant_license_id: &str,
        ) -> Result<Vec<TenantLicenseDocumentRow>> {
            Ok(vec![])
        }
        async fn get_document_by_id(
            &self,
            _tenant_id: &TenantId,
            _document_id: &str,
        ) -> Result<Option<TenantLicenseDocumentRow>> {
            Ok(None)
        }
        async fn get_document_by_requirement(
            &self,
            _tenant_id: &TenantId,
            _tenant_license_id: &str,
            _requirement_id: &str,
        ) -> Result<Option<TenantLicenseDocumentRow>> {
            Ok(None)
        }
        async fn create_document(
            &self,
            _request: &CreateLicenseDocumentRequest,
        ) -> Result<TenantLicenseDocumentRow> {
            Err(ramp_common::Error::NotFound("not implemented".to_string()))
        }
        async fn update_document_status(
            &self,
            _tenant_id: &TenantId,
            _document_id: &str,
            _status: DocumentStatus,
            _reviewed_by: Option<&str>,
            _review_notes: Option<&str>,
            _rejection_reason: Option<&str>,
        ) -> Result<()> {
            Ok(())
        }
        async fn count_approved_documents(
            &self,
            _tenant_id: &TenantId,
            _tenant_license_id: &str,
        ) -> Result<i64> {
            Ok(0)
        }

        async fn find_expiring_before(&self, before: DateTime<Utc>) -> Result<Vec<LicenseRow>> {
            let licenses = self.licenses.lock().unwrap();
            Ok(licenses
                .iter()
                .filter(|l| l.expires_at <= before && l.expires_at > Utc::now())
                .cloned()
                .collect())
        }

        async fn get_by_id(&self, _tenant_id: &TenantId, id: &str) -> Result<Option<LicenseRow>> {
            let licenses = self.licenses.lock().unwrap();
            Ok(licenses.iter().find(|l| l.id == id).cloned())
        }

        async fn list_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<LicenseRow>> {
            let licenses = self.licenses.lock().unwrap();
            Ok(licenses
                .iter()
                .filter(|l| l.tenant_id == tenant_id.0)
                .cloned()
                .collect())
        }

        async fn create(&self, license: &LicenseRow) -> Result<()> {
            self.licenses.lock().unwrap().push(license.clone());
            Ok(())
        }

        async fn update(&self, license: &LicenseRow) -> Result<()> {
            let mut licenses = self.licenses.lock().unwrap();
            if let Some(idx) = licenses.iter().position(|l| l.id == license.id) {
                licenses[idx] = license.clone();
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_check_expiring_licenses_7_days() {
        let license_repo = Arc::new(MockLicenseRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        // Add license expiring in 5 days
        let expires_in_5 = LicenseRow {
            id: "lic_1".to_string(),
            tenant_id: "tenant_1".to_string(),
            license_type: "exchange".to_string(),
            license_number: "EX-001".to_string(),
            issued_by: "SBV".to_string(),
            issued_at: Utc::now() - Duration::days(365),
            expires_at: Utc::now() + Duration::days(5),
            status: "ACTIVE".to_string(),
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        license_repo.add_license(expires_in_5);

        let checker = LicenseDeadlineChecker::new(license_repo, event_publisher.clone());
        let count = checker.check_expiring_licenses().await.unwrap();

        assert_eq!(count, 1);

        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "regulatory.license_expiring");
    }

    #[tokio::test]
    async fn test_check_expiring_licenses_30_days() {
        let license_repo = Arc::new(MockLicenseRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        // Add license expiring in 29 days (within 28-30 window)
        let expires_in_29 = LicenseRow {
            id: "lic_2".to_string(),
            tenant_id: "tenant_1".to_string(),
            license_type: "payment".to_string(),
            license_number: "PAY-001".to_string(),
            issued_by: "SBV".to_string(),
            issued_at: Utc::now() - Duration::days(365),
            expires_at: Utc::now() + Duration::days(29),
            status: "ACTIVE".to_string(),
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        license_repo.add_license(expires_in_29);

        let checker = LicenseDeadlineChecker::new(license_repo, event_publisher.clone());
        let count = checker.check_expiring_licenses().await.unwrap();

        assert_eq!(count, 1);

        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
    }
}
