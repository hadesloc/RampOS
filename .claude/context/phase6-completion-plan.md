# Phase 6: Completion Plan - RampOS

**Created**: 2026-02-02
**Status**: Planning
**Goal**: Hoàn thiện 100% tất cả tính năng còn thiếu

---

## Executive Summary

Dựa trên audit codebase, còn thiếu các phần sau:

### Critical Gaps (Must Fix)

1. **AA API Routes** - Backend ramp-aa có logic nhưng chưa expose qua REST API
2. **On-chain Services** - Thiếu DepositService, WithdrawService trong ramp-core
3. **Temporal Workflows** - PayinWorkflow, TradeWorkflow chỉ là skeleton
4. **Frontend Portal API Integration** - UI có nhưng chưa connect API

### Medium Priority

5. **Request Validation Middleware** - ValidatedJson chưa apply đồng bộ
6. **Payout Reversal Logic** - Logic hoàn tiền khi bank reject
7. **Frontend Tests** - Chưa có unit/e2e tests cho frontend

### Low Priority

8. **Task Breakdown Sync** - Cập nhật task-breakdown.json đồng bộ với thực tế

---

## Task Breakdown

### T-6.1: AA API Routes (Critical)
**Estimated**: 4h | **Assignee**: worker-agent

- [ ] Tạo `crates/ramp-api/src/handlers/aa.rs`
- [ ] Implement endpoints:
  - POST `/v1/aa/accounts` - Create smart wallet
  - GET `/v1/aa/accounts/:address` - Get wallet info
  - POST `/v1/aa/user-operations` - Submit UserOp
  - POST `/v1/aa/user-operations/estimate` - Estimate gas
- [ ] Register routes trong `router.rs`
- [ ] Add OpenAPI docs

### T-6.2: On-chain Services (Critical)
**Estimated**: 6h | **Assignee**: worker-agent

- [ ] Tạo `crates/ramp-core/src/service/deposit.rs`
- [ ] Tạo `crates/ramp-core/src/service/withdraw.rs`
- [ ] Integrate với ramp-aa crate
- [ ] Add ledger patterns cho on-chain operations
- [ ] Add blockchain event listener logic

### T-6.3: Complete Temporal Workflows (Critical)
**Estimated**: 4h | **Assignee**: worker-agent

- [ ] Implement PayinWorkflow activities với logic thực
- [ ] Implement TradeWorkflow activities với logic thực
- [ ] Add proper error handling và compensation
- [ ] Configure cho Temporal Server thật (không simulation)

### T-6.4: Frontend Portal API Integration (Critical)
**Estimated**: 6h | **Assignee**: worker-agent

- [ ] Connect KYC flow với backend API
- [ ] Connect Deposit/Withdraw forms với AA SDK
- [ ] Connect Transaction History với API
- [ ] Add AA Wallet display trong dashboard
- [ ] Implement WebAuthn/Passkey logic

### T-6.5: Request Validation Middleware (Medium)
**Estimated**: 2h | **Assignee**: worker-agent

- [ ] Apply ValidatedJson cho tất cả POST/PUT handlers
- [ ] Add validation rules cho DTOs
- [ ] Update OpenAPI với validation constraints

### T-6.6: Payout Reversal Logic (Medium)
**Estimated**: 2h | **Assignee**: worker-agent

- [ ] Implement reverse_funds trong PayoutWorkflow
- [ ] Add ledger pattern cho reversal
- [ ] Handle BANK_REJECTED state properly

### T-6.7: Frontend Tests (Medium)
**Estimated**: 4h | **Assignee**: worker-agent

- [ ] Setup Vitest cho frontend
- [ ] Add unit tests cho components
- [ ] Add integration tests cho API calls

### T-6.8: Documentation Sync (Low)
**Estimated**: 1h | **Assignee**: worker-agent

- [ ] Update task-breakdown.json
- [ ] Update dashboard.md
- [ ] Update current-state.md
- [ ] Create handoff document

---

## Dependencies

```
T-6.1 (AA API) ──┐
                 ├──> T-6.4 (Frontend Integration)
T-6.2 (Services) ┘

T-6.3 (Workflows) ──> Independent

T-6.5 (Validation) ──> Independent

T-6.6 (Reversal) ──> Independent

T-6.7 (Tests) ──> After T-6.4

T-6.8 (Docs) ──> After all tasks
```

---

## Parallel Execution Plan

**Batch 1** (Parallel):
- T-6.1: AA API Routes
- T-6.2: On-chain Services
- T-6.3: Temporal Workflows
- T-6.5: Validation Middleware
- T-6.6: Payout Reversal

**Batch 2** (After Batch 1):
- T-6.4: Frontend Integration
- T-6.7: Frontend Tests

**Batch 3** (Final):
- T-6.8: Documentation Sync

---

## Success Criteria

- [ ] All AA endpoints respond correctly
- [ ] Deposit/Withdraw flows work end-to-end
- [ ] All Temporal workflows have real logic
- [ ] Frontend connects to all APIs
- [ ] `cargo build` passes without errors
- [ ] `cargo test` passes
- [ ] `npm run build` passes for frontend
