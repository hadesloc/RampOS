# RampOS Frontend Tech Stack

**Version:** 2.0 (UI/UX Refactor)
**Date:** 2026-02-03
**Status:** Approved for Implementation

---

## 1. Overview

This document details the frontend technology stack for the RampOS UI/UX refactor. The stack is optimized for developer experience, performance, and maintainability while enabling world-class fintech UI/UX.

---

## 2. Core Framework

### Next.js 14+

| Aspect | Detail |
|--------|--------|
| **Version** | 14.x (App Router) |
| **Rendering** | Server Components + Client Components |
| **Routing** | File-based App Router |
| **Data Fetching** | Server Actions, fetch with cache |

**Why Next.js:**
- Server-side rendering for SEO and performance
- App Router for modern React patterns
- Built-in optimization (images, fonts, scripts)
- Excellent TypeScript support
- Industry standard for React applications

```json
{
  "dependencies": {
    "next": "^14.0.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  }
}
```

---

## 3. Styling

### 3.1 Tailwind CSS

| Aspect | Detail |
|--------|--------|
| **Version** | 3.4.x |
| **Configuration** | Custom theme with design tokens |
| **Plugins** | tailwindcss-animate |

**Configuration Approach:**
- Extend default theme with fintech color palette
- Custom shadows for elevation system
- Custom border radius scale
- Animation keyframes for micro-interactions

```typescript
// tailwind.config.ts key extensions
{
  theme: {
    extend: {
      colors: {
        border: "hsl(var(--border))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        // ... semantic colors
      },
      fontFamily: {
        sans: ["var(--font-sans)", "system-ui", "sans-serif"],
        mono: ["var(--font-mono)", "ui-monospace", "monospace"],
      },
      boxShadow: {
        xs: "0 1px 2px rgba(0,0,0,0.05)",
        // ... elevation system
      },
    },
  },
}
```

### 3.2 shadcn/ui

| Aspect | Detail |
|--------|--------|
| **Approach** | Copy-paste components (not npm package) |
| **Base** | Radix UI primitives |
| **Styling** | Tailwind CSS + CVA variants |

**Why shadcn/ui:**
- Full control over component code
- Accessible by default (Radix primitives)
- Easy to customize and extend
- No dependency lock-in
- Beautiful, professional defaults

**Components Used:**
- Button, Card, Input, Badge, Avatar
- Table, Dialog, Dropdown, Tabs
- Alert, Toast, Progress, Skeleton
- Form (with react-hook-form integration)

### 3.3 Radix UI

| Aspect | Detail |
|--------|--------|
| **Version** | Latest stable |
| **Usage** | Via shadcn/ui components |

**Radix Primitives:**
- Accessible by default (ARIA, keyboard nav)
- Unstyled (we add Tailwind styles)
- Composable and flexible

---

## 4. Typography

### IBM Plex Font Family

| Font | Weight | Usage |
|------|--------|-------|
| IBM Plex Sans | 300, 400, 500, 600, 700 | Headings, body text |
| IBM Plex Mono | 400, 500, 600 | Wallet addresses, amounts, code |

**Loading Strategy:**

```typescript
// Option 1: Google Fonts (recommended)
import { IBM_Plex_Sans, IBM_Plex_Mono } from "next/font/google";

const plex = IBM_Plex_Sans({
  subsets: ["latin"],
  weight: ["300", "400", "500", "600", "700"],
  variable: "--font-sans",
  display: "swap",
});

const plexMono = IBM_Plex_Mono({
  subsets: ["latin"],
  weight: ["400", "500", "600"],
  variable: "--font-mono",
  display: "swap",
});

// Option 2: @fontsource (self-hosted)
// npm install @fontsource/ibm-plex-sans @fontsource/ibm-plex-mono
```

---

## 5. Icons

### Lucide React

| Aspect | Detail |
|--------|--------|
| **Version** | Latest stable |
| **Style** | Outlined, 24x24 base |
| **Tree-shaking** | Yes (import individual icons) |

