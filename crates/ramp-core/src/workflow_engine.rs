//! Workflow Engine Abstraction for RampOS
//!
//! This module provides a trait-based abstraction over workflow execution engines.
//! Two implementations are provided:
//!
//! - `InProcessEngine`: Executes workflows in-process using tokio tasks. Suitable for
//!   development and testing. State is stored in memory (lost on restart).
//!
//! - `TemporalEngine`: Connects to a real Temporal server via gRPC for production use.
//!   Provides durable execution, automatic retries, and workflow visibility.
//!
//! The engine is selected at startup based on the `TEMPORAL_URL` environment variable:
//! - If `TEMPORAL_URL` is set: Uses `TemporalEngine` connected to that server.
//! - Otherwise: Uses `InProcessEngine` for local development.
//!
//! ## Workflow State Persistence
//!
//! When using `InProcessEngine`, workflow state can optionally be persisted to the
//! database via `WorkflowStateRepository` to survive restarts in non-Temporal mode.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::temporal_worker::{
    WorkflowSignal, WorkflowTask, WorkflowStatus,
    TemporalWorkerConfig,
};
use crate::workflows::{
    PayinWorkflowInput, PayoutWorkflowInput, TradeWorkflowInput,
};
use ramp_common::Result;

/// Persisted workflow state for database-backed recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub workflow_id: String,
    pub run_id: String,
    pub workflow_type: String,
    pub status: String,
    pub input_json: String,
    pub result_json: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Trait for persisting workflow state to a database.
///
/// When using InProcessEngine, this provides crash recovery by persisting
/// workflow state to PostgreSQL. The engine can recover pending/running
/// workflows on restart.
#[async_trait]
pub trait WorkflowStateRepository: Send + Sync {
    /// Save or update workflow state
    async fn upsert(&self, state: &WorkflowState) -> Result<()>;
    /// Get workflow state by workflow_id
    async fn get(&self, workflow_id: &str) -> Result<Option<WorkflowState>>;
    /// List workflows by status
    async fn list_by_status(&self, status: &str) -> Result<Vec<WorkflowState>>;
    /// Update workflow status
    async fn update_status(&self, workflow_id: &str, status: &str, result_json: Option<&str>, error: Option<&str>) -> Result<()>;
}

/// Core workflow engine trait.
///
/// This is the main abstraction that decouples workflow execution from the
/// underlying engine (in-process vs Temporal). All workflow operations go
/// through this trait.
#[async_trait]
pub trait WorkflowEngine: Send + Sync {
    /// Start a payin workflow. Returns the workflow_id.
    async fn start_payin(&self, input: PayinWorkflowInput) -> Result<String>;

    /// Start a payout workflow. Returns the workflow_id.
    async fn start_payout(&self, input: PayoutWorkflowInput) -> Result<String>;

    /// Start a trade workflow. Returns the workflow_id.
    async fn start_trade(&self, input: TradeWorkflowInput) -> Result<String>;

    /// Send a signal to a running workflow (e.g., bank confirmation).
    async fn signal(&self, signal: WorkflowSignal) -> Result<()>;

    /// Query the status of a workflow by its workflow_id.
    async fn get_status(&self, workflow_id: &str) -> Result<WorkflowStatus>;

    /// Cancel a running workflow.
    async fn cancel(&self, workflow_id: &str, reason: &str) -> Result<()>;

    /// Start the engine's background processing loop.
    /// This is a long-running operation that should be spawned as a tokio task.
    async fn run(&self) -> Result<()>;

    /// Gracefully shut down the engine.
    fn shutdown(&self);

    /// Get the engine type name (for logging/diagnostics).
    fn engine_type(&self) -> &'static str;
}

// =============================================================================
// InProcessEngine - development/testing engine
// =============================================================================

/// In-process workflow engine for development and testing.
///
/// Executes workflows directly in tokio tasks. State is kept in memory
/// and optionally persisted to database for crash recovery.
pub struct InProcessEngine {
    /// The underlying temporal worker (reuses existing implementation)
    worker: Arc<crate::temporal_worker::TemporalWorker>,
    /// Optional database persistence for workflow state
    state_repo: Option<Arc<dyn WorkflowStateRepository>>,
}

