'use client'

import React, { useState } from 'react'
import {
  ColumnDef,
  SortingState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
  ColumnFiltersState,
  PaginationState,
} from '@tanstack/react-table'
import {
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
} from 'lucide-react'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { EmptyState } from '@/components/shared/empty-state'
import { cn } from '@/lib/utils'

export interface DataTableProps<TData, TValue> {
  columns: ColumnDef<TData, TValue>[]
  data: TData[]
  loading?: boolean
  /** Number of skeleton rows shown while loading. Default 7. */
  skeletonRows?: number
  /** Renders when data is empty and not loading. */
  emptyState?: React.ReactNode
  /** Enables the built-in pagination footer. */
  pagination?: boolean
  /** Initial page size. Default 10. */
  pageSize?: number
  /** Global filter value (controlled externally via Toolbar). */
  globalFilter?: string
  /** Callback for controlled global filter. */
  onGlobalFilterChange?: (value: string) => void
  className?: string
  /** Extra class for the wrapper div. */
  wrapperClassName?: string
}

export function DataTable<TData, TValue>({
  columns,
  data,
  loading = false,
  skeletonRows = 7,
  emptyState,
  pagination: enablePagination = false,
  pageSize: initialPageSize = 10,
  globalFilter,
  onGlobalFilterChange,
  className,
  wrapperClassName,
}: DataTableProps<TData, TValue>) {
  const [sorting, setSorting] = useState<SortingState>([])
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([])
  const [internalGlobalFilter, setInternalGlobalFilter] = useState('')
  const [{ pageIndex, pageSize }, setPagination] = useState<PaginationState>({
    pageIndex: 0,
    pageSize: initialPageSize,
  })

  const resolvedGlobalFilter =
    globalFilter !== undefined ? globalFilter : internalGlobalFilter
  const resolvedSetGlobalFilter =
    onGlobalFilterChange !== undefined
      ? onGlobalFilterChange
      : setInternalGlobalFilter

  const table = useReactTable({
    data,
    columns,
    state: {
      sorting,
      columnFilters,
      globalFilter: resolvedGlobalFilter,
      pagination: { pageIndex, pageSize },
    },
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    onGlobalFilterChange: resolvedSetGlobalFilter,
    onPaginationChange: setPagination,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getPaginationRowModel: enablePagination ? getPaginationRowModel() : undefined,
    manualPagination: false,
  })

  return (
    <div className={cn('flex flex-col gap-0', wrapperClassName)}>
      <div className={cn('relative w-full overflow-auto', className)}>
        <table className="w-full caption-bottom text-sm">
          {/* Sticky header */}
          <thead className="sticky top-0 z-10 bg-[#111113]">
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id} className="border-b border-white/[0.06]">
                {headerGroup.headers.map((header) => {
                  const canSort = header.column.getCanSort()
                  const sortDir = header.column.getIsSorted()
                  return (
                    <th
                      key={header.id}
                      className={cn(
                        'h-10 px-3 text-left align-middle text-xs font-medium text-muted-foreground whitespace-nowrap select-none',
                        canSort && 'cursor-pointer hover:text-foreground transition-colors'
                      )}
                      onClick={canSort ? header.column.getToggleSortingHandler() : undefined}
                      aria-sort={
                        sortDir === 'asc'
                          ? 'ascending'
                          : sortDir === 'desc'
                          ? 'descending'
                          : canSort
                          ? 'none'
                          : undefined
                      }
                    >
                      <span className="inline-flex items-center gap-1">
                        {header.isPlaceholder
                          ? null
                          : flexRender(
                              header.column.columnDef.header,
                              header.getContext()
                            )}
                        {canSort && (
                          <span className="text-muted-foreground/40" aria-hidden="true">
                            {sortDir === 'asc' ? (
                              <ArrowUp className="h-3 w-3" />
                            ) : sortDir === 'desc' ? (
                              <ArrowDown className="h-3 w-3" />
                            ) : (
                              <ArrowUpDown className="h-3 w-3" />
                            )}
                          </span>
                        )}
                      </span>
                    </th>
                  )
                })}
              </tr>
            ))}
          </thead>

          <tbody className="[&_tr:last-child]:border-0">
            {loading ? (
              Array.from({ length: skeletonRows }).map((_, rowIdx) => (
                <tr
                  key={rowIdx}
                  className="border-b border-white/[0.04]"
                  aria-hidden="true"
                >
                  {columns.map((_, colIdx) => (
                    <td key={colIdx} className="px-3 py-3.5 align-middle">
                      <Skeleton className="h-4 w-[80%] bg-white/5" />
                    </td>
                  ))}
                </tr>
              ))
            ) : table.getRowModel().rows.length === 0 ? (
              <tr>
                <td colSpan={columns.length} className="text-center">
                  {emptyState ?? (
                    <EmptyState
                      title="No results"
                      description="No records match your current filters."
                    />
                  )}
                </td>
              </tr>
            ) : (
              table.getRowModel().rows.map((row) => (
                <tr
                  key={row.id}
                  className="border-b border-white/[0.04] transition-colors hover:bg-white/[0.02] data-[state=selected]:bg-white/[0.04]"
                  data-state={row.getIsSelected() ? 'selected' : undefined}
                >
                  {row.getVisibleCells().map((cell) => (
                    <td
                      key={cell.id}
                      className="px-3 py-3.5 align-middle tabular-nums"
                    >
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {enablePagination && !loading && (
        <div className="flex items-center justify-between px-3 py-3 border-t border-white/[0.06]">
          <p className="text-xs text-muted-foreground tabular-nums">
            {table.getFilteredRowModel().rows.length === 0 ? (
              'No results'
            ) : (
              <>
                {pageIndex * pageSize + 1}–
                {Math.min((pageIndex + 1) * pageSize, table.getFilteredRowModel().rows.length)}{' '}
                of {table.getFilteredRowModel().rows.length}
              </>
            )}
          </p>
          <div className="flex items-center gap-1" role="navigation" aria-label="Pagination">
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => table.firstPage()}
              disabled={!table.getCanPreviousPage()}
              aria-label="First page"
            >
              <ChevronsLeft className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => table.previousPage()}
              disabled={!table.getCanPreviousPage()}
              aria-label="Previous page"
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <span className="text-xs text-muted-foreground tabular-nums px-2">
              {pageIndex + 1} / {table.getPageCount() || 1}
            </span>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => table.nextPage()}
              disabled={!table.getCanNextPage()}
              aria-label="Next page"
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => table.lastPage()}
              disabled={!table.getCanNextPage()}
              aria-label="Last page"
            >
              <ChevronsRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
