"use client"

import {
  Building2,
  CheckCircle2,
  CircleDashed,
  Landmark,
  ShieldCheck,
  AlertTriangle,
  Loader2,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Panel, StatusBadge } from "@/components/shared"
import {
  VenueFundingEligibilityResponse,
  VenueFundingPrepareResponse,
  VenueFundingStatusResponse,
  VenueFundingSubmitResponse,
  VenueSummary,
} from "@/lib/portal-api"
import { cn } from "@/lib/utils"

interface VenueFundingCardProps {
  venues: VenueSummary[]
  selectedVenueKey: string | null
  amount: string
  venueConnectionId: string
  venueAccountId: string
  walletAttestationId: string
  walletTransferReference: string
  eligibility?: VenueFundingEligibilityResponse | null
  preparedFlow?: VenueFundingPrepareResponse | null
  submittedFlow?: VenueFundingSubmitResponse | null
  statusSnapshot?: VenueFundingStatusResponse | null
  isLoading?: boolean
  isWorking?: boolean
  error?: string | null
  onSelectVenue: (venueKey: string) => void
  onConnect: () => void
  onCheckEligibility: () => void
  onPrepare: () => void
  onSubmit: (preparedFlowId: string, walletTransferReference: string) => void
  onRefreshStatus?: () => void
  onAmountChange: (value: string) => void
  onVenueConnectionIdChange: (value: string) => void
  onVenueAccountIdChange: (value: string) => void
  onWalletAttestationIdChange: (value: string) => void
  onWalletTransferReferenceChange: (value: string) => void
}

