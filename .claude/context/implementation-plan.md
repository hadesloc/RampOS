# RampOS UI/UX Refactor - Implementation Plan

**Version:** 2.0
**Date:** 2026-02-03
**Status:** Approved for Implementation
**Document ID:** PLAN-UIUX-001
**Estimated Duration:** 5-7 days
**Model Assignment:** All tasks use `sonnet` for UI quality

---

## Executive Summary

This implementation plan details the systematic refactor of RampOS frontend to achieve world-class fintech UI/UX standards. The plan is organized into 6 phases with clear dependencies, quality gates, and risk mitigation strategies.

---

## Phase Overview

| Phase | Name | Tasks | Duration | Dependencies |
|-------|------|-------|----------|--------------|
| 1 | Foundation | T-001 to T-006 | 1 day | None |
| 2 | Core Components | T-007 to T-014 | 1.5 days | Phase 1 |
| 3 | Layout Components | T-015 to T-020 | 1 day | Phase 1 |
| 4 | Dashboard Components | T-021 to T-028 | 1 day | Phase 2, 3 |
| 5 | Portal Components | T-029 to T-036 | 1 day | Phase 2, 3 |
| 6 | Page Refactors | T-037 to T-055 | 2 days | Phase 4, 5 |

**Total Tasks:** 55
**Parallel Execution:** Phases 2 & 3 can run in parallel after Phase 1

---

## Phase 1: Foundation (T-001 to T-006)

### Overview
Establish the design system foundation including design tokens, fonts, and Tailwind configuration.

### Tasks

#### T-001: Fintech Color Palette CSS Variables
- **Description:** Update globals.css with new fintech color palette in HSL format for both light and dark modes
- **Files:** `frontend/src/app/globals.css`
- **Estimated Time:** 30 min

#### T-002: IBM Plex Font Integration
- **Description:** Add IBM Plex Sans and Mono fonts via Google Fonts, configure in layout and Tailwind
- **Files:** `frontend/src/app/layout.tsx`, `frontend/tailwind.config.ts`
- **Estimated Time:** 20 min

#### T-003: Elevation Shadow System
- **Description:** Add shadow-xs through shadow-2xl custom shadows to Tailwind config
- **Files:** `frontend/tailwind.config.ts`
- **Estimated Time:** 20 min

#### T-004: Border Radius Scale
- **Description:** Extend Tailwind border radius with fintech-appropriate values
- **Files:** `frontend/tailwind.config.ts`
- **Estimated Time:** 15 min

#### T-005: Animation Keyframes Library
- **Description:** Add shimmer, fade-in, slide-up, pulse-subtle keyframes and animations
- **Files:** `frontend/tailwind.config.ts`, `frontend/src/app/globals.css`
- **Estimated Time:** 30 min

#### T-006: Reduced Motion Support
- **Description:** Add prefers-reduced-motion media query styles
- **Files:** `frontend/src/app/globals.css`
- **Estimated Time:** 15 min

### Quality Gate
- [ ] All CSS variables defined for light and dark modes
- [ ] Fonts loading correctly (check Network tab)
- [ ] No build errors
- [ ] Existing components still render

---

## Phase 2: Core Components (T-007 to T-014)

### Overview
Refactor base UI components to match the new design system.

### Tasks

#### T-007: Button Component Refactor
- **Description:** Add loading state, success variant, improve focus rings, add icon positioning
- **Files:** `frontend/src/components/ui/button.tsx`
- **Estimated Time:** 45 min
- **Dependencies:** T-001

#### T-008: Card Component Enhancement
- **Description:** Add elevation prop, hover states, gradient border variant, header/footer slots
- **Files:** `frontend/src/components/ui/card.tsx`
- **Estimated Time:** 40 min
- **Dependencies:** T-001, T-003

#### T-009: Input Component Refactor
- **Description:** Add icon slots, validation states with colors, size variants
- **Files:** `frontend/src/components/ui/input.tsx`
- **Estimated Time:** 45 min
- **Dependencies:** T-001

#### T-010: Badge Component Enhancement
- **Description:** Add success, warning, info variants, dot indicator, pill option
- **Files:** `frontend/src/components/ui/badge.tsx`
- **Estimated Time:** 30 min
- **Dependencies:** T-001

#### T-011: Avatar Component Creation
- **Description:** Create Avatar with size variants, online indicator, fallback initials
- **Files:** `frontend/src/components/ui/avatar.tsx`
- **Estimated Time:** 35 min
- **Dependencies:** T-001, T-004

#### T-012: Skeleton Component Creation
- **Description:** Create Skeleton with shimmer animation, variants for text/circle/rect
- **Files:** `frontend/src/components/ui/skeleton.tsx` (new)
- **Estimated Time:** 40 min
- **Dependencies:** T-005