```typescript
// Import individual icons for tree-shaking
import {
  ArrowRight,
  Check,
  Copy,
  Loader2,
  ChevronDown,
  Wallet,
  Shield,
  TrendingUp,
} from "lucide-react";

// Standard sizing
<Icon className="h-4 w-4" />  // Small
<Icon className="h-5 w-5" />  // Default
<Icon className="h-6 w-6" />  // Large
```

**Why Lucide:**
- Consistent design language
- Large icon library (1000+ icons)
- Excellent React support
- MIT license
- Same style as Radix icons

---

## 6. Data Visualization

### Recharts

| Aspect | Detail |
|--------|--------|
| **Version** | 2.x |
| **Types** | Line, Bar, Area, Pie |
| **Responsive** | Yes (ResponsiveContainer) |

```typescript
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";

// Theme-aware chart
<LineChart data={data}>
  <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
  <XAxis stroke="hsl(var(--muted-foreground))" />
  <YAxis stroke="hsl(var(--muted-foreground))" />
  <Line stroke="hsl(var(--primary))" />
</LineChart>
```

**Theming Strategy:**
- Use CSS variables for colors
- Consistent with design tokens
- Dark/light mode support

---

## 7. Forms

### React Hook Form + Zod

| Package | Purpose |
|---------|---------|
| react-hook-form | Form state management |
| zod | Schema validation |
| @hookform/resolvers | Zod integration |

```typescript
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";

const schema = z.object({
  email: z.string().email("Invalid email"),
  amount: z.number().positive("Must be positive"),
});

type FormData = z.infer<typeof schema>;

const { register, handleSubmit, formState: { errors } } = useForm<FormData>({
  resolver: zodResolver(schema),
});
```

---

## 8. State Management

### React Context + Hooks

| State Type | Solution |
|------------|----------|
| Auth state | AuthContext |
| Theme | next-themes |
| UI state | useState/useReducer |
| Server state | React Query (optional) |

**Current Implementation:**
```typescript
// Auth Context
const AuthContext = createContext<AuthContextType | null>(null);
export const useAuth = () => useContext(AuthContext);

// Theme (next-themes)
import { ThemeProvider } from "next-themes";
```

---

## 9. Animation

### Tailwind CSS + Framer Motion (Optional)

| Approach | Usage |
|----------|-------|
| Tailwind transitions | Simple state changes |
| CSS keyframes | Shimmer, pulse effects |
| Framer Motion | Complex page transitions |

**Built-in Animations:**
```css
/* globals.css */
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
```

```typescript
// tailwind.config.ts
animation: {
  shimmer: "shimmer 2s linear infinite",
  "fade-in": "fade-in 0.2s ease-out",
  "slide-up": "slide-up 0.3s ease-out",
}
```

---

## 10. Utilities

### Class Variance Authority (CVA)

```typescript
import { cva, type VariantProps } from "class-variance-authority";

const buttonVariants = cva("base-classes", {
  variants: {
    variant: { default: "...", outline: "..." },
    size: { sm: "...", default: "...", lg: "..." },
  },
  defaultVariants: {
    variant: "default",
    size: "default",
  },
});
```

### clsx + tailwind-merge (cn utility)

```typescript
// lib/utils.ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// Usage
<div className={cn(
  "base-class",
  condition && "conditional-class",
  className
)} />
```

---

## 11. Testing

| Tool | Purpose |
|------|---------|
| Vitest | Unit testing |
| @testing-library/react | Component testing |
| Playwright | E2E testing |

```json
{
  "devDependencies": {
    "vitest": "^1.0.0",
    "@testing-library/react": "^14.0.0",
    "@testing-library/jest-dom": "^6.0.0",
    "@vitejs/plugin-react": "^4.0.0"
  }
}
```

---

## 12. Development Tools

| Tool | Purpose |
|------|---------|
| TypeScript | Type safety |
| ESLint | Code linting |
| Prettier | Code formatting |
| Husky | Git hooks |

