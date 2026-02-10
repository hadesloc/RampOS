//! Execution Engine
//!
//! Executes an ExecutionPlan step-by-step with:
//! - State machine tracking (Created -> Planning -> Executing -> Completed/Failed)
//! - Failure handling with rollback/retry
//! - Progress event emission

use super::spec::{ExecutionPlan, ExecutionStepKind, IntentSpec};
use super::solver::IntentSolver;
use chrono::{DateTime, Utc};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// State of an intent execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    /// Intent created, not yet planned
    Created,
    /// Solver is computing the execution plan
    Planning,
    /// Plan computed, waiting for user approval
    Planned,
    /// Executing steps
    Executing,
    /// All steps completed successfully
    Completed,
    /// Execution failed
    Failed,
    /// Rollback in progress
    RollingBack,
    /// Rollback completed
    RolledBack,
    /// Intent cancelled by user
    Cancelled,
}

impl ExecutionState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ExecutionState::Completed
                | ExecutionState::Failed
                | ExecutionState::RolledBack
                | ExecutionState::Cancelled
        )
    }

    pub fn can_transition_to(&self, next: ExecutionState) -> bool {
        matches!(
            (self, next),
            (ExecutionState::Created, ExecutionState::Planning)
                | (ExecutionState::Planning, ExecutionState::Planned)
                | (ExecutionState::Planning, ExecutionState::Failed)
                | (ExecutionState::Planned, ExecutionState::Executing)
                | (ExecutionState::Planned, ExecutionState::Cancelled)
                | (ExecutionState::Executing, ExecutionState::Completed)
                | (ExecutionState::Executing, ExecutionState::Failed)
                | (ExecutionState::Failed, ExecutionState::RollingBack)
                | (ExecutionState::Failed, ExecutionState::Executing) // retry
                | (ExecutionState::RollingBack, ExecutionState::RolledBack)
                | (ExecutionState::RollingBack, ExecutionState::Failed)
        )
    }
}

/// Status of a single step in execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepExecutionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
    RolledBack,
}

/// Record of a step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRecord {
    /// Step index
    pub index: u32,
    /// Step kind
    pub kind: ExecutionStepKind,
    /// Execution status
    pub status: StepExecutionStatus,
    /// Transaction hash (if applicable)
    pub tx_hash: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Started at
    pub started_at: Option<DateTime<Utc>>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
    /// Gas used
    pub gas_used: Option<u64>,
}

/// Progress event emitted during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Execution ID
    pub execution_id: String,
    /// Event type
    pub event_type: ProgressEventType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional data
    pub data: serde_json::Value,
}

/// Types of progress events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressEventType {
    ExecutionCreated,
    PlanningStarted,
    PlanReady,
    StepStarted { step_index: u32 },
    StepCompleted { step_index: u32 },
    StepFailed { step_index: u32 },
    ExecutionCompleted,
    ExecutionFailed,
    RollbackStarted,
    RollbackCompleted,
}

/// Full execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentExecution {
    /// Unique execution ID
    pub id: String,
    /// The intent being executed
    pub spec: IntentSpec,
    /// Current state
    pub state: ExecutionState,
    /// Execution plan (available after Planning state)
    pub plan: Option<ExecutionPlan>,
    /// Step records
    pub steps: Vec<StepRecord>,
    /// Progress events
    pub events: Vec<ProgressEvent>,
    /// Current step index
    pub current_step: u32,
    /// Retry count
    pub retry_count: u32,
    /// Maximum retries
    pub max_retries: u32,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

impl IntentExecution {
    pub fn new(spec: IntentSpec) -> Self {
        let id = format!("exec_{}", Uuid::now_v7());
        let now = Utc::now();

        let mut execution = Self {
            id: id.clone(),
            spec,
            state: ExecutionState::Created,
            plan: None,
            steps: Vec::new(),
            events: Vec::new(),
            current_step: 0,
            retry_count: 0,
            max_retries: 3,
            created_at: now,
            updated_at: now,
            completed_at: None,
        };

        execution.emit_event(ProgressEventType::ExecutionCreated, serde_json::json!({}));
        execution
    }

