# RampOS Frontend Architecture

**Version:** 2.0 (UI/UX Refactor)
**Date:** 2026-02-03
**Status:** Approved for Implementation

---

## 1. Overview

This document defines the frontend architecture for the RampOS UI/UX refactor. The architecture follows a component-driven design pattern with strict separation of concerns, ensuring scalability, maintainability, and consistency.

---

## 2. Directory Structure

```
frontend/
├── src/
│   ├── app/                          # Next.js App Router pages
│   │   ├── (admin)/                  # Admin dashboard routes (grouped)
│   │   │   ├── layout.tsx            # Admin layout wrapper
│   │   │   ├── page.tsx              # Admin dashboard
│   │   │   ├── intents/
│   │   │   ├── users/
│   │   │   ├── compliance/
│   │   │   ├── ledger/
│   │   │   ├── webhooks/
│   │   │   └── settings/
│   │   ├── portal/                   # User portal routes
│   │   │   ├── layout.tsx            # Portal layout wrapper
│   │   │   ├── page.tsx              # Portal dashboard
│   │   │   ├── login/
│   │   │   ├── register/
│   │   │   ├── assets/
│   │   │   ├── deposit/
│   │   │   ├── withdraw/
│   │   │   ├── transactions/
│   │   │   ├── kyc/
│   │   │   └── settings/
│   │   ├── globals.css               # Global styles + design tokens
│   │   └── layout.tsx                # Root layout
│   │
│   ├── components/                   # Reusable components
│   │   ├── ui/                       # Base UI primitives (shadcn/ui)
│   │   │   ├── button.tsx
│   │   │   ├── card.tsx
│   │   │   ├── input.tsx
│   │   │   ├── badge.tsx
│   │   │   ├── avatar.tsx
│   │   │   ├── skeleton.tsx
│   │   │   ├── table.tsx
│   │   │   ├── dialog.tsx
│   │   │   ├── dropdown-menu.tsx
│   │   │   ├── tabs.tsx
│   │   │   ├── alert.tsx
│   │   │   ├── toast.tsx
│   │   │   └── ...
│   │   │
│   │   ├── layout/                   # Layout components
│   │   │   ├── sidebar.tsx           # Admin sidebar
│   │   │   ├── portal-sidebar.tsx    # User portal sidebar
│   │   │   ├── page-header.tsx       # Page header with breadcrumbs
│   │   │   ├── page-container.tsx    # Page content wrapper
│   │   │   └── footer.tsx            # Footer component
│   │   │
│   │   ├── dashboard/                # Dashboard-specific components
│   │   │   ├── stat-card.tsx         # KPI stat card
│   │   │   ├── chart-container.tsx   # Chart wrapper
│   │   │   ├── data-table.tsx        # Enhanced data table
│   │   │   ├── activity-feed.tsx     # Activity timeline
│   │   │   ├── kpi-card.tsx          # Large KPI display
│   │   │   ├── status-badge.tsx      # Transaction status badge
│   │   │   ├── quick-stats.tsx       # Stats grid
│   │   │   └── recent-activity.tsx   # Recent activity table
│   │   │
│   │   └── portal/                   # Portal-specific components
│   │       ├── wallet-card.tsx       # Wallet display card
│   │       ├── balance-display.tsx   # Balance with breakdown
│   │       ├── transaction-row.tsx   # Transaction list item
│   │       ├── kyc-progress.tsx      # KYC step indicator
│   │       ├── deposit-card.tsx      # Deposit flow card
│   │       ├── withdraw-card.tsx     # Withdraw flow card
│   │       ├── asset-row.tsx         # Asset list item
│   │       └── quick-actions.tsx     # Quick action buttons
│   │
│   ├── contexts/                     # React contexts
│   │   ├── auth-context.tsx          # Authentication state
│   │   ├── theme-context.tsx         # Theme (dark/light) state
│   │   └── wallet-context.tsx        # Wallet state
│   │
│   ├── hooks/                        # Custom React hooks
│   │   ├── use-auth.ts
│   │   ├── use-wallet.ts
│   │   ├── use-theme.ts
│   │   └── use-media-query.ts
│   │
│   ├── lib/                          # Utility libraries
│   │   ├── utils.ts                  # cn() and other utilities
│   │   ├── api.ts                    # Admin API client
│   │   ├── portal-api.ts             # Portal API client
│   │   └── formatters.ts             # Currency/date formatters
│   │
│   └── types/                        # TypeScript type definitions
│       ├── api.ts                    # API response types
│       ├── components.ts             # Component prop types
│       └── index.ts                  # Re-exports
│
├── public/                           # Static assets
│   ├── fonts/                        # Local font files
│   └── images/                       # Images and icons
│
├── tailwind.config.ts                # Tailwind configuration
├── next.config.js                    # Next.js configuration
├── tsconfig.json                     # TypeScript configuration
└── package.json                      # Dependencies
```

