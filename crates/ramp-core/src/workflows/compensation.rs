//! Compensation (Saga) Pattern for Workflow Rollback
//!
//! This module implements the compensation pattern for handling workflow failures.
//! When a workflow step fails, we need to undo (compensate) any previously completed
//! steps to maintain data consistency.
//!
//! ## Example Usage
//! ```ignore
//! let mut chain = CompensationChain::new("payin-workflow");
//!
//! // Step 1: Credit balance
//! credit_balance().await?;
//! chain.add(CompensationAction::new(
//!     "credit_balance",
//!     Box::pin(async move { reverse_credit().await }),
//! ));
//!
//! // Step 2: This fails
//! if step2().await.is_err() {
//!     chain.compensate().await; // Runs reverse_credit
//!     return Err(...);
//! }
//! ```

use std::future::Future;
use std::pin::Pin;
use tracing::{error, info, instrument, warn};

/// A compensation action that can be executed to undo a workflow step
pub struct CompensationAction {
    /// Name of the step being compensated
    pub name: String,
    /// The compensation function to execute
    pub compensate_fn: Option<Pin<Box<dyn Future<Output = Result<(), String>> + Send>>>,
    /// Whether this compensation has been executed
    pub executed: bool,
}

impl std::fmt::Debug for CompensationAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompensationAction")
            .field("name", &self.name)
            .field("executed", &self.executed)
            .finish()
    }
}

impl CompensationAction {
    /// Create a new compensation action
    pub fn new<F>(name: impl Into<String>, compensate_fn: F) -> Self
    where
        F: Future<Output = Result<(), String>> + Send + 'static,
    {
        Self {
            name: name.into(),
            compensate_fn: Some(Box::pin(compensate_fn)),
            executed: false,
        }
    }

    /// Create a no-op compensation action (for steps that don't need rollback)
    pub fn noop(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            compensate_fn: None,
            executed: false,
        }
    }

    /// Execute the compensation
    pub async fn execute(&mut self) -> Result<(), String> {
        if self.executed {
            warn!(name = %self.name, "Compensation already executed, skipping");
            return Ok(());
        }

        if let Some(fut) = self.compensate_fn.take() {
            info!(name = %self.name, "Executing compensation");
            let result = fut.await;
            self.executed = true;

            if let Err(ref e) = result {
                error!(name = %self.name, error = %e, "Compensation failed");
            } else {
                info!(name = %self.name, "Compensation succeeded");
            }

            result
        } else {
            info!(name = %self.name, "No compensation action needed");
            self.executed = true;
            Ok(())
        }
    }
}

/// A chain of compensation actions that can be rolled back in reverse order
#[derive(Debug)]
pub struct CompensationChain {
    /// Name of the workflow this chain belongs to
    workflow_name: String,
    /// Stack of compensation actions (LIFO)
    actions: Vec<CompensationAction>,
    /// Whether the chain has been compensated
    compensated: bool,
}

impl CompensationChain {
    /// Create a new compensation chain
    pub fn new(workflow_name: impl Into<String>) -> Self {
        Self {
            workflow_name: workflow_name.into(),
            actions: Vec::new(),
            compensated: false,
        }
    }

    /// Add a compensation action to the chain
    pub fn add(&mut self, action: CompensationAction) {
        self.actions.push(action);
    }

    /// Get the number of actions in the chain
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Execute all compensation actions in reverse order (LIFO)
    #[instrument(skip(self), fields(workflow = %self.workflow_name))]
    pub async fn compensate(&mut self) -> CompensationResult {
        if self.compensated {
            warn!("Compensation chain already executed");
            return CompensationResult {
                total: 0,
                succeeded: 0,
                failed: 0,
                errors: vec![],
            };
        }

        info!(steps = self.actions.len(), "Starting compensation rollback");

        let mut result = CompensationResult {
            total: self.actions.len(),
            succeeded: 0,
            failed: 0,
            errors: vec![],
        };

        // Execute in reverse order (LIFO)
        while let Some(mut action) = self.actions.pop() {
            match action.execute().await {
                Ok(()) => result.succeeded += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push((action.name.clone(), e));
                }
            }
        }

        self.compensated = true;

        info!(
            succeeded = result.succeeded,
            failed = result.failed,
            "Compensation rollback completed"
        );

        result
    }

    /// Mark the chain as complete (no compensation needed)
    ///
    /// Call this when the workflow completes successfully to prevent
    /// accidental compensation.
    pub fn complete(&mut self) {
        self.actions.clear();
        self.compensated = true;
    }
}

