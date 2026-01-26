# Task 6.2.5: Add Sanctions Screening

## Changes Implemented

1.  **Modified `RuleContext` in `crates/ramp-compliance/src/rules.rs`**
    *   Added fields for sanctions screening:
        *   `user_full_name: Option<String>`
        *   `user_country: Option<String>`
        *   `user_address: Option<String>`
    *   Updated `AmlEngine` and tests to populate these new fields.

2.  **Created `SanctionsRule` in `crates/ramp-compliance/src/rules/sanctions.rs`**
    *   Implements `AmlRule` trait.
    *   Uses `SanctionsProvider` to check individual names and addresses.
    *   Returns `RuleResult` with `CaseSeverity::Critical` and `risk_score` of 100.0 on match.
    *   Logs warnings when matches are found.

3.  **Integrated `SanctionsRule` into `AmlEngine` in `crates/ramp-compliance/src/aml.rs`**
    *   Updated `AmlEngine::default_rules` to include `SanctionsRule` if a provider is available.
    *   Updated `AmlEngine` constructor to pass the provider to `default_rules`.
    *   Ensured `SanctionsRule` runs alongside existing rules (Velocity, Structuring, etc.).

4.  **Updated Tests**
    *   Updated existing tests in `aml.rs`, `aml/device_anomaly.rs`, and `rule_parser.rs` to initialize `RuleContext` with new fields (set to `None`).
    *   Added unit tests for `SanctionsRule` verifying match and no-match scenarios.

## Files Modified
*   `crates/ramp-compliance/src/rules.rs`
*   `crates/ramp-compliance/src/aml.rs`
*   `crates/ramp-compliance/src/rules/sanctions.rs` (Created)
*   `crates/ramp-compliance/src/aml/device_anomaly.rs`
*   `crates/ramp-compliance/src/rule_parser.rs`

## Verification
*    ran `cargo test` in `crates/ramp-compliance`. All 49 tests passed.
*   Verified that `SanctionsRule` correctly flags high-risk transactions when a match is found in the mocked provider.
*   Verified that existing rules continue to work as expected.

## Notes
*   The `SanctionsProvider` is used both directly in `AmlEngine` (legacy check) and now within `SanctionsRule` (standardized rule check). The legacy check in `AmlEngine::check_transaction` (lines 146-206) might be redundant now but was left to preserve existing behavior while adding the new rule structure. The task requirement was to "Create a SanctionsRule implementing AmlRule", which is done.
*   The mocked `SanctionsProvider` was used for testing. In production, a real provider (e.g., OpenSanctions) would be injected.
