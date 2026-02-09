#[cfg(test)]
mod tests {
    use crate::intent::*;

    // ============================================================================
    // PayinState Tests
    // ============================================================================

    #[test]
    fn test_payin_state_happy_path() {
        // Created -> InstructionIssued -> FundsPending -> FundsConfirmed -> VndCredited -> Completed
        let mut state = PayinState::Created;

        assert!(state.can_transition_to(PayinState::InstructionIssued));
        state = PayinState::InstructionIssued;

        assert!(state.can_transition_to(PayinState::FundsPending));
        state = PayinState::FundsPending;

        assert!(state.can_transition_to(PayinState::FundsConfirmed));
        state = PayinState::FundsConfirmed;

        assert!(state.can_transition_to(PayinState::VndCredited));
        state = PayinState::VndCredited;

        assert!(state.can_transition_to(PayinState::Completed));
        state = PayinState::Completed;

        assert!(state.is_terminal());
        assert!(!state.is_error());
        assert!(state.allowed_transitions().is_empty());
    }

    #[test]
    fn test_payin_state_error_paths() {
        // Expired paths
        assert!(PayinState::InstructionIssued.can_transition_to(PayinState::Expired));
        assert!(PayinState::FundsPending.can_transition_to(PayinState::Expired));
        assert!(PayinState::Expired.is_terminal());
        assert!(PayinState::Expired.is_error());

        // Cancelled paths
        assert!(PayinState::Created.can_transition_to(PayinState::Cancelled));
        assert!(PayinState::InstructionIssued.can_transition_to(PayinState::Cancelled));
        assert!(PayinState::ManualReview.can_transition_to(PayinState::Cancelled));
        assert!(PayinState::Cancelled.is_terminal());
        assert!(!PayinState::Cancelled.is_error());

        // MismatchedAmount path
        assert!(PayinState::FundsPending.can_transition_to(PayinState::MismatchedAmount));
        assert!(PayinState::MismatchedAmount.can_transition_to(PayinState::ManualReview));
        assert!(PayinState::MismatchedAmount.can_transition_to(PayinState::VndCredited));
        assert!(PayinState::MismatchedAmount.is_error());

        // SuspectedFraud path
        assert!(PayinState::FundsConfirmed.can_transition_to(PayinState::SuspectedFraud));
        assert!(PayinState::ManualReview.can_transition_to(PayinState::SuspectedFraud));
        assert!(PayinState::SuspectedFraud.is_terminal());
        assert!(PayinState::SuspectedFraud.is_error());

        // ManualReview path
        assert!(PayinState::FundsConfirmed.can_transition_to(PayinState::ManualReview));
        assert!(PayinState::MismatchedAmount.can_transition_to(PayinState::ManualReview));
        assert!(PayinState::ManualReview.can_transition_to(PayinState::VndCredited));
    }

    #[test]
    fn test_payin_state_invalid_transitions() {
        assert!(!PayinState::Created.can_transition_to(PayinState::Completed));
        assert!(!PayinState::Completed.can_transition_to(PayinState::Created));
        assert!(!PayinState::Expired.can_transition_to(PayinState::Created));
    }

    // ============================================================================
    // PayoutState Tests
    // ============================================================================

    #[test]
    fn test_payout_state_happy_path() {
        // Created -> PolicyApproved -> Submitted -> Confirmed -> Completed
        let mut state = PayoutState::Created;

        assert!(state.can_transition_to(PayoutState::PolicyApproved));
        state = PayoutState::PolicyApproved;

        assert!(state.can_transition_to(PayoutState::Submitted));
        state = PayoutState::Submitted;

        assert!(state.can_transition_to(PayoutState::Confirmed));
        state = PayoutState::Confirmed;

        assert!(state.can_transition_to(PayoutState::Completed));
        state = PayoutState::Completed;

        assert!(state.is_terminal());
        assert!(!state.is_error());
    }