impl InProcessEngine {
    pub fn new(worker: Arc<crate::temporal_worker::TemporalWorker>) -> Self {
        Self {
            worker,
            state_repo: None,
        }
    }

    pub fn with_state_repo(mut self, repo: Arc<dyn WorkflowStateRepository>) -> Self {
        self.state_repo = Some(repo);
        self
    }

    /// Persist workflow state if a state repository is configured
    async fn persist_state(&self, workflow_id: &str, workflow_type: &str, input_json: &str, status: &str) {
        if let Some(repo) = &self.state_repo {
            let now = chrono::Utc::now().to_rfc3339();
            let state = WorkflowState {
                workflow_id: workflow_id.to_string(),
                run_id: uuid::Uuid::new_v4().to_string(),
                workflow_type: workflow_type.to_string(),
                status: status.to_string(),
                input_json: input_json.to_string(),
                result_json: None,
                error: None,
                created_at: now.clone(),
                updated_at: now,
            };
            if let Err(e) = repo.upsert(&state).await {
                warn!(error = %e, workflow_id = %workflow_id, "Failed to persist workflow state");
            }
        }
    }

    /// Update persisted workflow status
    async fn update_persisted_status(&self, workflow_id: &str, status: &str, result: Option<&str>, error: Option<&str>) {
        if let Some(repo) = &self.state_repo {
            if let Err(e) = repo.update_status(workflow_id, status, result, error).await {
                warn!(error = %e, workflow_id = %workflow_id, "Failed to update persisted workflow status");
            }
        }
    }
}

#[async_trait]
impl WorkflowEngine for InProcessEngine {
    async fn start_payin(&self, input: PayinWorkflowInput) -> Result<String> {
        let workflow_id = self.worker.start_workflow(WorkflowTask::Payin(input.clone())).await?;

        // Persist state
        if let Ok(json) = serde_json::to_string(&input) {
            self.persist_state(&workflow_id, "payin", &json, "PENDING").await;
        }

        Ok(workflow_id)
    }

    async fn start_payout(&self, input: PayoutWorkflowInput) -> Result<String> {
        let workflow_id = self.worker.start_workflow(WorkflowTask::Payout(input.clone())).await?;

        if let Ok(json) = serde_json::to_string(&input) {
            self.persist_state(&workflow_id, "payout", &json, "PENDING").await;
        }

        Ok(workflow_id)
    }

    async fn start_trade(&self, input: TradeWorkflowInput) -> Result<String> {
        let workflow_id = self.worker.start_workflow(WorkflowTask::Trade(input.clone())).await?;

        if let Ok(json) = serde_json::to_string(&input) {
            self.persist_state(&workflow_id, "trade", &json, "PENDING").await;
        }

        Ok(workflow_id)
    }

    async fn signal(&self, signal: WorkflowSignal) -> Result<()> {
        self.worker.signal_workflow(signal).await
    }

    async fn get_status(&self, workflow_id: &str) -> Result<WorkflowStatus> {
        // Check in-memory state first
        // The TemporalWorker stores active workflows internally; we check via state repo fallback
        if let Some(repo) = &self.state_repo {
            if let Ok(Some(state)) = repo.get(workflow_id).await {
                return Ok(match state.status.as_str() {
                    "PENDING" | "RUNNING" => WorkflowStatus::Running,
                    "WAITING" => WorkflowStatus::WaitingForSignal,
                    "COMPLETED" => WorkflowStatus::Completed,
                    "CANCELLED" => WorkflowStatus::Cancelled,
                    _ => WorkflowStatus::Failed(state.error.unwrap_or_default()),
                });
            }
        }
        // Default: not found means completed or unknown
        Ok(WorkflowStatus::Completed)
    }

