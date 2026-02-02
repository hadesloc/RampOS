//! Temporal Worker Implementation for RampOS
//!
//! This module provides the Temporal worker that executes workflows for:
//! - PayinWorkflow: Handles VND pay-in flow
//! - PayoutWorkflow: Handles VND pay-out flow
//! - TradeWorkflow: Handles trade execution flow
//!
//! The worker connects to a Temporal server and polls for workflow tasks,
//! executing the appropriate activities based on the workflow type.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn, instrument, Instrument};
use serde::{Deserialize, Serialize};

use crate::workflows::{
    PayinWorkflowInput, PayinWorkflowResult, BankConfirmation,
    PayoutWorkflowInput, PayoutWorkflowResult, SettlementResult,
    TradeWorkflowInput, TradeWorkflowResult,
    WorkflowConfig, RetryPolicy,
    payin_activities, trade_activities, // payout_activities removed
};
use crate::workflows::payout::PayoutWorkflow;
use crate::repository::intent::IntentRepository;
use crate::repository::ledger::LedgerRepository;
use crate::event::EventPublisher;
use ramp_common::{Result, Error};
use ramp_common::types::*;
use ramp_compliance::aml::AmlEngine;
use ramp_compliance::sanctions::SanctionsScreeningService;

/// Temporal worker configuration
#[derive(Debug, Clone)]
pub struct TemporalWorkerConfig {
    /// Temporal server address
    pub server_url: String,
    /// Namespace for workflows
    pub namespace: String,
    /// Task queue to poll
    pub task_queue: String,
    /// Worker identity (unique per worker instance)
    pub worker_id: String,
    /// Maximum concurrent workflow tasks
    pub max_concurrent_workflows: usize,
    /// Maximum concurrent activity tasks
    pub max_concurrent_activities: usize,
    /// Workflow poll interval
    pub poll_interval: Duration,
}

impl Default for TemporalWorkerConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:7233".to_string(),
            namespace: "rampos".to_string(),
            task_queue: "rampos-workflows".to_string(),
            worker_id: format!("worker-{}", uuid::Uuid::new_v4()),
            max_concurrent_workflows: 100,
            max_concurrent_activities: 200,
            poll_interval: Duration::from_millis(100),
        }
    }
}

impl TemporalWorkerConfig {
    pub fn from_env() -> Self {
        Self {
            server_url: std::env::var("TEMPORAL_SERVER_URL")
                .unwrap_or_else(|_| "http://localhost:7233".to_string()),
            namespace: std::env::var("TEMPORAL_NAMESPACE")
                .unwrap_or_else(|_| "rampos".to_string()),
            task_queue: std::env::var("TEMPORAL_TASK_QUEUE")
                .unwrap_or_else(|_| "rampos-workflows".to_string()),
            worker_id: std::env::var("TEMPORAL_WORKER_ID")
                .unwrap_or_else(|_| format!("worker-{}", uuid::Uuid::new_v4())),
            max_concurrent_workflows: std::env::var("TEMPORAL_MAX_WORKFLOWS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            max_concurrent_activities: std::env::var("TEMPORAL_MAX_ACTIVITIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(200),
            poll_interval: Duration::from_millis(100),
        }
    }
}

/// Workflow task types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowTask {
    Payin(PayinWorkflowInput),
    Payout(PayoutWorkflowInput),
    Trade(TradeWorkflowInput),
}

/// Workflow execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowResult {
    Payin(PayinWorkflowResult),
    Payout(PayoutWorkflowResult),
    Trade(TradeWorkflowResult),
}

/// Signal types for workflow communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "signal_type")]
pub enum WorkflowSignal {
    /// Bank confirmation received for payin
    BankConfirmation {
        intent_id: String,
        confirmation: BankConfirmation,
    },
    /// Settlement confirmation for payout
    SettlementConfirmation {
        bank_tx_id: String,
        result: SettlementResult,
    },
    /// Cancel workflow
    Cancel {
        intent_id: String,
        reason: String,
    },
}

/// Temporal worker service
///
/// This is a simplified implementation that simulates Temporal behavior
/// using in-process task queues. For production, integrate with actual
/// Temporal SDK (temporal-sdk-core).
pub struct TemporalWorker {
    config: TemporalWorkerConfig,
    intent_repo: Arc<dyn IntentRepository>,
    ledger_repo: Arc<dyn LedgerRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    aml_engine: Arc<AmlEngine>,
    sanctions_service: Option<Arc<SanctionsScreeningService>>,
    /// Pending workflow tasks (simulated queue)
    pending_tasks: Arc<RwLock<Vec<PendingWorkflow>>>,
    /// Active workflows
    active_workflows: Arc<RwLock<std::collections::HashMap<String, ActiveWorkflow>>>,
    /// Signal buffer
    signals: Arc<RwLock<Vec<WorkflowSignal>>>,
    /// Shutdown flag
    shutdown: Arc<tokio::sync::Notify>,
}

