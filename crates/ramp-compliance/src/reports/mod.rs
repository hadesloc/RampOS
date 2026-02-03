pub mod ctr;
pub mod generator;
pub mod templates;
pub mod types;

#[cfg(test)]
mod tests;

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
