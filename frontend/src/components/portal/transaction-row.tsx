import { JSX } from "react"
import { ArrowDownToLine, ArrowUpFromLine, RefreshCw, ArrowRightLeft } from "lucide-react"
import { cn } from "@/lib/utils"
import { StatusBadge } from "@/components/shared"
import { formatRelativeTime, toLabel } from "@/lib/format"

interface TransactionRowProps {
  id: string
  type: "PAYIN_VND" | "PAYOUT_VND" | "TRADE_EXECUTED" | "DEPOSIT" | "WITHDRAW" | "TRADE" | string
  amount: string
  currency: string
  status: string
  createdAt: string
  onClick?: () => void
}

function formatAmount(val: string, currency: string): string {
  const num = parseFloat(val)
  if (isNaN(num)) return val
  if (currency === "VND") {
    return new Intl.NumberFormat("vi-VN", {
      style: "currency",
      currency: "VND",
      maximumFractionDigits: 0,
    }).format(num)
  }
  const safeCurrency = currency === "USDT" ? "USD" : currency
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: safeCurrency,
    minimumFractionDigits: 2,
    maximumFractionDigits: 6,
  }).format(num)
}

function typeIcon(type: string): JSX.Element {
  if (type.includes("PAYIN") || type === "DEPOSIT") {
    return (
      <div className="p-2.5 rounded-xl bg-[#00FF87]/10 border border-[#00FF87]/20">
        <ArrowDownToLine className="h-4 w-4 text-[#00FF87]" />
      </div>
    )
  }
  if (type.includes("PAYOUT") || type === "WITHDRAW") {
    return (
      <div className="p-2.5 rounded-xl bg-[#FFB800]/10 border border-[#FFB800]/20">
        <ArrowUpFromLine className="h-4 w-4 text-[#FFB800]" />
      </div>
    )
  }
  if (type.includes("TRADE")) {
    return (
      <div className="p-2.5 rounded-xl bg-[#7B61FF]/10 border border-[#7B61FF]/20">
        <ArrowRightLeft className="h-4 w-4 text-[#7B61FF]" />
      </div>
    )
  }
  return (
    <div className="p-2.5 rounded-xl bg-white/5 border border-white/[0.06]">
      <RefreshCw className="h-4 w-4 text-muted-foreground" />
    </div>
  )
}

function amountColor(type: string): string {
  if (type.includes("PAYIN") || type === "DEPOSIT") return "text-[#00FF87]"
  if (type.includes("PAYOUT") || type === "WITHDRAW") return "text-[#FFB800]"
  return "text-foreground"
}

function amountPrefix(type: string): string {
  if (type.includes("PAYIN") || type === "DEPOSIT") return "+"
  if (type.includes("PAYOUT") || type === "WITHDRAW") return "-"
  return ""
}

function safeRelativeTime(date: string): string {
  try {
    const d = new Date(date)
    if (isNaN(d.getTime())) return "—"
    return formatRelativeTime(d)
  } catch {
    return "—"
  }
}

export function TransactionRow({
  type,
  amount,
  currency,
  status,
  createdAt,
  onClick,
}: TransactionRowProps) {
  return (
    <div
      className={cn(
        "flex items-center justify-between p-4 rounded-xl border border-white/[0.06] bg-[#111113]/60 backdrop-blur-sm transition-all duration-200",
        onClick
          ? "cursor-pointer hover:border-white/[0.12] hover:bg-[#111113]/90"
          : "cursor-default"
      )}
      onClick={onClick}
    >
      <div className="flex items-center gap-4">
        {typeIcon(type)}
        <div>
          <div className="font-semibold text-sm text-foreground">
            {toLabel(type)}
          </div>
          <div className="text-xs text-muted-foreground mt-0.5">
            {safeRelativeTime(createdAt)}
          </div>
        </div>
      </div>

      <div className="flex flex-col items-end gap-1.5">
        <span
          className={cn(
            "font-bold text-sm tabular-nums",
            amountColor(type)
          )}
        >
          {amountPrefix(type)}
          {formatAmount(amount, currency)}
        </span>
        <StatusBadge status={status} />
      </div>
    </div>
  )
}
