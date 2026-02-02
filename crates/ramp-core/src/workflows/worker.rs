//! Temporal Worker Implementation for RampOS
//!
//! This module provides the Temporal worker that executes workflows for:
//! - PayinWorkflow: Handles VND pay-in flow
//! - PayoutWorkflow: Handles VND pay-out flow
//! - TradeWorkflow: Handles trade execution flow
//!
//! ## Configuration
//!
//! The worker can run in two modes controlled by the `TEMPORAL_MODE` environment variable:
//! - `simulation` (default): Uses in-process task queues for development/testing
//! - `production`: Connects to a real Temporal server
//!
//! ## Environment Variables
//!
//! - `TEMPORAL_MODE`: "simulation" or "production" (default: simulation)
//! - `TEMPORAL_SERVER_URL`: Temporal server address (default: http://localhost:7233)
//! - `TEMPORAL_NAMESPACE`: Workflow namespace (default: rampos)
//! - `TEMPORAL_TASK_QUEUE`: Task queue name (default: rampos-workflows)
//! - `TEMPORAL_WORKER_ID`: Unique worker identifier

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn, instrument};
use serde::{Deserialize, Serialize};

use crate::workflows::{
    PayinWorkflowInput, PayinWorkflowResult, BankConfirmation,
    PayoutWorkflowInput, PayoutWorkflowResult, SettlementResult,
    TradeWorkflowInput, TradeWorkflowResult,
    WorkflowConfig, RetryPolicy,
    payin_activities, trade_activities,
};
use crate::workflows::payout::PayoutWorkflow;
use crate::workflows::trade::TradeWorkflow;
use crate::workflows::payin::PayinWorkflow;
use crate::repository::intent::IntentRepository;
use crate::repository::ledger::LedgerRepository;
use crate::event::EventPublisher;
use ramp_common::{Result, Error};
use ramp_common::types::*;
use ramp_compliance::aml::AmlEngine;

/// Temporal execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TemporalMode {
    /// Simulation mode - uses in-process task queues
    #[default]
    Simulation,
    /// Production mode - connects to real Temporal server
    Production,
}

impl TemporalMode {
    /// Parse from environment variable
    pub fn from_env() -> Self {
        match std::env::var("TEMPORAL_MODE").unwrap_or_default().to_lowercase().as_str() {
            "production" | "prod" => TemporalMode::Production,
            _ => TemporalMode::Simulation,
        }
    }

    /// Check if in production mode
    pub fn is_production(&self) -> bool {
        matches!(self, TemporalMode::Production)
    }

    /// Check if in simulation mode
    pub fn is_simulation(&self) -> bool {
        matches!(self, TemporalMode::Simulation)
    }
}

/// Temporal worker configuration
#[derive(Debug, Clone)]
pub struct TemporalWorkerConfig {
    /// Temporal execution mode
    pub mode: TemporalMode,
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
    /// Activity retry policy
    pub retry_policy: RetryPolicy,
}

impl Default for TemporalWorkerConfig {
    fn default() -> Self {
        Self {
            mode: TemporalMode::Simulation,
            server_url: "http://localhost:7233".to_string(),
            namespace: "rampos".to_string(),
            task_queue: "rampos-workflows".to_string(),
            worker_id: format!("worker-{}", uuid::Uuid::new_v4()),
            max_concurrent_workflows: 100,
            max_concurrent_activities: 200,
            poll_interval: Duration::from_millis(100),
            retry_policy: RetryPolicy::default(),
        }
    }
}