/// Result of a compensation rollback
#[derive(Debug)]
pub struct CompensationResult {
    /// Total number of compensation actions
    pub total: usize,
    /// Number of successful compensations
    pub succeeded: usize,
    /// Number of failed compensations
    pub failed: usize,
    /// List of errors (action name, error message)
    pub errors: Vec<(String, String)>,
}

impl CompensationResult {
    /// Check if all compensations succeeded
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }

    /// Get a summary message
    pub fn summary(&self) -> String {
        if self.is_success() {
            format!("All {} compensations succeeded", self.succeeded)
        } else {
            format!(
                "{}/{} compensations succeeded, {} failed",
                self.succeeded, self.total, self.failed
            )
        }
    }
}

/// Builder for creating compensation-aware workflow steps
pub struct CompensatedStep<T> {
    /// The main action result
    result: Option<T>,
    /// The compensation action if the step succeeded
    compensation: Option<CompensationAction>,
}

impl<T> Default for CompensatedStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CompensatedStep<T> {
    /// Create a new compensated step
    pub fn new() -> Self {
        Self {
            result: None,
            compensation: None,
        }
    }

    /// Execute the step and prepare compensation
    pub async fn execute<F, C, CF>(
        &mut self,
        action: F,
        compensation_name: impl Into<String>,
        compensation_fn: C,
    ) -> Result<&T, String>
    where
        F: Future<Output = Result<T, String>>,
        C: FnOnce(T) -> CF,
        CF: Future<Output = Result<(), String>> + Send + 'static,
        T: Clone,
    {
        let result = action.await?;
        let comp = compensation_fn(result.clone());
        self.compensation = Some(CompensationAction::new(compensation_name, comp));
        self.result = Some(result);
        Ok(self.result.as_ref().unwrap())
    }

    /// Take the compensation action (to add to a chain)
    pub fn take_compensation(&mut self) -> Option<CompensationAction> {
        self.compensation.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_compensation_chain_rollback() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut chain = CompensationChain::new("test-workflow");

        // Add three compensation actions
        let c1 = counter.clone();
        chain.add(CompensationAction::new("step1", async move {
            c1.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));

        let c2 = counter.clone();
        chain.add(CompensationAction::new("step2", async move {
            c2.fetch_add(10, Ordering::SeqCst);
            Ok(())
        }));

        let c3 = counter.clone();
        chain.add(CompensationAction::new("step3", async move {
            c3.fetch_add(100, Ordering::SeqCst);
            Ok(())
        }));

        assert_eq!(chain.len(), 3);

        // Compensate
        let result = chain.compensate().await;

        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 3);
        assert_eq!(result.failed, 0);
        assert!(result.is_success());

        // Counter should be 111 (1 + 10 + 100)
        // Order doesn't matter for the sum, but LIFO would execute 3, 2, 1
        assert_eq!(counter.load(Ordering::SeqCst), 111);
    }

    #[tokio::test]
    async fn test_compensation_chain_with_failure() {
        let mut chain = CompensationChain::new("test-workflow");

        chain.add(CompensationAction::new("step1", async { Ok(()) }));

        chain.add(CompensationAction::new("step2", async {
            Err("Compensation failed".to_string())
        }));

        chain.add(CompensationAction::new("step3", async { Ok(()) }));

        let result = chain.compensate().await;

        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 1);
        assert!(!result.is_success());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].0, "step2");
    }

    #[tokio::test]
    async fn test_compensation_noop() {
        let mut action = CompensationAction::noop("no-op-step");
        let result = action.execute().await;
        assert!(result.is_ok());
        assert!(action.executed);
    }

    #[tokio::test]
    async fn test_compensation_chain_complete() {
        let mut chain = CompensationChain::new("test-workflow");

        chain.add(CompensationAction::noop("step1"));
        chain.add(CompensationAction::noop("step2"));

        assert_eq!(chain.len(), 2);

        chain.complete();

        assert!(chain.is_empty());
        assert!(chain.compensated);
    }

    #[tokio::test]
    async fn test_double_compensation_is_noop() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut chain = CompensationChain::new("test-workflow");

        let c1 = counter.clone();
        chain.add(CompensationAction::new("step1", async move {
            c1.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));

        // First compensation
        let result1 = chain.compensate().await;
        assert_eq!(result1.succeeded, 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Second compensation should be a no-op
        let result2 = chain.compensate().await;
        assert_eq!(result2.total, 0);
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Still 1
    }
}