    /// Transition to a new state
    pub fn transition(&mut self, new_state: ExecutionState) -> Result<()> {
        if !self.state.can_transition_to(new_state) {
            return Err(Error::Validation(format!(
                "Invalid state transition: {:?} -> {:?}",
                self.state, new_state
            )));
        }

        self.state = new_state;
        self.updated_at = Utc::now();

        if new_state.is_terminal() {
            self.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Set the execution plan and populate step records
    pub fn set_plan(&mut self, plan: ExecutionPlan) {
        self.steps = plan
            .steps
            .iter()
            .map(|s| StepRecord {
                index: s.index,
                kind: s.kind.clone(),
                status: StepExecutionStatus::Pending,
                tx_hash: None,
                error: None,
                started_at: None,
                completed_at: None,
                gas_used: None,
            })
            .collect();

        self.plan = Some(plan);
    }

    /// Mark the current step as started
    pub fn start_current_step(&mut self) -> bool {
        let idx = self.current_step as usize;
        if idx < self.steps.len() {
            self.steps[idx].status = StepExecutionStatus::InProgress;
            self.steps[idx].started_at = Some(Utc::now());
            let step_kind_str = format!("{}", self.steps[idx].kind);
            self.emit_event(
                ProgressEventType::StepStarted { step_index: self.current_step },
                serde_json::json!({ "step_kind": step_kind_str }),
            );
            true
        } else {
            false
        }
    }

    /// Mark the current step as completed and advance
    pub fn complete_current_step(&mut self, tx_hash: Option<String>, gas_used: Option<u64>) {
        let idx = self.current_step as usize;
        if idx < self.steps.len() {
            let step = &mut self.steps[idx];
            step.status = StepExecutionStatus::Completed;
            step.completed_at = Some(Utc::now());
            step.tx_hash = tx_hash;
            step.gas_used = gas_used;

            self.emit_event(
                ProgressEventType::StepCompleted { step_index: self.current_step },
                serde_json::json!({}),
            );

            self.current_step += 1;
            self.updated_at = Utc::now();
        }
    }

    /// Mark the current step as failed
    pub fn fail_current_step(&mut self, error: &str) {
        let idx = self.current_step as usize;
        if idx < self.steps.len() {
            self.steps[idx].status = StepExecutionStatus::Failed;
            self.steps[idx].error = Some(error.to_string());
            self.steps[idx].completed_at = Some(Utc::now());

            self.emit_event(
                ProgressEventType::StepFailed { step_index: self.current_step },
                serde_json::json!({ "error": error }),
            );
        }
    }

    /// Check if all steps are completed
    pub fn all_steps_completed(&self) -> bool {
        self.steps.iter().all(|s| {
            matches!(
                s.status,
                StepExecutionStatus::Completed | StepExecutionStatus::Skipped
            )
        })
    }

    /// Check if execution can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && self.state == ExecutionState::Failed
    }

    /// Get the number of completed steps
    pub fn completed_steps(&self) -> usize {
        self.steps.iter().filter(|s| s.status == StepExecutionStatus::Completed).count()
    }

    /// Get progress as a percentage
    pub fn progress_pct(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.0;
        }
        (self.completed_steps() as f64 / self.steps.len() as f64) * 100.0
    }

    /// Emit a progress event
    fn emit_event(&mut self, event_type: ProgressEventType, data: serde_json::Value) {
        self.events.push(ProgressEvent {
            execution_id: self.id.clone(),
            event_type,
            timestamp: Utc::now(),
            data,
        });
    }
}

/// Execution engine - manages intent execution lifecycle
pub struct ExecutionEngine {
    solver: Arc<dyn IntentSolver>,
    executions: RwLock<HashMap<String, IntentExecution>>,
    auto_rollback: bool,
}

