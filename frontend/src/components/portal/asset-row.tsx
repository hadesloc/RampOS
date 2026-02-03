import React from "react"
import { cn } from "@/lib/utils"

interface AssetRowProps {
  name: string;
  symbol: string;
  icon?: React.ReactNode;
  balance: string;
  value?: string;
  onClick?: () => void;
}

export function AssetRow({ name, symbol, icon, balance, value, onClick }: AssetRowProps) {
  return (
    <div
      className={cn(
        "flex items-center justify-between p-4 rounded-xl border bg-card/50 transition-all duration-200",
        onClick
            ? "hover:bg-accent/50 hover:shadow-sm hover:border-primary/20 cursor-pointer"
            : "cursor-default"
      )}
      onClick={onClick}
    >
      <div className="flex items-center gap-4">
        <div className="flex items-center justify-center h-10 w-10 rounded-full bg-muted border">
          {icon ? icon : <span className="font-bold text-xs">{symbol.slice(0, 2)}</span>}
        </div>
        <div className="flex flex-col">
          <span className="font-semibold text-sm">{name}</span>
          <span className="text-xs text-muted-foreground font-medium">{symbol}</span>
        </div>
      </div>

      <div className="flex flex-col items-end">
        <span className="font-semibold text-sm font-mono">{balance}</span>
        {value && (
            <span className="text-xs text-muted-foreground">{value}</span>
        )}
      </div>
    </div>
  )
}
