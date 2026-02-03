# RampOS UI/UX Refactor - Product Specification

**Version:** 2.0
**Date:** 2026-02-03
**Status:** Approved for Implementation
**Document ID:** SPEC-UIUX-001
**Supersedes:** v1.0 (Backend Specification - retained in archive)

---

## 1. Executive Summary

This specification defines the comprehensive UI/UX refactor of RampOS to transform it from a functional prototype into a world-class fintech platform. The refactor will implement premium design patterns matching industry leaders like Stripe, Revolut, and Wise.

### 1.1 Vision Statement

> Transform RampOS into Vietnam's most trusted and visually compelling crypto exchange platform, where every interaction reinforces trust, professionalism, and financial security.

### 1.2 Project Scope

- **In Scope:** All frontend components, layouts, pages, and styling in `/frontend`
- **Out of Scope:** Backend logic, API changes, business logic modifications, landing page

---

## 2. Goals and Objectives

### 2.1 Primary Goals

| ID | Goal | Success Metric |
|----|------|----------------|
| G-001 | Premium Fintech Aesthetic | User feedback rating >= 4.5/5 |
| G-002 | WCAG AAA Accessibility | 100% compliance audit pass |
| G-003 | Consistent Design System | 0 design inconsistencies |
| G-004 | Performance Excellence | Lighthouse scores > 90 |
| G-005 | Zero Visual Bugs | 0 layout shifts, no broken states |

### 2.2 Secondary Goals

- Reduce cognitive load through clear visual hierarchy
- Establish strong brand identity through consistent color palette
- Improve perceived trustworthiness through professional typography
- Enable seamless dark/light mode transitions

---

## 3. Target User Experience

### 3.1 User Personas

#### Admin User (Back-office Operator)
- **Environment:** Dark mode OLED dashboard
- **Priorities:** Data density, quick scanning, efficient workflows
- **Key Metrics:** Tables, charts, compliance status, transaction monitoring

#### Portal User (End Customer)
- **Environment:** Light mode with dark option
- **Priorities:** Trust, clarity, ease of use, mobile-first
- **Key Actions:** Deposit, withdraw, view balances, complete KYC

### 3.2 Emotional Design Goals

| Emotion | Visual Strategy |
|---------|----------------|
| Trust | Navy blue primary, corporate typography, consistent borders |
| Security | Shield icons, verification badges, subtle lock iconography |
| Professionalism | Clean spacing, IBM Plex fonts, minimal decorative elements |
| Clarity | High contrast text, clear labels, obvious CTAs |
| Confidence | Smooth animations, instant feedback, progress indicators |

---

## 4. Design System Specification

### 4.1 Color Palette (Enterprise Gateway Pattern)

#### Primary Colors
| Token | Hex | HSL | Usage |
|-------|-----|-----|-------|
| `--primary` | #1E40AF | 221 72% 40% | Brand blue, primary actions |
| `--primary-light` | #3B82F6 | 217 91% 60% | Hover states, links |
| `--primary-foreground` | #FFFFFF | 0 0% 100% | Text on primary |

#### Semantic Colors
| Token | Hex | HSL | Usage |
|-------|-----|-----|-------|
| `--accent` | #10B981 | 160 84% 39% | Success, positive amounts |
| `--warning` | #F59E0B | 38 92% 50% | Pending, attention |
| `--destructive` | #EF4444 | 0 84% 60% | Errors, negative amounts |

#### Surface Colors (Dark Mode)
| Token | Hex | HSL | Usage |
|-------|-----|-----|-------|
| `--background` | #0F172A | 222 47% 11% | Page background |
| `--card` | #1E293B | 217 33% 17% | Card surfaces |
| `--muted` | #334155 | 215 25% 27% | Disabled, secondary |
| `--border` | #334155 | 215 25% 27% | Borders, dividers |

#### Surface Colors (Light Mode)
| Token | Hex | HSL | Usage |
|-------|-----|-----|-------|
| `--background` | #F8FAFC | 210 40% 98% | Page background |
| `--card` | #FFFFFF | 0 0% 100% | Card surfaces |
| `--muted` | #F1F5F9 | 210 40% 96% | Disabled, secondary |
| `--border` | #E2E8F0 | 214 32% 91% | Borders, dividers |

### 4.2 Typography

#### Font Family
```css
--font-heading: 'IBM Plex Sans', system-ui, sans-serif;
--font-body: 'IBM Plex Sans', system-ui, sans-serif;
--font-mono: 'IBM Plex Mono', 'SF Mono', monospace;
```

#### Type Scale
| Class | Size | Weight | Line Height | Usage |
|-------|------|--------|-------------|-------|
| `text-display` | 48px | 700 | 1.1 | Landing hero |
| `text-h1` | 36px | 700 | 1.2 | Page titles |
| `text-h2` | 30px | 600 | 1.3 | Section headers |
| `text-h3` | 24px | 600 | 1.4 | Card titles |
| `text-h4` | 20px | 500 | 1.4 | Subsections |
| `text-body` | 16px | 400 | 1.5 | Body text |
| `text-sm` | 14px | 400 | 1.5 | Labels, captions |
| `text-xs` | 12px | 400 | 1.4 | Badges, hints |

