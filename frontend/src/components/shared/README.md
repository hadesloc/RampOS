# RampOS Shared UI Kit

Import everything from `@/components/shared`.

## Components

| Component | Props summary | When to use |
|---|---|---|
| `PageHeader` | `title, description?, actions?, breadcrumb?` | Top of every page, above the content body |
| `StatCard` | `title, value, icon?, trend?, loading?, subtitle?, accentColor?` | KPI metric card with neon glow |
| `StatGrid` | `cols? (1–4), children` | Responsive wrapper for 1–4 StatCards |
| `Panel` / `SectionCard` | `header?, footer?, variant? ('glass'\|'solid'), children` | Section wrapper with optional header/footer |
| `ChartCard` | `title, description?, loading?, empty?, height?, actions?, children` | Recharts wrapper with skeleton + empty fallback |
| `DataTable<T>` | `columns, data, loading?, pagination?, globalFilter?, onGlobalFilterChange?` | Generic TanStack-table with dark styling |
| `EmptyState` | `title, icon?, description?, action?` | Centred empty placeholder |
| `ErrorState` | `title?, message, retry?` | Destructive-accented error display |
| `TableSkeleton` | `rows?, columns?` | Loading rows for a table |
| `CardGridSkeleton` | `cards?` | Loading grid of stat cards |
| `ChartSkeleton` | `height?` | Loading bar chart shape |
| `StatusBadge` | `status, severity?, dot?` | Colour-coded status pill; severity is auto-inferred |
| `Toolbar` | `searchValue?, onSearchChange?, filters?, actions?` | Filter/search/action bar above a table |

## Chart helpers (re-exported from chart-card)

```ts
import { chartColors, cartesianGridProps, axisProps, tooltipStyle } from '@/components/shared'
// use chartColors.green, chartColors[1], etc.
// spread cartesianGridProps / axisProps / tooltipStyle onto Recharts primitives
```

## Formatter helpers

```ts
import { formatMoney, formatNumber, formatPercent, formatCompact,
         formatDate, formatDateTime, formatRelativeTime,
         formatDuration, truncateMiddle, capitalize, toLabel } from '@/lib/format'
```

## Example page

```tsx
import { PageHeader, StatGrid, StatCard, Panel, ChartCard, DataTable, Toolbar, StatusBadge } from '@/components/shared'
import { formatMoney } from '@/lib/format'
import { DollarSign } from 'lucide-react'

export default function ExamplePage() {
  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Settlements"
        description="Track and manage fiat settlement runs."
        breadcrumb={[{ label: 'Admin', href: '/admin' }, { label: 'Settlements' }]}
        actions={<button>Export</button>}
      />
      <StatGrid cols={4}>
        <StatCard title="Total Volume" value={formatMoney(9_240_000)} accentColor="green" icon={<DollarSign />} />
        <StatCard title="Pending" value="14" accentColor="amber" />
        <StatCard title="Failed" value="2" accentColor="violet" trend={{ value: 5, isPositive: false }} />
        <StatCard title="Completed" value="1,402" accentColor="cyan" subtitle="last 30 days" />
      </StatGrid>
      <Panel header={{ title: 'Recent Settlements', actions: <button>View all</button> }}>
        <Toolbar searchPlaceholder="Search settlements…" onSearchChange={() => {}} />
        <DataTable columns={[]} data={[]} pagination />
      </Panel>
    </main>
  )
}
```