impl ExecutionEngine {
    pub fn new(solver: Arc<dyn IntentSolver>) -> Self {
        Self {
            solver,
            executions: RwLock::new(HashMap::new()),
            auto_rollback: true,
        }
    }

    pub fn with_auto_rollback(mut self, enabled: bool) -> Self {
        self.auto_rollback = enabled;
        self
    }

    /// Submit an intent for execution
    pub async fn submit(&self, spec: IntentSpec) -> Result<IntentExecution> {
        // Validate
        spec.validate().map_err(|e| Error::Validation(e))?;

        let mut execution = IntentExecution::new(spec.clone());
        let exec_id = execution.id.clone();

        info!(execution_id = %exec_id, intent_id = %spec.id, "Intent submitted");

        // Transition to Planning
        execution.transition(ExecutionState::Planning)?;
        execution.emit_event(ProgressEventType::PlanningStarted, serde_json::json!({}));

        // Solve
        match self.solver.solve(&spec).await {
            Ok(plan) => {
                execution.set_plan(plan);
                execution.transition(ExecutionState::Planned)?;
                execution.emit_event(ProgressEventType::PlanReady, serde_json::json!({}));
            }
            Err(e) => {
                error!(execution_id = %exec_id, error = %e, "Planning failed");
                execution.transition(ExecutionState::Failed)?;
                execution.emit_event(
                    ProgressEventType::ExecutionFailed,
                    serde_json::json!({ "error": e.to_string() }),
                );
            }
        }

        // Store execution
        {
            let mut executions = self.executions.write().await;
            executions.insert(exec_id.clone(), execution.clone());
        }

        Ok(execution)
    }