### 4.3 Elevation System

| Level | Shadow | Usage |
|-------|--------|-------|
| `shadow-xs` | 0 1px 2px rgba(0,0,0,0.05) | Subtle depth |
| `shadow-sm` | 0 1px 3px rgba(0,0,0,0.1) | Cards, inputs |
| `shadow-md` | 0 4px 6px rgba(0,0,0,0.1) | Dropdowns, popovers |
| `shadow-lg` | 0 10px 15px rgba(0,0,0,0.1) | Modals, dialogs |
| `shadow-xl` | 0 20px 25px rgba(0,0,0,0.15) | Toast notifications |

### 4.4 Border Radius Scale

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-sm` | 4px | Badges, chips |
| `--radius` | 8px | Buttons, inputs |
| `--radius-md` | 12px | Cards |
| `--radius-lg` | 16px | Modals, large cards |
| `--radius-full` | 9999px | Avatars, pills |

### 4.5 Spacing Scale

Uses Tailwind default scale (4px base):
- `space-1` = 4px
- `space-2` = 8px
- `space-3` = 12px
- `space-4` = 16px
- `space-6` = 24px
- `space-8` = 32px

---

## 5. Component Requirements

### 5.1 Core Components (FR-001 to FR-006)

| ID | Component | Priority | Requirements |
|----|-----------|----------|--------------|
| FR-001 | Button | P1 | 6 variants (default, secondary, outline, ghost, destructive, link), 4 sizes (sm, default, lg, icon), loading state with spinner, icon support left/right |
| FR-002 | Card | P1 | Elevation levels (1-4), header/content/footer slots, hover state option, gradient border option |
| FR-003 | Input | P1 | Icon slots (left/right), validation states (error, success), size variants, disabled state |
| FR-004 | Badge | P1 | 6 color variants (default, success, warning, destructive, info, outline), pill option, dot indicator |
| FR-005 | Avatar | P2 | Size variants (xs, sm, md, lg, xl), online indicator, fallback initials, image support |
| FR-006 | Skeleton | P1 | Shimmer animation, width/height props, text/circle/rect variants |

### 5.2 Layout Components (FR-007 to FR-011)

| ID | Component | Priority | Requirements |
|----|-----------|----------|--------------|
| FR-007 | Sidebar | P1 | Collapsible (72px collapsed), mobile responsive drawer, active/hover states, icon + text items, section dividers |
| FR-008 | PortalSidebar | P1 | Same as Sidebar but for user portal, user avatar section at bottom |
| FR-009 | PageHeader | P1 | Breadcrumbs, title, description, actions slot (right), responsive |
| FR-010 | PageContainer | P1 | Max-width 1400px, responsive padding, proper margins |
| FR-011 | Footer | P3 | Links, copyright, social icons, responsive |

### 5.3 Dashboard Components (FR-012 to FR-016)

| ID | Component | Priority | Requirements |
|----|-----------|----------|--------------|
| FR-012 | StatCard | P1 | Icon slot, value, label, trend indicator (+/-%), sparkline option, loading skeleton |
| FR-013 | ChartContainer | P1 | Title, description, consistent theming, loading state, error state |
| FR-014 | DataTable | P1 | Sort headers, filter row, pagination, row selection, loading skeleton rows |
| FR-015 | ActivityFeed | P2 | Timeline layout, status icons, timestamps, expandable details |
| FR-016 | KPICard | P2 | Large number display, trend arrow, comparison period |

### 5.4 Portal Components (FR-017 to FR-021)

| ID | Component | Priority | Requirements |
|----|-----------|----------|--------------|
| FR-017 | WalletCard | P1 | Premium gradient design, address display with copy, deployed status, network badge |
| FR-018 | BalanceDisplay | P1 | VND/Crypto formatting, available/locked breakdown, refresh button |
| FR-019 | TransactionRow | P1 | Type icon, amount with color, status badge, timestamp, expandable details |
| FR-020 | KYCProgress | P2 | Step indicator (1-4), current step highlight, completion status per step |
| FR-021 | DepositWithdrawCard | P2 | Form container with instructions, QR code slot, amount input |

---

## 6. Animation Guidelines

### 6.1 Timing Functions

```css
--ease-in: cubic-bezier(0.4, 0, 1, 1);
--ease-out: cubic-bezier(0, 0, 0.2, 1);
--ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
```

### 6.2 Duration Scale

| Token | Value | Usage |
|-------|-------|-------|
| `duration-75` | 75ms | Micro-interactions |
| `duration-150` | 150ms | Button states |
| `duration-200` | 200ms | Card hover |
| `duration-300` | 300ms | Panel transitions |

### 6.3 Animation Rules

1. Always use `ease-out` for entering animations
2. Always use `ease-in` for exiting animations
3. Max duration 300ms for UI feedback
4. Respect `prefers-reduced-motion`
5. No continuous decorative animations
6. Skeleton shimmer is the only allowed loop

### 6.4 Keyframes Required

```css
@keyframes shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

