"use client"

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Lock, Unlock } from "lucide-react"
import { Panel, EmptyState } from "@/components/shared"
import { Skeleton } from "@/components/ui/skeleton"

interface BalanceDisplayProps {
  balances: { currency: string; total: string; available: string; locked: string }[]
  loading?: boolean
}

function parseCurrency(value: string, currency: string): string {
  const num = parseFloat(value)
  if (isNaN(num)) return value
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

export function BalanceDisplay({ balances, loading }: BalanceDisplayProps) {
  if (loading) {
    return (
      <div className="bg-[#111113]/80 backdrop-blur-sm border border-white/[0.06] rounded-xl p-6">
        <div className="flex justify-between items-center mb-6">
          <Skeleton className="h-3 w-24 bg-white/5" />
          <Skeleton className="h-7 w-28 rounded-md bg-white/5" />
        </div>
        <Skeleton className="h-10 w-48 mb-6 bg-white/5" />
        <div className="grid grid-cols-2 gap-4">
          <Skeleton className="h-20 w-full rounded-xl bg-white/5" />
          <Skeleton className="h-20 w-full rounded-xl bg-white/5" />
        </div>
      </div>
    )
  }

  if (!balances || balances.length === 0) {
    return (
      <Panel variant="solid">
        <EmptyState
          title="No balance data"
          description="Connect your wallet to see balances."
        />
      </Panel>
    )
  }

  return (
    <Panel variant="solid">
      <Tabs defaultValue={balances[0]?.currency} className="w-full">
        <div className="flex items-center justify-between mb-4">
          <p className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
            Total Balance
          </p>
          <TabsList className="h-7 bg-white/5 border border-white/[0.06]">
            {balances.map((balance) => (
              <TabsTrigger
                key={balance.currency}
                value={balance.currency}
                className="text-[10px] px-2.5 h-5 font-semibold data-[state=active]:bg-[#00FF87]/10 data-[state=active]:text-[#00FF87]"
              >
                {balance.currency}
              </TabsTrigger>
            ))}
          </TabsList>
        </div>

        {balances.map((balance) => (
          <TabsContent key={balance.currency} value={balance.currency} className="mt-0 space-y-5">
            <div className="text-4xl font-bold tracking-tight tabular-nums text-foreground">
              {parseCurrency(balance.total, balance.currency)}
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div className="p-4 rounded-xl bg-white/[0.03] border border-white/[0.06] space-y-1.5">
                <div className="flex items-center gap-1.5 text-[10px] font-semibold text-[#00FF87] uppercase tracking-wider">
                  <Unlock className="h-3 w-3" />
                  <span>Available</span>
                </div>
                <p className="text-base font-bold tabular-nums">
                  {parseCurrency(balance.available, balance.currency)}
                </p>
              </div>
              <div className="p-4 rounded-xl bg-white/[0.03] border border-white/[0.06] space-y-1.5">
                <div className="flex items-center gap-1.5 text-[10px] font-semibold text-[#7B61FF] uppercase tracking-wider">
                  <Lock className="h-3 w-3" />
                  <span>Locked</span>
                </div>
                <p className="text-base font-bold tabular-nums">
                  {parseCurrency(balance.locked, balance.currency)}
                </p>
              </div>
            </div>
          </TabsContent>
        ))}
      </Tabs>
    </Panel>
  )
}
