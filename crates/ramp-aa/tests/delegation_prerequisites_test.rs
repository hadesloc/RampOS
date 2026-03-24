use chrono::{Duration, Utc};
use ramp_aa::eip7702::{
    DelegationApprovalBoundary, DelegationExecutionEnvelope, DelegationPrerequisites,
    DelegationValidationError,
};
use ramp_aa::primitives::Address;

fn test_tool_surface() -> String {
    "admin.venue_trust.report_snapshot".to_string()
}

fn test_scope() -> String {
    "venue-trust:subject:merchant-123".to_string()
}

fn test_provenance() -> String {
    "truthful-auth:v1/session-123".to_string()
}

fn test_delegate() -> Address {
    "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        .parse()
        .unwrap()
}

#[test]
fn execution_envelope_without_explicit_contract_is_invalid() {
    let prerequisites = DelegationPrerequisites::new(test_delegate());
    let envelope = DelegationExecutionEnvelope::default();

    let result = prerequisites.validate_envelope(&envelope);

    assert!(matches!(
        result,
        Err(DelegationValidationError::MissingAllowedToolSurface)
            | Err(DelegationValidationError::MissingApprovalBoundary)
            | Err(DelegationValidationError::MissingExpiry)
            | Err(DelegationValidationError::MissingScope)
            | Err(DelegationValidationError::MissingProvenance)
    ));
}

#[test]
fn execution_envelope_with_explicit_contract_validates() {
    let tool_surface = test_tool_surface();
    let prerequisites = DelegationPrerequisites::new(test_delegate())
        .allow_tool_surface(tool_surface.clone())
        .with_approval_boundary(DelegationApprovalBoundary::ExplicitApprovalRequired);
    let envelope = DelegationExecutionEnvelope::new(
        test_delegate(),
        tool_surface,
        DelegationApprovalBoundary::ExplicitApprovalRequired,
        Utc::now() + Duration::minutes(15),
        test_scope(),
        test_provenance(),
    );

    let result = prerequisites.validate_envelope(&envelope);

    assert!(result.is_ok());
}