@keyframes fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes slide-up {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}

@keyframes pulse-subtle {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}
```

---

## 7. Accessibility Requirements

### 7.1 Color Contrast

- Normal text: 7:1 minimum (WCAG AAA)
- Large text: 4.5:1 minimum (WCAG AAA)
- UI components: 3:1 minimum

### 7.2 Interactive Elements

- All buttons must have visible focus rings (`ring-2 ring-offset-2`)
- Focus order follows visual layout
- Skip links for main content
- No keyboard traps

### 7.3 Screen Reader Support

- Semantic HTML elements (`nav`, `main`, `article`, `section`)
- ARIA labels on icon-only buttons
- Live regions for dynamic content (`aria-live="polite"`)
- Form inputs with associated labels (`htmlFor`)

### 7.4 Motion

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## 8. Responsive Design

### 8.1 Breakpoints

| Name | Min Width | Target Devices |
|------|-----------|----------------|
| `sm` | 640px | Large phones |
| `md` | 768px | Tablets |
| `lg` | 1024px | Laptops |
| `xl` | 1280px | Desktops |
| `2xl` | 1536px | Large monitors |

### 8.2 Mobile-First Rules

1. Design for 375px first
2. Touch targets minimum 44x44px
3. No horizontal scroll
4. Collapsible navigation
5. Stack grids on mobile

---

## 9. Page Requirements

### 9.1 Admin Pages

| Page | Route | Key Components |
|------|-------|----------------|
| Dashboard | `/` | StatCard x6, ChartContainer, DataTable |
| Intents | `/intents` | DataTable with filters, StatusBadge |
| Users | `/users` | DataTable, Avatar, Badge |
| Compliance | `/compliance` | DataTable, KPICard, ActivityFeed |
| Ledger | `/ledger` | DataTable with balance columns |
| Webhooks | `/webhooks` | DataTable, StatusBadge |
| Settings | `/settings` | Form sections, Switch components |

### 9.2 Portal Pages

| Page | Route | Key Components |
|------|-------|----------------|
| Dashboard | `/portal` | WalletCard, BalanceDisplay x3, Quick Actions |
| Assets | `/portal/assets` | Asset list, BalanceDisplay |
| Deposit | `/portal/deposit` | DepositWithdrawCard, Instructions |
| Withdraw | `/portal/withdraw` | DepositWithdrawCard, Form |
| Transactions | `/portal/transactions` | TransactionRow list, Filters |
| KYC | `/portal/kyc` | KYCProgress, Form sections |
| Settings | `/portal/settings` | Profile form, Security settings |
| Login | `/portal/login` | Auth form, Brand header |
| Register | `/portal/register` | Auth form, Terms checkbox |

---

## 10. Success Metrics

### 10.1 Quantitative Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Lighthouse Performance | > 90 | Automated audit |
| Lighthouse Accessibility | > 95 | Automated audit |
| First Contentful Paint | < 1.5s | Core Web Vitals |
| Cumulative Layout Shift | < 0.1 | Core Web Vitals |
| WCAG AAA Compliance | 100% | Manual + automated audit |

### 10.2 Qualitative Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Visual consistency | 0 inconsistencies | Design review |
| Professional appearance | Comparable to Stripe | Stakeholder review |
| User trust signals | Increased | User feedback |

---

## 11. Quality Checklist

### 11.1 Visual Quality
- [ ] No emojis used as icons (use SVG: Lucide React)
- [ ] All icons from consistent icon set (Lucide)
- [ ] Hover states don't cause layout shift
- [ ] Use theme colors via CSS variables
- [ ] Consistent spacing using Tailwind scale

### 11.2 Interaction
- [ ] All clickable elements have `cursor-pointer`
- [ ] Hover states provide clear visual feedback
- [ ] Transitions are smooth (150-300ms)
- [ ] Focus states visible for keyboard navigation
- [ ] Loading states prevent double-submission

### 11.3 Light/Dark Mode
- [ ] Both modes have sufficient text contrast
- [ ] Borders visible in both modes
- [ ] Cards have proper elevation in both modes
- [ ] Charts themed for current mode

### 11.4 Layout
- [ ] Content properly spaced from fixed navbars
- [ ] Responsive at all breakpoints
- [ ] No content overflow or horizontal scroll
- [ ] Proper padding on mobile

---

## Appendix A: Reference Designs

### Stripe Dashboard
- Clean data tables with subtle row hovers
- Minimal shadow usage, relies on borders
- Clear typography hierarchy with Inter font
- Minimal color, uses blue sparingly

### Revolut App
- Bold numbers with large font weights
- Card-based layout with rounded corners
- Smooth page transitions
- Premium dark mode

### Wise Dashboard
- Trust-focused with green accents
- Clear status indicators
- Excellent mobile experience
- Professional IBM-style typography

---

*Document prepared by Planner Agent*
*All implementation tasks use model: sonnet*