    #[test]
    fn test_payout_state_error_paths() {
        // RejectedByPolicy
        assert!(PayoutState::Created.can_transition_to(PayoutState::RejectedByPolicy));
        assert!(PayoutState::ManualReview.can_transition_to(PayoutState::RejectedByPolicy));
        assert!(PayoutState::RejectedByPolicy.is_terminal());
        assert!(PayoutState::RejectedByPolicy.is_error());

        // BankRejected
        assert!(PayoutState::Submitted.can_transition_to(PayoutState::BankRejected));
        assert!(PayoutState::BankRejected.is_terminal());
        assert!(PayoutState::BankRejected.is_error());

        // Timeout
        assert!(PayoutState::Submitted.can_transition_to(PayoutState::Timeout));
        assert!(PayoutState::Timeout.can_transition_to(PayoutState::Submitted)); // Retry
        assert!(PayoutState::Timeout.can_transition_to(PayoutState::ManualReview));
        assert!(PayoutState::Timeout.is_error());
        assert!(!PayoutState::Timeout.is_terminal());

        // ManualReview
        assert!(PayoutState::Created.can_transition_to(PayoutState::ManualReview));
        assert!(PayoutState::Timeout.can_transition_to(PayoutState::ManualReview));
        assert!(PayoutState::ManualReview.can_transition_to(PayoutState::PolicyApproved));
        assert!(PayoutState::ManualReview.can_transition_to(PayoutState::Cancelled));

        // Cancelled
        assert!(PayoutState::PolicyApproved.can_transition_to(PayoutState::Cancelled));
        assert!(PayoutState::Cancelled.is_terminal());
        assert!(!PayoutState::Cancelled.is_error());
    }

    #[test]
    fn test_payout_state_invalid_transitions() {
        assert!(!PayoutState::Created.can_transition_to(PayoutState::Completed));
        assert!(!PayoutState::Completed.can_transition_to(PayoutState::Created));
    }

    // ============================================================================
    // TradeState Tests
    // ============================================================================

    #[test]
    fn test_trade_state_happy_path() {
        // Recorded -> PostTradeChecked -> SettledLedger -> Completed
        let mut state = TradeState::Recorded;

        assert!(state.can_transition_to(TradeState::PostTradeChecked));
        state = TradeState::PostTradeChecked;

        assert!(state.can_transition_to(TradeState::SettledLedger));
        state = TradeState::SettledLedger;

        assert!(state.can_transition_to(TradeState::Completed));
        state = TradeState::Completed;

        assert!(state.is_terminal());
    }

    #[test]
    fn test_trade_state_error_paths() {
        // ComplianceHold
        assert!(TradeState::Recorded.can_transition_to(TradeState::ComplianceHold));
        assert!(TradeState::ComplianceHold.can_transition_to(TradeState::ManualReview));
        assert!(TradeState::ComplianceHold.can_transition_to(TradeState::Rejected));
        assert!(TradeState::ComplianceHold.is_error());

        // ManualReview
        assert!(TradeState::PostTradeChecked.can_transition_to(TradeState::ManualReview));
        assert!(TradeState::ManualReview.can_transition_to(TradeState::PostTradeChecked));
        assert!(TradeState::ManualReview.can_transition_to(TradeState::Rejected));
        assert!(!TradeState::ManualReview.is_error());

        // Rejected
        assert!(TradeState::Rejected.is_terminal());
        assert!(TradeState::Rejected.is_error());
    }

    #[test]
    fn test_trade_state_invalid_transitions() {
        assert!(!TradeState::Recorded.can_transition_to(TradeState::Completed));
        assert!(!TradeState::Completed.can_transition_to(TradeState::Recorded));
    }

    // ============================================================================
    // DepositState Tests
    // ============================================================================

