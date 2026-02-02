# Architecture Documentation Handoff

## Task Summary
**Task**: Write Full Architecture Documentation for RampOS
**Status**: Completed
**Date**: 2026-02-02

## Deliverables

All documentation files created in `docs/architecture/`:

| File | Size | Description |
|------|------|-------------|
| `overview.md` | 11KB | System overview, high-level architecture, tech stack |
| `state-machine.md` | 17KB | Intent state machines with ASCII diagrams |
| `ledger.md` | 16KB | Double-entry bookkeeping design |
| `compliance.md` | 21KB | KYC tiers, AML rules, case management |

## Documentation Contents

### 1. overview.md
- System introduction and purpose
- High-level architecture diagram (ASCII)
- Component overview (7 crates)
- Crate dependency graph
- Technology stack (Rust, PostgreSQL, Redis, NATS, ClickHouse)
- Multi-tenant architecture explanation
- Key domain types
- API structure
- Event-driven architecture
- Deployment architecture (Docker, Kubernetes)
- Performance characteristics

### 2. state-machine.md
- All 5 intent types documented:
  - PayinVnd (11 states)
  - PayoutVnd (10 states)
  - Trade (7 states)
  - Deposit (9 states)
  - Withdraw (14 states)
- ASCII state diagrams for each
- State definitions with descriptions and actions
- Valid transition tables with Rust code examples
- Unified IntentState enum
- State transition validation
- Event emission on state changes

### 3. ledger.md
- Double-entry principle explanation
- Account types (13 accounts across 5 categories)
- LedgerEntry and LedgerTransaction data structures
- LedgerTransactionBuilder pattern
- Common transaction patterns:
  - VND Pay-in
  - VND Pay-out (initiated and confirmed)
  - Crypto/VND Trade (buy and sell)
- Balance query examples
- Transaction flow diagrams
- Error handling
- Database schema
- Reconciliation process

### 4. compliance.md
- KYC tier system (Tier 0-3)
- Tier limits table
- Tier upgrade requirements
- AML Engine architecture
- 6 built-in AML rules:
  - Velocity Rule
  - Structuring Rule
  - Large Transaction Rule
  - Unusual Payout Rule
  - Device Anomaly Rule
  - Sanctions Screening Rule
- Risk scoring (0-100 scale)
- Case management system
- Case types and severity levels
- Case status flow
- KYT (Know Your Transaction) integration
- Sanctions screening providers
- Compliance check result structure
- Rule configuration and caching
- SAR reporting
- API endpoints

## Code References

Documentation includes code examples from:
- `crates/ramp-common/src/intent.rs` - State machine definitions
- `crates/ramp-common/src/ledger.rs` - Ledger types and builder
- `crates/ramp-common/src/types.rs` - Domain types
- `crates/ramp-compliance/src/types.rs` - KYC/AML types
- `crates/ramp-compliance/src/aml.rs` - AML engine and rules
- `crates/ramp-compliance/src/kyc/tier.rs` - Tier management
- `crates/ramp-compliance/src/case.rs` - Case management
- `crates/ramp-adapter/src/traits.rs` - Rails adapter interface

## Quality Notes

- All diagrams use ASCII art for maximum compatibility
- Code examples extracted directly from codebase
- Tables provide quick reference for limits and configurations
- Flow diagrams show complete transaction lifecycles
- Best practices sections included in each document

## Next Steps (Recommendations)

1. Add API reference documentation (`docs/api/`)
2. Add SDK usage examples (`docs/sdk/`)
3. Add deployment guide (`docs/deployment/`)
4. Add troubleshooting guide (`docs/troubleshooting/`)
5. Generate OpenAPI spec from `ramp-api` crate
