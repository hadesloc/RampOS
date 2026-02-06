//! Compliance repository - Database access for regulatory compliance monitoring

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceBreach {
    pub tenant_id: String,
    pub alert_type: String,
    pub threshold: String,
    pub current_value: String,
    pub breach_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbvReportSchedule {
    pub tenant_id: String,
    pub report_type: String,
    pub due_date: DateTime<Utc>,
    pub status: String,
}

#[async_trait]
pub trait ComplianceRepository: Send + Sync {
    async fn find_threshold_breaches(&self) -> Result<Vec<ComplianceBreach>>;
    async fn find_upcoming_sbv_reports(&self, before: DateTime<Utc>) -> Result<Vec<SbvReportSchedule>>;
}
