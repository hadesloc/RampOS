# RampOS Venue-Agnostic Connector and Cashflow Model

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Define a venue-agnostic architecture for on/off-ramp across chains and venues, explain why Hyperliquid should be treated as connector `#1` instead of a special-case platform, and clarify why sequencing starts with cash-out without excluding cash-in.

**Architecture:** The correct target is not a Hyperliquid-specific product. The correct target is a universal ramp control plane with a generic venue substrate, generic wallet and payout controls, and pluggable venue connectors for CEX, DEX, and perp venues. Hyperliquid is useful as the first proving-ground connector because it fits the existing wallet-centric off-ramp seams especially well, not because the core system should be shaped around it permanently.

**Tech Stack:** Rust workspace (`ramp-core`, `ramp-api`, `ramp-compliance`, `ramp-adapter`), PostgreSQL, NATS, Temporal, Next.js, widget/SDK shells, Python CLI, Solidity AA contracts.

---

## 1. Executive Answer

The answer to "do we need to build for Hyperliquid first?" is:

- **No** for the platform architecture
- **Yes** for the first connector rollout

Meaning:

- the platform must be venue-agnostic
- the first real connector should still be a specific venue
- Hyperliquid is a good first connector because it validates the generic model through a concrete real flow

The answer to "why do we only talk about cash-out?" is:

- we should **not** only build cash-out
- we should build **both** cash-out and cash-in
- but the first production wedge should be **cash-out before cash-in**

That is a sequencing choice, not a product limitation.

## 2. Core Product Decision

RampOS should model venue connectivity as:

1. `Universal ramp core`
2. `Generic venue substrate`
3. `Connector implementations`
4. `Jurisdiction-aware product policy`

The mistake to avoid is:

- building a Hyperliquid-shaped platform

The opposite mistake to avoid is:

- trying to design the perfect abstraction for all venues before shipping any real connector

The correct middle path is:

- define the minimum generic substrate
- implement one real connector against it
- let the connector pressure-test the abstraction

## 3. Layered Architecture

```text
+--------------------------------------------------------------------------------+
|                           RampOS Product Policy Layer                          |
|--------------------------------------------------------------------------------|
| user type | jurisdiction | allowed actions | disclosures | review thresholds   |
| retail    | pro          | cex            | dex         | perp                |
+--------------------------------------------------------------------------------+
                                       |
                                       v
+--------------------------------------------------------------------------------+
|                          RampOS Universal Venue Substrate                      |
|--------------------------------------------------------------------------------|
| wallet attestation | venue account link | venue transfer tracking              |
| source-of-funds    | beneficiary trust  | compliance eligibility               |
| payout routing     | RFQ/onramp routing | ledger + settlement + audit          |
+--------------------------------------------------------------------------------+
                     |                           |                           |
                     v                           v                           v
+---------------------------+   +---------------------------+   +---------------------------+
| DEX / Perp Connector      |   | CEX Connector             |   | Wallet / Self-Custody     |
|---------------------------|   |---------------------------|   |---------------------------|
| Hyperliquid               |   | Binance / OKX / Bybit     |   | EVM / Solana / TON       |
| Lighter                   |   | Coinbase / Kraken         |   | deposit ownership proof  |
| dYdX / others later       |   | others later              |   | wallet risk / address    |
+---------------------------+   +---------------------------+   +---------------------------+
                     |                           |                           |
                     +---------------------------+---------------------------+
                                                 |
                                                 v
+--------------------------------------------------------------------------------+
|                        Existing RampOS Core Execution Seams                    |
|--------------------------------------------------------------------------------|
| offramp | RFQ | settlement | provider routing | Travel Rule | KYB evidence    |
| ledger  | webhook | treasury | corridor packs | partner registry            |
+--------------------------------------------------------------------------------+
```

## 4. Shared Generic Model

These objects should exist independently of any one venue.

### 4.1 Universal objects