---

## 3. Component Hierarchy

### 3.1 Layered Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Pages                                 │
│  (app/(admin)/page.tsx, app/portal/page.tsx, etc.)          │
├─────────────────────────────────────────────────────────────┤
│                    Layout Components                         │
│  (Sidebar, PageHeader, PageContainer)                       │
├─────────────────────────────────────────────────────────────┤
│                  Composite Components                        │
│  (StatCard, WalletCard, DataTable, TransactionRow)          │
├─────────────────────────────────────────────────────────────┤
│                   Base UI Primitives                         │
│  (Button, Card, Input, Badge, Avatar, Skeleton)             │
├─────────────────────────────────────────────────────────────┤
│                    Design Tokens                             │
│  (CSS Variables, Tailwind Theme)                            │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Component Categories

| Category | Purpose | Example Components |
|----------|---------|-------------------|
| **Primitives** | Base UI elements, highly reusable | Button, Card, Input, Badge |
| **Layout** | Page structure and navigation | Sidebar, PageHeader, Footer |
| **Composite** | Domain-specific, built from primitives | StatCard, WalletCard |
| **Pages** | Full page compositions | Dashboard, Deposit, KYC |

---

## 4. State Management

### 4.1 State Strategy

| State Type | Solution | Example |
|------------|----------|---------|
| **Server State** | React Query (tanstack/query) | API data, user info |
| **UI State** | React useState/useReducer | Modal open, form values |
| **Global State** | React Context | Auth, Theme, Wallet |
| **URL State** | Next.js searchParams | Filters, pagination |

### 4.2 Context Structure

```tsx
// Auth Context
interface AuthContextType {
  user: User | null;
  wallet: Wallet | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (email: string) => Promise<void>;
  logout: () => void;
  createWallet: () => Promise<void>;
  refreshWallet: () => Promise<void>;
}

// Theme Context
interface ThemeContextType {
  theme: 'light' | 'dark' | 'system';
  setTheme: (theme: 'light' | 'dark' | 'system') => void;
}
```

---

## 5. Styling Architecture

### 5.1 Design Token Flow

```
CSS Variables (globals.css)
         ↓
Tailwind Theme (tailwind.config.ts)
         ↓
Component Styles (className)
         ↓
Runtime (browser)
```

### 5.2 Styling Layers

1. **CSS Variables** - Define color values, spacing, etc.
2. **Tailwind Config** - Map variables to utility classes
3. **CVA Variants** - Component variant definitions
4. **className** - Applied Tailwind classes

### 5.3 Class Organization (BEM-like with Tailwind)

```tsx
// Pattern: Layout → Spacing → Typography → Colors → Effects → States
<button className={cn(
  // Layout
  "inline-flex items-center justify-center",
  // Spacing
  "px-4 py-2 gap-2",
  // Typography
  "text-sm font-medium",
  // Colors
  "bg-primary text-primary-foreground",
  // Effects
  "shadow-sm rounded-md",
  // States
  "hover:bg-primary/90 focus-visible:ring-2",
  // Transitions
  "transition-colors duration-150",
  className
)} />
```

---

## 6. Data Flow

### 6.1 Admin Dashboard

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Backend   │────▶│   api.ts     │────▶│   React     │
│   (Rust)    │     │   (fetch)    │     │   Query     │
└─────────────┘     └──────────────┘     └──────────────
                                                │
                                                ▼
                                         ┌─────────────┐
                                         │  Components │
                                         └─────────────┘
```

### 6.2 User Portal

```
┌─────────────┐     ┌───────────────┐     ┌─────────────┐
│   Backend   │────▶│ portal-api.ts │────▶│   Auth      │
│   (Rust)    │     │   (fetch)     │     │   Context   │
└─────────────┘     └───────────────┘     └──────────────
                                                │
                                                ▼
                                         ┌─────────────┐
                                         │   Portal    │
                                         │   Pages     │
                                         └─────────────┘
