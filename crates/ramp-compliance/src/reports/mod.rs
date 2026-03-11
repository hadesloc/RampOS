pub mod ctr;
pub mod generator;
pub mod sbv_ctr;
pub mod sbv_sar;
pub mod sbv_scheduler;
pub mod templates;
pub mod types;

#[cfg(test)]
mod tests;

pub use ctr::{
    CtrFilingStatus, CtrRecord, CtrService, GenerateCtrReportRequest, GeneratedCtrReport,
    CTR_THRESHOLD_VND,
};
pub use generator::{ReportGenerator, ReportType};
pub use templates::{AmlReport, DailyReport, KycReport, SarReport};
pub use types::{
    AmlReport as AmlReportType,
    AmlReportParams,
    // We re-export these from types as well to ensure consistency if used from types
    DailyReport as DailyReportType,
    KycReport as KycReportType,
    KycReportParams,
    Report,
    SuspiciousActivityReport,
};

// SBV-specific exports
pub use sbv_ctr::{
    SbvCtrReport, SbvCtrTransaction, SbvCustomerInfo, SbvFilingInstitution, SbvIdType,
    SbvReportStatus, SbvTransactionType, SBV_CTR_THRESHOLD_VND,
};
pub use sbv_sar::{
    RecommendedAction, RiskIndicator, SbvRiskLevel, SbvSarReport, SbvSarStatus,
    SbvSarWeeklySummary, SuspiciousActivityType,
};
pub use sbv_scheduler::{SbvMonthlyComplianceReport, SbvReportScheduler, SbvScheduleConfig};