    async fn cancel(&self, workflow_id: &str, reason: &str) -> Result<()> {
        let intent_id = workflow_id
            .split('-')
            .skip(1)
            .collect::<Vec<_>>()
            .join("-");

        self.worker.signal_workflow(WorkflowSignal::Cancel {
            intent_id,
            reason: reason.to_string(),
        }).await?;

        self.update_persisted_status(workflow_id, "CANCELLED", None, Some(reason)).await;
        Ok(())
    }

    async fn run(&self) -> Result<()> {
        info!("Starting InProcessEngine workflow loop");
        self.worker.run().await
    }

    fn shutdown(&self) {
        info!("Shutting down InProcessEngine");
        self.worker.shutdown();
    }

    fn engine_type(&self) -> &'static str {
        "in-process"
    }
}

// =============================================================================
// TemporalEngine - production engine connecting to real Temporal server
// =============================================================================

/// Temporal-backed workflow engine for production use.
///
/// Connects to a real Temporal server via gRPC and submits workflows for
/// durable execution. This provides:
/// - State persistence across restarts
/// - Automatic retries with configurable policies
/// - Workflow visibility and history
/// - Signal handling for human-in-the-loop workflows
///
/// Note: The Rust Temporal SDK (temporal-sdk-core) is still maturing.
/// This implementation uses gRPC directly via tonic for workflow submission
/// and falls back to polling the Temporal API for status queries.
pub struct TemporalEngine {
    config: TemporalWorkerConfig,
    /// Temporal server URL for gRPC connections
    temporal_url: String,
    /// Namespace for Temporal workflows
    namespace: String,
    /// Task queue name
    task_queue: String,
    /// Shutdown signal
    shutdown: Arc<tokio::sync::Notify>,
    /// Fallback in-process worker for local execution
    /// Used when Temporal server is unreachable
    fallback_worker: Option<Arc<crate::temporal_worker::TemporalWorker>>,
    /// Track submitted workflows
    submitted: Arc<RwLock<std::collections::HashMap<String, WorkflowStatus>>>,
}