    #[test]
    fn test_deposit_state_happy_path() {
        // Detected -> Confirming -> Confirmed -> KytChecked -> Credited -> Completed
        let mut state = DepositState::Detected;

        assert!(state.can_transition_to(DepositState::Confirming));
        state = DepositState::Confirming;

        assert!(state.can_transition_to(DepositState::Confirmed));
        state = DepositState::Confirmed;

        assert!(state.can_transition_to(DepositState::KytChecked));
        state = DepositState::KytChecked;

        assert!(state.can_transition_to(DepositState::Credited));
        state = DepositState::Credited;

        assert!(state.can_transition_to(DepositState::Completed));
        state = DepositState::Completed;

        assert!(state.is_terminal());
    }

    #[test]
    fn test_deposit_state_error_paths() {
        // KytFlagged
        assert!(DepositState::Confirmed.can_transition_to(DepositState::KytFlagged));
        assert!(DepositState::KytFlagged.can_transition_to(DepositState::ManualReview));
        assert!(DepositState::KytFlagged.can_transition_to(DepositState::Rejected));
        assert!(DepositState::KytFlagged.is_error());

        // ManualReview
        assert!(DepositState::ManualReview.can_transition_to(DepositState::Credited));
        assert!(DepositState::ManualReview.can_transition_to(DepositState::Rejected));
        assert!(!DepositState::ManualReview.is_error());

        // Rejected
        assert!(DepositState::Rejected.is_terminal());
        assert!(DepositState::Rejected.is_error());
    }

    #[test]
    fn test_deposit_state_invalid_transitions() {
        assert!(!DepositState::Detected.can_transition_to(DepositState::Completed));
        assert!(!DepositState::Completed.can_transition_to(DepositState::Detected));
    }

    // ============================================================================
    // WithdrawState Tests
    // ============================================================================

    #[test]
    fn test_withdraw_state_happy_path() {
        // Created -> PolicyApproved -> KytChecked -> Signed -> Broadcasted -> Confirming -> Confirmed -> Completed
        let mut state = WithdrawState::Created;

        assert!(state.can_transition_to(WithdrawState::PolicyApproved));
        state = WithdrawState::PolicyApproved;

        assert!(state.can_transition_to(WithdrawState::KytChecked));
        state = WithdrawState::KytChecked;

        assert!(state.can_transition_to(WithdrawState::Signed));
        state = WithdrawState::Signed;

        assert!(state.can_transition_to(WithdrawState::Broadcasted));
        state = WithdrawState::Broadcasted;

        assert!(state.can_transition_to(WithdrawState::Confirming));
        state = WithdrawState::Confirming;

        assert!(state.can_transition_to(WithdrawState::Confirmed));
        state = WithdrawState::Confirmed;

        assert!(state.can_transition_to(WithdrawState::Completed));
        state = WithdrawState::Completed;

        assert!(state.is_terminal());
    }

    #[test]
    fn test_withdraw_state_error_paths() {
        // RejectedByPolicy
        assert!(WithdrawState::Created.can_transition_to(WithdrawState::RejectedByPolicy));
        assert!(WithdrawState::RejectedByPolicy.is_terminal());
        assert!(WithdrawState::RejectedByPolicy.is_error());

        // KytFlagged
        assert!(WithdrawState::PolicyApproved.can_transition_to(WithdrawState::KytFlagged));
        assert!(WithdrawState::KytFlagged.can_transition_to(WithdrawState::ManualReview));
        assert!(WithdrawState::KytFlagged.can_transition_to(WithdrawState::Cancelled));
        assert!(WithdrawState::KytFlagged.is_error());

        // BroadcastFailed
        assert!(WithdrawState::Signed.can_transition_to(WithdrawState::BroadcastFailed));
        assert!(WithdrawState::BroadcastFailed.can_transition_to(WithdrawState::Signed)); // Retry
        assert!(WithdrawState::BroadcastFailed.can_transition_to(WithdrawState::ManualReview));
        assert!(WithdrawState::BroadcastFailed.is_error());

        // ManualReview
        assert!(WithdrawState::Confirming.can_transition_to(WithdrawState::ManualReview));
        assert!(WithdrawState::ManualReview.can_transition_to(WithdrawState::PolicyApproved));
        assert!(WithdrawState::ManualReview.can_transition_to(WithdrawState::Cancelled));
        assert!(!WithdrawState::ManualReview.is_error());
    }