| Object | Purpose | Applies to |
|---|---|---|
| `WalletAttestation` | proves control and trust state of a self-custody wallet | DEX, perp, CEX withdrawal wallet |
| `VenueConnection` | represents a logical link between a user/org and a venue | all venues |
| `VenueAccount` | stores venue account identity, mode, and trust metadata | all venues |
| `VenueTransfer` | tracks movement between wallet and venue | all venues |
| `BeneficiaryProfile` | trusted payout destination with cooldown / verification state | all fiat exits |
| `SourceOfFundsPackage` | evidence bundle for review and compliance fast-lane decisions | all high-value flows |
| `ProductEligibilityDecision` | says whether a user may see or execute a flow | all product surfaces |

### 4.2 Venue-specific optional fields

These should be optional extensions, not baked into the core object shape.

| Venue trait | Examples |
|---|---|
| `api_key_mode` | CEX API keys, Hyperliquid API wallet, Lighter operator API |
| `subaccount_mode` | CEX subaccounts, Hyperliquid subaccounts |
| `public_pool_mode` | Lighter public pool operator linkage |
| `builder_mode` | Hyperliquid builder approvals and revenue controls |
| `proof_anchor_mode` | Lighter / protocol-specific proof or exit evidence |

## 5. Generic Cashflow Model

RampOS should support both directions.

### 5.1 Cash-out

This is the easiest first wedge.

```text
venue position / venue balance
        ->
venue withdraw
        ->
user self-custody wallet
        ->
wallet attestation + source-of-funds + destination trust checks
        ->
RFQ / off-ramp routing
        ->
fiat payout
        ->
ledger / settlement / webhook / audit
```

### 5.2 Cash-in

This is also part of the target architecture, but should follow after the wallet and destination trust layers are ready.

```text
fiat source
    ->
pay-in rail / partner / onramp quote
    ->
crypto delivered to approved user wallet
    ->
wallet ownership + jurisdiction + product eligibility checks
    ->
user signs venue deposit or venue transfer
    ->
venue receives funds
    ->
ledger / compliance / routing audit
```

### 5.3 Default rule

For early phases:

- `cash-out` should be `wallet-first`
- `cash-in` should also be `wallet-first`

Meaning:

- do not send partner or LP funds directly into a venue account when venue credit semantics depend on sender address or deposit attribution
- deliver to the user-controlled approved wallet first
- then let the user or approved automation move funds into the venue

## 6. Why Cash-Out Comes First

Cash-out is first because it is a better wedge, not because cash-in is unimportant.

### 6.1 Cash-out is better aligned with current repo seams

The repo already fits:

- wallet -> off-ramp intent
- RFQ route selection
- fiat payout
- settlement and audit

Relevant existing seams:

- `crates/ramp-api/src/handlers/portal/offramp.rs`
- `crates/ramp-core/src/service/rfq.rs`
- `crates/ramp-core/src/service/settlement.rs`
- `crates/ramp-compliance/src/provider_routing.rs`

### 6.2 Cash-out has clearer compliance framing

Cash-out naturally asks:

- where did the funds come from
- which wallet received them
- which bank account are they going to
- is this destination trusted

These are hard problems, but they are easier to frame and defend than:

- "we are helping retail users move fiat into leveraged perp venues"

### 6.3 Cash-in has more product and legal edge cases

Cash-in is harder because it adds:

- venue deposit attribution semantics
- wrong-network / wrong-asset user errors
- chain-specific funding constraints
- more aggressive regulatory concerns if the flow looks like a direct derivatives funnel
- additional mis-credit and support burden

So the correct statement is:

- `cash-out first`
- `cash-in second`
- `not cash-out only`

## 7. Connector Taxonomy

The connector layer should be family-based.

### 7.1 DEX / Perp wallet-centric connectors

Examples:

- Hyperliquid
- Lighter
- dYdX style connectors later

Typical characteristics:

- self-custody wallet is central
- venue funding often starts from wallet
- venue withdrawal often lands back in wallet
- read-only linking is usually possible earlier than write-side automation

### 7.2 CEX API / withdrawal connectors

Examples:

- Binance
- OKX
- Bybit
- Coinbase
- Kraken

Typical characteristics:

- API keys or OAuth-like scopes matter more
- subaccounts and permission scoping matter more
- withdrawal address allowlisting and cooldowns are critical
- custody and key-handling requirements are much stricter

