import { ReactNode } from "react"
import { cn } from "@/lib/utils"

interface AssetRowProps {
  name: string
  symbol: string
  icon?: ReactNode
  balance: string
  value?: string
  onClick?: () => void
}

export function AssetRow({ name, symbol, icon, balance, value, onClick }: AssetRowProps) {
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
        <div className="flex items-center justify-center h-10 w-10 rounded-full bg-[#00FF87]/10 border border-[#00FF87]/20 shrink-0">
          {icon ? (
            <span className="text-[#00FF87]">{icon}</span>
          ) : (
            <span className="font-bold text-xs text-[#00FF87]">{symbol.slice(0, 2)}</span>
          )}
        </div>
        <div>
          <span className="font-semibold text-sm text-foreground">{name}</span>
          <div className="text-xs text-muted-foreground font-medium">{symbol}</div>
        </div>
      </div>

      <div className="flex flex-col items-end">
        <span className="font-bold text-sm tabular-nums text-foreground">{balance}</span>
        {value && (
          <span className="text-xs text-muted-foreground tabular-nums">{value}</span>
        )}
      </div>
    </div>
  )
}
