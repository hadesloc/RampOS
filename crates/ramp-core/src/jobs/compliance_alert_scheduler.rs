//! Compliance alert scheduler job
//! Monitors compliance thresholds and SBV report deadlines

use crate::event::{AlertSeverity, EventPublisher, RegulatoryEvent};
use crate::repository::compliance::ComplianceRepository;
use chrono::{Duration, Utc};
use ramp_common::{types::TenantId, Result};
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct ComplianceAlertScheduler<C, E> {
    compliance_repo: Arc<C>,
    event_publisher: Arc<E>,
}

impl<C: ComplianceRepository, E: EventPublisher> ComplianceAlertScheduler<C, E> {
    pub fn new(compliance_repo: Arc<C>, event_publisher: Arc<E>) -> Self {
        Self {
            compliance_repo,
            event_publisher,
        }
    }

    pub async fn run(&self) {
        info!("Starting compliance alert scheduler");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // Hourly

        loop {
            interval.tick().await;
            if let Err(e) = self.check_compliance_thresholds().await {
                error!(error = %e, "Failed to check compliance thresholds");
            }
            if let Err(e) = self.check_sbv_deadlines().await {
                error!(error = %e, "Failed to check SBV deadlines");
            }
        }
    }

    pub async fn check_compliance_thresholds(&self) -> Result<usize> {
        let mut alerts_sent = 0;

        let breaches = self.compliance_repo.find_threshold_breaches().await?;

        for breach in breaches {
            let severity = match breach.breach_percentage {
                p if p >= 100.0 => AlertSeverity::Critical,
                p if p >= 90.0 => AlertSeverity::High,
                p if p >= 75.0 => AlertSeverity::Medium,
                _ => AlertSeverity::Low,
            };

            let event = RegulatoryEvent::ComplianceAlert {
                alert_type: breach.alert_type.clone(),
                threshold: breach.threshold.clone(),
                current_value: breach.current_value.clone(),
                severity,
            };

            if let Err(e) = self
                .event_publisher
                .publish_regulatory_event(&TenantId(breach.tenant_id.clone()), event)
                .await
            {
                warn!(
                    tenant_id = %breach.tenant_id,
                    alert_type = %breach.alert_type,
                    error = %e,
                    "Failed to publish compliance alert"
                );
            } else {
                alerts_sent += 1;
            }
        }

        info!(count = alerts_sent, "Compliance threshold check complete");
        Ok(alerts_sent)
    }

    pub async fn check_sbv_deadlines(&self) -> Result<usize> {
        let mut notifications_sent = 0;
        let now = Utc::now();

        let upcoming_reports = self
            .compliance_repo
            .find_upcoming_sbv_reports(now + Duration::days(14))
            .await?;

        for report in upcoming_reports {
            let days_remaining = (report.due_date - now).num_days() as i32;

            // Send notifications at 14, 7, 3, and 1 day marks
            if days_remaining == 14 || days_remaining == 7 || days_remaining == 3 || days_remaining == 1 {
                let event = RegulatoryEvent::SbvSubmissionDue {
                    report_type: report.report_type.clone(),
                    due_date: report.due_date,
                    days_remaining,
                };

                if let Err(e) = self
                    .event_publisher
                    .publish_regulatory_event(&TenantId(report.tenant_id.clone()), event)
                    .await
                {
                    warn!(
                        tenant_id = %report.tenant_id,
                        report_type = %report.report_type,
                        error = %e,
                        "Failed to publish SBV deadline notification"
                    );
                } else {
                    notifications_sent += 1;
                }
            }
        }

        info!(count = notifications_sent, "SBV deadline check complete");
        Ok(notifications_sent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::InMemoryEventPublisher;
    use crate::repository::compliance::{
        ComplianceBreach, ComplianceRepository, SbvReportSchedule,
    };
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    struct MockComplianceRepository {
        breaches: Mutex<Vec<ComplianceBreach>>,
        reports: Mutex<Vec<SbvReportSchedule>>,
    }

    impl MockComplianceRepository {
        fn new() -> Self {
            Self {
                breaches: Mutex::new(Vec::new()),
                reports: Mutex::new(Vec::new()),
            }
        }

        fn add_breach(&self, breach: ComplianceBreach) {
            self.breaches.lock().unwrap().push(breach);
        }

        fn add_report(&self, report: SbvReportSchedule) {
            self.reports.lock().unwrap().push(report);
        }
    }

    #[async_trait]
    impl ComplianceRepository for MockComplianceRepository {
        async fn find_threshold_breaches(&self) -> Result<Vec<ComplianceBreach>> {
            Ok(self.breaches.lock().unwrap().clone())
        }

        async fn find_upcoming_sbv_reports(
            &self,
            before: DateTime<Utc>,
        ) -> Result<Vec<SbvReportSchedule>> {
            let reports = self.reports.lock().unwrap();
            Ok(reports
                .iter()
                .filter(|r| r.due_date <= before && r.due_date > Utc::now())
                .cloned()
                .collect())
        }
    }

    #[tokio::test]
    async fn test_compliance_threshold_alert() {
        let compliance_repo = Arc::new(MockComplianceRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        compliance_repo.add_breach(ComplianceBreach {
            tenant_id: "tenant_1".to_string(),
            alert_type: "daily_volume_limit".to_string(),
            threshold: "1000000000".to_string(),
            current_value: "950000000".to_string(),
            breach_percentage: 95.0,
        });

        let scheduler =
            ComplianceAlertScheduler::new(compliance_repo, event_publisher.clone());
        let count = scheduler.check_compliance_thresholds().await.unwrap();

        assert_eq!(count, 1);

        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "regulatory.compliance_alert");
    }

    #[tokio::test]
    async fn test_sbv_deadline_notification() {
        let compliance_repo = Arc::new(MockComplianceRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());

        // Set due_date exactly 7 days from now (at the same time) so days_remaining == 7
        let due_date = Utc::now() + Duration::days(7) + Duration::hours(1);

        compliance_repo.add_report(SbvReportSchedule {
            tenant_id: "tenant_1".to_string(),
            report_type: "monthly_transaction".to_string(),
            due_date,
            status: "PENDING".to_string(),
        });

        let scheduler =
            ComplianceAlertScheduler::new(compliance_repo, event_publisher.clone());
        let count = scheduler.check_sbv_deadlines().await.unwrap();

        assert_eq!(count, 1);

        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "regulatory.sbv_submission_due");
    }
}
