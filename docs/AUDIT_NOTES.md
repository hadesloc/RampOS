# Audit Notes & Known Risks

This document tracks known security risks, assumptions, and mitigation strategies for the external audit.

## 1. Scope

### In Scope
- Core Rust crates (`ramp-core`, `ramp-api`, `ramp-ledger`)
- Smart contracts (`contracts/src/*.sol`)
- Infrastructure configuration (`k8s/`, `docker-compose.yml`)
- API endpoints and authentication flow

### Out of Scope
- Frontend applications
- Third-party integrations (e.g., banking partners) outside of the integration layer
- Development tooling

## 2. Known Risks & Assumptions

### 2.1. Cryptography
- **Risk**: Key management relies on environment variables in some deployment configurations.
- **Mitigation**: Production deployments MUST use a dedicated secret manager (e.g., AWS Secrets Manager, HashiCorp Vault).
- **Assumption**: The `verifyingSigner` address in `RampOSPaymaster` is secured by a highly available and secure signing service (HSM recommended).

### 2.2. Smart Contracts (ERC-4337)
- **Risk**: The `RampOSPaymaster` relies on off-chain signature validation. If the signing key is compromised, an attacker could drain the paymaster's gas deposit.
- **Mitigation**:
  - `tenantDailyLimit` and `userDailyOps` provide defense-in-depth limits.
  - Rate limiting is enforced on-chain.
  - Verify signature includes `validUntil` and `validAfter` to prevent replay attacks (though nonce management is handled by EntryPoint).
- **Assumption**: The EntryPoint contract is the canonical singleton deployed by the Ethereum Foundation/community and is bug-free.

### 2.3. Tenant Isolation
- **Risk**: Logical isolation in a shared database could be bypassed via SQL injection or code bugs.
- **Mitigation**:
  - `sqlx` prepared statements are used exclusively.
  - Row Level Security (RLS) is enabled in PostgreSQL as a backup enforcement layer.
  - All API handlers use a `TenantId` extractor that validates the tenant context.

### 2.4. Precision & Rounding
- **Risk**: Financial calculations might suffer from floating-point errors.
- **Mitigation**: All monetary values use `rust_decimal` or integer math (in Solidity). No binary floating-point types (`f32`, `f64`) are used for money.

## 3. Automated Scan Results

### 3.1. Slither (Solidity)
*Run `slither .` in `contracts/` directory.*

- **Expected findings**:
  - *Reentrancy*: Should be mitigated by Checks-Effects-Interactions pattern and OpenZeppelin's `ReentrancyGuard` (if used, though `RampOSAccount` relies on EntryPoint architecture which mitigates most reentrancy vectors).
  - *Uninitialized state variables*: Ensure all logic is in `initialize` for upgradeable contracts.

### 3.2. Cargo Audit (Rust)
*Run `cargo audit` in root.*

- **Action items**: Address any vulnerabilities reported in the generated `security-reports/rust-audit.txt`.

## 4. Manual Review Checklist

- [ ] Verify `RampOSAccount` upgradeability control (only owner).
- [ ] Check `RampOSPaymaster` withdrawal logic (only owner).
- [ ] Confirm all `unsafe` blocks in Rust code (if any) are justified and documented.
- [ ] Validate that sensitive headers (Authorization) are stripped from logs.
- [ ] Ensure `validator` crate is applied to all DTOs.