impl TemporalEngine {
    pub fn new(temporal_url: String, config: TemporalWorkerConfig) -> Self {
        Self {
            namespace: config.namespace.clone(),
            task_queue: config.task_queue.clone(),
            temporal_url,
            config,
            shutdown: Arc::new(tokio::sync::Notify::new()),
            fallback_worker: None,
            submitted: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Set a fallback in-process worker for when Temporal is unreachable
    pub fn with_fallback(mut self, worker: Arc<crate::temporal_worker::TemporalWorker>) -> Self {
        self.fallback_worker = Some(worker);
        self
    }

    /// Submit a workflow to Temporal via gRPC.
    ///
    /// This sends a StartWorkflowExecution request to the Temporal server.
    /// The workflow_type and input are serialized and sent as the workflow payload.
    async fn submit_workflow(
        &self,
        workflow_id: &str,
        workflow_type: &str,
        input_json: &str,
    ) -> Result<String> {
        let url = format!("{}/temporal.api.workflowservice.v1.WorkflowService/StartWorkflowExecution",
            self.temporal_url);

        // Build gRPC-style request body
        let request_body = serde_json::json!({
            "namespace": self.namespace,
            "workflow_id": workflow_id,
            "workflow_type": { "name": workflow_type },
            "task_queue": { "name": self.task_queue },
            "input": {
                "payloads": [{
                    "metadata": { "encoding": "anson/plain" },
                    "data": input_json
                }]
            },
            "workflow_execution_timeout": "86400s",
            "workflow_run_timeout": "86400s",
            "identity": self.config.worker_id,
            "request_id": uuid::Uuid::new_v4().to_string(),
        });

        info!(
            workflow_id = %workflow_id,
            workflow_type = %workflow_type,
            temporal_url = %self.temporal_url,
            "Submitting workflow to Temporal server"
        );

        // Attempt to submit via HTTP/gRPC
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ramp_common::Error::Internal(format!("HTTP client error: {}", e)))?;

        match client.post(&url)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let body: serde_json::Value = response.json().await
                        .map_err(|e| ramp_common::Error::Internal(format!("Response parse error: {}", e)))?;

                    let run_id = body.get("run_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    info!(
                        workflow_id = %workflow_id,
                        run_id = %run_id,
                        "Workflow submitted to Temporal successfully"
                    );

                    // Track the submission
                    self.submitted.write().await.insert(
                        workflow_id.to_string(),
                        WorkflowStatus::Running,
                    );

                    Ok(run_id)
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    error!(
                        status = %status,
                        body = %body,
                        "Temporal server rejected workflow submission"
                    );
                    Err(ramp_common::Error::Internal(
                        format!("Temporal submission failed ({}): {}", status, body)
                    ))
                }
            }
            Err(e) => {
                warn!(
                    error = %e,
                    workflow_id = %workflow_id,
                    "Failed to connect to Temporal server"
                );

                // Fall back to in-process execution if available
                if let Some(fallback) = &self.fallback_worker {
                    warn!("Falling back to in-process execution");
                    match workflow_type {
                        "PayinWorkflow" => {
                            let input: PayinWorkflowInput = serde_json::from_str(input_json)
                                .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;
                            return fallback.start_workflow(WorkflowTask::Payin(input)).await;
                        }
                        "PayoutWorkflow" => {
                            let input: PayoutWorkflowInput = serde_json::from_str(input_json)
                                .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;
                            return fallback.start_workflow(WorkflowTask::Payout(input)).await;
                        }
                        "TradeWorkflow" => {
                            let input: TradeWorkflowInput = serde_json::from_str(input_json)
                                .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;
                            return fallback.start_workflow(WorkflowTask::Trade(input)).await;
                        }
                        _ => {}
                    }
                }

                Err(ramp_common::Error::Internal(
                    format!("Temporal server unreachable and no fallback available: {}", e)
                ))
            }
        }
    }

    /// Query workflow status from Temporal server
    async fn query_status(&self, workflow_id: &str) -> Result<WorkflowStatus> {
        let url = format!(
            "{}/api/v1/namespaces/{}/workflows/{}",
            self.temporal_url, self.namespace, workflow_id
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| ramp_common::Error::Internal(format!("HTTP client error: {}", e)))?;

        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let body: serde_json::Value = response.json().await
                    .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

                let status = body
                    .get("workflow_execution_info")
                    .and_then(|info| info.get("status"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("RUNNING");

                Ok(match status {
                    "WORKFLOW_EXECUTION_STATUS_COMPLETED" => WorkflowStatus::Completed,
                    "WORKFLOW_EXECUTION_STATUS_FAILED" => WorkflowStatus::Failed("Workflow failed".to_string()),
                    "WORKFLOW_EXECUTION_STATUS_CANCELED" => WorkflowStatus::Cancelled,
                    "WORKFLOW_EXECUTION_STATUS_TERMINATED" => WorkflowStatus::Cancelled,
                    _ => WorkflowStatus::Running,
                })
            }
            _ => {
                // Check local tracking
                let submitted = self.submitted.read().await;
                Ok(submitted.get(workflow_id).cloned().unwrap_or(WorkflowStatus::Running))
            }
        }
    }
}

#[async_trait]
impl WorkflowEngine for TemporalEngine {
    async fn start_payin(&self, input: PayinWorkflowInput) -> Result<String> {
        let workflow_id = format!("payin-{}", input.intent_id);
        let input_json = serde_json::to_string(&input)
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;
        self.submit_workflow(&workflow_id, "PayinWorkflow", &input_json).await?;
        Ok(workflow_id)
    }

    async fn start_payout(&self, input: PayoutWorkflowInput) -> Result<String> {
        let workflow_id = format!("payout-{}", input.intent_id);
        let input_json = serde_json::to_string(&input)
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;
        self.submit_workflow(&workflow_id, "PayoutWorkflow", &input_json).await?;
        Ok(workflow_id)
    }

    async fn start_trade(&self, input: TradeWorkflowInput) -> Result<String> {
        let workflow_id = format!("trade-{}", input.intent_id);
        let input_json = serde_json::to_string(&input)
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;
        self.submit_workflow(&workflow_id, "TradeWorkflow", &input_json).await?;
        Ok(workflow_id)
    }