#### T-013: Alert Component Enhancement
- **Description:** Improve Alert styling with new colors, icon support, close button
- **Files:** `frontend/src/components/ui/alert.tsx`
- **Estimated Time:** 25 min
- **Dependencies:** T-001

#### T-014: Table Component Enhancement
- **Description:** Improve Table with hover rows, sticky header option, loading state
- **Files:** `frontend/src/components/ui/table.tsx`
- **Estimated Time:** 35 min
- **Dependencies:** T-001

### Quality Gate
- [ ] All components render without errors
- [ ] Dark mode works correctly
- [ ] Focus states visible
- [ ] Existing tests pass

---

## Phase 3: Layout Components (T-015 to T-020)

### Overview
Refactor navigation and layout components for premium feel.

### Tasks

#### T-015: Admin Sidebar Redesign
- **Description:** Add collapsible state, improve hover/active states, add section dividers
- **Files:** `frontend/src/components/layout/sidebar.tsx`
- **Estimated Time:** 60 min
- **Dependencies:** T-001, T-002

#### T-016: Portal Sidebar Redesign
- **Description:** Match Admin sidebar styling, improve user section at bottom
- **Files:** `frontend/src/components/layout/portal-sidebar.tsx`
- **Estimated Time:** 50 min
- **Dependencies:** T-015

#### T-017: Page Header Component
- **Description:** Create reusable PageHeader with breadcrumbs, title, description, actions slot
- **Files:** `frontend/src/components/layout/page-header.tsx` (new)
- **Estimated Time:** 40 min
- **Dependencies:** T-001, T-002

#### T-018: Page Container Component
- **Description:** Create PageContainer wrapper with max-width, responsive padding
- **Files:** `frontend/src/components/layout/page-container.tsx` (new)
- **Estimated Time:** 25 min
- **Dependencies:** T-001

#### T-019: Admin Layout Refinement
- **Description:** Update admin layout with improved spacing, transitions
- **Files:** `frontend/src/app/(admin)/layout.tsx`
- **Estimated Time:** 35 min
- **Dependencies:** T-015, T-018

#### T-020: Portal Layout Refinement
- **Description:** Update portal layout with improved spacing, mobile responsiveness
- **Files:** `frontend/src/app/portal/layout.tsx`
- **Estimated Time:** 35 min
- **Dependencies:** T-016, T-018

### Quality Gate
- [ ] Navigation works on all breakpoints
- [ ] Sidebar collapses correctly
- [ ] Mobile menu functions
- [ ] No layout shifts

---

## Phase 4: Dashboard Components (T-021 to T-028)

### Overview
Create and enhance dashboard-specific components.

### Tasks

#### T-021: StatCard Component
- **Description:** Create StatCard with icon, value, label, trend indicator, loading skeleton
- **Files:** `frontend/src/components/dashboard/stat-card.tsx` (new)
- **Estimated Time:** 45 min
- **Dependencies:** T-008, T-012

#### T-022: ChartContainer Component
- **Description:** Create wrapper for Recharts with consistent theming and loading state
- **Files:** `frontend/src/components/dashboard/chart-container.tsx` (new)
- **Estimated Time:** 40 min
- **Dependencies:** T-008

#### T-023: Enhanced DataTable Component
- **Description:** Create DataTable wrapper with sort, filter, pagination, row selection
- **Files:** `frontend/src/components/dashboard/data-table.tsx` (new)
- **Estimated Time:** 60 min
- **Dependencies:** T-014, T-012

#### T-024: ActivityFeed Component
- **Description:** Create timeline-style activity feed with status icons
- **Files:** `frontend/src/components/dashboard/activity-feed.tsx` (new)
- **Estimated Time:** 45 min
- **Dependencies:** T-010, T-011

#### T-025: KPICard Component
- **Description:** Create KPI card with large number, trend arrow, comparison text
- **Files:** `frontend/src/components/dashboard/kpi-card.tsx` (new)
- **Estimated Time:** 35 min
- **Dependencies:** T-008

#### T-026: StatusBadge Component
- **Description:** Create specialized badge for transaction/intent statuses
- **Files:** `frontend/src/components/dashboard/status-badge.tsx` (new)
- **Estimated Time:** 25 min
- **Dependencies:** T-010

#### T-027: QuickStats Grid Component
- **Description:** Create responsive grid layout for stat cards
- **Files:** `frontend/src/components/dashboard/quick-stats.tsx` (new)
- **Estimated Time:** 30 min
- **Dependencies:** T-021

#### T-028: RecentActivity Table
- **Description:** Create styled recent activity table with status badges
- **Files:** `frontend/src/components/dashboard/recent-activity.tsx` (new)
- **Estimated Time:** 40 min
- **Dependencies:** T-023, T-026