    #[test]
    fn test_withdraw_state_invalid_transitions() {
        assert!(!WithdrawState::Created.can_transition_to(WithdrawState::Completed));
        assert!(!WithdrawState::Completed.can_transition_to(WithdrawState::Created));
    }

    // ============================================================================
    // From<&str> Conversion Tests
    // ============================================================================

    #[test]
    fn test_payin_state_from_str() {
        assert_eq!(PayinState::from("PAYIN_CREATED"), PayinState::Created);
        assert_eq!(PayinState::from("CREATED"), PayinState::Created);
        assert_eq!(PayinState::from("INSTRUCTION_ISSUED"), PayinState::InstructionIssued);
        assert_eq!(PayinState::from("FUNDS_PENDING"), PayinState::FundsPending);
        assert_eq!(PayinState::from("FUNDS_CONFIRMED"), PayinState::FundsConfirmed);
        assert_eq!(PayinState::from("VND_CREDITED"), PayinState::VndCredited);
        assert_eq!(PayinState::from("COMPLETED"), PayinState::Completed);
        assert_eq!(PayinState::from("EXPIRED"), PayinState::Expired);
        assert_eq!(PayinState::from("MISMATCHED_AMOUNT"), PayinState::MismatchedAmount);
        assert_eq!(PayinState::from("SUSPECTED_FRAUD"), PayinState::SuspectedFraud);
        assert_eq!(PayinState::from("MANUAL_REVIEW"), PayinState::ManualReview);
        assert_eq!(PayinState::from("CANCELLED"), PayinState::Cancelled);
        assert_eq!(PayinState::from("UNKNOWN"), PayinState::Created);
    }

    #[test]
    fn test_payout_state_from_str() {
        assert_eq!(PayoutState::from("PAYOUT_CREATED"), PayoutState::Created);
        assert_eq!(PayoutState::from("CREATED"), PayoutState::Created);
        assert_eq!(PayoutState::from("POLICY_APPROVED"), PayoutState::PolicyApproved);
        assert_eq!(PayoutState::from("PAYOUT_SUBMITTED"), PayoutState::Submitted);
        assert_eq!(PayoutState::from("SUBMITTED"), PayoutState::Submitted);
        assert_eq!(PayoutState::from("PAYOUT_CONFIRMED"), PayoutState::Confirmed);
        assert_eq!(PayoutState::from("CONFIRMED"), PayoutState::Confirmed);
        assert_eq!(PayoutState::from("COMPLETED"), PayoutState::Completed);
        assert_eq!(PayoutState::from("REJECTED_BY_POLICY"), PayoutState::RejectedByPolicy);
        assert_eq!(PayoutState::from("BANK_REJECTED"), PayoutState::BankRejected);
        assert_eq!(PayoutState::from("TIMEOUT"), PayoutState::Timeout);
        assert_eq!(PayoutState::from("REVERSED"), PayoutState::Reversed);
        assert_eq!(PayoutState::from("UNKNOWN"), PayoutState::Created);
    }