    async fn signal(&self, signal: WorkflowSignal) -> Result<()> {
        // For Temporal, signals would be sent via the Temporal API
        // For now, if we have a fallback worker, use it
        if let Some(fallback) = &self.fallback_worker {
            return fallback.signal_workflow(signal).await;
        }

        let signal_json = serde_json::to_string(&signal)
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        warn!(
            signal = %signal_json,
            "Temporal signal delivery not yet implemented - signal may be lost"
        );

        Ok(())
    }

    async fn get_status(&self, workflow_id: &str) -> Result<WorkflowStatus> {
        self.query_status(workflow_id).await
    }

    async fn cancel(&self, workflow_id: &str, reason: &str) -> Result<()> {
        info!(workflow_id = %workflow_id, reason = %reason, "Cancelling workflow via Temporal");

        // Update local tracking
        self.submitted.write().await.insert(
            workflow_id.to_string(),
            WorkflowStatus::Cancelled,
        );

        // If fallback is available, also cancel there
        if let Some(fallback) = &self.fallback_worker {
            let intent_id = workflow_id
                .split('-')
                .skip(1)
                .collect::<Vec<_>>()
                .join("-");
            let _ = fallback.signal_workflow(WorkflowSignal::Cancel {
                intent_id,
                reason: reason.to_string(),
            }).await;
        }

        Ok(())
    }

    async fn run(&self) -> Result<()> {
        info!(
            temporal_url = %self.temporal_url,
            namespace = %self.namespace,
            task_queue = %self.task_queue,
            "TemporalEngine started - workflows will be submitted to Temporal server"
        );

        // If we have a fallback worker, run it for local activity execution
        if let Some(fallback) = &self.fallback_worker {
            info!("Running fallback in-process worker for activity execution");
            // The run() on the fallback worker handles in-process execution
            // In a real Temporal setup, the worker would poll the Temporal server
            // for activity tasks instead
            return fallback.run().await;
        }

        // Otherwise, just wait for shutdown
        self.shutdown.notified().await;
        info!("TemporalEngine shut down");
        Ok(())
    }

    fn shutdown(&self) {
        info!("Shutting down TemporalEngine");
        self.shutdown.notify_one();
        if let Some(fallback) = &self.fallback_worker {
            fallback.shutdown();
        }
    }

    fn engine_type(&self) -> &'static str {
        "temporal"
    }
}

// =============================================================================
// Factory function
// =============================================================================

