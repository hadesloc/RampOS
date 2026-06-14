import { format, formatDistanceToNow, formatDuration as dfnFormatDuration, intervalToDuration } from 'date-fns'

// ── Money ─────────────────────────────────────────────────────────────────────

/**
 * Format a numeric amount as a currency string.
 * Renders tabular-nums-friendly output (no trailing ".00" abbreviations).
 *
 * @example formatMoney(1234567.89) => "$1,234,567.89"
 * @example formatMoney(1234567.89, 'EUR') => "€1,234,567.89"
 */
export function formatMoney(
  amount: number,
  currency = 'USD',
  locale = 'en-US'
): string {
  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(amount)
}

// ── Numbers ───────────────────────────────────────────────────────────────────

/**
 * Format a number with thousands separators.
 * @example formatNumber(1234567) => "1,234,567"
 */
export function formatNumber(value: number, decimals = 0, locale = 'en-US'): string {
  return new Intl.NumberFormat(locale, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(value)
}

/**
 * Format a ratio/fraction as a percentage string.
 * @example formatPercent(0.1234) => "12.34%"
 * @example formatPercent(0.1234, 1) => "12.3%"
 */
export function formatPercent(value: number, decimals = 2, locale = 'en-US'): string {
  return new Intl.NumberFormat(locale, {
    style: 'percent',
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(value)
}

/**
 * Format a large number with compact K/M/B suffixes.
 * @example formatCompact(1_234_567) => "1.2M"
 */
export function formatCompact(value: number, locale = 'en-US'): string {
  return new Intl.NumberFormat(locale, {
    notation: 'compact',
    maximumFractionDigits: 1,
  }).format(value)
}

// ── Dates ─────────────────────────────────────────────────────────────────────

/**
 * Format a date as a short human-readable date string.
 * @example formatDate(new Date('2025-01-15')) => "Jan 15, 2025"
 */
export function formatDate(date: Date | string | number): string {
  return format(new Date(date), 'MMM d, yyyy')
}

/**
 * Format a date as a date+time string.
 * @example formatDateTime(new Date('2025-01-15T14:30:00')) => "Jan 15, 2025 · 14:30"
 */
export function formatDateTime(date: Date | string | number): string {
  return format(new Date(date), "MMM d, yyyy · HH:mm")
}

/**
 * Format a date as a relative time string.
 * @example formatRelativeTime(somePastDate) => "3 hours ago"
 */
export function formatRelativeTime(date: Date | string | number): string {
  return formatDistanceToNow(new Date(date), { addSuffix: true })
}

// ── Duration ──────────────────────────────────────────────────────────────────

/**
 * Format a duration in seconds to a human-readable string.
 * @example formatDuration(90061) => "1 day, 1 hour, 1 minute"
 * @example formatDuration(3661, ['hours', 'minutes', 'seconds']) => "1 hour, 1 minute, 1 second"
 */
export function formatDuration(
  seconds: number,
  units: Array<'years' | 'months' | 'weeks' | 'days' | 'hours' | 'minutes' | 'seconds'> = [
    'days',
    'hours',
    'minutes',
  ]
): string {
  const duration = intervalToDuration({ start: 0, end: seconds * 1000 })
  const formatted = dfnFormatDuration(duration, { format: units })
  return formatted || '< 1 minute'
}

// ── Strings ───────────────────────────────────────────────────────────────────

/**
 * Truncate a long string (hash / address / tx-id) to a middle ellipsis form.
 * @example truncateMiddle('0xabcdef1234567890', 6, 4) => "0xabcd…7890"
 */
export function truncateMiddle(value: string, startChars = 6, endChars = 4): string {
  if (value.length <= startChars + endChars + 1) return value
  return `${value.slice(0, startChars)}…${value.slice(-endChars)}`
}

/**
 * Capitalise the first letter of a string.
 */
export function capitalize(value: string): string {
  if (!value) return ''
  return value.charAt(0).toUpperCase() + value.slice(1)
}

/**
 * Convert a snake_case or kebab-case string to a readable label.
 * @example toLabel('settlement_status') => "Settlement Status"
 */
export function toLabel(value: string): string {
  return value
    .replace(/[_-]/g, ' ')
    .replace(/\b\w/g, (c) => c.toUpperCase())
}
