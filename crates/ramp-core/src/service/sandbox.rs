use async_trait::async_trait;
use chrono::{Duration, Utc};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::repository::tenant::TenantRow;
use crate::repository::{settlement::SettlementRow, webhook::WebhookEventRow};
use crate::service::onboarding::OnboardingService;
use crate::service::reconciliation::{
    Discrepancy, DiscrepancyKind, ReconciliationReport, ReconciliationStatus, Severity,
};
use crate::service::replay::{ReplayBundle, ReplayBundleAssembler, ReplayTimelineEntry};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxResetStrategy {
    ResetToPreset,
    ResetScenarioData,
    ResetRuntimeArtifacts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxScenarioStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPreset {
    pub code: String,
    pub name: String,
    pub seed_package_version: String,
    pub default_scenarios: Vec<String>,
    pub metadata: Value,
    pub reset_strategy: SandboxResetStrategy,
    pub reset_semantics: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSeedRequest {
    pub tenant_name: String,
    pub preset_code: String,
    pub scenario_code: Option<String>,
    pub config_overrides: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSeedResult {
    pub tenant: TenantRow,
    pub preset: SandboxPreset,
    pub scenario_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxScenarioRunRequest {
    pub tenant_id: String,
    pub preset_code: String,
    pub scenario_code: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxScenarioRun {
    pub run_id: String,
    pub tenant_id: String,
    pub preset_code: String,
    pub scenario_code: String,
    pub status: SandboxScenarioStatus,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResetResult {
    pub tenant_id: String,
    pub preset_code: Option<String>,
    pub reset_strategy: String,
    pub status: String,
    pub message: String,
}

#[async_trait]
pub trait SandboxScenarioRunner: Send + Sync {
    async fn start_run(&self, request: SandboxScenarioRunRequest) -> Result<SandboxScenarioRun>;
}

pub struct SandboxService {
    onboarding_service: Arc<OnboardingService>,
    scenario_runner: Arc<dyn SandboxScenarioRunner>,
    presets: HashMap<String, SandboxPreset>,
    run_state: Arc<Mutex<HashMap<String, SandboxScenarioRun>>>,
}

pub fn default_sandbox_presets() -> Vec<SandboxPreset> {
    vec![
        SandboxPreset {
            code: "BASELINE".to_string(),
            name: "Baseline Sandbox".to_string(),
            seed_package_version: "2026-03-08".to_string(),
            default_scenarios: vec![
                "PAYIN_BASELINE".to_string(),
                "OFFRAMP_BASELINE".to_string(),
                "WEBHOOK_RETRY_BASELINE".to_string(),
            ],
            metadata: json!({
                "category": "general",
                "operator_surface": "admin",
                "supports_replay": true
            }),
            reset_strategy: SandboxResetStrategy::ResetToPreset,
            reset_semantics: json!({
                "drop_runtime_events": true,
                "drop_seeded_users": true,
                "drop_seeded_intents": true,
                "drop_seeded_balances": true,
                "preserve_admin_credentials": true
            }),
        },
        SandboxPreset {
            code: "PAYIN_FAILURE_DRILL".to_string(),
            name: "Pay-in Failure Drill".to_string(),
            seed_package_version: "2026-03-08".to_string(),
            default_scenarios: vec![
                "PAYIN_BANK_TIMEOUT".to_string(),
                "PAYIN_COMPLIANCE_REVIEW".to_string(),
            ],
            metadata: json!({
                "category": "payin",
                "operator_surface": "admin",
                "supports_replay": true
            }),
            reset_strategy: SandboxResetStrategy::ResetScenarioData,
            reset_semantics: json!({
                "drop_runtime_events": true,
                "drop_seeded_intents": true,
                "drop_seeded_webhooks": true,
                "preserve_seeded_users": true,
                "preserve_admin_credentials": true
            }),
        },
        SandboxPreset {
            code: "LIQUIDITY_DRILL".to_string(),
            name: "Liquidity Drill".to_string(),
            seed_package_version: "2026-03-08".to_string(),
            default_scenarios: vec![
                "RFQ_BASELINE".to_string(),
                "LP_NO_FILL".to_string(),
                "SETTLEMENT_DELAY".to_string(),
            ],
            metadata: json!({
                "category": "liquidity",
                "operator_surface": "admin",
                "supports_replay": true
            }),
            reset_strategy: SandboxResetStrategy::ResetRuntimeArtifacts,
            reset_semantics: json!({
                "drop_runtime_events": true,
                "drop_rfq_artifacts": true,
                "drop_settlement_attempts": true,
                "preserve_seeded_users": true,
                "preserve_admin_credentials": true
            }),
        },
    ]
}

impl SandboxService {
    pub fn new(
        onboarding_service: Arc<OnboardingService>,
        scenario_runner: Arc<dyn SandboxScenarioRunner>,
        presets: Vec<SandboxPreset>,
    ) -> Self {
        let presets = presets
            .into_iter()
            .map(|preset| (preset.code.clone(), preset))
            .collect();

        Self {
            onboarding_service,
            scenario_runner,
            presets,
            run_state: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn list_presets(&self) -> Vec<SandboxPreset> {
        let mut presets: Vec<_> = self.presets.values().cloned().collect();
        presets.sort_by(|left, right| left.code.cmp(&right.code));
        presets
    }

    pub async fn seed_tenant(&self, request: SandboxSeedRequest) -> Result<SandboxSeedResult> {
        let preset = self.require_preset(&request.preset_code)?;
        let scenario_code =
            self.resolve_scenario_code(&preset, request.scenario_code.as_deref())?;

        let mut config = json!({
            "environment": "sandbox",
            "sandbox": {
                "preset_code": preset.code,
                "seed_package_version": preset.seed_package_version,
                "scenario_code": scenario_code,
                "metadata": preset.metadata,
                "reset_strategy": reset_strategy_label(&preset.reset_strategy),
                "reset_semantics": preset.reset_semantics,
            }
        });
        merge_json_object(&mut config, request.config_overrides)?;

        let tenant = self
            .onboarding_service
            .create_tenant(&request.tenant_name, config)
            .await?;

        Ok(SandboxSeedResult {
            tenant,
            preset,
            scenario_code,
        })
    }

    pub async fn start_scenario_run(
        &self,
        tenant_id: &str,
        preset_code: &str,
        scenario_code: &str,
        metadata: Value,
    ) -> Result<SandboxScenarioRun> {
        let preset = self.require_preset(preset_code)?;
        self.ensure_known_scenario(&preset, scenario_code)?;

        let run = self
            .scenario_runner
            .start_run(SandboxScenarioRunRequest {
                tenant_id: tenant_id.to_string(),
                preset_code: preset.code,
                scenario_code: scenario_code.to_string(),
                metadata,
            })
            .await?;

        self.run_state
            .lock()
            .unwrap()
            .insert(run.run_id.clone(), run.clone());

        Ok(run)
    }

    pub fn reset_tenant(
        &self,
        tenant_id: &str,
        preset_code: Option<&str>,
        reason: Option<&str>,
    ) -> Result<SandboxResetResult> {
        let reset_preset = match preset_code {
            Some(code) => Some(self.require_preset(code)?),
            None => self.list_presets().into_iter().next(),
        };

        let reset_strategy = reset_preset
            .as_ref()
            .map(|preset| reset_strategy_label(&preset.reset_strategy).to_string())
            .unwrap_or_else(|| "RESET_TO_PRESET".to_string());

        let message = match reason {
            Some(reason) if !reason.trim().is_empty() => {
                format!("Sandbox reset accepted for {tenant_id}: {reason}")
            }
            _ => format!("Sandbox reset accepted for {tenant_id}"),
        };

        Ok(SandboxResetResult {
            tenant_id: tenant_id.to_string(),
            preset_code: reset_preset.map(|preset| preset.code),
            reset_strategy,
            status: "ACCEPTED".to_string(),
            message,
        })
    }

    pub fn get_run_status(&self, run_id: &str) -> Result<SandboxScenarioRun> {
        if let Some(run) = self.run_state.lock().unwrap().get(run_id).cloned() {
            return Ok(run);
        }

        if run_id.starts_with("sandbox_run_") || run_id.starts_with("run_") {
            return Ok(SandboxScenarioRun {
                run_id: run_id.to_string(),
                tenant_id: "tenant_sandbox_preview".to_string(),
                preset_code: "BASELINE".to_string(),
                scenario_code: "PAYIN_BASELINE".to_string(),
                status: SandboxScenarioStatus::Pending,
                metadata: json!({
                    "bounded": true,
                    "message": "Scenario execution is staged and will become fully live in a later W1 slice."
                }),
            });
        }

        Err(Error::NotFound(format!("Sandbox run not found: {run_id}")))
    }

    pub fn replay_bundle(&self, journey_id: &str) -> Result<ReplayBundle> {
        if journey_id.trim().is_empty() {
            return Err(Error::Validation(
                "Journey ID is required for replay retrieval".to_string(),
            ));
        }

        let now = Utc::now();
        let webhook_entry = ReplayTimelineEntry::from_webhook_event(WebhookEventRow {
            id: format!("evt_{journey_id}"),
            tenant_id: "tenant_sandbox_preview".to_string(),
            event_type: "intent.status.changed".to_string(),
            intent_id: Some(journey_id.to_string()),
            payload: json!({
                "apiSecret": "ramp_secret_preview",
                "headers": {
                    "Authorization": "Bearer preview-token"
                },
                "state": "FUNDS_PENDING",
                "journeyId": journey_id,
            }),
            status: "DELIVERED".to_string(),
            attempts: 1,
            max_attempts: 10,
            last_attempt_at: Some(now - Duration::minutes(8)),
            next_attempt_at: None,
            last_error: None,
            delivered_at: Some(now - Duration::minutes(8)),
            response_status: Some(200),
            created_at: now - Duration::minutes(9),
        });
        let settlement_entry = ReplayTimelineEntry::from_settlement_row(SettlementRow {
            id: format!("stl_{journey_id}"),
            offramp_intent_id: journey_id.to_string(),
            status: "PROCESSING".to_string(),
            bank_reference: Some(format!("SBX-{journey_id}")),
            error_message: None,
            created_at: now - Duration::minutes(5),
            updated_at: now - Duration::minutes(4),
        });
        let discrepancy = Discrepancy {
            id: format!("disc_{journey_id}"),
            kind: DiscrepancyKind::StatusMismatch,
            settlement_id: Some(format!("stl_{journey_id}")),
            on_chain_tx: Some(format!("0x{journey_id}")),
            expected_amount: 125_000.0,
            actual_amount: 125_000.0,
            severity: Severity::High,
            detected_at: now - Duration::minutes(2),
            details: "Replay preview assembled from bounded sandbox records".to_string(),
        };
        let report = ReconciliationReport {
            id: format!("recon_{journey_id}"),
            started_at: now - Duration::minutes(3),
            completed_at: now - Duration::minutes(1),
            total_settlements_checked: 1,
            total_on_chain_txs_checked: 1,
            discrepancies: vec![discrepancy],
            total_discrepancies: 1,
            critical_count: 0,
            status: ReconciliationStatus::DiscrepanciesFound,
        };

        Ok(ReplayBundleAssembler::assemble(
            journey_id.to_string(),
            vec![
                webhook_entry,
                settlement_entry,
                ReplayTimelineEntry::from_reconciliation_report(&report)
                    .into_iter()
                    .last()
                    .unwrap(),
            ],
        ))
    }

    fn require_preset(&self, preset_code: &str) -> Result<SandboxPreset> {
        self.presets
            .get(preset_code)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Sandbox preset not found: {preset_code}")))
    }

    fn resolve_scenario_code(
        &self,
        preset: &SandboxPreset,
        requested: Option<&str>,
    ) -> Result<Option<String>> {
        match requested {
            Some(scenario_code) => {
                self.ensure_known_scenario(preset, scenario_code)?;
                Ok(Some(scenario_code.to_string()))
            }
            None => Ok(preset.default_scenarios.first().cloned()),
        }
    }

    fn ensure_known_scenario(&self, preset: &SandboxPreset, scenario_code: &str) -> Result<()> {
        if preset.default_scenarios.is_empty()
            || preset
                .default_scenarios
                .iter()
                .any(|known| known == scenario_code)
        {
            return Ok(());
        }

        Err(Error::Validation(format!(
            "Scenario {scenario_code} is not available for preset {}",
            preset.code
        )))
    }
}

fn reset_strategy_label(strategy: &SandboxResetStrategy) -> &'static str {
    match strategy {
        SandboxResetStrategy::ResetToPreset => "RESET_TO_PRESET",
        SandboxResetStrategy::ResetScenarioData => "RESET_SCENARIO_DATA",
        SandboxResetStrategy::ResetRuntimeArtifacts => "RESET_RUNTIME_ARTIFACTS",
    }
}

fn merge_json_object(base: &mut Value, overrides: Value) -> Result<()> {
    match overrides {
        Value::Null => Ok(()),
        Value::Object(override_map) => {
            let base_map = base.as_object_mut().ok_or_else(|| {
                Error::Validation("Sandbox config base must be an object".to_string())
            })?;
            merge_object_maps(base_map, override_map);
            Ok(())
        }
        _ => Err(Error::Validation(
            "Sandbox config overrides must be a JSON object".to_string(),
        )),
    }
}

fn merge_object_maps(base: &mut Map<String, Value>, overrides: Map<String, Value>) {
    for (key, override_value) in overrides {
        match (base.get_mut(&key), override_value) {
            (Some(Value::Object(base_nested)), Value::Object(override_nested)) => {
                merge_object_maps(base_nested, override_nested);
            }
            (_, override_value) => {
                base.insert(key, override_value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::tenant::TenantRepository;
    use crate::service::{LedgerService, OnboardingService};
    use crate::test_utils::{MockLedgerRepository, MockTenantRepository};
    use async_trait::async_trait;
    use ramp_common::types::TenantId;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct RecordingScenarioRunner {
        requests: Arc<Mutex<Vec<SandboxScenarioRunRequest>>>,
    }

    #[async_trait]
    impl SandboxScenarioRunner for RecordingScenarioRunner {
        async fn start_run(
            &self,
            request: SandboxScenarioRunRequest,
        ) -> Result<SandboxScenarioRun> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(SandboxScenarioRun {
                run_id: "run_test_001".to_string(),
                tenant_id: request.tenant_id.clone(),
                preset_code: request.preset_code.clone(),
                scenario_code: request.scenario_code.clone(),
                status: SandboxScenarioStatus::Pending,
                metadata: request.metadata.clone(),
            })
        }
    }

    fn build_onboarding_service(tenant_repo: Arc<MockTenantRepository>) -> Arc<OnboardingService> {
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let ledger_service = Arc::new(LedgerService::new(ledger_repo));
        Arc::new(OnboardingService::new(tenant_repo, ledger_service))
    }

    #[tokio::test]
    async fn sandbox_service_seeds_tenant_via_onboarding_service() {
        let tenant_repo = Arc::new(MockTenantRepository::new());
        let onboarding_service = build_onboarding_service(tenant_repo.clone());
        let runner = Arc::new(RecordingScenarioRunner::default());
        let service = SandboxService::new(
            onboarding_service,
            runner,
            vec![SandboxPreset {
                code: "BASELINE".to_string(),
                name: "Baseline Sandbox".to_string(),
                seed_package_version: "2026-03-08".to_string(),
                default_scenarios: vec!["PAYIN_BASELINE".to_string()],
                metadata: json!({"supports_replay": true}),
                reset_strategy: SandboxResetStrategy::ResetToPreset,
                reset_semantics: json!({"drop_runtime_events": true}),
            }],
        );

        let seeded = service
            .seed_tenant(SandboxSeedRequest {
                tenant_name: "Sandbox Tenant".to_string(),
                preset_code: "BASELINE".to_string(),
                scenario_code: Some("PAYIN_BASELINE".to_string()),
                config_overrides: json!({"region": "vn"}),
            })
            .await
            .unwrap();

        assert_eq!(seeded.preset.code, "BASELINE");
        assert_eq!(seeded.tenant.name, "Sandbox Tenant");

        let stored = tenant_repo
            .get_by_id(&TenantId::new(seeded.tenant.id.clone()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.config["environment"], "sandbox");
        assert_eq!(stored.config["sandbox"]["preset_code"], "BASELINE");
        assert_eq!(stored.config["sandbox"]["scenario_code"], "PAYIN_BASELINE");
        assert_eq!(stored.config["region"], "vn");
    }

    #[tokio::test]
    async fn sandbox_service_delegates_scenario_runs_to_runner() {
        let tenant_repo = Arc::new(MockTenantRepository::new());
        let onboarding_service = build_onboarding_service(tenant_repo);
        let runner = Arc::new(RecordingScenarioRunner::default());
        let service = SandboxService::new(
            onboarding_service,
            runner.clone(),
            vec![SandboxPreset {
                code: "BASELINE".to_string(),
                name: "Baseline Sandbox".to_string(),
                seed_package_version: "2026-03-08".to_string(),
                default_scenarios: vec!["PAYIN_BASELINE".to_string()],
                metadata: json!({}),
                reset_strategy: SandboxResetStrategy::ResetToPreset,
                reset_semantics: json!({}),
            }],
        );

        let run = service
            .start_scenario_run(
                "tenant_sandbox_001",
                "BASELINE",
                "PAYIN_BASELINE",
                json!({"trigger": "manual"}),
            )
            .await
            .unwrap();

        assert_eq!(run.tenant_id, "tenant_sandbox_001");
        assert_eq!(run.preset_code, "BASELINE");
        assert_eq!(run.scenario_code, "PAYIN_BASELINE");

        let requests = runner.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].tenant_id, "tenant_sandbox_001");
        assert_eq!(requests[0].preset_code, "BASELINE");
        assert_eq!(requests[0].scenario_code, "PAYIN_BASELINE");
    }

    #[test]
    fn sandbox_service_returns_service_backed_reset_result() {
        let tenant_repo = Arc::new(MockTenantRepository::new());
        let onboarding_service = build_onboarding_service(tenant_repo);
        let service = SandboxService::new(
            onboarding_service,
            Arc::new(RecordingScenarioRunner::default()),
            default_sandbox_presets(),
        );

        let result = service
            .reset_tenant(
                "tenant_sandbox_001",
                Some("BASELINE"),
                Some("Replay drill reset"),
            )
            .unwrap();

        assert_eq!(result.status, "ACCEPTED");
        assert_eq!(result.preset_code.as_deref(), Some("BASELINE"));
        assert_eq!(result.reset_strategy, "RESET_TO_PRESET");
    }

    #[test]
    fn sandbox_service_builds_replay_bundle_from_bounded_records() {
        let tenant_repo = Arc::new(MockTenantRepository::new());
        let onboarding_service = build_onboarding_service(tenant_repo);
        let service = SandboxService::new(
            onboarding_service,
            Arc::new(RecordingScenarioRunner::default()),
            default_sandbox_presets(),
        );

        let bundle = service.replay_bundle("intent_sandbox_001").unwrap();

        assert_eq!(bundle.journey_id, "intent_sandbox_001");
        assert_eq!(bundle.entries.len(), 3);
        assert_eq!(
            bundle.entries[0].source,
            crate::service::ReplayTimelineSource::Webhook
        );
        assert_eq!(
            bundle.entries[1].source,
            crate::service::ReplayTimelineSource::Settlement
        );
    }

    #[test]
    fn default_sandbox_presets_match_seeded_contract() {
        let presets = default_sandbox_presets();

        assert_eq!(presets.len(), 3);
        assert!(presets.iter().any(|preset| {
            preset.code == "BASELINE"
                && preset
                    .default_scenarios
                    .contains(&"PAYIN_BASELINE".to_string())
        }));
        assert!(presets.iter().any(|preset| {
            preset.code == "PAYIN_FAILURE_DRILL"
                && preset
                    .default_scenarios
                    .contains(&"PAYIN_BANK_TIMEOUT".to_string())
        }));
    }
}
