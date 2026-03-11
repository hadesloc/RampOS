use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::service::reconciliation::{
    OnChainTransaction, ReconciliationQueueItem, ReconciliationReport, ReconciliationService,
    SettlementRecord,
};
use crate::service::settlement::Settlement;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationWorkbenchSnapshot {
    pub generated_at: DateTime<Utc>,
    pub report: ReconciliationReport,
    pub queue: Vec<ReconciliationQueueItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationWorkbench {
    pub generated_at: DateTime<Utc>,
    pub settlements: Vec<Settlement>,
    pub report: ReconciliationReport,
    pub queue: Vec<ReconciliationQueueItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationExportArtifact {
    pub file_name: String,
    pub content_type: String,
    pub contents: Vec<u8>,
}

pub struct ReconciliationExportService;

impl ReconciliationExportService {
    pub fn new() -> Self {
        Self
    }

    pub fn build_snapshot(
        service: &ReconciliationService,
        on_chain_txs: &[OnChainTransaction],
        settlements: &[SettlementRecord],
    ) -> ReconciliationWorkbenchSnapshot {
        let report = service.reconcile(on_chain_txs, settlements);
        let queue = service.build_break_queue(&report, settlements);

        ReconciliationWorkbenchSnapshot {
            generated_at: Utc::now(),
            report,
            queue,
        }
    }

    pub fn build_workbench(&self, settlements: &[Settlement]) -> ReconciliationWorkbench {
        let service = ReconciliationService::new();
        let settlement_records = settlements
            .iter()
            .map(|settlement| SettlementRecord {
                id: settlement.id.clone(),
                tx_hash: None,
                amount: 0.0,
                currency: "USDT".to_string(),
                status: settlement.status.as_db_str().to_string(),
                created_at: settlement.created_at,
                updated_at: settlement.updated_at,
            })
            .collect::<Vec<_>>();
        let snapshot = Self::build_snapshot(&service, &[], &settlement_records);

        ReconciliationWorkbench {
            generated_at: snapshot.generated_at,
            settlements: settlements.to_vec(),
            report: snapshot.report,
            queue: snapshot.queue,
        }
    }

    pub fn export_snapshot_json(snapshot: &ReconciliationWorkbenchSnapshot) -> serde_json::Value {
        json!(snapshot)
    }

    pub fn export_queue_csv(snapshot: &ReconciliationWorkbenchSnapshot) -> String {
        let mut lines = vec![
            "discrepancy_id,report_id,owner_lane,root_cause,age_bucket,severity,detected_at,settlement_id,on_chain_tx,summary,suggested_match_count"
                .to_string(),
        ];

        for item in &snapshot.queue {
            lines.push(format!(
                "{},{},{},{},{},{},{},{},{},{},{}",
                csv_cell(&item.discrepancy_id),
                csv_cell(&item.report_id),
                csv_cell(&format!("{:?}", item.owner_lane)),
                csv_cell(&format!("{:?}", item.root_cause)),
                csv_cell(&format!("{:?}", item.age_bucket)),
                csv_cell(&format!("{:?}", item.severity)),
                csv_cell(&item.detected_at.to_rfc3339()),
                csv_cell(item.settlement_id.as_deref().unwrap_or("")),
                csv_cell(item.on_chain_tx.as_deref().unwrap_or("")),
                csv_cell(&item.summary),
                item.suggested_matches.len(),
            ));
        }

        lines.join("\n")
    }

    pub fn export_workbench_json(
        &self,
        workbench: &ReconciliationWorkbench,
    ) -> Result<ReconciliationExportArtifact, String> {
        let contents = serde_json::to_vec_pretty(&json!({
            "generatedAt": workbench.generated_at.to_rfc3339(),
            "settlements": workbench
                .settlements
                .iter()
                .map(export_settlement)
                .collect::<Vec<_>>(),
            "report": export_report(&workbench.report),
            "queue": workbench.queue,
        }))
        .map_err(|error| format!("Failed to serialize workbench export: {}", error))?;

        Ok(ReconciliationExportArtifact {
            file_name: "reconciliation_workbench.json".to_string(),
            content_type: "application/json".to_string(),
            contents,
        })
    }

    pub fn export_evidence_json(
        &self,
        report: &ReconciliationReport,
        settlements: &[Settlement],
        discrepancy_id: &str,
    ) -> Result<ReconciliationExportArtifact, String> {
        let service = ReconciliationService::new();
        let evidence_pack = service.build_evidence_pack(report, settlements, discrepancy_id)?;
        let contents = serde_json::to_vec_pretty(&json!({
            "evidencePack": evidence_pack,
        }))
        .map_err(|error| format!("Failed to serialize evidence export: {}", error))?;

        Ok(ReconciliationExportArtifact {
            file_name: format!("reconciliation_evidence_{}.json", discrepancy_id),
            content_type: "application/json".to_string(),
            contents,
        })
    }
}

fn csv_cell(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn export_report(report: &ReconciliationReport) -> serde_json::Value {
    json!({
        "id": report.id,
        "startedAt": report.started_at.to_rfc3339(),
        "completedAt": report.completed_at.to_rfc3339(),
        "totalSettlementsChecked": report.total_settlements_checked,
        "totalOnChainTxsChecked": report.total_on_chain_txs_checked,
        "discrepancies": report.discrepancies.iter().map(export_discrepancy).collect::<Vec<_>>(),
        "totalDiscrepancies": report.total_discrepancies,
        "criticalCount": report.critical_count,
        "status": format!("{:?}", report.status),
    })
}

fn export_discrepancy(discrepancy: &crate::service::reconciliation::Discrepancy) -> serde_json::Value {
    json!({
        "id": discrepancy.id,
        "kind": format!("{:?}", discrepancy.kind),
        "settlementId": discrepancy.settlement_id,
        "onChainTx": discrepancy.on_chain_tx,
        "expectedAmount": discrepancy.expected_amount,
        "actualAmount": discrepancy.actual_amount,
        "severity": format!("{:?}", discrepancy.severity),
        "detectedAt": discrepancy.detected_at.to_rfc3339(),
        "details": discrepancy.details,
    })
}

fn export_settlement(settlement: &Settlement) -> serde_json::Value {
    json!({
        "id": settlement.id,
        "offrampIntentId": settlement.offramp_intent_id,
        "status": settlement.status.as_db_str(),
        "bankReference": settlement.bank_reference,
        "errorMessage": settlement.error_message,
        "createdAt": settlement.created_at.to_rfc3339(),
        "updatedAt": settlement.updated_at.to_rfc3339(),
    })
}
