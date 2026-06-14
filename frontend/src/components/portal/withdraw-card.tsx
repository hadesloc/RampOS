"use client"

import { useState, FormEvent } from "react"
import { Loader2, ArrowUpFromLine } from "lucide-react"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Panel } from "@/components/shared"
import { Skeleton } from "@/components/ui/skeleton"

interface WithdrawCardProps {
  type?: "VND" | "CRYPTO"
  availableBalance: string
  onSubmit?: (amount: string, destination: string) => void
  loading?: boolean
}

function formatBalance(val: string, type: "VND" | "CRYPTO"): string {
  const num = parseFloat(val)
  if (isNaN(num)) return val
  if (type === "VND") {
    return new Intl.NumberFormat("vi-VN", {
      style: "currency",
      currency: "VND",
      maximumFractionDigits: 0,
    }).format(num)
  }
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 6,
  }).format(num)
}

export function WithdrawCard({
  type = "VND",
  availableBalance,
  onSubmit,
  loading,
}: WithdrawCardProps) {
  const [activeType, setActiveType] = useState<"VND" | "CRYPTO">(type)
  const [amount, setAmount] = useState("")
  const [destination, setDestination] = useState("")
  const [isSubmitting, setIsSubmitting] = useState(false)

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault()
    if (!amount || !destination) return
    setIsSubmitting(true)
    try {
      await onSubmit?.(amount, destination)
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleMax = () => {
    setAmount(availableBalance)
  }

  if (loading) {
    return (
      <div className="bg-[#111113]/80 backdrop-blur-sm border border-white/[0.06] rounded-xl p-6 h-[400px]">
        <Skeleton className="h-5 w-24 mb-1 bg-white/5" />
        <Skeleton className="h-3 w-48 mb-6 bg-white/5" />
        <Skeleton className="h-8 w-full mb-6 bg-white/5" />
        <Skeleton className="h-16 w-full mb-4 bg-white/5" />
        <Skeleton className="h-16 w-full mb-6 bg-white/5" />
        <Skeleton className="h-10 w-full bg-white/5" />
      </div>
    )
  }

  return (
    <Panel
      variant="solid"
      header={{
        title: "Withdraw",
        description: "Withdraw funds to your bank or wallet",
        actions: (
          <div className="p-1.5 bg-[#FFB800]/10 rounded-lg border border-[#FFB800]/20">
            <ArrowUpFromLine className="h-4 w-4 text-[#FFB800]" />
          </div>
        ),
      }}
    >
      <Tabs
        value={activeType}
        onValueChange={(v) => setActiveType(v as "VND" | "CRYPTO")}
        className="w-full"
      >
        <TabsList className="grid w-full grid-cols-2 mb-6 bg-white/5 border border-white/[0.06]">
          <TabsTrigger
            value="VND"
            className="text-xs font-semibold data-[state=active]:bg-[#FFB800]/10 data-[state=active]:text-[#FFB800]"
          >
            Fiat (VND)
          </TabsTrigger>
          <TabsTrigger
            value="CRYPTO"
            className="text-xs font-semibold data-[state=active]:bg-[#7B61FF]/10 data-[state=active]:text-[#7B61FF]"
          >
            Crypto (USDT)
          </TabsTrigger>
        </TabsList>

        <form onSubmit={handleSubmit} className="space-y-5">
          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <Label htmlFor="withdraw-amount" className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Amount
              </Label>
              <button
                type="button"
                className="text-xs text-muted-foreground hover:text-[#FFB800] transition-colors flex items-center gap-1"
                onClick={handleMax}
              >
                Available:{" "}
                <span className="font-bold text-foreground tabular-nums">
                  {formatBalance(availableBalance, activeType)}
                </span>
              </button>
            </div>
            <div className="relative">
              <Input
                id="withdraw-amount"
                placeholder="0.00"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                disabled={isSubmitting}
                type="number"
                step="any"
                className="pr-14 bg-white/[0.03] border-white/[0.08] focus:border-[#FFB800]/50 tabular-nums"
              />
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="absolute right-1 top-1 h-7 text-[10px] font-bold text-[#FFB800] hover:bg-[#FFB800]/10"
                onClick={handleMax}
                disabled={isSubmitting}
                aria-label="Use maximum balance"
              >
                MAX
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="withdraw-destination" className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              {activeType === "VND" ? "Bank Account Number" : "Wallet Address"}
            </Label>
            <Input
              id="withdraw-destination"
              placeholder={
                activeType === "VND"
                  ? "Enter bank account number"
                  : "Enter wallet address (0x…)"
              }
              value={destination}
              onChange={(e) => setDestination(e.target.value)}
              disabled={isSubmitting}
              className="bg-white/[0.03] border-white/[0.08] focus:border-[#FFB800]/50 font-mono text-sm"
            />
          </div>

          <Button
            type="submit"
            className="w-full bg-[#FFB800] hover:bg-[#FFB800]/90 text-black font-bold"
            disabled={isSubmitting || !amount || !destination}
          >
            {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Withdraw {activeType === "VND" ? "VND" : "USDT"}
          </Button>
        </form>
      </Tabs>
    </Panel>
  )
}
