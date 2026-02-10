# F09/F11 Decision Record

**Date:** 2026-02-10
**Status:** Decided
**Author:** Production Readiness Rebaseline Team

---

## F09 ZK-KYC

**Decision:** Path B - Scope Downgrade to Planned (Post-MVP)

**Current State (Evidence-Based):**
- `contracts/src/zk/ZkKycVerifier.sol` (91 lines): Simulated ZK proof verifier
  - Accepts any non-zero 32+ byte proof as valid
  - No real pairing checks, no verification key, no trusted setup
  - Comments explicitly state: "In production, this would be generated from a Circom/Groth16 circuit"
- `contracts/src/zk/ZkKycRegistry.sol` (255 lines): On-chain registry for verified KYC commitments
  - Role-based access control (admin, verifier roles)
  - Commitment registration, revocation, enumeration
  - `registerVerificationWithProof()` calls the simulated verifier
  - Registry logic itself is architecturally sound
- `contracts/test/ZkKyc.t.sol` (347 lines): 24 tests covering verifier + registry
  - All tests pass but validate simulated verification logic

**Rationale:**
- ZK proof pipeline requires circuit compilation toolchain (circom/snarkjs or Noir/Barretenberg) not yet integrated
- Verifier contract operates in simulated mode -- accepts any non-trivial byte sequence
- Production ZK-KYC requires: circuit design, trusted setup ceremony, verifier contract generation, proof generation service
- Estimated effort: 6-10 weeks including circuit design and audit

**Action:**
- Relabel F09 from "Simulated" to "Planned (Post-MVP)"
- Remove from production-readiness claims
- Keep existing contracts for development/testing reference
- Registry contract access control logic is reusable for production

**Impact:** No production functionality affected. KYC is currently handled via traditional API flow in `crates/ramp-compliance/`.

---

## F11 MPC Custody

**Decision:** Path B - Scope Downgrade to Planned (Post-MVP)

**Current State (Evidence-Based):**
- `crates/ramp-core/src/custody/mpc_key.rs` (587 lines): Simulated 2-of-3 Shamir Secret Sharing
  - GF(256) arithmetic is mathematically correct (verified by exhaustive inverse test)
  - Public key = SHA-256(secret) -- NOT real elliptic curve derivation
  - In-memory storage, no persistence
- `crates/ramp-core/src/custody/mpc_signing.rs` (557 lines): Threshold signing sessions
  - Correct state machine workflow (Pending -> Approved -> Signed)
  - Partial signatures are SHA-256 hashes, not real ECDSA/EdDSA
  - Signature combination is hash concatenation, not MPC aggregation
- `crates/ramp-core/src/custody/policy.rs` (510 lines): Policy engine
  - **Production-viable**: whitelisting, daily limits, time restrictions, multi-approval thresholds
  - 14 unit tests with boundary coverage
  - NOT simulated -- this is real, usable policy logic
- Total: 39 unit tests passing across the custody module

**Rationale:**
- MPC threshold signing requires external library integration (e.g., GG20, FROST) and dedicated security audit
- Current simulated module is architecturally correct but cryptographically meaningless for production custody
- Single-signer custody model with HSM is production-viable for initial launch
- Policy engine is independently production-ready

**Action:**
- Relabel F11 from "Simulated" to "Planned (Post-MVP)"
- Remove from production-readiness claims
- Policy engine remains production-ready independent of MPC status
- Keep simulated code as architecture reference for future MPC integration

**Impact:** Custody currently uses single-signer model which is production-viable for initial launch with proper operational controls (HSM, key backup, access controls).

---

## Status Ledger Updates

| Feature | Old Label | New Label | Rationale |
|---------|-----------|-----------|-----------|
| F09 | Simulated | Planned | ZK circuit toolchain not integrated |
| F11 | Simulated | Planned | MPC library integration + audit needed |

**Summary count changes:**
- Simulated: 2 -> 0
- Planned: 0 -> 2
- Partial: 14 (unchanged)
- Complete: 0 (unchanged)

---

## Future Roadmap

### F09 ZK-KYC (Post-MVP)
1. Select ZK framework (Circom+Groth16 or Noir+Barretenberg)
2. Design KYC circuit (prove age/nationality without revealing data)
3. Implement trusted setup or use universal SRS
4. Generate production verifier contract
5. Build proof generation service
6. Security audit of circuit + verifier

### F11 MPC Custody (Post-MVP)
1. Select MPC library (multi-party-ecdsa GG20 or FROST)
2. Design distributed key generation (DKG) protocol
3. Implement threshold signing with real ECDSA
4. Add HSM integration for share storage
5. Build key recovery and rotation procedures
6. Security audit of MPC implementation
