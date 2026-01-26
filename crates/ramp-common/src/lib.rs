//! RampOS Common - Shared types and utilities

pub mod crypto;
pub mod error;
pub mod intent;
#[cfg(test)]
pub mod intent_test;
pub mod ledger;
pub mod types;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use types::*;

#[cfg(test)]
mod inline_tests {
    use super::*;
    use crate::intent::PayinState;

    #[test]
    fn test_money_arithmetic() {
        let a = VndAmount::from_i64(100);
        let b = VndAmount::from_i64(50);
        let c = a + b;
        assert_eq!(c, VndAmount::from_i64(150));

        let d = a - b;
        assert_eq!(d, VndAmount::from_i64(50));

        let zero = VndAmount::zero();
        assert!(zero.is_zero());
        assert!(!a.is_zero());
        assert!(a.is_positive());
    }

    #[test]
    fn test_intent_state_transitions() {
        let state = PayinState::Created;
        assert!(state.can_transition_to(PayinState::InstructionIssued));
        assert!(state.can_transition_to(PayinState::Cancelled));
        assert!(!state.can_transition_to(PayinState::Completed));

        let terminal = PayinState::Completed;
        assert!(terminal.is_terminal());
        assert!(terminal.allowed_transitions().is_empty());
    }

    #[test]
    fn test_error_conversions() {
        let err = Error::IntentNotFound("test".to_string());
        assert_eq!(err.error_code(), "INTENT_NOT_FOUND");
        assert!(!err.is_retryable());

        let db_err = Error::Database("connection failed".to_string());
        assert_eq!(db_err.error_code(), "DATABASE_ERROR");
        assert!(db_err.is_retryable());
    }
}
