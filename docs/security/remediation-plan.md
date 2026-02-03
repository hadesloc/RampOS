# RAMPOS AUDIT REMEDIATION PLAN

**Date:** 2026-01-26
**Objective:** Fix critical (P0) and high priority (P1) issues identified in the audit report.

## PHASE 1: COMPLIANCE MODULE FIXES (Highest Priority)
**Goal:** Make the compliance engine functional and regulatory compliant.

- [ ] **TASK-C01: Implement Database Layer for Compliance**
  - Add `sqlx` migrations for `compliance_cases`, `case_notes`, `transaction_history`.
  - Implement `PostgresCaseStore` struct implementing `CaseStore` trait.
  - Wire up `create_case`, `update_case`, `get_case` to actual DB queries.

- [ ] **TASK-C02: Implement AML Rules Logic**
  - Update `VelocityRule`: Query `transaction_history` for user's volume in last 24h.
  - Update `StructuringRule`: Query for pattern of transactions just below thresholds.
  - Enable these rules in the default rule set.

- [ ] **TASK-C03: Sanctions Fail-Secure**
  - Update `opensanctions.rs`: If API request fails/times out, return `Err` instead of `Ok(Pass)`.
  - Update `aml.rs`: Catch error -> Block transaction -> Alert admin.

## PHASE 2: INFRASTRUCTURE HARDENING
**Goal:** Secure credentials and improve production readiness.

- [ ] **TASK-I01: Remove Secrets from Git**
  - Delete `k8s/base/secret.yaml`.
  - Create a template `secret.example.yaml` with dummy values.
  - Update `.gitignore` to exclude `k8s/**/secret.yaml`.

- [ ] **TASK-I02: Database Security & HA**
  - Update `postgres-statefulset.yaml`: Increase replicas to 3 (if using standard HA operator) or simply document "Use RDS for Prod".
  - Add `securityContext` to all deployments (runAsNonRoot).

## PHASE 3: SMART CONTRACT PATCHES
**Goal:** Fix accounting logic in Paymaster.

- [ ] **TASK-SC01: Fix Paymaster Gas Logic**
  - Modify `RampOSPaymaster.sol`:
    - In `postOp`: Calculate refund = `maxCost - actualGasCost`.
    - Update `tenantDailySpent`: Subtract refund amount.

## EXECUTION STRATEGY
We will execute these tasks in parallel where possible using specialized agents.