### Quality Gate
- [ ] All dashboard components render correctly
- [ ] Loading states work
- [ ] Responsive on all breakpoints
- [ ] Dark mode looks premium

---

## Phase 5: Portal Components (T-029 to T-036)

### Overview
Create portal-specific components for end users.

### Tasks

#### T-029: WalletCard Component
- **Description:** Create premium wallet card with gradient, address copy, status badge
- **Files:** `frontend/src/components/portal/wallet-card.tsx` (new)
- **Estimated Time:** 50 min
- **Dependencies:** T-008, T-010

#### T-030: BalanceDisplay Component
- **Description:** Create balance display with VND/crypto formatting, breakdown
- **Files:** `frontend/src/components/portal/balance-display.tsx` (new)
- **Estimated Time:** 40 min
- **Dependencies:** T-008

#### T-031: TransactionRow Component
- **Description:** Create transaction list item with type icon, amount, status, timestamp
- **Files:** `frontend/src/components/portal/transaction-row.tsx` (new)
- **Estimated Time:** 45 min
- **Dependencies:** T-010, T-026

#### T-032: KYCProgress Component
- **Description:** Create KYC step indicator with status colors and completion marks
- **Files:** `frontend/src/components/portal/kyc-progress.tsx` (new)
- **Estimated Time:** 40 min
- **Dependencies:** T-001

#### T-033: DepositCard Component
- **Description:** Create deposit flow card with instructions and QR code slot
- **Files:** `frontend/src/components/portal/deposit-card.tsx` (new)
- **Estimated Time:** 45 min
- **Dependencies:** T-008, T-009

#### T-034: WithdrawCard Component
- **Description:** Create withdraw flow card with form and validation
- **Files:** `frontend/src/components/portal/withdraw-card.tsx` (new)
- **Estimated Time:** 45 min
- **Dependencies:** T-008, T-009

#### T-035: AssetRow Component
- **Description:** Create asset list item with icon, name, balance
- **Files:** `frontend/src/components/portal/asset-row.tsx` (new)
- **Estimated Time:** 35 min
- **Dependencies:** T-030

#### T-036: QuickActions Component
- **Description:** Create quick action buttons grid (Deposit, Withdraw, etc.)
- **Files:** `frontend/src/components/portal/quick-actions.tsx` (new)
- **Estimated Time:** 30 min
- **Dependencies:** T-007

### Quality Gate
- [ ] All portal components render correctly
- [ ] Currency formatting works
- [ ] Copy to clipboard works
- [ ] Mobile layouts correct

---

## Phase 6: Page Refactors (T-037 to T-055)

### Overview
Refactor all pages to use new components and design system.

### Admin Pages

#### T-037: Admin Dashboard Page Refactor
- **Description:** Refactor dashboard with StatCard, ChartContainer, RecentActivity
- **Files:** `frontend/src/app/(admin)/page.tsx`
- **Estimated Time:** 60 min
- **Dependencies:** T-021, T-022, T-028

#### T-038: Intents Page Refactor
- **Description:** Refactor with DataTable, StatusBadge, PageHeader
- **Files:** `frontend/src/app/(admin)/intents/page.tsx`
- **Estimated Time:** 50 min
- **Dependencies:** T-023, T-026, T-017

#### T-039: Users Page Refactor
- **Description:** Refactor with DataTable, Avatar, Badge, PageHeader
- **Files:** `frontend/src/app/(admin)/users/page.tsx`
- **Estimated Time:** 50 min
- **Dependencies:** T-023, T-011, T-017

#### T-040: Compliance Page Refactor
- **Description:** Refactor with DataTable, ActivityFeed, KPICard
- **Files:** `frontend/src/app/(admin)/compliance/page.tsx`
- **Estimated Time:** 55 min
- **Dependencies:** T-023, T-024, T-025

#### T-041: Ledger Page Refactor
- **Description:** Refactor with enhanced DataTable styling
- **Files:** `frontend/src/app/(admin)/ledger/page.tsx`
- **Estimated Time:** 45 min
- **Dependencies:** T-023, T-017

#### T-042: Webhooks Page Refactor
- **Description:** Refactor with DataTable, StatusBadge
- **Files:** `frontend/src/app/(admin)/webhooks/page.tsx`
- **Estimated Time:** 45 min
- **Dependencies:** T-023, T-026

#### T-043: Admin Settings Page Refactor
- **Description:** Refactor with improved form styling, sections
- **Files:** `frontend/src/app/(admin)/settings/page.tsx`
- **Estimated Time:** 40 min
- **Dependencies:** T-009, T-017

### Portal Pages

#### T-044: Portal Dashboard Page Refactor
- **Description:** Refactor with WalletCard, BalanceDisplay, QuickActions
- **Files:** `frontend/src/app/portal/page.tsx`
- **Estimated Time:** 60 min
- **Dependencies:** T-029, T-030, T-036

