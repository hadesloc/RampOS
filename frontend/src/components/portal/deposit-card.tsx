"use client"

import { useState, ReactNode } from "react"
import Image from "next/image"
import { Copy, Check, ArrowDownToLine, AlertTriangle } from "lucide-react"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Panel, EmptyState } from "@/components/shared"
import { Skeleton } from "@/components/ui/skeleton"

interface DepositCardProps {
  type?: "VND" | "CRYPTO"
  onTypeChange?: (type: "VND" | "CRYPTO") => void
  instructions?: ReactNode
  venueFundingHref?: string
  qrCode?: string
  loading?: boolean
  bankDetails?: {
    bankName: string
    accountName: string
    accountNumber: string
    content: string
  }
  walletAddress?: string
  network?: string
}

function CopyField({ label, value }: { label: string; value: string }) {
  const [copied, setCopied] = useState(false)
  const handleCopy = () => {
    navigator.clipboard.writeText(value)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }
  return (
    <div className="space-y-1.5">
      <p className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
        {label}
      </p>
      <div className="flex items-center gap-2 bg-white/[0.03] border border-white/[0.08] rounded-lg px-3 py-2">
        <span className="font-mono text-sm text-foreground flex-1 truncate">{value}</span>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6 shrink-0 text-muted-foreground hover:text-[#00FF87] hover:bg-[#00FF87]/10"
          onClick={handleCopy}
          aria-label={`Copy ${label}`}
        >
          {copied ? (
            <Check className="h-3 w-3 text-[#00FF87]" />
          ) : (
            <Copy className="h-3 w-3" />
          )}
        </Button>
      </div>
    </div>
  )
}

export function DepositCard({
  type = "VND",
  onTypeChange,
  instructions,
  venueFundingHref,
  qrCode,
  loading,
  bankDetails,
  walletAddress,
  network,
}: DepositCardProps) {
  const [activeType, setActiveType] = useState<"VND" | "CRYPTO">(type)

  const handleTypeChange = (val: string) => {
    const newType = val as "VND" | "CRYPTO"
    setActiveType(newType)
    onTypeChange?.(newType)
  }

  if (loading) {
    return (
      <div className="bg-[#111113]/80 backdrop-blur-sm border border-white/[0.06] rounded-xl p-6 h-[480px]">
        <Skeleton className="h-5 w-36 mb-1 bg-white/5" />
        <Skeleton className="h-3 w-64 mb-6 bg-white/5" />
        <Skeleton className="h-8 w-full mb-6 bg-white/5" />
        <div className="space-y-4">
          {[1, 2, 3, 4].map((i) => (
            <Skeleton key={i} className="h-14 w-full bg-white/5" />
          ))}
        </div>
      </div>
    )
  }

  return (
    <Panel
      variant="solid"
      header={{
        title: "Deposit to Wallet",
        description:
          "Fund your governed RampOS wallet. Venue funding is a separate wallet-to-venue workflow.",
        actions: (
          <div className="p-1.5 bg-[#00FF87]/10 rounded-lg border border-[#00FF87]/20">
            <ArrowDownToLine className="h-4 w-4 text-[#00FF87]" />
          </div>
        ),
      }}
    >
      <Tabs value={activeType} onValueChange={handleTypeChange} className="w-full">
        <TabsList className="grid w-full grid-cols-2 mb-6 bg-white/5 border border-white/[0.06]">
          <TabsTrigger
            value="VND"
            className="text-xs font-semibold data-[state=active]:bg-[#00FF87]/10 data-[state=active]:text-[#00FF87]"
          >
            Fiat (VND)
          </TabsTrigger>
          <TabsTrigger
            value="CRYPTO"
            className="text-xs font-semibold data-[state=active]:bg-[#00D4FF]/10 data-[state=active]:text-[#00D4FF]"
          >
            Crypto (USDT)
          </TabsTrigger>
        </TabsList>

        <div className="flex flex-col md:flex-row gap-6">
          <div className="flex-1 space-y-4">
            {activeType === "VND" && bankDetails ? (
              <>
                <CopyField label="Bank Name" value={bankDetails.bankName} />
                <CopyField label="Account Name" value={bankDetails.accountName} />
                <CopyField label="Account Number" value={bankDetails.accountNumber} />
                <div className="space-y-1.5">
                  <CopyField label="Transfer Content" value={bankDetails.content} />
                  <div className="flex items-start gap-2 p-3 rounded-lg bg-[#FFB800]/5 border border-[#FFB800]/20 mt-2">
                    <AlertTriangle className="h-3.5 w-3.5 text-[#FFB800] shrink-0 mt-0.5" />
                    <p className="text-[10px] text-[#FFB800]">
                      Include the exact transfer content in your bank transfer description or the deposit will not be credited.
                    </p>
                  </div>
                </div>
              </>
            ) : activeType === "CRYPTO" && walletAddress ? (
              <>
                <div className="space-y-1.5">
                  <p className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
                    Network
                  </p>
                  <span className="inline-flex items-center px-2 py-0.5 rounded-md text-xs font-bold bg-[#00D4FF]/10 border border-[#00D4FF]/20 text-[#00D4FF]">
                    {network || "TRC20"}
                  </span>
                </div>
                <CopyField label="Wallet Address" value={walletAddress} />
              </>
            ) : (
              <EmptyState
                title="No deposit details"
                description="Contact support to get deposit information."
              />
            )}

            {instructions && (
              <div className="pt-4 border-t border-white/[0.06] text-xs text-muted-foreground">
                {instructions}
              </div>
            )}

            {venueFundingHref && (
              <div className="rounded-xl border border-[#7B61FF]/20 bg-[#7B61FF]/5 p-4">
                <p className="text-sm font-semibold text-foreground">
                  Need to move wallet funds into a venue?
                </p>
                <p className="mt-1 text-xs text-muted-foreground">
                  Finish the wallet deposit first, then continue in the dedicated venue funding workspace.
                </p>
                <Button
                  asChild
                  variant="outline"
                  size="sm"
                  className="mt-3 border-[#7B61FF]/30 text-[#7B61FF] hover:bg-[#7B61FF]/10"
                >
                  <a href={venueFundingHref}>Open Venue Funding</a>
                </Button>
              </div>
            )}
          </div>

          {qrCode && (
            <div className="flex flex-col items-center justify-start pt-2 shrink-0">
              <div className="p-3 border border-white/[0.06] rounded-xl bg-white">
                <Image
                  src={qrCode}
                  alt="QR Code for deposit"
                  width={160}
                  height={160}
                  unoptimized
                  className="w-32 h-32 md:w-40 md:h-40 object-contain mix-blend-multiply"
                />
              </div>
              <p className="text-[10px] text-muted-foreground mt-3 font-semibold uppercase tracking-widest">
                Scan to Pay
              </p>
            </div>
          )}
        </div>
      </Tabs>
    </Panel>
  )
}
