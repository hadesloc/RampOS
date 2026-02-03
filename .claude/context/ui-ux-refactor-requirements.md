# RampOS UI/UX Refactor Requirements

## Project Overview

**Project Name:** RampOS UI/UX Professional Refactor
**Goal:** Transform RampOS into a world-class fintech platform with premium, professional UI/UX matching industry leaders like Stripe, Revolut, and Wise.
**Date:** 2026-02-03
**Status:** Discovery Complete

---

## Current State Analysis

### Tech Stack
- **Framework:** Next.js 14+ with App Router
- **Styling:** Tailwind CSS + shadcn/ui + Radix UI
- **Icons:** Lucide React
- **Charts:** Recharts
- **Fonts:** Inter (Google Fonts)
- **State:** React hooks + Context API

### Current Issues Identified
1. **Design System:** Using default shadcn/ui Zinc theme - generic, not fintech-optimized
2. **Color Palette:** Grayscale-heavy, lacks brand identity and fintech trust signals
3. **Typography:** Single font (Inter) - functional but not premium fintech feel
4. **Visual Hierarchy:** Inconsistent spacing, card treatments, and component styling
5. **Animations:** Minimal to none - feels static and dated
6. **Dark Mode:** Basic implementation, needs refinement for premium feel
7. **Data Visualization:** Basic Recharts usage without consistent theming
8. **Loading States:** Simple spinners, needs skeleton screens and smooth transitions

---

## Target Design System (World-Class Fintech)

### Pattern: Enterprise Gateway
- Corporate Navy/Grey color strategy
- High integrity, conservative accents
- Trust signals prominent throughout
- Professional mega-menu navigation

### Style: Dark Mode (OLED) + Light Mode Professional
- Dark theme with high contrast for admin dashboard
- Light mode option for user portal
- WCAG AAA accessibility compliance

### Color Palette
| Role | Hex | CSS Variable | Usage |
|------|-----|--------------|-------|
| Primary | #1E40AF | --primary | Brand blue - trust, stability |
| Primary Light | #3B82F6 | --primary-light | Interactive hover states |
| Accent/Success | #10B981 | --accent | Positive actions, success |
| Warning | #F59E0B | --warning | Attention states |
| Destructive | #EF4444 | --destructive | Errors, negative actions |
| Background Dark | #0F172A | --background-dark | Dashboard dark mode |
| Background Light | #F8FAFC | --background-light | Portal light sections |
| Card Dark | #1E293B | --card-dark | Elevated surfaces dark |
| Card Light | #FFFFFF | --card-light | Elevated surfaces light |
| Border | #334155 | --border | Subtle borders |
| Text Primary | #F8FAFC | --text-primary | Main text (dark mode) |
| Text Muted | #94A3B8 | --text-muted | Secondary text |

### Typography
- **Heading:** IBM Plex Sans (700, 600, 500)
- **Body:** IBM Plex Sans (400, 500)
- **Mono:** IBM Plex Mono (for wallet addresses, amounts, code)
- **Mood:** Financial, trustworthy, professional, corporate

```css
@import url('https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600&family=IBM+Plex+Sans:wght@300;400;500;600;700&display=swap');
```

### Visual Effects
- Subtle shadows with proper elevation system (4 levels)
- Smooth transitions (150-300ms, ease-out)
- Skeleton loading screens with shimmer effect
- Micro-interactions on buttons and cards
- Respect `prefers-reduced-motion`
- Subtle glow effects for active states

### Animation Guidelines
- Use `ease-out` for entering animations
- Use `ease-in` for exiting animations
- Duration: 150-300ms for micro-interactions
- Use `animate-spin` only for loading indicators
- Check `prefers-reduced-motion` media query
- No continuous decorative animations

---

## Component Upgrades Required

### Phase 1: Foundation (Design Tokens & Theme)
- [ ] T-001: New CSS variables for fintech color palette
- [ ] T-002: IBM Plex font family integration
- [ ] T-003: Elevation/shadow system (shadow-xs to shadow-2xl)
- [ ] T-004: Consistent border-radius scale
- [ ] T-005: Animation keyframes library
- [ ] T-006: Update tailwind.config.ts with new theme