#### T-045: Portal Assets Page Refactor
- **Description:** Refactor with AssetRow, BalanceDisplay
- **Files:** `frontend/src/app/portal/assets/page.tsx`
- **Estimated Time:** 45 min
- **Dependencies:** T-035, T-030

#### T-046: Portal Deposit Page Refactor
- **Description:** Refactor with DepositCard, improved instructions
- **Files:** `frontend/src/app/portal/deposit/page.tsx`
- **Estimated Time:** 50 min
- **Dependencies:** T-033

#### T-047: Portal Withdraw Page Refactor
- **Description:** Refactor with WithdrawCard, form validation styling
- **Files:** `frontend/src/app/portal/withdraw/page.tsx`
- **Estimated Time:** 50 min
- **Dependencies:** T-034

#### T-048: Portal Transactions Page Refactor
- **Description:** Refactor with TransactionRow list, filters
- **Files:** `frontend/src/app/portal/transactions/page.tsx`
- **Estimated Time:** 55 min
- **Dependencies:** T-031

#### T-049: Portal KYC Page Refactor
- **Description:** Refactor with KYCProgress, improved form sections
- **Files:** `frontend/src/app/portal/kyc/page.tsx`
- **Estimated Time:** 55 min
- **Dependencies:** T-032, T-009

#### T-050: Portal Settings Page Refactor
- **Description:** Refactor with improved form styling, sections
- **Files:** `frontend/src/app/portal/settings/page.tsx`
- **Estimated Time:** 45 min
- **Dependencies:** T-009, T-017

#### T-051: Portal Login Page Refactor
- **Description:** Premium login form with brand styling
- **Files:** `frontend/src/app/portal/login/page.tsx`
- **Estimated Time:** 50 min
- **Dependencies:** T-007, T-009, T-008

#### T-052: Portal Register Page Refactor
- **Description:** Premium register form matching login
- **Files:** `frontend/src/app/portal/register/page.tsx`
- **Estimated Time:** 45 min
- **Dependencies:** T-051

### Final Polish

#### T-053: Dark Mode Audit and Fixes
- **Description:** Audit all pages/components for dark mode issues, fix any problems
- **Files:** Multiple files
- **Estimated Time:** 60 min
- **Dependencies:** T-037 to T-052

#### T-054: Accessibility Audit and Fixes
- **Description:** Run accessibility audit, fix focus states, ARIA labels, contrast
- **Files:** Multiple files
- **Estimated Time:** 60 min
- **Dependencies:** T-053

#### T-055: Performance Optimization
- **Description:** Optimize bundle size, lazy load components, check Lighthouse
- **Files:** Multiple files
- **Estimated Time:** 45 min
- **Dependencies:** T-054

### Quality Gate
- [ ] All pages render correctly
- [ ] No console errors
- [ ] Lighthouse Performance > 90
- [ ] Lighthouse Accessibility > 95
- [ ] All existing tests pass
- [ ] Dark/Light mode works perfectly

---

## Risk Mitigation

### Risk 1: Breaking Existing Functionality
- **Mitigation:** Run existing tests after each phase
- **Rollback:** Git commits after each task completion

### Risk 2: Design Inconsistency
- **Mitigation:** Use design tokens exclusively, no hardcoded values
- **Review:** Visual review at each phase gate

### Risk 3: Performance Regression
- **Mitigation:** Lighthouse audit at Phase 6
- **Fix:** Optimize images, lazy load non-critical components

### Risk 4: Accessibility Regression
- **Mitigation:** Dedicated accessibility audit task (T-054)
- **Tools:** axe-core, manual keyboard testing

---

## Dependencies Diagram

```
Phase 1 (Foundation)
    |
    +---> Phase 2 (Core Components) --+
    |                                  |
    +---> Phase 3 (Layout Components) -+
                                       |
                                       v
                    +---> Phase 4 (Dashboard) --+
                    |                           |
                    +---> Phase 5 (Portal) -----+
                                                |
                                                v
                                    Phase 6 (Page Refactors)
```

---

## Success Criteria

1. **Visual:** Matches premium fintech standards (Stripe, Revolut, Wise)
2. **Performance:** Lighthouse scores > 90
3. **Accessibility:** WCAG AAA compliance
4. **Consistency:** 0 design inconsistencies
5. **Stability:** All existing tests pass
6. **Quality:** Zero visual bugs or layout shifts

---

## Handoff Requirements

Each task must produce:
1. Updated/created files
2. Test verification (if tests exist)
3. Visual verification screenshot (optional)
4. Handoff file: `.claude/handoffs/T-XXX.md`

---

*Plan prepared by Planner Agent*
*All tasks assigned model: sonnet*
