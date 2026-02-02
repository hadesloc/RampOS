# RampOS - Frontend Expansion Plan v1.0

**Document Type**: Implementation Plan
**Version**: 1.0
**Date**: 2026-01-28
**Status**: Draft

---

## 1. Executive Summary

This plan outlines the expansion of the RampOS frontend to include a comprehensive **User Portal** for end-users, a high-impact **Landing Page** for marketing, and polish improvements for the existing **Admin Dashboard**. This expansion aims to provide a complete user experience from onboarding to transaction management, while ensuring security and compliance integration.

---

## 2. Architecture & Tech Stack

### 2.1 Common Stack
- **Framework**: Next.js 14+ (App Router)
- **Styling**: Tailwind CSS
- **UI Components**: Shadcn UI (Radix UI + Tailwind)
- **State Management**: React Query (TanStack Query) + Zustand
- **Form Handling**: React Hook Form + Zod
- **Icons**: Lucide React
- **Animations**: Framer Motion
- **Charts**: Recharts

### 2.2 User Portal Specifics
- **Auth**: WebAuthn/Passkey (primary), Magic Link (fallback)
- **Security**: Content Security Policy (CSP), CSRF protection, HttpOnly cookies

### 2.3 Landing Page Specifics
- **Optimization**: SEO-first, high performance (Lighthouse 100)
- **Animations**: Scroll-triggered animations, interactive elements

---

## 3. Implementation Phases

### Phase 1: Foundation & Landing Page (Week 1)

#### 1.1 Project Structure Setup
- [ ] Initialize monorepo structure (if moving to monorepo) or separate apps within `frontend/`
  - `frontend/apps/admin` (existing)
  - `frontend/apps/user-portal` (new)
  - `frontend/apps/landing` (new)
  - `frontend/packages/ui` (shared components)
- [ ] Configure shared tooling (ESLint, Prettier, Tailwind config)
- [ ] Set up CI/CD for new apps

#### 1.2 Shared UI Library
- [ ] Extract common Shadcn UI components to shared package
- [ ] Define design system tokens (colors, typography, spacing)
- [ ] Create shared hooks (API client, auth hooks)

#### 1.3 Landing Page Development
- [ ] **Hero Section**: High-impact visual, clear value prop, CTA
- [ ] **Features Section**: Security, Compliance, Speed cards with animations
- [ ] **How it Works**: Step-by-step visual flow
- [ ] **Developer API**: Code snippet showcase
- [ ] **CTA Section**: "Start Building" / "Contact Sales"
- [ ] **Footer**: Links, legal, social

### Phase 2: User Portal Core (Week 2-3)

#### 2.1 Authentication & Onboarding
- [ ] Implement WebAuthn/Passkey registration & login
- [ ] Build KYC onboarding flow (steps: Identity, Document Upload, Liveness)
- [ ] Create User Dashboard layout (Sidebar/Topnav)

#### 2.2 Wallet & Assets
- [ ] **Asset Dashboard**: Total balance (VND + Crypto), asset breakdown chart
- [ ] **Deposit UI**:
  - VND: Virtual account display, copy-to-clipboard
  - Crypto: QR code, address display
- [ ] **Withdraw UI**:
  - VND: Bank account selector, amount input, 2FA/Passkey confirmation
  - Crypto: Address input, network selector, gas estimation display

#### 2.3 Transaction Management
- [ ] **Transaction History**: DataTable with filters (Type, Status, Date)
- [ ] **Transaction Details**: Modal/Page with detailed status tracking (timeline)
- [ ] **Export**: CSV/PDF export functionality

### Phase 3: Admin Dashboard Polish (Week 3)

#### 3.1 UI/UX Improvements
- [ ] **Dark Mode**: Verify and fix dark mode support across all pages
- [ ] **Navigation**: Improve sidebar organization and mobile responsiveness
- [ ] **Data Visualization**: Enhance charts on the main dashboard (volume, user growth)

#### 3.2 Feature Enhancements
- [ ] **Advanced Filters**: Add complex filtering to Intent and User tables
- [ ] **Bulk Actions**: Enable bulk status updates or exports
- [ ] **Activity Logs**: detailed view of admin actions

---

## 4. Task Breakdown

### 4.1 Shared Foundation
| ID | Task | Description | Priority |
|----|------|-------------|----------|
| F-001 | Setup Monorepo/Structure | Reorganize frontend folder for multiple apps | High |
| F-002 | Extract UI Components | Move Shadcn UI components to shared lib | High |
| F-003 | Setup Shared Config | Tailwind, TypeScript, ESLint shared configs | Medium |

### 4.2 Landing Page
| ID | Task | Description | Priority |
|----|------|-------------|----------|
| L-001 | Hero Section | Implement responsive hero with animations | High |
| L-002 | Features Section | Cards grid with hover effects | Medium |
| L-003 | How It Works | Interactive step flow | Medium |
| L-004 | API Section | Syntax highlighted code block component | Low |

### 4.3 User Portal
| ID | Task | Description | Priority |
|----|------|-------------|----------|
| U-001 | Auth Integration | WebAuthn implementation | Critical |
| U-002 | Dashboard Layout | Shell with nav and responsive structure | High |
| U-003 | KYC Flow | Multi-step form for identity verification | Critical |
| U-004 | Asset Overview | Balance cards and charts | High |
| U-005 | Deposit/Withdraw | Forms with validation and API integration | Critical |
| U-006 | Transaction History | Filterable table with status badges | Medium |
| U-007 | Settings Profile | User profile and security settings | Low |

### 4.4 Admin Dashboard Polish
| ID | Task | Description | Priority |
|----|------|-------------|----------|
| A-001 | Dark Mode Fixes | Ensure all components support dark mode | Medium |
| A-002 | Chart Upgrade | Improve Recharts visualizations | Low |
| A-003 | Table Enhancements | Add density toggle, column visibility | Low |

---

## 5. Security Considerations

- **Authentication**: Rely on WebAuthn where possible to minimize credential theft risk.
- **XSS Prevention**: Strict CSP headers, sanitize all user inputs.
- **Data Protection**: Mask sensitive data (PII) in the UI unless explicitly revealed by user.
- **State Management**: Clear sensitive state on logout or timeout.

## 6. Success Criteria

1. **User Portal**: Users can sign up, complete KYC, deposit, and withdraw without errors.
2. **Landing Page**: Loads in < 1.5s (LCP), scores > 95 on SEO.
3. **Admin Dashboard**: Dark mode works flawlessly, data visualization is interactive.
