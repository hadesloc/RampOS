use serde::{Deserialize, Serialize};

use crate::travel_rule::TravelRuleRequirement;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DisclosureLifecycleStage {
    Pending,
    Ready,
    Sent,
    Acknowledged,
    Failed,
    Exception,
    Waived,
}

impl DisclosureLifecycleStage {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Acknowledged | Self::Waived)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransportAttemptStatus {
    Pending,
    Sent,
    Acknowledged,
    Failed,
    Timeout,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExceptionQueueStatus {
    Open,
    InReview,
    Escalated,
    Resolved,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisclosureLifecycleEvent {
    MarkReady {
        transport_profile: Option<String>,
    },
    TransportUpdated {
        status: TransportAttemptStatus,
        attempt_number: u32,
    },
    ExceptionQueueUpdated {
        status: ExceptionQueueStatus,
    },
    Waived {
        reason_code: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisclosureLifecycleEventKind {
    MarkReady,
    TransportUpdated,
    ExceptionQueueUpdated,
    Waived,
}

impl From<&DisclosureLifecycleEvent> for DisclosureLifecycleEventKind {
    fn from(value: &DisclosureLifecycleEvent) -> Self {
        match value {
            DisclosureLifecycleEvent::MarkReady { .. } => Self::MarkReady,
            DisclosureLifecycleEvent::TransportUpdated { .. } => Self::TransportUpdated,
            DisclosureLifecycleEvent::ExceptionQueueUpdated { .. } => Self::ExceptionQueueUpdated,
            DisclosureLifecycleEvent::Waived { .. } => Self::Waived,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosureTransitionRequest {
    pub current_stage: DisclosureLifecycleStage,
    pub event: DisclosureLifecycleEvent,
    pub failure_count: u32,
    pub max_failures_before_exception: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosureTransitionResult {
    pub stage: DisclosureLifecycleStage,
    pub queue_status: Option<ExceptionQueueStatus>,
    pub failure_count: u32,
    pub unmet_requirements: Vec<TravelRuleRequirement>,
    pub terminal: bool,
    pub retry_recommended: bool,
}

impl DisclosureTransitionResult {
    pub fn entered_exception_queue(&self) -> bool {
        self.stage == DisclosureLifecycleStage::Exception
            && self.queue_status == Some(ExceptionQueueStatus::Open)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DisclosureTransitionError {
    #[error("invalid travel rule disclosure transition from {from:?} with event {event:?}")]
    InvalidTransition {
        from: DisclosureLifecycleStage,
        event: DisclosureLifecycleEventKind,
    },
    #[error("transport attempt number must be greater than zero")]
    InvalidAttemptNumber,
}

pub struct DisclosureStateMachine;

impl DisclosureStateMachine {
    pub fn transition(
        request: &DisclosureTransitionRequest,
    ) -> Result<DisclosureTransitionResult, DisclosureTransitionError> {
        let threshold = request.max_failures_before_exception.max(1);

        match &request.event {
            DisclosureLifecycleEvent::MarkReady { transport_profile } => transition_mark_ready(
                request.current_stage,
                request.failure_count,
                transport_profile,
            ),
            DisclosureLifecycleEvent::TransportUpdated {
                status,
                attempt_number,
            } => transition_transport_status(
                request.current_stage,
                request.failure_count,
                threshold,
                *status,
                *attempt_number,
            ),
            DisclosureLifecycleEvent::ExceptionQueueUpdated { status } => {
                transition_exception_queue(request.current_stage, request.failure_count, *status)
            }
            DisclosureLifecycleEvent::Waived { .. } => {
                transition_waived(request.current_stage, request.failure_count)
            }
        }
    }
}

fn transition_mark_ready(
    current_stage: DisclosureLifecycleStage,
    failure_count: u32,
    transport_profile: &Option<String>,
) -> Result<DisclosureTransitionResult, DisclosureTransitionError> {
    match current_stage {
        DisclosureLifecycleStage::Pending | DisclosureLifecycleStage::Failed => {
            if transport_profile
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_some()
            {
                Ok(DisclosureTransitionResult {
                    stage: DisclosureLifecycleStage::Ready,
                    queue_status: None,
                    failure_count,
                    unmet_requirements: Vec::new(),
                    terminal: false,
                    retry_recommended: false,
                })
            } else {
                Ok(DisclosureTransitionResult {
                    stage: current_stage,
                    queue_status: None,
                    failure_count,
                    unmet_requirements: vec![TravelRuleRequirement::TransportProfile],
                    terminal: current_stage.is_terminal(),
                    retry_recommended: current_stage == DisclosureLifecycleStage::Failed,
                })
            }
        }
        DisclosureLifecycleStage::Ready => Ok(DisclosureTransitionResult {
            stage: DisclosureLifecycleStage::Ready,
            queue_status: None,
            failure_count,
            unmet_requirements: Vec::new(),
            terminal: false,
            retry_recommended: false,
        }),
        _ => Err(invalid_transition(
            current_stage,
            DisclosureLifecycleEventKind::MarkReady,
        )),
    }
}

fn transition_transport_status(
    current_stage: DisclosureLifecycleStage,
    failure_count: u32,
    threshold: u32,
    status: TransportAttemptStatus,
    attempt_number: u32,
) -> Result<DisclosureTransitionResult, DisclosureTransitionError> {
    if attempt_number == 0 {
        return Err(DisclosureTransitionError::InvalidAttemptNumber);
    }

    match current_stage {
        DisclosureLifecycleStage::Ready
        | DisclosureLifecycleStage::Sent
        | DisclosureLifecycleStage::Failed => {
            let result = match status {
                TransportAttemptStatus::Pending => DisclosureTransitionResult {
                    stage: DisclosureLifecycleStage::Ready,
                    queue_status: None,
                    failure_count,
                    unmet_requirements: Vec::new(),
                    terminal: false,
                    retry_recommended: false,
                },
                TransportAttemptStatus::Sent => DisclosureTransitionResult {
                    stage: DisclosureLifecycleStage::Sent,
                    queue_status: None,
                    failure_count,
                    unmet_requirements: Vec::new(),
                    terminal: false,
                    retry_recommended: false,
                },
                TransportAttemptStatus::Acknowledged => DisclosureTransitionResult {
                    stage: DisclosureLifecycleStage::Acknowledged,
                    queue_status: None,
                    failure_count,
                    unmet_requirements: Vec::new(),
                    terminal: true,
                    retry_recommended: false,
                },
                TransportAttemptStatus::Failed
                | TransportAttemptStatus::Timeout
                | TransportAttemptStatus::Rejected => {
                    let next_failure_count = failure_count.saturating_add(1);
                    if next_failure_count >= threshold {
                        DisclosureTransitionResult {
                            stage: DisclosureLifecycleStage::Exception,
                            queue_status: Some(ExceptionQueueStatus::Open),
                            failure_count: next_failure_count,
                            unmet_requirements: Vec::new(),
                            terminal: false,
                            retry_recommended: false,
                        }
                    } else {
                        DisclosureTransitionResult {
                            stage: DisclosureLifecycleStage::Failed,
                            queue_status: None,
                            failure_count: next_failure_count,
                            unmet_requirements: Vec::new(),
                            terminal: false,
                            retry_recommended: true,
                        }
                    }
                }
            };

            Ok(result)
        }
        _ => Err(invalid_transition(
            current_stage,
            DisclosureLifecycleEventKind::TransportUpdated,
        )),
    }
}

fn transition_exception_queue(
    current_stage: DisclosureLifecycleStage,
    failure_count: u32,
    status: ExceptionQueueStatus,
) -> Result<DisclosureTransitionResult, DisclosureTransitionError> {
    if current_stage != DisclosureLifecycleStage::Exception {
        return Err(invalid_transition(
            current_stage,
            DisclosureLifecycleEventKind::ExceptionQueueUpdated,
        ));
    }

    let stage = match status {
        ExceptionQueueStatus::Resolved | ExceptionQueueStatus::Dismissed => {
            DisclosureLifecycleStage::Failed
        }
        ExceptionQueueStatus::Open
        | ExceptionQueueStatus::InReview
        | ExceptionQueueStatus::Escalated => DisclosureLifecycleStage::Exception,
    };

    Ok(DisclosureTransitionResult {
        stage,
        queue_status: Some(status),
        failure_count,
        unmet_requirements: Vec::new(),
        terminal: stage.is_terminal(),
        retry_recommended: stage == DisclosureLifecycleStage::Failed,
    })
}

fn transition_waived(
    current_stage: DisclosureLifecycleStage,
    failure_count: u32,
) -> Result<DisclosureTransitionResult, DisclosureTransitionError> {
    match current_stage {
        DisclosureLifecycleStage::Pending
        | DisclosureLifecycleStage::Ready
        | DisclosureLifecycleStage::Sent
        | DisclosureLifecycleStage::Failed
        | DisclosureLifecycleStage::Exception => Ok(DisclosureTransitionResult {
            stage: DisclosureLifecycleStage::Waived,
            queue_status: None,
            failure_count,
            unmet_requirements: Vec::new(),
            terminal: true,
            retry_recommended: false,
        }),
        _ => Err(invalid_transition(
            current_stage,
            DisclosureLifecycleEventKind::Waived,
        )),
    }
}

fn invalid_transition(
    from: DisclosureLifecycleStage,
    event: DisclosureLifecycleEventKind,
) -> DisclosureTransitionError {
    DisclosureTransitionError::InvalidTransition { from, event }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sent_attempt_advances_ready_disclosure() {
        let result = DisclosureStateMachine::transition(&DisclosureTransitionRequest {
            current_stage: DisclosureLifecycleStage::Ready,
            event: DisclosureLifecycleEvent::TransportUpdated {
                status: TransportAttemptStatus::Sent,
                attempt_number: 1,
            },
            failure_count: 0,
            max_failures_before_exception: 3,
        })
        .expect("transition should succeed");

        assert_eq!(result.stage, DisclosureLifecycleStage::Sent);
        assert_eq!(result.failure_count, 0);
        assert!(!result.terminal);
    }

    #[test]
    fn failed_attempt_moves_to_exception_after_limit() {
        let result = DisclosureStateMachine::transition(&DisclosureTransitionRequest {
            current_stage: DisclosureLifecycleStage::Sent,
            event: DisclosureLifecycleEvent::TransportUpdated {
                status: TransportAttemptStatus::Failed,
                attempt_number: 3,
            },
            failure_count: 2,
            max_failures_before_exception: 3,
        })
        .expect("transition should succeed");

        assert_eq!(result.stage, DisclosureLifecycleStage::Exception);
        assert_eq!(result.queue_status, Some(ExceptionQueueStatus::Open));
        assert_eq!(result.failure_count, 3);
        assert!(result.entered_exception_queue());
        assert!(!result.retry_recommended);
    }

    #[test]
    fn waiver_terminal_transition_is_explicit() {
        let waived = DisclosureStateMachine::transition(&DisclosureTransitionRequest {
            current_stage: DisclosureLifecycleStage::Exception,
            event: DisclosureLifecycleEvent::Waived {
                reason_code: "manual-waiver".to_string(),
            },
            failure_count: 1,
            max_failures_before_exception: 3,
        })
        .expect("waiver should succeed");

        assert_eq!(waived.stage, DisclosureLifecycleStage::Waived);
        assert!(waived.terminal);

        let invalid_follow_up = DisclosureStateMachine::transition(&DisclosureTransitionRequest {
            current_stage: DisclosureLifecycleStage::Waived,
            event: DisclosureLifecycleEvent::TransportUpdated {
                status: TransportAttemptStatus::Acknowledged,
                attempt_number: 2,
            },
            failure_count: waived.failure_count,
            max_failures_before_exception: 3,
        });

        assert!(invalid_follow_up.is_err());
    }
}