**ESLint Configuration:**
```json
{
  "extends": [
    "next/core-web-vitals",
    "plugin:@typescript-eslint/recommended"
  ],
  "rules": {
    "@typescript-eslint/no-unused-vars": "error",
    "react-hooks/exhaustive-deps": "warn"
  }
}
```

---

## 13. Package Summary

### Production Dependencies

```json
{
  "dependencies": {
    "next": "^14.0.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "@radix-ui/react-avatar": "^1.0.0",
    "@radix-ui/react-dialog": "^1.0.0",
    "@radix-ui/react-dropdown-menu": "^2.0.0",
    "@radix-ui/react-label": "^2.0.0",
    "@radix-ui/react-progress": "^1.0.0",
    "@radix-ui/react-select": "^2.0.0",
    "@radix-ui/react-slot": "^1.0.0",
    "@radix-ui/react-switch": "^1.0.0",
    "@radix-ui/react-tabs": "^1.0.0",
    "@radix-ui/react-toast": "^1.0.0",
    "class-variance-authority": "^0.7.0",
    "clsx": "^2.0.0",
    "lucide-react": "^0.300.0",
    "next-themes": "^0.2.0",
    "react-hook-form": "^7.0.0",
    "recharts": "^2.10.0",
    "sonner": "^1.0.0",
    "tailwind-merge": "^2.0.0",
    "zod": "^3.22.0"
  }
}
```

### Development Dependencies

```json
{
  "devDependencies": {
    "@types/node": "^20.0.0",
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@typescript-eslint/eslint-plugin": "^6.0.0",
    "@typescript-eslint/parser": "^6.0.0",
    "autoprefixer": "^10.4.0",
    "eslint": "^8.0.0",
    "eslint-config-next": "^14.0.0",
    "postcss": "^8.4.0",
    "tailwindcss": "^3.4.0",
    "tailwindcss-animate": "^1.0.0",
    "typescript": "^5.3.0",
    "vitest": "^1.0.0",
    "@testing-library/react": "^14.0.0"
  }
}
```

---

## 14. Design Token Approach

### CSS Variables (Source of Truth)

```css
/* globals.css */
:root {
  /* Colors - HSL format for opacity support */
  --background: 210 40% 98%;
  --foreground: 222 47% 11%;
  --card: 0 0% 100%;
  --card-foreground: 222 47% 11%;
  --primary: 221 72% 40%;
  --primary-foreground: 0 0% 100%;
  --secondary: 210 40% 96%;
  --secondary-foreground: 222 47% 11%;
  --muted: 210 40% 96%;
  --muted-foreground: 215 16% 47%;
  --accent: 160 84% 39%;
  --accent-foreground: 0 0% 100%;
  --destructive: 0 84% 60%;
  --destructive-foreground: 0 0% 100%;
  --border: 214 32% 91%;
  --input: 214 32% 91%;
  --ring: 221 72% 40%;

  /* Radius */
  --radius: 0.5rem;
}

.dark {
  --background: 222 47% 11%;
  --foreground: 210 40% 98%;
  --card: 217 33% 17%;
  --card-foreground: 210 40% 98%;
  /* ... dark mode overrides */
}
```

### Tailwind Integration

```typescript
// tailwind.config.ts
colors: {
  background: "hsl(var(--background))",
  foreground: "hsl(var(--foreground))",
  primary: {
    DEFAULT: "hsl(var(--primary))",
    foreground: "hsl(var(--primary-foreground))",
  },
  // ... maps CSS vars to Tailwind
}
```

---

## 15. Browser Support

| Browser | Minimum Version |
|---------|----------------|
| Chrome | 90+ |
| Firefox | 90+ |
| Safari | 14+ |
| Edge | 90+ |
| Mobile Safari | 14+ |
| Chrome Android | 90+ |

**Features Used:**
- CSS Custom Properties (variables)
- CSS Grid and Flexbox
- CSS :has() selector (Safari 15.4+)
- ES2020+ JavaScript

---

*Document prepared by Planner Agent*
*All UI work uses model: sonnet*