#[derive(Debug, Clone)]
struct PendingWorkflow {
    workflow_id: String,
    run_id: String,
    task: WorkflowTask,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct ActiveWorkflow {
    workflow_id: String,
    run_id: String,
    task: WorkflowTask,
    status: WorkflowStatus,
    started_at: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowStatus {
    Running,
    WaitingForSignal,
    Completed,
    Failed(String),
    Cancelled,
}

impl TemporalWorker {
    pub fn new(
        config: TemporalWorkerConfig,
        intent_repo: Arc<dyn IntentRepository>,
        ledger_repo: Arc<dyn LedgerRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        aml_engine: Arc<AmlEngine>,
        sanctions_service: Option<Arc<SanctionsScreeningService>>,
    ) -> Self {
        Self {
            config,
            intent_repo,
            ledger_repo,
            event_publisher,
            aml_engine,
            sanctions_service,
            pending_tasks: Arc::new(RwLock::new(Vec::new())),
            active_workflows: Arc::new(RwLock::new(std::collections::HashMap::new())),
            signals: Arc::new(RwLock::new(Vec::new())),
            shutdown: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Start the workflow from input
    pub async fn start_workflow(&self, task: WorkflowTask) -> Result<String> {
        let workflow_id = match &task {
            WorkflowTask::Payin(input) => format!("payin-{}", input.intent_id),
            WorkflowTask::Payout(input) => format!("payout-{}", input.intent_id),
            WorkflowTask::Trade(input) => format!("trade-{}", input.intent_id),
        };

        let run_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let pending = PendingWorkflow {
            workflow_id: workflow_id.clone(),
            run_id,
            task,
            created_at: now,
        };

        self.pending_tasks.write().await.push(pending);

        info!(
            workflow_id = %workflow_id,
            task_queue = %self.config.task_queue,
            "Workflow scheduled"
        );

        Ok(workflow_id)
    }

    /// Send a signal to a workflow
    pub async fn signal_workflow(&self, signal: WorkflowSignal) -> Result<()> {
        self.signals.write().await.push(signal);
        Ok(())
    }

    /// Run the worker loop
    pub async fn run(&self) -> Result<()> {
        info!(
            worker_id = %self.config.worker_id,
            task_queue = %self.config.task_queue,
            "Starting Temporal worker"
        );

        let worker = self.clone_refs();

        loop {
            tokio::select! {
                _ = self.shutdown.notified() => {
                    info!("Temporal worker shutting down");
                    break;
                }
                _ = tokio::time::sleep(self.config.poll_interval) => {
                    if let Err(e) = worker.poll_and_execute().await {
                        error!(error = %e, "Error in workflow execution");
                    }
                }
            }
        }

        Ok(())
    }

    /// Graceful shutdown
    pub fn shutdown(&self) {
        self.shutdown.notify_one();
    }

    fn clone_refs(&self) -> TemporalWorkerRefs {
        TemporalWorkerRefs {
            config: self.config.clone(),
            intent_repo: self.intent_repo.clone(),
            ledger_repo: self.ledger_repo.clone(),
            event_publisher: self.event_publisher.clone(),
            aml_engine: self.aml_engine.clone(),
            sanctions_service: self.sanctions_service.clone(),
            pending_tasks: self.pending_tasks.clone(),
            active_workflows: self.active_workflows.clone(),
            signals: self.signals.clone(),
        }
    }
}

/// Reference struct for worker operations
#[derive(Clone)]
struct TemporalWorkerRefs {
    config: TemporalWorkerConfig,
    intent_repo: Arc<dyn IntentRepository>,
    ledger_repo: Arc<dyn LedgerRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    aml_engine: Arc<AmlEngine>,
    sanctions_service: Option<Arc<SanctionsScreeningService>>,
    pending_tasks: Arc<RwLock<Vec<PendingWorkflow>>>,
    active_workflows: Arc<RwLock<std::collections::HashMap<String, ActiveWorkflow>>>,
    signals: Arc<RwLock<Vec<WorkflowSignal>>>,
}

impl TemporalWorkerRefs {
    #[instrument(skip(self), level = "debug")]
    async fn poll_and_execute(&self) -> Result<()> {
        // Check for pending tasks
        let task = {
            let mut pending = self.pending_tasks.write().await;
            if pending.is_empty() {
                return Ok(());
            }
            pending.remove(0)
        };

        let workflow_id = task.workflow_id.clone();
        let run_id = task.run_id.clone();
        let now = chrono::Utc::now();

        // Add to active workflows
        {
            let mut active = self.active_workflows.write().await;
            active.insert(workflow_id.clone(), ActiveWorkflow {
                workflow_id: workflow_id.clone(),
                run_id: run_id.clone(),
                task: task.task.clone(),
                status: WorkflowStatus::Running,
                started_at: now,
                last_activity: now,
            });
        }

        // Execute the workflow
        let result = self.execute_workflow(task.task.clone()).await;

        // Update workflow status
        {
            let mut active = self.active_workflows.write().await;
            if let Some(wf) = active.get_mut(&workflow_id) {
                wf.status = match &result {
                    Ok(_) => WorkflowStatus::Completed,
                    Err(e) => WorkflowStatus::Failed(e.to_string()),
                };
                wf.last_activity = chrono::Utc::now();
            }
        }

        match result {
            Ok(wf_result) => {
                info!(
                    workflow_id = %workflow_id,
                    "Workflow completed successfully"
                );
            }
            Err(e) => {
                error!(
                    workflow_id = %workflow_id,
                    error = %e,
                    "Workflow failed"
                );
            }
        }

        Ok(())
    }

    async fn execute_workflow(&self, task: WorkflowTask) -> Result<WorkflowResult> {
        match task {
            WorkflowTask::Payin(input) => {
                let result = self.execute_payin_workflow(input).await?;
                Ok(WorkflowResult::Payin(result))
            }
            WorkflowTask::Payout(input) => {
                let result = self.execute_payout_workflow(input).await?;
                Ok(WorkflowResult::Payout(result))
            }
            WorkflowTask::Trade(input) => {
                let result = self.execute_trade_workflow(input).await?;
                Ok(WorkflowResult::Trade(result))
            }
        }
    }

    #[instrument(skip(self), fields(intent_id = %input.intent_id))]
    async fn execute_payin_workflow(&self, input: PayinWorkflowInput) -> Result<PayinWorkflowResult> {
        info!("Starting payin workflow");

        // Activity 1: Issue payment instruction
        let reference = match payin_activities::issue_instruction(&input).await {
            Ok(ref_code) => ref_code,
            Err(e) => {
                return Ok(PayinWorkflowResult {
                    intent_id: input.intent_id,
                    status: "FAILED".to_string(),
                    bank_tx_id: None,
                    completed_at: None,
                });
            }
        };

        // Update intent state
        let intent_id = IntentId(input.intent_id.clone());
        let tenant_id = TenantId::new(input.tenant_id.clone());
        self.intent_repo
            .update_state(&tenant_id, &intent_id, "INSTRUCTION_ISSUED")
            .await?;

        // Activity 2: Wait for bank confirmation (with timeout)
        let timeout = Duration::from_secs(24 * 60 * 60); // 24 hours
        let confirmation = self.wait_for_bank_signal(&input.intent_id, timeout).await;

        match confirmation {
            Some(conf) => {
                // Activity 3: Credit VND balance
                if let Err(e) = payin_activities::credit_vnd_balance(
                    &input.tenant_id,
                    &input.user_id,
                    &input.intent_id,
                    conf.amount,
                ).await {
                    error!(error = %e, "Failed to credit balance");
                    return Ok(PayinWorkflowResult {
                        intent_id: input.intent_id,
                        status: "FAILED".to_string(),
                        bank_tx_id: Some(conf.bank_tx_id),
                        completed_at: None,
                    });
                }

                // Update intent to completed
                self.intent_repo
                    .update_state(&tenant_id, &intent_id, "COMPLETED")
                    .await?;

                // Activity 4: Send webhook
                let _ = payin_activities::send_webhook(
                    &input.tenant_id,
                    "intent.completed",
                    serde_json::json!({
                        "intent_id": input.intent_id,
                        "status": "COMPLETED",
                        "bank_tx_id": conf.bank_tx_id,
                    }),
                ).await;

                Ok(PayinWorkflowResult {
                    intent_id: input.intent_id,
                    status: "COMPLETED".to_string(),
                    bank_tx_id: Some(conf.bank_tx_id),
                    completed_at: Some(chrono::Utc::now().to_rfc3339()),
                })
            }
            None => {
                // Timeout - mark as expired
                self.intent_repo
                    .update_state(&tenant_id, &intent_id, "EXPIRED")
                    .await?;

                Ok(PayinWorkflowResult {
                    intent_id: input.intent_id,
                    status: "EXPIRED".to_string(),
                    bank_tx_id: None,
                    completed_at: None,
                })
            }
        }
    }

    #[instrument(skip(self), fields(intent_id = %input.intent_id))]
    async fn execute_payout_workflow(&self, input: PayoutWorkflowInput) -> Result<PayoutWorkflowResult> {
        let workflow = PayoutWorkflow::new(
            self.intent_repo.clone(),
            self.ledger_repo.clone(),
            self.event_publisher.clone(),
            self.aml_engine.clone(),
            self.sanctions_service.clone(),
        );

        let this = self.clone();

        let result = workflow.run(input, move |bank_tx_id, timeout| {
            let this = this.clone();
            async move {
                this.wait_for_settlement_signal(&bank_tx_id, timeout).await
            }
        }).await?;

        Ok(result)
    }

    #[instrument(skip(self), fields(intent_id = %input.intent_id, trade_id = %input.trade_id))]
    async fn execute_trade_workflow(&self, input: TradeWorkflowInput) -> Result<TradeWorkflowResult> {
        info!("Starting trade workflow");

        let intent_id = IntentId(input.intent_id.clone());
        let tenant_id = TenantId::new(input.tenant_id.clone());

        // Activity 1: Post-trade compliance check
        let compliance_ok = match trade_activities::run_post_trade_check(&input).await {
            Ok(ok) => ok,
            Err(e) => {
                error!(error = %e, "Post-trade compliance check failed");
                false
            }
        };

        let mut compliance_hold = false;

        if !compliance_ok {
            // Flag for review but don't block
            let case_id = trade_activities::flag_for_review(
                &input.intent_id,
                "Large trade requiring review",
            ).await?;

            compliance_hold = true;
            warn!(case_id = %case_id, "Trade flagged for compliance review");
        }

        // Activity 2: Settle in ledger
        if let Err(e) = trade_activities::settle_in_ledger(&input).await {
            error!(error = %e, "Failed to settle trade in ledger");

            self.intent_repo
                .update_state(&tenant_id, &intent_id, "SETTLEMENT_FAILED")
                .await?;

            return Ok(TradeWorkflowResult {
                intent_id: input.intent_id,
                status: "FAILED".to_string(),
                completed_at: None,
                compliance_hold,
            });
        }

        // Update to completed
        self.intent_repo
            .update_state(&tenant_id, &intent_id, "COMPLETED")
            .await?;

        Ok(TradeWorkflowResult {
            intent_id: input.intent_id,
            status: if compliance_hold { "COMPLETED_WITH_HOLD" } else { "COMPLETED" }.to_string(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            compliance_hold,
        })
    }

    /// Wait for bank confirmation signal
    async fn wait_for_bank_signal(
        &self,
        intent_id: &str,
        timeout: Duration,
    ) -> Option<BankConfirmation> {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            if tokio::time::Instant::now() >= deadline {
                return None;
            }

            // Check for matching signal
            {
                let mut signals = self.signals.write().await;
                let pos = signals.iter().position(|s| {
                    matches!(s, WorkflowSignal::BankConfirmation { intent_id: id, .. } if id == intent_id)
                });

                if let Some(idx) = pos {
                    if let WorkflowSignal::BankConfirmation { confirmation, .. } = signals.remove(idx) {
                        return Some(confirmation);
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// Wait for settlement confirmation signal
    async fn wait_for_settlement_signal(
        &self,
        bank_tx_id: &str,
        timeout: Duration,
    ) -> Option<SettlementResult> {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            if tokio::time::Instant::now() >= deadline {
                return None;
            }

            // Check for matching signal
            {
                let mut signals = self.signals.write().await;
                let pos = signals.iter().position(|s| {
                    matches!(s, WorkflowSignal::SettlementConfirmation { bank_tx_id: id, .. } if id == bank_tx_id)
                });

                if let Some(idx) = pos {
                    if let WorkflowSignal::SettlementConfirmation { result, .. } = signals.remove(idx) {
                        return Some(result);
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

/// Workflow client for starting and managing workflows
pub struct WorkflowClient {
    worker: Arc<TemporalWorker>,
}

impl WorkflowClient {
    pub fn new(worker: Arc<TemporalWorker>) -> Self {
        Self { worker }
    }

    /// Start a payin workflow
    pub async fn start_payin(&self, input: PayinWorkflowInput) -> Result<String> {
        self.worker.start_workflow(WorkflowTask::Payin(input)).await
    }

    /// Start a payout workflow
    pub async fn start_payout(&self, input: PayoutWorkflowInput) -> Result<String> {
        self.worker.start_workflow(WorkflowTask::Payout(input)).await
    }

    /// Start a trade workflow
    pub async fn start_trade(&self, input: TradeWorkflowInput) -> Result<String> {
        self.worker.start_workflow(WorkflowTask::Trade(input)).await
    }

    /// Signal bank confirmation
    pub async fn signal_bank_confirmation(
        &self,
        intent_id: String,
        bank_tx_id: String,
        amount: i64,
        settled_at: String,
    ) -> Result<()> {
        self.worker.signal_workflow(WorkflowSignal::BankConfirmation {
            intent_id,
            confirmation: BankConfirmation {
                bank_tx_id,
                amount,
                settled_at,
            },
        }).await
    }

    /// Signal settlement result
    pub async fn signal_settlement(
        &self,
        bank_tx_id: String,
        success: bool,
        settled_at: Option<String>,
        rejection_reason: Option<String>,
    ) -> Result<()> {
        self.worker.signal_workflow(WorkflowSignal::SettlementConfirmation {
            bank_tx_id: bank_tx_id.clone(),
            result: SettlementResult {
                success,
                bank_tx_id,
                settled_at,
                rejection_reason,
            },
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{MockIntentRepository, MockLedgerRepository};
    use crate::event::InMemoryEventPublisher;
    use crate::workflows::BankAccountInfo;
    use ramp_compliance::{
        case::CaseManager,
        InMemoryCaseStore,
        MockTransactionHistoryStore,
    };

    #[tokio::test]
    async fn test_start_payin_workflow() {
        let config = TemporalWorkerConfig::default();
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let case_store = Arc::new(InMemoryCaseStore::new());
        let aml_engine = Arc::new(AmlEngine::new(
            Arc::new(CaseManager::new(case_store)),
            None,
            Arc::new(ramp_compliance::aml::MockDeviceHistoryStore::new()),
            Arc::new(MockTransactionHistoryStore::new()),
        ));

        let worker = Arc::new(TemporalWorker::new(
            config,
            intent_repo,
            ledger_repo,
            event_publisher,
            aml_engine,
            None,
        ));

        let client = WorkflowClient::new(worker);

        let input = PayinWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-123".to_string(),
            amount_vnd: 1000000,
            rails_provider: "VCB".to_string(),
            reference_code: "REF123".to_string(),
            expires_at: "2026-01-24T00:00:00Z".to_string(),
        };

        let workflow_id = client.start_payin(input).await.unwrap();
        assert!(workflow_id.starts_with("payin-"));
    }

    #[tokio::test]
    async fn test_start_payout_workflow() {
        let config = TemporalWorkerConfig::default();
        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let case_store = Arc::new(InMemoryCaseStore::new());
        let aml_engine = Arc::new(AmlEngine::new(
            Arc::new(CaseManager::new(case_store)),
            None,
            Arc::new(ramp_compliance::aml::MockDeviceHistoryStore::new()),
            Arc::new(MockTransactionHistoryStore::new()),
        ));

        let worker = Arc::new(TemporalWorker::new(
            config,
            intent_repo,
            ledger_repo,
            event_publisher,
            aml_engine,
            None,
        ));

        let client = WorkflowClient::new(worker);

        let input = PayoutWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-456".to_string(),
            amount_vnd: 500000,
            rails_provider: "VCB".to_string(),
            bank_account: BankAccountInfo {
                bank_code: "VCB".to_string(),
                account_number: "123456789".to_string(),
                account_name: "NGUYEN VAN A".to_string(),
            },
        };

        let workflow_id = client.start_payout(input).await.unwrap();
        assert!(workflow_id.starts_with("payout-"));
    }

    #[tokio::test]
    async fn test_workflow_config_from_env() {
        std::env::set_var("TEMPORAL_SERVER_URL", "http://test:7233");
        std::env::set_var("TEMPORAL_NAMESPACE", "test-ns");

        let config = TemporalWorkerConfig::from_env();

        assert_eq!(config.server_url, "http://test:7233");
        assert_eq!(config.namespace, "test-ns");

        std::env::remove_var("TEMPORAL_SERVER_URL");
        std::env::remove_var("TEMPORAL_NAMESPACE");
    }
}
