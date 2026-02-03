# RampOS Frontend Code Conventions

**Version:** 2.0 (UI/UX Refactor)
**Date:** 2026-02-03
**Status:** Approved for Implementation

---

## 1. Overview

This document establishes coding conventions for the RampOS frontend UI/UX refactor. All developers and agents must follow these conventions to ensure consistency, maintainability, and quality.

---

## 2. TypeScript Conventions

### 2.1 General Rules

```typescript
// Use strict mode
// tsconfig.json: "strict": true

// Use type imports
import type { ButtonProps } from "./types";
import { Button } from "./button";

// Prefer interfaces for component props
interface CardProps {
  title: string;
  children: React.ReactNode;
}

// Use type for unions/primitives
type Variant = "default" | "destructive" | "outline";
type Size = "sm" | "default" | "lg";
```

### 2.2 Component Typing

```typescript
// Use React.forwardRef for all UI primitives
const Button = React.forwardRef<
  HTMLButtonElement,
  ButtonProps
>(({ className, variant, size, ...props }, ref) => {
  // ...
});
Button.displayName = "Button";

// Extend HTML attributes properly
interface InputProps
  extends React.InputHTMLAttributes<HTMLInputElement> {
  error?: string;
  icon?: React.ReactNode;
}
```

### 2.3 Props Interface Naming

```typescript
// Component name + "Props"
interface ButtonProps { }
interface CardProps { }
interface StatCardProps { }

// Context types
interface AuthContextType { }
interface ThemeContextType { }

// API response types (in types/api.ts)
interface ApiResponse<T> { }
interface PaginatedResponse<T> { }
```

---

## 3. React Conventions

### 3.1 Component Structure

```typescript
// 1. Imports (external, then internal, then types)
import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

// 2. Type definitions
interface ButtonProps extends ... { }

// 3. Variant definitions (if using CVA)
const buttonVariants = cva("...", { ... });

// 4. Component definition
const Button = React.forwardRef<...>(({ ... }, ref) => {
  // Hooks first
  const [state, setState] = React.useState();

  // Derived values
  const computedValue = ...;

  // Event handlers
  const handleClick = () => { };

  // Render
  return ( ... );
});

// 5. Display name
Button.displayName = "Button";

// 6. Exports
export { Button, buttonVariants };
export type { ButtonProps };
```

### 3.2 Hook Rules

```typescript
// Custom hooks start with "use"
function useAuth() { }
function useTheme() { }
function useMediaQuery() { }

// Hooks at the top of the component
const Component = () => {
  // All hooks first
  const [state, setState] = useState();
  const router = useRouter();
  const { user } = useAuth();

  // Then derived values and handlers
  // Then render
};
```

### 3.3 Event Handler Naming

```typescript
// Prefix with "handle" for internal handlers
const handleClick = () => { };
const handleSubmit = () => { };
const handleChange = () => { };

// Prefix with "on" for props
interface Props {
  onClick?: () => void;
  onSubmit?: (data: FormData) => void;
  onChange?: (value: string) => void;
}
```

---

## 4. Tailwind CSS Conventions

### 4.1 Class Ordering

Follow this order for Tailwind classes:

```
1. Layout (display, position, flex/grid)
2. Sizing (width, height, min/max)
3. Spacing (margin, padding, gap)
4. Typography (font, text)
5. Colors (background, text color, border color)
6. Borders (border, rounded)
7. Effects (shadow, opacity)
8. States (hover, focus, active)
9. Transitions (transition, duration)
10. Responsive (sm:, md:, lg:)
```

Example:
```tsx
<div className={cn(
  // Layout
  "flex items-center justify-between",
  // Sizing
  "w-full h-16",
  // Spacing
  "px-4 py-2 gap-4",
  // Typography
  "text-sm font-medium",
  // Colors
  "bg-card text-card-foreground",
  // Borders
  "border rounded-lg",
  // Effects
  "shadow-sm",
  // States
  "hover:bg-accent hover:text-accent-foreground",
  // Transitions
  "transition-colors duration-150",
  // Responsive
  "md:px-6 lg:h-20"
)}>
```

### 4.2 Use Design Tokens

```tsx
// GOOD: Use semantic color variables
<div className="bg-card text-card-foreground border-border" />
<button className="bg-primary text-primary-foreground" />
<span className="text-muted-foreground" />
<div className="text-destructive" />

// BAD: Hardcoded colors
<div className="bg-zinc-900 text-white" />
<button className="bg-blue-600 text-white" />
```

### 4.3 Spacing Scale