export function VenueFundingCard({
  venues,
  selectedVenueKey,
  amount = "",
  venueConnectionId = "",
  venueAccountId = "",
  walletAttestationId = "",
  walletTransferReference,
  eligibility,
  preparedFlow,
  submittedFlow,
  statusSnapshot,
  isLoading = false,
  isWorking = false,
  error,
  onSelectVenue,
  onConnect,
  onCheckEligibility,
  onPrepare,
  onSubmit,
  onRefreshStatus,
  onAmountChange,
  onVenueConnectionIdChange,
  onVenueAccountIdChange,
  onWalletAttestationIdChange,
  onWalletTransferReferenceChange,
}: VenueFundingCardProps) {
  const selectedVenue = venues.find((v) => v.venueKey === selectedVenueKey) ?? null
  const activeFlowId = submittedFlow?.id ?? preparedFlow?.id
  const canPrepare =
    amount.trim().length > 0 &&
    venueConnectionId.trim().length > 0 &&
    venueAccountId.trim().length > 0 &&
    walletAttestationId.trim().length > 0

  return (
    <Panel
      variant="solid"
      header={{
        title: "Venue Funding",
        description:
          "Move governed wallet funds into a connected venue after wallet settlement. This workflow stays separate from generic deposit so the operator trail remains explicit.",
        actions: (
          <div className="flex items-center gap-2 text-xs font-semibold text-[#7B61FF]">
            <Landmark className="h-4 w-4" />
            <span>Wallet first, venue second</span>
          </div>
        ),
      }}
    >
      <div className="space-y-6">
        {/* Venue selector */}
        <div className="grid gap-3 md:grid-cols-2">
          {venues.map((venue) => {
            const isSelected = venue.venueKey === selectedVenueKey
            const canSelect = venue.supportsWalletFunding
            return (
              <button
                key={venue.venueKey}
                type="button"
                className={cn(
                  "rounded-xl border p-4 text-left transition-all duration-200",
                  isSelected
                    ? "border-[#7B61FF]/60 bg-[#7B61FF]/5 shadow-[0_0_16px_rgba(123,97,255,0.1)]"
                    : canSelect
                    ? "border-white/[0.08] hover:border-[#7B61FF]/30 hover:bg-[#7B61FF]/5"
                    : "border-white/[0.05] opacity-50 cursor-not-allowed"
                )}
                onClick={() => {
                  if (canSelect) onSelectVenue(venue.venueKey)
                }}
                disabled={!canSelect}
              >
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <h3 className="text-sm font-semibold text-foreground">{venue.displayName}</h3>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {canSelect
                        ? "Active wallet-funding pilot for Hyperliquid USDT lanes"
                        : "Visible for discovery only; wallet funding is not active for this venue"}
                    </p>
                  </div>
                  <StatusBadge status={venue.status} />
                </div>
              </button>
            )
          })}
        </div>

        {/* Bounded defaults notice */}
        <div className="rounded-xl border border-white/[0.08] bg-white/[0.02] p-4">
          <div className="flex items-start gap-3">
            <ShieldCheck className="mt-0.5 h-4 w-4 text-[#7B61FF] shrink-0" />
            <div className="space-y-0.5 text-xs">
              <p className="font-semibold text-foreground">Bounded portal defaults</p>
              <p className="text-muted-foreground">
                This slice assumes the Hyperliquid pilot only: jurisdiction{" "}
                <strong className="text-foreground">VN</strong>, asset{" "}
                <strong className="text-foreground">USDT</strong>, and network{" "}
                <strong className="text-foreground">Ethereum</strong> until broader venue funding
                configuration lands.
              </p>
            </div>
          </div>
        </div>

        {/* Active venue form */}
        {selectedVenue && (
          <div className="space-y-4 rounded-xl border border-white/[0.08] bg-white/[0.02] p-5">
            <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
              <Building2 className="h-4 w-4 text-[#7B61FF]" />
              Active venue: {selectedVenue.displayName}
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              {[
                {
                  id: "funding-amount",
                  label: "Funding amount",
                  value: amount,
                  onChange: onAmountChange,
                  placeholder: "125",
                },
                {
                  id: "venue-connection-id",
                  label: "Venue connection ID",
                  value: venueConnectionId,
                  onChange: onVenueConnectionIdChange,
                  placeholder: "conn_123",
                },
                {
                  id: "venue-account-id",
                  label: "Venue account ID",
                  value: venueAccountId,
                  onChange: onVenueAccountIdChange,
                  placeholder: "acct_123",
                },
                {
                  id: "wallet-attestation-id",
                  label: "Wallet attestation ID",
                  value: walletAttestationId,
                  onChange: onWalletAttestationIdChange,
                  placeholder: "550e8400-e29b-41d4-a716-446655440000",
                },
              ].map((field) => (
                <div key={field.id} className="space-y-1.5">
                  <label
                    htmlFor={field.id}
                    className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground"
                  >
                    {field.label}
                  </label>
                  <Input
                    id={field.id}
                    value={field.value}
                    onChange={(e) => field.onChange(e.target.value)}
                    placeholder={field.placeholder}
                    className="bg-white/[0.03] border-white/[0.08] focus:border-[#7B61FF]/50 font-mono text-sm"
                  />
                </div>
              ))}
            </div>

            <p className="text-xs text-muted-foreground">
              Connect can prefill the venue connection and account IDs. Hyperliquid funding stays
              wallet-linked and USDT-only in this pilot.
            </p>

            <div className="flex flex-wrap gap-3">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={onConnect}
                disabled={isWorking}
                className="border-white/[0.12] hover:border-[#7B61FF]/40 hover:bg-[#7B61FF]/10 hover:text-[#7B61FF]"
              >
                {isWorking && <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />}
                Connect venue
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={onCheckEligibility}
                disabled={isWorking}
                className="border-white/[0.12] hover:border-[#00D4FF]/40 hover:bg-[#00D4FF]/10 hover:text-[#00D4FF]"
              >
                Check eligibility
              </Button>
              <Button
                type="button"
                size="sm"
                onClick={onPrepare}
                disabled={isWorking || !canPrepare}
                className="bg-[#7B61FF] hover:bg-[#7B61FF]/80 text-white"
              >
                Prepare wallet funding
              </Button>
              {activeFlowId && onRefreshStatus && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={onRefreshStatus}
                  disabled={isWorking}
                  className="text-muted-foreground hover:text-foreground"
                >
                  Refresh status
                </Button>
              )}
            </div>
          </div>
        )}

        {/* Eligibility result */}
        {eligibility && (
          <div className="rounded-xl border border-white/[0.08] bg-white/[0.02] p-4 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                <CheckCircle2 className="h-4 w-4 text-[#00FF87]" />
                Eligibility
              </div>
              <StatusBadge status={eligibility.decision} />
            </div>
            <p className="text-xs text-muted-foreground">
              Source of funds policy:{" "}
              <strong className="text-foreground">{eligibility.sourceOfFunds.action}</strong> via{" "}
              {eligibility.sourceOfFunds.source}.
            </p>
            {eligibility.reasons.length > 0 && (
              <ul className="space-y-2">
                {eligibility.reasons.map((reason) => (
                  <li
                    key={`${reason.code}-${reason.message}`}
                    className="rounded-lg bg-white/[0.03] border border-white/[0.06] px-3 py-2 text-xs"
                  >
                    <span className="font-semibold text-foreground">{reason.code}</span>
                    {" — "}
                    <span className="text-muted-foreground">{reason.message}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}

        {/* Prepared flow */}
        {preparedFlow && (
          <div className="rounded-xl border border-white/[0.08] bg-white/[0.02] p-4 space-y-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                <CircleDashed className="h-4 w-4 text-[#7B61FF]" />
                Prepared transfer
              </div>
              <StatusBadge status={preparedFlow.status} severity="pending" />
            </div>
            <p className="text-xs text-muted-foreground">
              Durable transfer ID:{" "}
              <strong className="text-foreground font-mono">{preparedFlow.id}</strong>
            </p>
            {preparedFlow.checklist.length > 0 && (
              <ul className="space-y-2">
                {preparedFlow.checklist.map((item) => (
                  <li
                    key={`${item.code}-${item.message}`}
                    className="rounded-lg bg-white/[0.03] border border-white/[0.06] px-3 py-2 text-xs"
                  >
                    <span className="font-semibold text-foreground">{item.code}</span>
                    {" — "}
                    <span className="text-muted-foreground">{item.message}</span>
                  </li>
                ))}
              </ul>
            )}
            <div className="space-y-1.5">
              <label
                htmlFor="wallet-transfer-reference"
                className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground"
              >
                Wallet transfer reference
              </label>
              <Input
                id="wallet-transfer-reference"
                value={walletTransferReference}
                onChange={(e) => onWalletTransferReferenceChange(e.target.value)}
                placeholder="wallet_tx_001"
                className="bg-white/[0.03] border-white/[0.08] focus:border-[#7B61FF]/50 font-mono text-sm"
              />
            </div>
            <Button
              type="button"
              disabled={isWorking || walletTransferReference.trim().length === 0}
              onClick={() => onSubmit(preparedFlow.id, walletTransferReference)}
              className="bg-[#7B61FF] hover:bg-[#7B61FF]/80 text-white"
            >
              Mark wallet transfer submitted
            </Button>
          </div>
        )}

        {/* Submitted flow */}
        {submittedFlow && (
          <div className="rounded-xl border border-[#00FF87]/20 bg-[#00FF87]/5 p-4">
            <div className="flex items-center gap-2 text-sm font-semibold text-[#00FF87] mb-2">
              <CheckCircle2 className="h-4 w-4" />
              Wallet transfer submitted
            </div>
            <p className="text-xs text-muted-foreground">
              Reference{" "}
              <strong className="text-foreground font-mono">
                {submittedFlow.walletTransferReference}
              </strong>{" "}
              is recorded. Next action: {submittedFlow.nextAction}.
            </p>
          </div>
        )}

        {/* Status snapshot */}
        {statusSnapshot && (
          <div className="rounded-xl border border-white/[0.08] bg-white/[0.02] p-4 space-y-3">
            <div className="flex items-center justify-between">
              <p className="text-sm font-semibold text-foreground">Latest venue funding status</p>
              <StatusBadge status={statusSnapshot.status} />
            </div>
            <p className="text-xs text-muted-foreground">
              Eligibility:{" "}
              <strong className="text-foreground">{statusSnapshot.eligibilityDecision}</strong>
            </p>
            {statusSnapshot.blockingReasons.length > 0 && (
              <ul className="space-y-2">
                {statusSnapshot.blockingReasons.map((reason) => (
                  <li
                    key={`${reason.code}-${reason.message}`}
                    className="rounded-lg bg-white/[0.03] border border-white/[0.06] px-3 py-2 text-xs"
                  >
                    <span className="font-semibold text-foreground">{reason.code}</span>
                    {" — "}
                    <span className="text-muted-foreground">{reason.message}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}

        {/* Loading state */}
        {isLoading && (
          <div className="rounded-xl border border-white/[0.08] bg-white/[0.02] p-4 flex items-center gap-3 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin text-[#7B61FF]" />
            Loading venue funding workspace…
          </div>
        )}

        {/* Error state */}
        {error && (
          <div className="rounded-xl border border-red-400/20 bg-red-400/5 p-4 flex items-start gap-3">
            <AlertTriangle className="h-4 w-4 text-red-400 shrink-0 mt-0.5" />
            <p className="text-xs text-red-400">{error}</p>
          </div>
        )}
      </div>
    </Panel>
  )
}