impl TemporalWorkerConfig {
    pub fn from_env() -> Self {
        Self {
            mode: TemporalMode::from_env(),
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
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Create a config for testing
    pub fn for_testing() -> Self {
        Self {
            mode: TemporalMode::Simulation,
            ..Default::default()
        }
    }

    /// Create a config for production
    pub fn for_production(server_url: impl Into<String>) -> Self {
        Self {
            mode: TemporalMode::Production,
            server_url: server_url.into(),
            ..Default::default()
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
    sanctions_service: Option<Arc<ramp_compliance::sanctions::SanctionsScreeningService>>,
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
        sanctions_service: Option<Arc<ramp_compliance::sanctions::SanctionsScreeningService>>,
    ) -> Self {
        info!(
            mode = ?config.mode,
            task_queue = %config.task_queue,
            "Creating Temporal worker"
        );

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

    /// Get the current execution mode
    pub fn mode(&self) -> TemporalMode {
        self.config.mode
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

        if self.config.mode.is_production() {
            // In production mode, we would submit to real Temporal
            // For now, log a warning that production mode is not fully implemented
            warn!(
                workflow_id = %workflow_id,
                "Production mode: Would submit to Temporal server at {}",
                self.config.server_url
            );

            // TODO: Integrate with temporal-sdk-core
            // let client = TemporalClient::connect(&self.config.server_url).await?;
            // client.start_workflow(workflow_id, task).await?;
        }

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
            mode = ?self.config.mode,
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
            mode = ?self.config.mode,
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

    /// Get count of pending workflows
    pub async fn pending_count(&self) -> usize {
        self.pending_tasks.read().await.len()
    }

    /// Get count of active workflows
    pub async fn active_count(&self) -> usize {
        self.active_workflows.read().await.len()
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
    sanctions_service: Option<Arc<ramp_compliance::sanctions::SanctionsScreeningService>>,
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

        // Execute the workflow with retry logic
        let result = self.execute_with_retry(task.task.clone()).await;

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

    /// Execute workflow with retry logic based on retry policy
    async fn execute_with_retry(&self, task: WorkflowTask) -> Result<WorkflowResult> {
        let mut attempts = 0;
        let max_attempts = self.config.retry_policy.maximum_attempts;
        let mut delay = self.config.retry_policy.initial_interval;

        loop {
            attempts += 1;

            match self.execute_workflow(task.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempts >= max_attempts {
                        error!(
                            attempts = attempts,
                            max_attempts = max_attempts,
                            error = %e,
                            "Workflow failed after max retries"
                        );
                        return Err(e);
                    }

                    warn!(
                        attempt = attempts,
                        max_attempts = max_attempts,
                        error = %e,
                        delay_ms = delay.as_millis(),
                        "Workflow failed, retrying"
                    );

                    tokio::time::sleep(delay).await;

                    // Apply exponential backoff
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * self.config.retry_policy.backoff_coefficient)
                            .min(self.config.retry_policy.maximum_interval.as_secs_f64())
                    );
                }
            }
        }
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
        info!("Executing payin workflow");

        let workflow = PayinWorkflow::new(self.intent_repo.clone());
        let this = self.clone();

        // Execute workflow with signal provider
        let result = workflow.execute(input, move |intent_id, timeout| {
            let this = this.clone();
            async move {
                this.wait_for_bank_signal(&intent_id, timeout).await
            }
        }).await.map_err(|e| Error::Internal(e))?;

        Ok(result)
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
        let workflow = TradeWorkflow::new(self.intent_repo.clone());
        let result = workflow.execute(input).await.map_err(|e| Error::Internal(e))?;
        Ok(result)
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

    /// Get the execution mode
    pub fn mode(&self) -> TemporalMode {
        self.worker.mode()
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
                settled_amount: None,
            },
        }).await
    }

    /// Cancel a workflow
    pub async fn cancel_workflow(&self, intent_id: String, reason: String) -> Result<()> {
        self.worker.signal_workflow(WorkflowSignal::Cancel {
            intent_id,
            reason,
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
    async fn test_temporal_mode_from_env() {
        std::env::set_var("TEMPORAL_MODE", "production");
        assert_eq!(TemporalMode::from_env(), TemporalMode::Production);

        std::env::set_var("TEMPORAL_MODE", "simulation");
        assert_eq!(TemporalMode::from_env(), TemporalMode::Simulation);

        std::env::set_var("TEMPORAL_MODE", "invalid");
        assert_eq!(TemporalMode::from_env(), TemporalMode::Simulation);

        std::env::remove_var("TEMPORAL_MODE");
        assert_eq!(TemporalMode::from_env(), TemporalMode::Simulation);
    }

    #[tokio::test]
    async fn test_start_payin_workflow() {
        let config = TemporalWorkerConfig::for_testing();
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

        assert!(worker.mode().is_simulation());

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
        let config = TemporalWorkerConfig::for_testing();
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
    async fn test_start_trade_workflow() {
        let config = TemporalWorkerConfig::for_testing();
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

        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-1".to_string(),
            trade_id: "trade-123".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -100_000_000,
            crypto_delta: "0.1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let workflow_id = client.start_trade(input).await.unwrap();
        assert!(workflow_id.starts_with("trade-"));
    }

    #[tokio::test]
    async fn test_workflow_config_from_env() {
        std::env::set_var("TEMPORAL_SERVER_URL", "http://test:7233");
        std::env::set_var("TEMPORAL_NAMESPACE", "test-ns");
        std::env::set_var("TEMPORAL_MODE", "production");

        let config = TemporalWorkerConfig::from_env();

        assert_eq!(config.server_url, "http://test:7233");
        assert_eq!(config.namespace, "test-ns");
        assert!(config.mode.is_production());

        std::env::remove_var("TEMPORAL_SERVER_URL");
        std::env::remove_var("TEMPORAL_NAMESPACE");
        std::env::remove_var("TEMPORAL_MODE");
    }

    #[tokio::test]
    async fn test_pending_and_active_counts() {
        let config = TemporalWorkerConfig::for_testing();
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

        let worker = TemporalWorker::new(
            config,
            intent_repo,
            ledger_repo,
            event_publisher,
            aml_engine,
            None,
        );

        assert_eq!(worker.pending_count().await, 0);
        assert_eq!(worker.active_count().await, 0);

        // Start a workflow
        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-count-test".to_string(),
            trade_id: "trade-count".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -100_000_000,
            crypto_delta: "0.1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        worker.start_workflow(WorkflowTask::Trade(input)).await.unwrap();

        assert_eq!(worker.pending_count().await, 1);
    }
}