    #[test]
    fn test_withdraw_state_from_str() {
        assert_eq!(WithdrawState::from("CREATED"), WithdrawState::Created);
        assert_eq!(WithdrawState::from("POLICY_APPROVED"), WithdrawState::PolicyApproved);
        assert_eq!(WithdrawState::from("KYT_CHECKED"), WithdrawState::KytChecked);
        assert_eq!(WithdrawState::from("SIGNED"), WithdrawState::Signed);
        assert_eq!(WithdrawState::from("BROADCASTED"), WithdrawState::Broadcasted);
        assert_eq!(WithdrawState::from("CONFIRMING"), WithdrawState::Confirming);
        assert_eq!(WithdrawState::from("CONFIRMED"), WithdrawState::Confirmed);
        assert_eq!(WithdrawState::from("COMPLETED"), WithdrawState::Completed);
        assert_eq!(WithdrawState::from("REJECTED_BY_POLICY"), WithdrawState::RejectedByPolicy);
        assert_eq!(WithdrawState::from("KYT_FLAGGED"), WithdrawState::KytFlagged);
        assert_eq!(WithdrawState::from("BROADCAST_FAILED"), WithdrawState::BroadcastFailed);
        assert_eq!(WithdrawState::from("MANUAL_REVIEW"), WithdrawState::ManualReview);
        assert_eq!(WithdrawState::from("CANCELLED"), WithdrawState::Cancelled);
        assert_eq!(WithdrawState::from("REJECTED_INSUFFICIENT_BALANCE"), WithdrawState::RejectedByPolicy);
        assert_eq!(WithdrawState::from("UNKNOWN"), WithdrawState::Created);
    }

    #[test]
    fn test_payin_state_roundtrip() {
        let states = vec![
            PayinState::Created, PayinState::InstructionIssued, PayinState::FundsPending,
            PayinState::FundsConfirmed, PayinState::VndCredited, PayinState::Completed,
            PayinState::Expired, PayinState::MismatchedAmount, PayinState::SuspectedFraud,
            PayinState::ManualReview, PayinState::Cancelled,
        ];
        for state in states {
            let s = state.to_string();
            let roundtripped = PayinState::from(s.as_str());
            assert_eq!(state, roundtripped, "Roundtrip failed for {:?} -> {} -> {:?}", state, s, roundtripped);
        }
    }

    #[test]
    fn test_payout_state_roundtrip() {
        let states = vec![
            PayoutState::Created, PayoutState::PolicyApproved, PayoutState::Submitted,
            PayoutState::Confirmed, PayoutState::Completed, PayoutState::RejectedByPolicy,
            PayoutState::BankRejected, PayoutState::Timeout, PayoutState::ManualReview,
            PayoutState::Cancelled, PayoutState::Reversed,
        ];
        for state in states {
            let s = state.to_string();
            let roundtripped = PayoutState::from(s.as_str());
            assert_eq!(state, roundtripped, "Roundtrip failed for {:?} -> {} -> {:?}", state, s, roundtripped);
        }
    }

    #[test]
    fn test_withdraw_state_roundtrip() {
        let states = vec![
            WithdrawState::Created, WithdrawState::PolicyApproved, WithdrawState::KytChecked,
            WithdrawState::Signed, WithdrawState::Broadcasted, WithdrawState::Confirming,
            WithdrawState::Confirmed, WithdrawState::Completed, WithdrawState::RejectedByPolicy,
            WithdrawState::KytFlagged, WithdrawState::BroadcastFailed, WithdrawState::ManualReview,
            WithdrawState::Cancelled,
        ];
        for state in states {
            let s = state.to_string();
            let roundtripped = WithdrawState::from(s.as_str());
            assert_eq!(state, roundtripped, "Roundtrip failed for {:?} -> {} -> {:?}", state, s, roundtripped);
        }
    }

    #[test]
    fn test_from_string_owned() {
        let s = String::from("COMPLETED");
        assert_eq!(PayinState::from(s.clone()), PayinState::Completed);
        assert_eq!(PayoutState::from(s.clone()), PayoutState::Completed);
        assert_eq!(WithdrawState::from(s.clone()), WithdrawState::Completed);
        assert_eq!(TradeState::from(s.clone()), TradeState::Completed);
        assert_eq!(DepositState::from(s), DepositState::Completed);
    }

    // ============================================================================
    // TradeState Roundtrip Tests
    // ============================================================================