    /// Execute a planned intent (after user approval)
    pub async fn execute(&self, execution_id: &str) -> Result<IntentExecution> {
        let mut execution = {
            let executions = self.executions.read().await;
            executions
                .get(execution_id)
                .cloned()
                .ok_or_else(|| Error::NotFound(format!("Execution {} not found", execution_id)))?
        };

        if execution.state != ExecutionState::Planned {
            return Err(Error::Validation(format!(
                "Execution must be in Planned state, currently: {:?}",
                execution.state
            )));
        }

        execution.transition(ExecutionState::Executing)?;

        info!(
            execution_id = %execution.id,
            steps = execution.steps.len(),
            "Starting execution"
        );

        // Execute steps sequentially
        while (execution.current_step as usize) < execution.steps.len() {
            let step_idx = execution.current_step;

            execution.start_current_step();

            // Simulate step execution
            // In production, this would call actual on-chain operations
            match self.execute_step(&execution.steps[step_idx as usize].kind).await {
                Ok((tx_hash, gas)) => {
                    execution.complete_current_step(tx_hash, gas);
                }
                Err(e) => {
                    error!(
                        execution_id = %execution.id,
                        step = step_idx,
                        error = %e,
                        "Step execution failed"
                    );
                    execution.fail_current_step(&e.to_string());
                    execution.transition(ExecutionState::Failed)?;
                    execution.emit_event(
                        ProgressEventType::ExecutionFailed,
                        serde_json::json!({ "error": e.to_string(), "step": step_idx }),
                    );

                    if self.auto_rollback {
                        self.rollback_execution(&mut execution).await?;
                    }

                    // Store and return
                    let mut executions = self.executions.write().await;
                    executions.insert(execution.id.clone(), execution.clone());
                    return Ok(execution);
                }
            }

            // Update stored execution after each step
            {
                let mut executions = self.executions.write().await;
                executions.insert(execution.id.clone(), execution.clone());
            }
        }

        // All steps completed
        execution.transition(ExecutionState::Completed)?;
        execution.emit_event(ProgressEventType::ExecutionCompleted, serde_json::json!({}));

        info!(
            execution_id = %execution.id,
            "Execution completed successfully"
        );

        // Final store
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution.id.clone(), execution.clone());
        }

        Ok(execution)
    }

    /// Get execution status
    pub async fn get_execution(&self, execution_id: &str) -> Result<Option<IntentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions.get(execution_id).cloned())
    }

    /// Cancel an execution (only if in Planned state)
    pub async fn cancel(&self, execution_id: &str) -> Result<IntentExecution> {
        let mut executions = self.executions.write().await;
        let execution = executions
            .get_mut(execution_id)
            .ok_or_else(|| Error::NotFound(format!("Execution {} not found", execution_id)))?;

        execution.transition(ExecutionState::Cancelled)?;
        Ok(execution.clone())
    }

    /// Retry a failed execution
    pub async fn retry(&self, execution_id: &str) -> Result<IntentExecution> {
        let mut execution = {
            let executions = self.executions.read().await;
            executions
                .get(execution_id)
                .cloned()
                .ok_or_else(|| Error::NotFound(format!("Execution {} not found", execution_id)))?
        };

        if !execution.can_retry() {
            return Err(Error::Validation("Execution cannot be retried".to_string()));
        }

        execution.retry_count += 1;
        execution.transition(ExecutionState::Executing)?;

        // Reset failed steps to pending
        for step in execution.steps.iter_mut() {
            if step.status == StepExecutionStatus::Failed {
                step.status = StepExecutionStatus::Pending;
                step.error = None;
                step.started_at = None;
                step.completed_at = None;
            }
        }

        // Store and re-execute
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution.id.clone(), execution.clone());
        }

        self.execute(&execution.id).await
    }

    /// Execute a single step (mock implementation)
    async fn execute_step(
        &self,
        kind: &ExecutionStepKind,
    ) -> Result<(Option<String>, Option<u64>)> {
        // In production, this would interact with actual blockchain/DEX/bridge APIs
        match kind {
            ExecutionStepKind::Approve { .. } => {
                // Mock: generate random tx hash
                let tx_hash = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
                Ok((Some(tx_hash), Some(50_000)))
            }
            ExecutionStepKind::Swap { .. } => {
                let tx_hash = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
                Ok((Some(tx_hash), Some(200_000)))
            }
            ExecutionStepKind::Bridge { .. } => {
                let tx_hash = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
                Ok((Some(tx_hash), Some(250_000)))
            }
            ExecutionStepKind::Transfer { .. } => {
                let tx_hash = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
                Ok((Some(tx_hash), Some(65_000)))
            }
            ExecutionStepKind::Stake { .. } => {
                let tx_hash = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
                Ok((Some(tx_hash), Some(150_000)))
            }
            ExecutionStepKind::WaitForBridge { .. } => {
                // No tx hash for waiting
                Ok((None, None))
            }
        }
    }

    /// Rollback completed steps in reverse order
    async fn rollback_execution(&self, execution: &mut IntentExecution) -> Result<()> {
        execution.transition(ExecutionState::RollingBack)?;
        execution.emit_event(ProgressEventType::RollbackStarted, serde_json::json!({}));

        warn!(
            execution_id = %execution.id,
            "Starting rollback"
        );

        // Mark completed steps as rolled back (in reverse)
        for step in execution.steps.iter_mut().rev() {
            if step.status == StepExecutionStatus::Completed {
                step.status = StepExecutionStatus::RolledBack;
            }
        }

        execution.transition(ExecutionState::RolledBack)?;
        execution.emit_event(ProgressEventType::RollbackCompleted, serde_json::json!({}));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intents::spec::{AssetId, IntentAction};
    use crate::intents::solver::LocalSolver;

    fn make_test_spec() -> IntentSpec {
        IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "1000000",
        )
    }

    #[test]
    fn test_execution_state_transitions() {
        assert!(ExecutionState::Created.can_transition_to(ExecutionState::Planning));
        assert!(ExecutionState::Planning.can_transition_to(ExecutionState::Planned));
        assert!(ExecutionState::Planned.can_transition_to(ExecutionState::Executing));
        assert!(ExecutionState::Executing.can_transition_to(ExecutionState::Completed));
        assert!(ExecutionState::Executing.can_transition_to(ExecutionState::Failed));
        assert!(ExecutionState::Failed.can_transition_to(ExecutionState::RollingBack));
        assert!(ExecutionState::RollingBack.can_transition_to(ExecutionState::RolledBack));

        // Invalid transitions
        assert!(!ExecutionState::Created.can_transition_to(ExecutionState::Completed));
        assert!(!ExecutionState::Completed.can_transition_to(ExecutionState::Executing));
        assert!(!ExecutionState::Cancelled.can_transition_to(ExecutionState::Executing));
    }

    #[test]
    fn test_execution_state_terminal() {
        assert!(ExecutionState::Completed.is_terminal());
        assert!(ExecutionState::Failed.is_terminal());
        assert!(ExecutionState::RolledBack.is_terminal());
        assert!(ExecutionState::Cancelled.is_terminal());

        assert!(!ExecutionState::Created.is_terminal());
        assert!(!ExecutionState::Planning.is_terminal());
        assert!(!ExecutionState::Executing.is_terminal());
    }

    #[test]
    fn test_intent_execution_creation() {
        let spec = make_test_spec();
        let execution = IntentExecution::new(spec);

        assert!(execution.id.starts_with("exec_"));
        assert_eq!(execution.state, ExecutionState::Created);
        assert!(execution.plan.is_none());
        assert!(execution.steps.is_empty());
        assert_eq!(execution.current_step, 0);
        assert_eq!(execution.retry_count, 0);
        // Should have one event: ExecutionCreated
        assert_eq!(execution.events.len(), 1);
    }

    #[test]
    fn test_intent_execution_transition() {
        let spec = make_test_spec();
        let mut execution = IntentExecution::new(spec);

        assert!(execution.transition(ExecutionState::Planning).is_ok());
        assert_eq!(execution.state, ExecutionState::Planning);

        assert!(execution.transition(ExecutionState::Planned).is_ok());
        assert_eq!(execution.state, ExecutionState::Planned);

        // Invalid transition
        let result = execution.transition(ExecutionState::Created);
        assert!(result.is_err());
    }

    #[test]
    fn test_intent_execution_progress() {
        let spec = make_test_spec();
        let execution = IntentExecution::new(spec);

        assert_eq!(execution.progress_pct(), 0.0);
        assert_eq!(execution.completed_steps(), 0);
    }

    #[test]
    fn test_step_record_lifecycle() {
        let spec = make_test_spec();
        let mut execution = IntentExecution::new(spec);

        // Manually add steps
        execution.steps.push(StepRecord {
            index: 0,
            kind: ExecutionStepKind::Approve {
                token: "USDC".to_string(),
                spender: "Router".to_string(),
                chain_id: 1,
            },
            status: StepExecutionStatus::Pending,
            tx_hash: None,
            error: None,
            started_at: None,
            completed_at: None,
            gas_used: None,
        });
        execution.steps.push(StepRecord {
            index: 1,
            kind: ExecutionStepKind::Swap {
                from_token: "USDC".to_string(),
                to_token: "USDT".to_string(),
                chain_id: 1,
                aggregator: None,
            },
            status: StepExecutionStatus::Pending,
            tx_hash: None,
            error: None,
            started_at: None,
            completed_at: None,
            gas_used: None,
        });

        // Start step 0
        execution.start_current_step();
        assert_eq!(execution.steps[0].status, StepExecutionStatus::InProgress);

        // Complete step 0
        execution.complete_current_step(Some("0xabc".to_string()), Some(50000));
        assert_eq!(execution.steps[0].status, StepExecutionStatus::Completed);
        assert_eq!(execution.current_step, 1);
        assert_eq!(execution.completed_steps(), 1);
        assert!((execution.progress_pct() - 50.0).abs() < 0.1);

        // Complete step 1
        execution.start_current_step();
        execution.complete_current_step(Some("0xdef".to_string()), Some(200000));
        assert_eq!(execution.completed_steps(), 2);
        assert!(execution.all_steps_completed());
        assert!((execution.progress_pct() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_step_failure() {
        let spec = make_test_spec();
        let mut execution = IntentExecution::new(spec);

        execution.steps.push(StepRecord {
            index: 0,
            kind: ExecutionStepKind::Swap {
                from_token: "USDC".to_string(),
                to_token: "USDT".to_string(),
                chain_id: 1,
                aggregator: None,
            },
            status: StepExecutionStatus::Pending,
            tx_hash: None,
            error: None,
            started_at: None,
            completed_at: None,
            gas_used: None,
        });

        execution.start_current_step();
        execution.fail_current_step("Insufficient liquidity");

        assert_eq!(execution.steps[0].status, StepExecutionStatus::Failed);
        assert_eq!(execution.steps[0].error.as_deref(), Some("Insufficient liquidity"));
    }

    #[test]
    fn test_can_retry() {
        let spec = make_test_spec();
        let mut execution = IntentExecution::new(spec);

        // Not failed, can't retry
        assert!(!execution.can_retry());

        execution.state = ExecutionState::Failed;
        assert!(execution.can_retry());

        execution.retry_count = 3;
        assert!(!execution.can_retry());
    }

    #[tokio::test]
    async fn test_engine_submit() {
        let solver = Arc::new(LocalSolver::new());
        let engine = ExecutionEngine::new(solver);

        let spec = make_test_spec();
        let execution = engine.submit(spec).await.unwrap();

        assert_eq!(execution.state, ExecutionState::Planned);
        assert!(execution.plan.is_some());
        assert!(!execution.steps.is_empty());
    }

    #[tokio::test]
    async fn test_engine_submit_and_execute() {
        let solver = Arc::new(LocalSolver::new());
        let engine = ExecutionEngine::new(solver);

        let spec = make_test_spec();
        let submitted = engine.submit(spec).await.unwrap();
        let exec_id = submitted.id.clone();

        let result = engine.execute(&exec_id).await.unwrap();
        assert_eq!(result.state, ExecutionState::Completed);
        assert!(result.all_steps_completed());
    }

    #[tokio::test]
    async fn test_engine_get_execution() {
        let solver = Arc::new(LocalSolver::new());
        let engine = ExecutionEngine::new(solver);

        let spec = make_test_spec();
        let submitted = engine.submit(spec).await.unwrap();

        let fetched = engine.get_execution(&submitted.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, submitted.id);

        let missing = engine.get_execution("nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_engine_cancel() {
        let solver = Arc::new(LocalSolver::new());
        let engine = ExecutionEngine::new(solver);

        let spec = make_test_spec();
        let submitted = engine.submit(spec).await.unwrap();

        let cancelled = engine.cancel(&submitted.id).await.unwrap();
        assert_eq!(cancelled.state, ExecutionState::Cancelled);
    }

    #[tokio::test]
    async fn test_engine_submit_invalid_spec() {
        let solver = Arc::new(LocalSolver::new());
        let engine = ExecutionEngine::new(solver);

        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "0", // invalid
        );

        let result = engine.submit(spec).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_engine_execute_not_planned() {
        let solver = Arc::new(LocalSolver::new());
        let engine = ExecutionEngine::new(solver);

        let result = engine.execute("nonexistent").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_progress_events_tracked() {
        let spec = make_test_spec();
        let execution = IntentExecution::new(spec);

        // Created event
        assert!(!execution.events.is_empty());
        match &execution.events[0].event_type {
            ProgressEventType::ExecutionCreated => {}
            _ => panic!("Expected ExecutionCreated event"),
        }
    }
}
