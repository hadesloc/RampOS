# RampOS Threat Model

## 1. System Overview
RampOS is a core banking and crypto-fiat ramp orchestration system. It manages ledgers, connects to banking rails, orchestrates crypto transactions, and handles compliance.

## 2. Assets
The primary assets we protect are:
*   **User Funds (Fiat & Crypto)**: The actual value held in custody or in transit.
*   **User Data (PII)**: KYC data, identity documents, transaction history.
*   **Private Keys**: Keys for signing crypto transactions, JWT signing keys, service account credentials.
*   **Ledger Integrity**: The correctness of balances and transaction records.
*   **System Availability**: The ability for the system to process transactions.

## 3. Threat Actors
*   **External Attackers**: Hackers attempting to steal funds or data.
*   **Malicious Insiders**: Employees with access attempting fraud.
*   **Compromised Users**: Legitimate users whose credentials have been stolen.
*   **Third-Party Providers**: Vulnerabilities in banking partners or blockchain nodes.

## 4. Attack Vectors & Mitigations

### 4.1. Network & API Attacks
*   **Vector**: DDoS attacks, API abuse, Replay attacks.
*   **Mitigation**:
    *   Cloudflare/WAF (planned).
    *   Rate limiting (Redis-backed, per-tenant).
    *   HMAC signatures with timestamps (prevent replays).
    *   Idempotency keys (prevent double processing).

### 4.2. Authentication & Authorization
*   **Vector**: Credential stuffing, Token theft, Privilege escalation.
*   **Mitigation**:
    *   JWT with short expiration.
    *   Role-Based Access Control (RBAC).
    *   Tenant isolation (Row-Level Security).
    *   API keys hashed (SHA-256).

### 4.3. Data & Ledger Integrity
*   **Vector**: SQL Injection, Race conditions (Double spend), Precision errors.
*   **Mitigation**:
    *   Use of ORM/Prepared statements (SQLx).
    *   Double-entry accounting (Debits = Credits).
    *   Database transactions with appropriate isolation levels.
    *   Decimal arithmetic (no floating point).
    *   Immutable ledger entries (append-only).

### 4.4. Crypto & Key Management
*   **Vector**: Private key leakage, Weak signatures.
*   **Mitigation**:
    *   Keys managed via HSM or secure vaults (e.g., AWS KMS / HashiCorp Vault - *Production Target*).
    *   Environment variables for secrets in dev/staging.
    *   Multi-signature / MPC wallets (for high value).
    *   ERC-4337 Account Abstraction (Social recovery, session keys).

### 4.5. Supply Chain & Dependencies
*   **Vector**: Malicious crates/packages, Vulnerable container images.
*   **Mitigation**:
    *   `Cargo.lock` and `package-lock.json` pinned versions.
    *   Automated scanning (`cargo audit`, `npm audit`).
    *   Minimal container images (distroless/alpine).

## 5. Known Limitations & Accepted Risks
*   **Centralized Database**: Currently relies on a single PostgreSQL instance (HA planned).
*   **Hot Wallet Risk**: While minimized, operational wallets must hold some funds.
*   **Provider Dependency**: Downtime of banking partners affects system availability.

## 6. Security Boundary
The security boundary includes the API service, the Worker nodes, and the Database. External banking APIs and Blockchains are considered untrusted zones.

---
*Last Updated: 2026-01-23*