    #[test]
    fn test_trade_state_roundtrip() {
        let states = vec![
            TradeState::Recorded, TradeState::PostTradeChecked, TradeState::SettledLedger,
            TradeState::Completed, TradeState::ComplianceHold, TradeState::ManualReview,
            TradeState::Rejected,
        ];
        for state in states {
            let s = state.to_string();
            let roundtripped = TradeState::from(s.as_str());
            assert_eq!(state, roundtripped, "Roundtrip failed for {:?} -> {} -> {:?}", state, s, roundtripped);
        }
    }

    #[test]
    fn test_trade_state_from_str() {
        assert_eq!(TradeState::from("RECORDED"), TradeState::Recorded);
        assert_eq!(TradeState::from("POST_TRADE_CHECKED"), TradeState::PostTradeChecked);
        assert_eq!(TradeState::from("SETTLED_LEDGER"), TradeState::SettledLedger);
        assert_eq!(TradeState::from("COMPLETED"), TradeState::Completed);
        assert_eq!(TradeState::from("COMPLIANCE_HOLD"), TradeState::ComplianceHold);
        assert_eq!(TradeState::from("MANUAL_REVIEW"), TradeState::ManualReview);
        assert_eq!(TradeState::from("REJECTED"), TradeState::Rejected);
        assert_eq!(TradeState::from("UNKNOWN"), TradeState::Recorded);
    }

    // ============================================================================
    // DepositState Roundtrip Tests
    // ============================================================================

    #[test]
    fn test_deposit_state_roundtrip() {
        let states = vec![
            DepositState::Detected, DepositState::Confirming, DepositState::Confirmed,
            DepositState::KytChecked, DepositState::Credited, DepositState::Completed,
            DepositState::KytFlagged, DepositState::ManualReview, DepositState::Rejected,
        ];
        for state in states {
            let s = state.to_string();
            let roundtripped = DepositState::from(s.as_str());
            assert_eq!(state, roundtripped, "Roundtrip failed for {:?} -> {} -> {:?}", state, s, roundtripped);
        }
    }

    #[test]
    fn test_deposit_state_from_str() {
        assert_eq!(DepositState::from("DETECTED"), DepositState::Detected);
        assert_eq!(DepositState::from("CONFIRMING"), DepositState::Confirming);
        assert_eq!(DepositState::from("CONFIRMED"), DepositState::Confirmed);
        assert_eq!(DepositState::from("KYT_CHECKED"), DepositState::KytChecked);
        assert_eq!(DepositState::from("CREDITED"), DepositState::Credited);
        assert_eq!(DepositState::from("COMPLETED"), DepositState::Completed);
        assert_eq!(DepositState::from("KYT_FLAGGED"), DepositState::KytFlagged);
        assert_eq!(DepositState::from("MANUAL_REVIEW"), DepositState::ManualReview);
        assert_eq!(DepositState::from("REJECTED"), DepositState::Rejected);
        assert_eq!(DepositState::from("UNKNOWN"), DepositState::Detected);
    }

    // ============================================================================
    // FromStr (Result-returning) Tests
    // ============================================================================

    #[test]
    fn test_payin_state_parse_valid() {
        assert_eq!("COMPLETED".parse::<PayinState>().unwrap(), PayinState::Completed);
        assert_eq!("PAYIN_CREATED".parse::<PayinState>().unwrap(), PayinState::Created);
        assert_eq!("CREATED".parse::<PayinState>().unwrap(), PayinState::Created);
        assert_eq!("INSTRUCTION_ISSUED".parse::<PayinState>().unwrap(), PayinState::InstructionIssued);
    }

    #[test]
    fn test_payin_state_parse_invalid() {
        let err = "INVALID_STATE".parse::<PayinState>().unwrap_err();
        assert_eq!(err.state_type, "PayinState");
        assert_eq!(err.value, "INVALID_STATE");
        assert!(err.to_string().contains("PayinState"));
    }