### Phase 2: Core Components
- [ ] T-007: Button variants (primary, secondary, ghost, destructive)
- [ ] T-008: Card component with elevation levels
- [ ] T-009: Input fields with icons and validation states
- [ ] T-010: Badge component with status colors
- [ ] T-011: Avatar component with online status
- [ ] T-012: Skeleton loading components

### Phase 3: Layout Components
- [ ] T-013: Admin Sidebar redesign with collapse state
- [ ] T-014: Portal Sidebar redesign
- [ ] T-015: Page header pattern with breadcrumbs
- [ ] T-016: Floating navigation refinements
- [ ] T-017: Footer component

### Phase 4: Dashboard Components
- [ ] T-018: Stat cards with icons and trends
- [ ] T-019: Chart container with consistent theming
- [ ] T-020: Data table with enhanced styling
- [ ] T-021: Activity feed component
- [ ] T-022: KPI cards with sparklines

### Phase 5: Portal-Specific
- [ ] T-023: Wallet card redesign (premium fintech style)
- [ ] T-024: Balance display with currency formatting
- [ ] T-025: Transaction list with status badges
- [ ] T-026: KYC progress stepper
- [ ] T-027: Deposit/Withdraw flow cards

### Phase 6: Page Refactors
- [ ] T-028: Admin Dashboard page
- [ ] T-029: Portal Dashboard page
- [ ] T-030: Login/Register pages
- [ ] T-031: Deposit/Withdraw pages
- [ ] T-032: Transaction history page
- [ ] T-033: KYC page
- [ ] T-034: Admin Users page
- [ ] T-035: Admin Intents page
- [ ] T-036: Admin Compliance page

---

## Quality Requirements

### Performance
- Lighthouse Performance > 90
- First Contentful Paint < 1.5s
- Cumulative Layout Shift < 0.1
- No layout shift on skeleton-to-content transitions

### Accessibility
- WCAG AAA compliance (4.5:1 contrast minimum)
- All interactive elements keyboard accessible
- Focus states visible and styled
- `aria-label` on icon-only buttons
- Form inputs have associated labels
- `prefers-reduced-motion` respected

### Responsiveness
- Mobile-first design approach
- Breakpoints: 375px, 768px, 1024px, 1440px
- No horizontal scroll on any viewport
- Touch targets minimum 44x44px

### Code Quality
- TypeScript strict mode compliance
- No ESLint errors/warnings
- Consistent component patterns (shadcn/ui style)
- Proper prop typing with TypeScript
- No hardcoded colors/values (use design tokens)
- Components follow single responsibility principle

---

## Pre-Delivery Checklist

### Visual Quality
- [ ] No emojis used as icons (use SVG: Lucide React)
- [ ] All icons from consistent icon set (Lucide)
- [ ] Hover states don't cause layout shift
- [ ] Use theme colors via CSS variables
- [ ] Consistent spacing using Tailwind scale

### Interaction
- [ ] All clickable elements have `cursor-pointer`
- [ ] Hover states provide clear visual feedback
- [ ] Transitions are smooth (150-300ms)
- [ ] Focus states visible for keyboard navigation
- [ ] Loading states prevent double-submission

### Light/Dark Mode
- [ ] Both modes have sufficient text contrast
- [ ] Borders visible in both modes
- [ ] Cards have proper elevation in both modes
- [ ] Charts themed for current mode

### Layout
- [ ] Content properly spaced from fixed navbars
- [ ] Responsive at all breakpoints
- [ ] No content overflow or horizontal scroll
- [ ] Proper padding on mobile

---

## Agent Assignment Strategy

| Task Category | Agent Type | Model |
|--------------|------------|-------|
| Design tokens & theme | sonnet | Fast, accurate CSS work |
| Component refactors | sonnet | UI work - produces beautiful code |
| Page refactors | sonnet | Complex UI composition |
| Testing & validation | haiku | Fast verification |
| Code review | sonnet | Quality assurance |

**Note:** User specified using **sonnet** agents for UI work as they produce beautiful results.

---

## Success Criteria

1. UI matches premium fintech standards (Stripe, Revolut, Wise aesthetic)
2. All pages responsive and accessible (WCAG AAA)
3. Consistent design system across all views
4. Smooth animations and transitions throughout
5. Fast loading with skeleton screens
6. Dark/Light mode working flawlessly
7. Zero visual bugs or layout shifts
8. All existing tests pass
9. Lighthouse scores > 90 on all metrics
10. User approval on final design
