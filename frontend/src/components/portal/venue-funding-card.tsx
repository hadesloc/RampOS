"use client";

import { Building2, CheckCircle2, CircleDashed, Landmark, ShieldCheck } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  VenueFundingEligibilityResponse,
  VenueFundingPrepareResponse,
  VenueFundingStatusResponse,
  VenueFundingSubmitResponse,
  VenueSummary,
} from "@/lib/portal-api";

interface VenueFundingCardProps {
  venues: VenueSummary[];
  selectedVenueKey: string | null;
  amount: string;
  venueConnectionId: string;
  venueAccountId: string;
  walletAttestationId: string;
  walletTransferReference: string;
  eligibility?: VenueFundingEligibilityResponse | null;
  preparedFlow?: VenueFundingPrepareResponse | null;
  submittedFlow?: VenueFundingSubmitResponse | null;
  statusSnapshot?: VenueFundingStatusResponse | null;
  isLoading?: boolean;
  isWorking?: boolean;
  error?: string | null;
  onSelectVenue: (venueKey: string) => void;
  onConnect: () => void;
  onCheckEligibility: () => void;
  onPrepare: () => void;
  onSubmit: (preparedFlowId: string, walletTransferReference: string) => void;
  onRefreshStatus?: () => void;
  onAmountChange: (value: string) => void;
  onVenueConnectionIdChange: (value: string) => void;
  onVenueAccountIdChange: (value: string) => void;
  onWalletAttestationIdChange: (value: string) => void;
  onWalletTransferReferenceChange: (value: string) => void;
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
  const selectedVenue = venues.find((venue) => venue.venueKey === selectedVenueKey) ?? null;
  const activeFlowId = submittedFlow?.id ?? preparedFlow?.id;
  const canPrepare =
    amount.trim().length > 0 &&
    venueConnectionId.trim().length > 0 &&
    venueAccountId.trim().length > 0 &&
    walletAttestationId.trim().length > 0;

