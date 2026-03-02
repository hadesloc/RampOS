# MPC Custody Evaluation (F11)

## Current State

The F11 MPC Custody module resides in `crates/ramp-core/src/custody/` with 4 files:

### mpc_key.rs - Key Generation (587 lines)
- Implements **simulated 2-of-3 Shamir Secret Sharing** over GF(256)
- `MpcKeyService` generates 32-byte random secrets, splits into 3 shares using polynomial evaluation
- Public key derivation: `SHA-256(secret)` -- **NOT real elliptic curve key derivation**
- Share refresh via `refresh_key_shares()` re-splits the reconstructed secret with new random polynomial
- In-memory `HashMap` storage behind `Mutex` (no persistence)
- 11 unit tests covering: generation, reconstruction, storage, refresh, GF(256) arithmetic

### mpc_signing.rs - Threshold Signing (557 lines)
- `MpcSigningService` manages signing sessions with Pending -> Approved -> Signed lifecycle
- 2-of-3 threshold: requester auto-approves (party 1), needs 1 more approval
- Partial signatures: `SHA-256(party_id || message_hash || random_nonce)` -- **simulated, not ECDSA/EdDSA**
- Final signature: `SHA-256(message_hash || partial_sig_1 || partial_sig_2)` -- **simulated combination**
- Session rejection workflow exists
- 14 unit tests covering: creation, approval, combination, rejection, error cases

### policy.rs - Policy Engine (510 lines)
- `PolicyEngine` enforces transaction authorization rules
- Features: address whitelisting, daily limits, multi-approval thresholds, time-based restrictions
- Permissive and strict policy presets
- 14 unit tests with boundary condition coverage
- **This module is NOT simulated** -- it's production-viable policy logic

### mod.rs - Module declaration (20 lines)
- Re-exports all types, explicitly documents simulated nature

## What Is Simulated

| Component | Simulated Aspect | Production Requirement |
|-----------|-----------------|----------------------|
| Key generation | Random bytes + SHA-256 hash as "public key" | Real EC key generation (secp256k1/ed25519) |
| Secret sharing | GF(256) Shamir SSS (mathematically correct) | DKG protocol (no trusted dealer) |
| Partial signatures | SHA-256 hash as "signature" | ECDSA/EdDSA threshold signing (e.g., GG20, FROST) |
| Signature combination | Hash concatenation | MPC signature aggregation |
| Key storage | In-memory HashMap | HSM/encrypted database with access controls |
| Share refresh | Re-split from reconstructed secret | Proactive secret sharing (no secret reconstruction) |

## What Is NOT Simulated (Production-Viable)

| Component | Status |
|-----------|--------|
| Policy engine | Fully functional: whitelist, limits, time restrictions, multi-approval |
| Signing session workflow | Correct state machine: Pending -> Approved -> Signed / Rejected |
| 2-of-3 threshold logic | Correct threshold enforcement |
| Shamir SSS math | GF(256) arithmetic is mathematically correct (verified in tests) |

## Decision Criteria

### Security Model
- **Current**: The Shamir SSS implementation is mathematically correct but the overall system operates as a single-signer model because the "MPC" layer simulates cryptographic operations
- **Gap**: No real threshold ECDSA/EdDSA, no distributed key generation (DKG), no key share encryption at rest, no HSM integration
- **Risk**: Cannot be used for real custody without replacing core cryptographic primitives

### Auditability
- Code is clean, well-documented, 39 passing tests
- Simulated nature is explicitly documented in comments and module docstrings
- Policy engine can be audited independently (not simulated)
- **Gap**: Core crypto simulation makes security audit of MPC aspects meaningless

### Performance SLO
- In-memory implementation is performant but non-persistent
- Real MPC protocols (GG20, FROST) involve multiple network round-trips
- Performance characteristics would change fundamentally with real MPC

### Integration Cost
- Requires integrating a third-party MPC library (e.g., `multi-party-ecdsa`, `threshold-bls`, `frost-secp256k1`)
- All 3 share holders need to run as separate services or processes
- Need: key backup/recovery, HSM integration, audit logging, key rotation protocol
- Estimated effort: 4-8 weeks including security review

## Options

### Path A: Real MPC Integration
- **Requires**: Third-party MPC library (e.g., ZenGo's `multi-party-ecdsa` for GG20, or FROST implementation)
- **Changes**: Replace `mpc_key.rs` key generation with DKG, replace `mpc_signing.rs` with real threshold ECDSA
- **Keep**: Policy engine (already production-viable), session workflow structure
- **Timeline**: 4-8 weeks (library integration + testing + security audit)
- **Risk**: High complexity, requires dedicated security audit, network protocol design for share holders

### Path B: Explicit Scope Downgrade
- **Action**: Relabel F11 from "Simulated" to "Planned (Post-MVP)"
- **Keep**: All existing code as development/testing infrastructure
- **Rationale**: Single-signer custody is production-viable for initial launch with appropriate operational controls
- **Impact**: No production functionality affected; custody uses standard key management

## Recommendation

**Path B recommended for MVP.**

Rationale:
1. Real MPC requires dedicated security audit (cost: $50K-100K+)
2. GG20/FROST integration is non-trivial and introduces new attack surface
3. Single-signer with proper key management (HSM, backup) is acceptable for initial production
4. The existing simulated code provides correct architecture patterns for future MPC integration
5. Policy engine is independently valuable and production-ready regardless of MPC status