    #[test]
    fn test_payout_state_parse_valid() {
        assert_eq!("COMPLETED".parse::<PayoutState>().unwrap(), PayoutState::Completed);
        assert_eq!("PAYOUT_CREATED".parse::<PayoutState>().unwrap(), PayoutState::Created);
        assert_eq!("REVERSED".parse::<PayoutState>().unwrap(), PayoutState::Reversed);
    }

    #[test]
    fn test_payout_state_parse_invalid() {
        let err = "INVALID_STATE".parse::<PayoutState>().unwrap_err();
        assert_eq!(err.state_type, "PayoutState");
    }

    #[test]
    fn test_trade_state_parse_valid() {
        assert_eq!("RECORDED".parse::<TradeState>().unwrap(), TradeState::Recorded);
        assert_eq!("COMPLIANCE_HOLD".parse::<TradeState>().unwrap(), TradeState::ComplianceHold);
    }

    #[test]
    fn test_trade_state_parse_invalid() {
        let err = "INVALID_STATE".parse::<TradeState>().unwrap_err();
        assert_eq!(err.state_type, "TradeState");
    }

    #[test]
    fn test_deposit_state_parse_valid() {
        assert_eq!("DETECTED".parse::<DepositState>().unwrap(), DepositState::Detected);
        assert_eq!("KYT_CHECKED".parse::<DepositState>().unwrap(), DepositState::KytChecked);
    }

    #[test]
    fn test_deposit_state_parse_invalid() {
        let err = "INVALID_STATE".parse::<DepositState>().unwrap_err();
        assert_eq!(err.state_type, "DepositState");
    }

    #[test]
    fn test_withdraw_state_parse_valid() {
        assert_eq!("CREATED".parse::<WithdrawState>().unwrap(), WithdrawState::Created);
        assert_eq!("REJECTED_INSUFFICIENT_BALANCE".parse::<WithdrawState>().unwrap(), WithdrawState::RejectedByPolicy);
    }

    #[test]
    fn test_withdraw_state_parse_invalid() {
        let err = "INVALID_STATE".parse::<WithdrawState>().unwrap_err();
        assert_eq!(err.state_type, "WithdrawState");
    }

    // ============================================================================
    // Unified IntentState Tests
    // ============================================================================

    #[test]
    fn test_unified_intent_state() {
        let payin = IntentState::Payin(PayinState::Completed);
        assert!(payin.is_terminal());
        assert!(!payin.is_error());
        assert_eq!(payin.as_string(), "COMPLETED");

        let payout = IntentState::Payout(PayoutState::Created);
        assert!(!payout.is_terminal());
        assert!(!payout.is_error());
        assert_eq!(payout.as_string(), "PAYOUT_CREATED");

        let trade = IntentState::Trade(TradeState::Completed);
        assert!(trade.is_terminal());
        assert!(!trade.is_error());
        assert_eq!(trade.as_string(), "COMPLETED");

        let trade_recorded = IntentState::Trade(TradeState::ComplianceHold);
        assert_eq!(trade_recorded.as_string(), "COMPLIANCE_HOLD");

        let deposit = IntentState::Deposit(DepositState::Completed);
        assert!(deposit.is_terminal());
        assert!(!deposit.is_error());
        assert_eq!(deposit.as_string(), "COMPLETED");

        let deposit_detected = IntentState::Deposit(DepositState::Detected);
        assert_eq!(deposit_detected.as_string(), "DETECTED");

        let withdraw = IntentState::Withdraw(WithdrawState::Completed);
        assert!(withdraw.is_terminal());
        assert!(!withdraw.is_error());
        assert_eq!(withdraw.as_string(), "COMPLETED");

        // Test error delegation
        let error_payin = IntentState::Payin(PayinState::Expired);
        assert!(error_payin.is_error());

        let error_trade = IntentState::Trade(TradeState::Rejected);
        assert!(error_trade.is_error());
    }
}