```tsx
// Use consistent spacing scale
// 1 = 4px, 2 = 8px, 3 = 12px, 4 = 16px, 6 = 24px, 8 = 32px

// GOOD
<div className="p-4 space-y-4" />
<div className="gap-6" />

// BAD: Arbitrary values unless truly necessary
<div className="p-[17px]" />
```

### 4.4 Responsive Patterns

```tsx
// Mobile-first approach
<div className="
  grid grid-cols-1      // Mobile: single column
  md:grid-cols-2        // Tablet: two columns
  lg:grid-cols-4        // Desktop: four columns
  gap-4
" />

// Hide/show at breakpoints
<div className="hidden md:block" />  // Hidden on mobile
<div className="block md:hidden" />  // Only on mobile
```

---

## 5. Component Patterns

### 5.1 Compound Components

```tsx
// Card compound component example
const Card = React.forwardRef<...>(...);
Card.displayName = "Card";

const CardHeader = React.forwardRef<...>(...);
CardHeader.displayName = "CardHeader";

const CardTitle = React.forwardRef<...>(...);
CardTitle.displayName = "CardTitle";

const CardContent = React.forwardRef<...>(...);
CardContent.displayName = "CardContent";

export { Card, CardHeader, CardTitle, CardContent };
```

### 5.2 Variant Pattern (CVA)

```tsx
import { cva, type VariantProps } from "class-variance-authority";

const buttonVariants = cva(
  // Base classes
  "inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        destructive: "bg-destructive text-destructive-foreground hover:bg-destructive/90",
        outline: "border border-input bg-background hover:bg-accent",
        ghost: "hover:bg-accent hover:text-accent-foreground",
      },
      size: {
        default: "h-10 px-4 py-2",
        sm: "h-9 rounded-md px-3",
        lg: "h-11 rounded-md px-8",
        icon: "h-10 w-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
);

interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}
```

### 5.3 Loading State Pattern

```tsx
interface ButtonProps {
  isLoading?: boolean;
  children: React.ReactNode;
}

const Button = ({ isLoading, children, disabled, ...props }) => (
  <button disabled={isLoading || disabled} {...props}>
    {isLoading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
    {children}
  </button>
);
```

### 5.4 Icon Usage

```tsx
// Import from lucide-react
import { ChevronRight, Copy, Check, Loader2 } from "lucide-react";

// Standard icon sizes
<Icon className="h-4 w-4" />   // Small (buttons, badges)
<Icon className="h-5 w-5" />   // Default
<Icon className="h-6 w-6" />   // Large
<Icon className="h-8 w-8" />   // Extra large (empty states)

// Icon-only buttons MUST have aria-label
<Button variant="ghost" size="icon" aria-label="Copy address">
  <Copy className="h-4 w-4" />
</Button>
```

---

## 6. Naming Conventions

### 6.1 Files and Directories

```
// Components: kebab-case
button.tsx
stat-card.tsx
page-header.tsx

// Pages: kebab-case in folders
app/(admin)/page.tsx
app/portal/login/page.tsx

// Hooks: camelCase with use prefix
useAuth.ts
useMediaQuery.ts

// Utils: camelCase
formatCurrency.ts
cn.ts

// Types: kebab-case
api-types.ts
component-types.ts
```

### 6.2 Variables and Functions

```typescript
// camelCase for variables and functions
const userName = "John";
const isLoading = true;
const handleClick = () => {};

// UPPER_SNAKE_CASE for constants
const MAX_RETRIES = 3;
const API_BASE_URL = "https://api.example.com";

// PascalCase for components and types
const StatCard = () => {};
interface ButtonProps {}
type Variant = "default" | "outline";
```

### 6.3 CSS Custom Properties

```css
/* kebab-case with category prefix */
--color-primary: ...;
--color-background: ...;
--spacing-md: ...;
--radius-lg: ...;
--shadow-sm: ...;
--font-heading: ...;
```

---

## 7. Accessibility Conventions

### 7.1 Interactive Elements

```tsx
// All buttons must be focusable with visible focus ring
<button className="focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2">

// Icon-only buttons need aria-label
<button aria-label="Close dialog">
  <X className="h-4 w-4" />
</button>

// Disabled state
<button disabled aria-disabled="true">
```

### 7.2 Form Inputs

```tsx
// All inputs must have labels
<div>
  <Label htmlFor="email">Email</Label>
  <Input id="email" type="email" />
</div>

// Error states with aria-describedby
<div>
  <Label htmlFor="password">Password</Label>
  <Input
    id="password"
    aria-describedby="password-error"
    aria-invalid={!!error}
  />
  {error && (
    <p id="password-error" className="text-destructive text-sm">
      {error}
    </p>
  )}
</div>
```