### 7.3 Shared connector contract

Every connector should implement the same categories:

```text
AccountLinker
BalanceReader
DepositMonitor
WithdrawMonitor
TransferTracker
PolicyHintsProvider
EvidenceExporter
```

Optional:

```text
ApiKeyVaultAdapter
SubaccountAdapter
BuilderRevenueAdapter
PublicPoolAdapter
```

## 8. Proposed Repo Shape

Do not mutate `ramp-core` into a venue-specific tree.

Preferred additive structure:

```text
crates/
  ramp-venue/
    src/
      common/
        mod.rs
        model.rs
        linker.rs
        transfer.rs
        evidence.rs
        policy.rs
      hyperliquid/
        mod.rs
        account_linker.rs
        withdraw_monitor.rs
        deposit_monitor.rs
        builder_adapter.rs
      lighter/
        mod.rs
        account_linker.rs
        withdraw_monitor.rs
        deposit_monitor.rs
        public_pool_adapter.rs
      cex/
        mod.rs
        api_key_vault.rs
        withdrawal_allowlist.rs
        subaccount_adapter.rs
```

And new additive persistence should start after the current migration sequence, for example:

- `051_wallet_attestations.sql`
- `052_venue_connections.sql`
- `053_venue_accounts.sql`
- `054_beneficiary_profiles.sql`
- `055_venue_transfers.sql`
- `056_source_of_funds_packages.sql`

## 9. Rollout Order

The rollout should be:

### Stage 0: Generic substrate

Build first:

- wallet attestation
- venue connection
- venue account
- venue transfer
- beneficiary registry / cooldown
- source-of-funds package
- product eligibility decision

### Stage 1: Generic cash-out

Support:

- wallet-based crypto -> fiat payout
- without any venue-specific branding yet

This gives a universal base for:

- DEX users
- CEX withdrawal users
- treasury users

### Stage 2: Hyperliquid connector `#1`

Use Hyperliquid to validate:

- read-only account linking
- venue withdrawal monitoring
- venue-linked cash-out

### Stage 3: Generic cash-in

After the substrate and first cash-out wedge:

- fiat -> approved wallet
- wallet -> venue deposit

This is where the system proves it is not "cash-out only".

### Stage 4: Hyperliquid funding

Only after generic cash-in works:

- wallet-first funding to Hyperliquid
- asset and network guardrails
- failure UX

### Stage 5: Lighter pro / operator

After that:

- operator-grade account linking
- public pool support
- statement and evidence exports
- API key vault for institutional or pro modes

### Stage 6: CEX connector family

Then expand into:

- withdrawal allowlists
- scoped API key models
- subaccount models
- custody-safe operational controls

## 10. What Hyperliquid Is Actually For

Hyperliquid should be treated as:

- the first connector
- the first perp venue proving ground
- the fastest way to validate wallet-centric venue cash-out

Hyperliquid should **not** be treated as:

- the architecture
- the universal domain model
- the only venue type worth modeling

## 11. What "Any Chain, Any Venue" Really Means

It should mean:

- same control plane
- same compliance and payout rails
- same trust objects
- different connector implementations
- different policy and eligibility outcomes

It should **not** mean:

- one giant generic flow with no venue-specific branches

The system should be generic at the core and specific at the edge.

## 12. Immediate Design Decision

The immediate design decision should be:

1. keep the platform venue-agnostic
2. add a generic venue substrate
3. support both cash-out and cash-in in the target design
4. sequence cash-out first
5. ship Hyperliquid as connector `#1`
6. add cash-in after the substrate is stable

## 13. Summary

The right answer is:

- we are not building a Hyperliquid-specific platform
- we are building a venue-agnostic ramp control plane
- Hyperliquid is just the first real connector
- cash-out is first because it is the cleanest wedge
- cash-in is absolutely part of the design, but it comes right after the trust substrate is ready

In one line:

`generic venue substrate -> generic cash-out -> Hyperliquid cash-out -> generic cash-in -> Hyperliquid funding -> broader venue family`