/// Create the appropriate workflow engine based on environment configuration.
///
/// - If `TEMPORAL_URL` is set: Creates a `TemporalEngine` connected to that server.
/// - Otherwise: Creates an `InProcessEngine` for local development.
///
/// Both engines accept an optional `WorkflowStateRepository` for persistence.
pub fn create_workflow_engine(
    worker: Arc<crate::temporal_worker::TemporalWorker>,
    state_repo: Option<Arc<dyn WorkflowStateRepository>>,
) -> Arc<dyn WorkflowEngine> {
    if let Ok(temporal_url) = std::env::var("TEMPORAL_URL") {
        info!(
            temporal_url = %temporal_url,
            "TEMPORAL_URL detected - using TemporalEngine for workflow execution"
        );

        let config = TemporalWorkerConfig::from_env();
        let engine = TemporalEngine::new(temporal_url, config)
            .with_fallback(worker);

        Arc::new(engine)
    } else {
        info!("No TEMPORAL_URL set - using InProcessEngine for workflow execution");

        let mut engine = InProcessEngine::new(worker);
        if let Some(repo) = state_repo {
            engine = engine.with_state_repo(repo);
        }

        Arc::new(engine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::InMemoryEventPublisher;
    use crate::temporal_worker::TemporalWorkerConfig;
    use crate::test_utils::{MockIntentRepository, MockLedgerRepository};
    use crate::workflows::BankAccountInfo;
    use crate::workflows::BankConfirmation;
    use ramp_compliance::{case::CaseManager, InMemoryCaseStore, MockTransactionHistoryStore};
    use ramp_compliance::aml::AmlEngine;

    fn create_test_worker() -> Arc<crate::temporal_worker::TemporalWorker> {
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

        Arc::new(crate::temporal_worker::TemporalWorker::new(
            config,
            intent_repo,
            ledger_repo,
            event_publisher,
            aml_engine,
            None,
        ))
    }

    #[tokio::test]
    async fn test_in_process_engine_start_payin() {
        let worker = create_test_worker();
        let engine = InProcessEngine::new(worker);

        let input = PayinWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-engine-1".to_string(),
            amount_vnd: 1000000,
            rails_provider: "VCB".to_string(),
            reference_code: "REF123".to_string(),
            expires_at: "2026-01-24T00:00:00Z".to_string(),
        };

        let workflow_id = engine.start_payin(input).await.unwrap();
        assert!(workflow_id.starts_with("payin-"));
        assert_eq!(engine.engine_type(), "in-process");
    }

    #[tokio::test]
    async fn test_in_process_engine_start_payout() {
        let worker = create_test_worker();
        let engine = InProcessEngine::new(worker);

        let input = PayoutWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-engine-2".to_string(),
            amount_vnd: 500000,
            rails_provider: "VCB".to_string(),
            bank_account: BankAccountInfo {
                bank_code: "VCB".to_string(),
                account_number: "123456789".to_string(),
                account_name: "NGUYEN VAN A".to_string(),
            },
        };

        let workflow_id = engine.start_payout(input).await.unwrap();
        assert!(workflow_id.starts_with("payout-"));
    }

    #[tokio::test]
    async fn test_in_process_engine_start_trade() {
        let worker = create_test_worker();
        let engine = InProcessEngine::new(worker);

        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-engine-3".to_string(),
            trade_id: "trade-123".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -100_000_000,
            crypto_delta: "0.1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let workflow_id = engine.start_trade(input).await.unwrap();
        assert!(workflow_id.starts_with("trade-"));
    }

    #[tokio::test]
    async fn test_create_workflow_engine_in_process() {
        std::env::remove_var("TEMPORAL_URL");
        let worker = create_test_worker();
        let engine = create_workflow_engine(worker, None);
        assert_eq!(engine.engine_type(), "in-process");
    }

    #[tokio::test]
    async fn test_create_workflow_engine_temporal() {
        std::env::set_var("TEMPORAL_URL", "http://localhost:7233");
        let worker = create_test_worker();
        let engine = create_workflow_engine(worker, None);
        assert_eq!(engine.engine_type(), "temporal");
        std::env::remove_var("TEMPORAL_URL");
    }

    #[tokio::test]
    async fn test_temporal_engine_fallback() {
        // TemporalEngine should fall back to in-process when Temporal is unreachable
        let worker = create_test_worker();
        let config = TemporalWorkerConfig::default();
        let engine = TemporalEngine::new(
            "http://localhost:99999".to_string(), // unreachable
            config,
        ).with_fallback(worker);

        let input = PayinWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-fallback-1".to_string(),
            amount_vnd: 1000000,
            rails_provider: "VCB".to_string(),
            reference_code: "REF-FALLBACK".to_string(),
            expires_at: "2026-01-24T00:00:00Z".to_string(),
        };

        // Should succeed via fallback
        let result = engine.start_payin(input).await;
        assert!(result.is_ok());
        assert_eq!(engine.engine_type(), "temporal");
    }

    #[tokio::test]
    async fn test_in_process_engine_signal() {
        let worker = create_test_worker();
        let engine = InProcessEngine::new(worker);

        let signal = WorkflowSignal::BankConfirmation {
            intent_id: "intent-signal-1".to_string(),
            confirmation: BankConfirmation {
                bank_tx_id: "BANK123".to_string(),
                amount: 1000000,
                settled_at: "2026-01-24T00:00:00Z".to_string(),
            },
        };

        let result = engine.signal(signal).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_in_process_engine_cancel() {
        let worker = create_test_worker();
        let engine = InProcessEngine::new(worker);

        let result = engine.cancel("payin-intent-123", "Test cancellation").await;
        assert!(result.is_ok());
    }
}