### 7.3 Semantic HTML

```tsx
// Use semantic elements
<nav>...</nav>
<main>...</main>
<article>...</article>
<section aria-labelledby="section-title">
  <h2 id="section-title">Section Title</h2>
</section>

// Use appropriate heading hierarchy
<h1>Page Title</h1>
<h2>Section Title</h2>
<h3>Subsection Title</h3>
```

---

## 8. Animation Conventions

### 8.1 Transition Durations

```tsx
// Use consistent durations
"duration-150"  // Fast: micro-interactions, button states
"duration-200"  // Default: card hover, panel toggle
"duration-300"  // Slow: modal open/close, page transitions
```

### 8.2 Easing Functions

```tsx
// Entering elements: ease-out
"transition-all ease-out"

// Exiting elements: ease-in
"transition-all ease-in"

// State changes: ease-in-out
"transition-colors ease-in-out"
```

### 8.3 Reduced Motion

```css
/* Always include in global styles */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## 9. Import Conventions

### 9.1 Import Order

```typescript
// 1. React
import * as React from "react";

// 2. External libraries (alphabetical)
import { cva } from "class-variance-authority";
import { Loader2 } from "lucide-react";

// 3. Internal aliases (alphabetical)
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

// 4. Relative imports
import { useLocalState } from "./hooks";

// 5. Type imports last
import type { VariantProps } from "class-variance-authority";
import type { ButtonProps } from "./types";
```

### 9.2 Path Aliases

```typescript
// Use @ alias for src directory
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useAuth } from "@/contexts/auth-context";

// Avoid relative paths for cross-directory imports
// BAD
import { Button } from "../../../components/ui/button";

// GOOD
import { Button } from "@/components/ui/button";
```

---

## 10. Testing Conventions

### 10.1 Test File Location

```
// Co-locate tests with components
components/
├── ui/
│   ├── button.tsx
│   ├── button.test.tsx
│   ├── __tests__/
│   │   └── button.test.tsx  // Alternative location
```

### 10.2 Test Naming

```typescript
describe("Button", () => {
  it("renders with correct text", () => {});
  it("calls onClick when clicked", () => {});
  it("shows loading state when isLoading is true", () => {});
  it("is disabled when disabled prop is true", () => {});
});
```

### 10.3 Test Structure

```typescript
import { render, screen, fireEvent } from "@testing-library/react";
import { Button } from "./button";

describe("Button", () => {
  // Arrange, Act, Assert pattern
  it("calls onClick handler when clicked", () => {
    // Arrange
    const handleClick = vi.fn();
    render(<Button onClick={handleClick}>Click me</Button>);

    // Act
    fireEvent.click(screen.getByRole("button"));

    // Assert
    expect(handleClick).toHaveBeenCalledTimes(1);
  });
});
```

---

## 11. Documentation Conventions

### 11.1 Component Documentation

```typescript
/**
 * StatCard displays a key metric with optional trend indicator.
 *
 * @example
 * <StatCard
 *   title="Total Revenue"
 *   value="$12,345"
 *   trend={{ value: 12, direction: "up" }}
 *   icon={<DollarSign className="h-4 w-4" />}
 * />
 */
export function StatCard({ title, value, trend, icon }: StatCardProps) {
  // ...
}
```

### 11.2 Prop Documentation

```typescript
interface StatCardProps {
  /** The label displayed above the value */
  title: string;

  /** The main metric value to display */
  value: string | number;

  /** Optional trend indicator showing change percentage */
  trend?: {
    value: number;
    direction: "up" | "down";
  };

  /** Optional icon displayed in the card header */
  icon?: React.ReactNode;
}
```

---

## 12. Quality Checklist

Before submitting any UI code:

- [ ] No TypeScript errors (`npm run type-check`)
- [ ] No ESLint warnings (`npm run lint`)
- [ ] Uses design tokens (no hardcoded colors)
- [ ] Follows class ordering convention
- [ ] All interactive elements have focus states
- [ ] All icon-only buttons have aria-label
- [ ] All form inputs have labels
- [ ] Component has displayName set
- [ ] Works in both light and dark mode
- [ ] Responsive at all breakpoints
- [ ] No layout shift on interaction
- [ ] Tests written and passing

---

*Document prepared by Planner Agent*
*All UI tasks use model: sonnet*