```

---

## 7. Component Design Principles

### 7.1 Composition Over Configuration

```tsx
// Good: Composable
<Card>
  <CardHeader>
    <CardTitle>Title</CardTitle>
  </CardHeader>
  <CardContent>Content</CardContent>
</Card>

// Avoid: Over-configured
<Card
  title="Title"
  content="Content"
  showHeader={true}
  headerStyle="default"
/>
```

### 7.2 Prop Patterns

```tsx
// Variant props with CVA
interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement>,
  VariantProps<typeof buttonVariants> {
  asChild?: boolean;
  isLoading?: boolean;
}

// Compound components
const Card = React.forwardRef<HTMLDivElement, CardProps>(...);
Card.displayName = "Card";
const CardHeader = React.forwardRef<HTMLDivElement, CardHeaderProps>(...);
CardHeader.displayName = "CardHeader";
```

### 7.3 Accessibility Patterns

```tsx
// Icon buttons must have aria-label
<Button variant="ghost" size="icon" aria-label="Copy to clipboard">
  <Copy className="h-4 w-4" />
</Button>

// Form inputs must have labels
<div className="space-y-2">
  <Label htmlFor="email">Email</Label>
  <Input id="email" type="email" />
</div>
```

---

## 8. Performance Considerations

### 8.1 Component Loading

| Strategy | When to Use |
|----------|-------------|
| **Eager** | Above-the-fold, critical path |
| **Lazy** | Modals, drawers, heavy charts |
| **Suspense** | Async components, data loading |

### 8.2 Optimization Patterns

```tsx
// Memoize expensive components
const MemoizedChart = React.memo(ChartComponent);

// Lazy load modals
const ConfirmDialog = React.lazy(() => import('./confirm-dialog'));

// Suspense boundaries
<Suspense fallback={<Skeleton />}>
  <HeavyComponent />
</Suspense>
```

### 8.3 Image Optimization

- Use Next.js `<Image>` component
- Serve WebP format with fallbacks
- Lazy load below-fold images
- Define explicit width/height

---

## 9. Error Handling

### 9.1 Error Boundaries

```tsx
// Page-level error boundary
export default function Error({
  error,
  reset,
}: {
  error: Error;
  reset: () => void;
}) {
  return (
    <div className="flex flex-col items-center justify-center h-full">
      <h2>Something went wrong!</h2>
      <Button onClick={() => reset()}>Try again</Button>
    </div>
  );
}
```

### 9.2 Loading States

```tsx
// Page-level loading
export default function Loading() {
  return (
    <div className="space-y-4">
      <Skeleton className="h-8 w-48" />
      <div className="grid gap-4 md:grid-cols-3">
        <Skeleton className="h-32" />
        <Skeleton className="h-32" />
        <Skeleton className="h-32" />
      </div>
    </div>
  );
}
```

---

## 10. Testing Strategy

### 10.1 Test Pyramid

| Level | Tool | Coverage |
|-------|------|----------|
| **Unit** | Vitest | Components, utils, hooks |
| **Integration** | Testing Library | Component interactions |
| **E2E** | Playwright | Critical user flows |

### 10.2 Component Testing Pattern

```tsx
import { render, screen } from '@testing-library/react';
import { Button } from './button';

describe('Button', () => {
  it('renders with correct text', () => {
    render(<Button>Click me</Button>);
    expect(screen.getByRole('button')).toHaveTextContent('Click me');
  });

  it('shows loading state', () => {
    render(<Button isLoading>Submit</Button>);
    expect(screen.getByRole('button')).toBeDisabled();
  });
});
```

---

## 11. Build and Deploy

### 11.1 Build Pipeline

```
Source → TypeScript Check → ESLint → Build → Test → Deploy
```

### 11.2 Environment Configuration

| Variable | Purpose |
|----------|---------|
| `NEXT_PUBLIC_API_URL` | Backend API URL |
| `NEXT_PUBLIC_WS_URL` | WebSocket URL |
| `NEXT_PUBLIC_CHAIN_ID` | Blockchain network |

---

*Document prepared by Planner Agent*
*Architecture follows Next.js 14+ App Router patterns*
