"use client"

import { useState } from "react"
import { Copy, Wallet, Check, ShieldCheck } from "lucide-react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { Panel } from "@/components/shared"
import { truncateMiddle } from "@/lib/format"
import { Skeleton } from "@/components/ui/skeleton"

interface WalletCardProps {
  address: string
  deployed: boolean
  onCopy?: () => void
  loading?: boolean
}

export function WalletCard({ address, deployed, onCopy, loading }: WalletCardProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = () => {
    if (onCopy) {
      onCopy()
    } else {
      navigator.clipboard.writeText(address)
    }
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  if (loading) {
    return (
      <div className="bg-[#111113]/80 backdrop-blur-sm border border-white/[0.06] rounded-xl p-6">
        <div className="flex justify-between items-start mb-5">
          <Skeleton className="h-9 w-9 rounded-xl bg-white/5" />
          <Skeleton className="h-5 w-20 rounded-md bg-white/5" />
        </div>
        <Skeleton className="h-3 w-24 mb-2 bg-white/5" />
        <div className="flex items-center gap-2">
          <Skeleton className="h-6 w-40 bg-white/5" />
          <Skeleton className="h-8 w-8 rounded-md bg-white/5" />
        </div>
      </div>
    )
  }

  return (
    <Panel variant="solid">
      <div className="flex justify-between items-start mb-5">
        <div className="p-2.5 bg-[#00FF87]/10 rounded-xl border border-[#00FF87]/20">
          <Wallet className="h-4 w-4 text-[#00FF87]" />
        </div>
        <span
          className={cn(
            "inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md text-xs font-semibold border",
            deployed
              ? "text-[#00FF87] bg-[#00FF87]/10 border-[#00FF87]/20"
              : "text-muted-foreground bg-white/5 border-white/[0.06]"
          )}
        >
          {deployed ? <ShieldCheck className="h-3 w-3" /> : null}
          {deployed ? "Deployed" : "Not Deployed"}
        </span>
      </div>

      <p className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground mb-1.5">
        Wallet Address
      </p>
      <div className="flex items-center gap-2">
        <span className="text-lg font-bold tracking-tight font-mono text-foreground tabular-nums">
          {address ? truncateMiddle(address, 6, 4) : "—"}
        </span>
        {address && (
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 text-muted-foreground hover:text-[#00FF87] hover:bg-[#00FF87]/10 transition-colors"
            onClick={handleCopy}
          >
            {copied ? (
              <Check className="h-3.5 w-3.5 text-[#00FF87]" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
            <span className="sr-only">Copy address</span>
          </Button>
        )}
      </div>
    </Panel>
  )
}
