use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use ramp_compliance::rescreening::{RescreeningRun, RestrictionStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescreeningAccountAction {
    pub restriction_status: RestrictionStatus,
    pub alert_codes: Vec<String>,
    pub risk_flags: Value,
}

pub struct RescreeningActionService;

impl RescreeningActionService {
    pub fn build_account_action(
        existing_risk_flags: &Value,
        run: &RescreeningRun,
    ) -> RescreeningAccountAction {
        let mut risk_flags = existing_risk_flags.clone();
        let rescreening = json!({
            "lastRunAt": run.scheduled_for.to_rfc3339(),
            "nextRunAt": run.next_run_at.to_rfc3339(),
            "latestOutcome": format!("{:?}", run.status),
            "restrictionStatus": format!("{:?}", run.restriction_status),
            "alertCodes": run.alert_codes,
            "updatedAt": Utc::now().to_rfc3339()
        });

        if let Some(object) = risk_flags.as_object_mut() {
            object.insert("rescreening".to_string(), rescreening);
        } else {
            risk_flags = json!({ "rescreening": rescreening });
        }

        RescreeningAccountAction {
            restriction_status: run.restriction_status.clone(),
            alert_codes: run.alert_codes.clone(),
            risk_flags,
        }
    }
}
