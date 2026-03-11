use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RescreeningTriggerKind {
    Scheduled,
    WatchlistDelta,
    DocumentExpiry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RescreeningPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestrictionStatus {
    None,
    ReviewRequired,
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RescreeningRunStatus {
    Pending,
    Alerted,
    Restricted,
    Cleared,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescreeningSubject {
    pub tenant_id: String,
    pub user_id: String,
    pub status: String,
    pub kyc_verified_at: Option<DateTime<Utc>>,
    pub risk_flags: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescreeningRun {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub trigger_kind: RescreeningTriggerKind,
    pub status: RescreeningRunStatus,
    pub priority: RescreeningPriority,
    pub restriction_status: RestrictionStatus,
    pub alert_codes: Vec<String>,
    pub scheduled_for: DateTime<Utc>,
    pub next_run_at: DateTime<Utc>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescreeningEngineConfig {
    pub cadence_days: i64,
    pub doc_expiry_notice_days: i64,
}

impl Default for RescreeningEngineConfig {
    fn default() -> Self {
        Self {
            cadence_days: 180,
            doc_expiry_notice_days: 30,
        }
    }
}

pub struct RescreeningEngine {
    config: RescreeningEngineConfig,
}

impl RescreeningEngine {
    pub fn new(config: RescreeningEngineConfig) -> Self {
        Self { config }
    }

    pub fn build_scheduled_runs(
        &self,
        subjects: &[RescreeningSubject],
        as_of: DateTime<Utc>,
    ) -> Vec<RescreeningRun> {
        subjects
            .iter()
            .filter(|subject| subject.status == "ACTIVE")
            .filter_map(|subject| self.build_due_run(subject, as_of))
            .collect()
    }

    pub fn evaluate_watchlist_delta(
        &self,
        subject: &RescreeningSubject,
        as_of: DateTime<Utc>,
        sanctions_hit: bool,
        pep_hit: bool,
        adverse_media_hit: bool,
    ) -> RescreeningRun {
        let mut alert_codes = Vec::new();
        if sanctions_hit {
            alert_codes.push("sanctions_delta".to_string());
        }
        if pep_hit {
            alert_codes.push("pep_delta".to_string());
        }
        if adverse_media_hit {
            alert_codes.push("adverse_media_delta".to_string());
        }

        let (priority, restriction_status, status) = if sanctions_hit {
            (
                RescreeningPriority::Critical,
                RestrictionStatus::Restricted,
                RescreeningRunStatus::Restricted,
            )
        } else if pep_hit || adverse_media_hit {
            (
                RescreeningPriority::High,
                RestrictionStatus::ReviewRequired,
                RescreeningRunStatus::Alerted,
            )
        } else {
            (
                RescreeningPriority::Low,
                RestrictionStatus::None,
                RescreeningRunStatus::Cleared,
            )
        };

        RescreeningRun {
            id: format!("rsr_{}_watchlist", subject.user_id),
            tenant_id: subject.tenant_id.clone(),
            user_id: subject.user_id.clone(),
            trigger_kind: RescreeningTriggerKind::WatchlistDelta,
            status,
            priority,
            restriction_status,
            alert_codes,
            scheduled_for: as_of,
            next_run_at: as_of + Duration::days(self.config.cadence_days),
            details: json!({
                "sanctionsHit": sanctions_hit,
                "pepHit": pep_hit,
                "adverseMediaHit": adverse_media_hit
            }),
        }
    }

    fn build_due_run(
        &self,
        subject: &RescreeningSubject,
        as_of: DateTime<Utc>,
    ) -> Option<RescreeningRun> {
        let computed_next_run_at = next_run_at(subject, self.config.cadence_days);
        let document_expiry = document_expiry_at(subject);
        let doc_expiry_alert = document_expiry
            .map(|value| value <= as_of + Duration::days(self.config.doc_expiry_notice_days))
            .unwrap_or(false);

        if computed_next_run_at > as_of && !doc_expiry_alert {
            return None;
        }

        let priority = if doc_expiry_alert {
            RescreeningPriority::High
        } else {
            RescreeningPriority::Medium
        };
        let restriction_status = if doc_expiry_alert {
            RestrictionStatus::ReviewRequired
        } else {
            RestrictionStatus::None
        };
        let status = if doc_expiry_alert {
            RescreeningRunStatus::Alerted
        } else {
            RescreeningRunStatus::Pending
        };

        Some(RescreeningRun {
            id: format!("rsr_{}_scheduled", subject.user_id),
            tenant_id: subject.tenant_id.clone(),
            user_id: subject.user_id.clone(),
            trigger_kind: if doc_expiry_alert {
                RescreeningTriggerKind::DocumentExpiry
            } else {
                RescreeningTriggerKind::Scheduled
            },
            status,
            priority,
            restriction_status,
            alert_codes: if doc_expiry_alert {
                vec!["document_expiry_due".to_string()]
            } else {
                vec!["periodic_rescreening_due".to_string()]
            },
            scheduled_for: as_of,
            next_run_at: as_of + Duration::days(self.config.cadence_days),
            details: json!({
                "previousNextRunAt": computed_next_run_at.to_rfc3339(),
                "documentExpiryAt": document_expiry.map(|value| value.to_rfc3339()),
            }),
        })
    }
}

impl Default for RescreeningEngine {
    fn default() -> Self {
        Self::new(RescreeningEngineConfig::default())
    }
}

pub fn next_run_at(subject: &RescreeningSubject, cadence_days: i64) -> DateTime<Utc> {
    subject
        .risk_flags
        .get("rescreening")
        .and_then(|value| value.get("nextRunAt"))
        .and_then(parse_timestamp)
        .unwrap_or_else(|| {
            subject
                .kyc_verified_at
                .unwrap_or_else(Utc::now)
                + Duration::days(cadence_days)
        })
}

pub fn document_expiry_at(subject: &RescreeningSubject) -> Option<DateTime<Utc>> {
    subject
        .risk_flags
        .get("rescreening")
        .and_then(|value| value.get("documentExpiryAt"))
        .and_then(parse_timestamp)
}

fn parse_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    value
        .as_str()
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|value| value.with_timezone(&Utc))
}