  return (
    <Card className="w-full border-primary/10 shadow-sm">
      <CardHeader className="space-y-3">
        <div className="flex items-center gap-2 text-sm font-medium text-primary">
          <Landmark className="h-4 w-4" />
          Wallet first, venue second
        </div>
        <CardTitle>Venue funding</CardTitle>
        <CardDescription>
          Move governed wallet funds into a connected venue after wallet settlement. This workflow
          stays separate from generic deposit so the operator trail remains explicit.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="grid gap-3 md:grid-cols-2">
          {venues.map((venue) => {
            const isSelected = venue.venueKey === selectedVenueKey;
            return (
              <button
                key={venue.venueKey}
                type="button"
                className={`rounded-xl border p-4 text-left transition ${
                  isSelected
                    ? "border-primary bg-primary/5 shadow-sm"
                    : venue.supportsWalletFunding
                      ? "border-border hover:border-primary/40 hover:bg-muted/30"
                      : "border-border opacity-70"
                }`}
                onClick={() => {
                  if (venue.supportsWalletFunding) {
                    onSelectVenue(venue.venueKey);
                  }
                }}
                disabled={!venue.supportsWalletFunding}
              >
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <h3 className="text-base font-semibold">{venue.displayName}</h3>
                    <p className="mt-1 text-sm text-muted-foreground">
                      {venue.supportsWalletFunding
                        ? "Active wallet-funding pilot for Hyperliquid USDT lanes"
                        : "Visible for discovery only; wallet funding is not active for this venue"}
                    </p>
                  </div>
                  <span className="rounded-full bg-muted px-2 py-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {venue.status}
                  </span>
                </div>
              </button>
            );
          })}
        </div>

        <div className="rounded-xl border border-dashed border-border bg-muted/20 p-4">
          <div className="flex items-start gap-3">
            <ShieldCheck className="mt-0.5 h-5 w-5 text-primary" />
            <div className="space-y-1 text-sm">
              <p className="font-medium text-foreground">Bounded portal defaults</p>
              <p className="text-muted-foreground">
                This slice assumes the Hyperliquid pilot only: jurisdiction <strong>VN</strong>,
                asset <strong>USDT</strong>, and network <strong>Ethereum</strong> until broader
                venue funding configuration lands.
              </p>
            </div>
          </div>
        </div>

        {selectedVenue && (
          <div className="space-y-4 rounded-xl border border-border bg-background p-4">
            <div className="flex items-center gap-2 text-sm font-medium text-foreground">
              <Building2 className="h-4 w-4 text-primary" />
              Active venue: {selectedVenue.displayName}
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <label htmlFor="funding-amount" className="text-sm font-medium text-foreground">
                  Funding amount
                </label>
                <Input
                  id="funding-amount"
                  value={amount}
                  onChange={(event) => onAmountChange(event.target.value)}
                  placeholder="125"
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="venue-connection-id" className="text-sm font-medium text-foreground">
                  Venue connection ID
                </label>
                <Input
                  id="venue-connection-id"
                  value={venueConnectionId}
                  onChange={(event) => onVenueConnectionIdChange(event.target.value)}
                  placeholder="conn_123"
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="venue-account-id" className="text-sm font-medium text-foreground">
                  Venue account ID
                </label>
                <Input
                  id="venue-account-id"
                  value={venueAccountId}
                  onChange={(event) => onVenueAccountIdChange(event.target.value)}
                  placeholder="acct_123"
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="wallet-attestation-id" className="text-sm font-medium text-foreground">
                  Wallet attestation ID
                </label>
                <Input
                  id="wallet-attestation-id"
                  value={walletAttestationId}
                  onChange={(event) => onWalletAttestationIdChange(event.target.value)}
                  placeholder="550e8400-e29b-41d4-a716-446655440000"
                />
              </div>
            </div>

            <p className="text-xs text-muted-foreground">
              Connect can prefill the venue connection and account IDs. Hyperliquid funding stays
              wallet-linked and USDT-only in this pilot. The attestation ID and amount must be
              explicit because prepare now creates a durable transfer draft.
            </p>

            <div className="flex flex-wrap gap-3">
              <Button type="button" variant="outline" onClick={onConnect} disabled={isWorking}>
                Connect venue
              </Button>
              <Button type="button" variant="outline" onClick={onCheckEligibility} disabled={isWorking}>
                Check eligibility
              </Button>
              <Button type="button" onClick={onPrepare} disabled={isWorking || !canPrepare}>
                Prepare wallet funding
              </Button>
              {activeFlowId && onRefreshStatus && (
                <Button type="button" variant="ghost" onClick={onRefreshStatus} disabled={isWorking}>
                  Refresh status
                </Button>
              )}
            </div>
          </div>
        )}

        {eligibility && (
          <div className="rounded-xl border border-border p-4">
            <div className="flex items-center gap-2 text-sm font-medium">
              <CheckCircle2 className="h-4 w-4 text-primary" />
              Eligibility: {eligibility.decision}
            </div>
            <p className="mt-2 text-sm text-muted-foreground">
              Source of funds policy: {eligibility.sourceOfFunds.action} via {eligibility.sourceOfFunds.source}.
            </p>
            {eligibility.reasons.length > 0 && (
              <ul className="mt-3 space-y-2 text-sm text-muted-foreground">
                {eligibility.reasons.map((reason) => (
                  <li key={`${reason.code}-${reason.message}`} className="rounded-lg bg-muted/40 px-3 py-2">
                    <span className="font-medium text-foreground">{reason.code}</span>
                    {" - "}
                    {reason.message}
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}

        {preparedFlow && (
          <div className="space-y-4 rounded-xl border border-border p-4">
            <div className="flex items-center gap-2 text-sm font-medium">
              <CircleDashed className="h-4 w-4 text-primary" />
              Prepared transfer: {preparedFlow.status}
            </div>
            <p className="text-sm text-muted-foreground">
              Durable transfer ID: <strong>{preparedFlow.id}</strong>
            </p>
            <div className="space-y-2 text-sm text-muted-foreground">
              {preparedFlow.checklist.map((item) => (
                <div key={`${item.code}-${item.message}`} className="rounded-lg bg-muted/40 px-3 py-2">
                  <span className="font-medium text-foreground">{item.code}</span>
                  {" - "}
                  {item.message}
                </div>
              ))}
            </div>
            <div className="space-y-2">
              <label htmlFor="wallet-transfer-reference" className="text-sm font-medium text-foreground">
                Wallet transfer reference
              </label>
              <Input
                id="wallet-transfer-reference"
                value={walletTransferReference}
                onChange={(event) => onWalletTransferReferenceChange(event.target.value)}
                placeholder="wallet_tx_001"
              />
            </div>
            <Button
              type="button"
              disabled={isWorking || walletTransferReference.trim().length === 0}
              onClick={() => onSubmit(preparedFlow.id, walletTransferReference)}
            >
              Mark wallet transfer submitted
            </Button>
          </div>
        )}

        {submittedFlow && (
          <div className="rounded-xl border border-green-500/30 bg-green-500/5 p-4">
            <div className="text-sm font-medium text-foreground">Wallet transfer submitted</div>
            <p className="mt-2 text-sm text-muted-foreground">
              Reference <strong>{submittedFlow.walletTransferReference}</strong> is recorded. Next action:{" "}
              {submittedFlow.nextAction}.
            </p>
          </div>
        )}

        {statusSnapshot && (
          <div className="rounded-xl border border-border p-4">
            <div className="text-sm font-medium text-foreground">Latest venue funding status</div>
            <p className="mt-2 text-sm text-muted-foreground">
              Status: <strong>{statusSnapshot.status}</strong>. Eligibility:{" "}
              <strong>{statusSnapshot.eligibilityDecision}</strong>.
            </p>
            {statusSnapshot.blockingReasons.length > 0 && (
              <ul className="mt-3 space-y-2 text-sm text-muted-foreground">
                {statusSnapshot.blockingReasons.map((reason) => (
                  <li key={`${reason.code}-${reason.message}`} className="rounded-lg bg-muted/40 px-3 py-2">
                    <span className="font-medium text-foreground">{reason.code}</span>
                    {" - "}
                    {reason.message}
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}

        {isLoading && (
          <div className="rounded-xl border border-border bg-muted/30 p-4 text-sm text-muted-foreground">
            Loading venue funding workspace...
          </div>
        )}

        {error && (
          <div className="rounded-xl border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
            {error}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
