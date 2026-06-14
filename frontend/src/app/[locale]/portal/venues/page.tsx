"use client";

import { useEffect, useState } from "react";

import { PageContainer } from "@/components/layout/page-container";
import { PageHeader } from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { useAuth } from "@/contexts/auth-context";
import { useRouter, Link } from "@/navigation";
import {
  VenueFundingConnectionResponse,
  VenueFundingEligibilityResponse,
  VenueFundingPrepareRequest,
  VenueFundingPrepareResponse,
  VenueFundingStatusResponse,
  VenueFundingSubmitResponse,
  VenueSummary,
  venueFundingApi,
} from "@/lib/portal-api";
import { VenueFundingCard } from "@/components/portal/venue-funding-card";
import { Wallet, Info } from "lucide-react";

const DEFAULT_FUNDING_REQUEST = {
  jurisdiction: "VN",
  asset: "USDT",
  network: "ethereum",
} as const;

export default function VenueFundingPage() {
  const router = useRouter();
  const { wallet, isAuthenticated, isLoading: authLoading } = useAuth();
  const [venues, setVenues] = useState<VenueSummary[]>([]);
  const [selectedVenueKey, setSelectedVenueKey] = useState<string | null>(null);
  const [connection, setConnection] = useState<VenueFundingConnectionResponse | null>(null);
  const [eligibility, setEligibility] = useState<VenueFundingEligibilityResponse | null>(null);
  const [preparedFlow, setPreparedFlow] = useState<VenueFundingPrepareResponse | null>(null);
  const [submittedFlow, setSubmittedFlow] = useState<VenueFundingSubmitResponse | null>(null);
  const [statusSnapshot, setStatusSnapshot] = useState<VenueFundingStatusResponse | null>(null);
  const [amount, setAmount] = useState("125");
  const [venueConnectionId, setVenueConnectionId] = useState("");
  const [venueAccountId, setVenueAccountId] = useState("");
  const [walletAttestationId, setWalletAttestationId] = useState("");
  const [walletTransferReference, setWalletTransferReference] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isWorking, setIsWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  useEffect(() => {
    if (!isAuthenticated) {
      return;
    }

    let cancelled = false;

    async function loadVenues() {
      setIsLoading(true);
      setError(null);

      try {
        const response = await venueFundingApi.listVenues();
        if (cancelled) {
          return;
        }

        setVenues(response.venues);
        const activePilotVenue =
          response.venues.find((venue) => venue.supportsWalletFunding)?.venueKey ??
          response.venues[0]?.venueKey ??
          null;
        setSelectedVenueKey((current) => current ?? activePilotVenue);
      } catch {
        if (!cancelled) {
          setError(
            "Failed to load venue funding lanes. Try again after the portal session is refreshed."
          );
        }
      } finally {
        if (!cancelled) {
          setIsLoading(false);
        }
      }
    }

    loadVenues();

    return () => {
      cancelled = true;
    };
  }, [isAuthenticated]);

  function buildFundingRequest(): VenueFundingPrepareRequest | null {
    if (!selectedVenueKey) {
      setError("Select a venue before continuing.");
      return null;
    }

    return {
      venueKey: selectedVenueKey,
      jurisdiction: DEFAULT_FUNDING_REQUEST.jurisdiction,
      asset: DEFAULT_FUNDING_REQUEST.asset,
      network: DEFAULT_FUNDING_REQUEST.network,
      venueConnectionId: venueConnectionId.trim(),
      venueAccountId: venueAccountId.trim(),
      walletAttestationId: walletAttestationId.trim(),
      amount: amount.trim(),
    };
  }

  async function handleConnect() {
    const request = buildFundingRequest();
    if (!request) {
      return;
    }

    setIsWorking(true);
    setError(null);

    try {
      const response = await venueFundingApi.connect(request);
      setConnection(response);
      setVenueConnectionId(response.connections[0]?.id ?? "");
      setVenueAccountId(response.accounts[0]?.id ?? "");
    } catch {
      setError("Venue connection could not be created for this wallet-first lane.");
    } finally {
      setIsWorking(false);
    }
  }

  async function handleCheckEligibility() {
    const request = buildFundingRequest();
    if (!request) {
      return;
    }

    setIsWorking(true);
    setError(null);

    try {
      const response = await venueFundingApi.getEligibility({
        ...request,
        connectionId: connection?.id,
      });
      setEligibility(response);
    } catch {
      setError("Eligibility could not be evaluated for the selected venue.");
    } finally {
      setIsWorking(false);
    }
  }

  async function handlePrepare() {
    const request = buildFundingRequest();
    if (!request) {
      return;
    }

    setIsWorking(true);
    setError(null);

    try {
      const response = await venueFundingApi.prepare(request);
      setPreparedFlow(response);
      setSubmittedFlow(null);
      setStatusSnapshot(null);
    } catch {
      setError("The portal could not prepare a wallet funding lane for this venue.");
    } finally {
      setIsWorking(false);
    }
  }

  async function handleSubmit(flowId: string, reference: string) {
    setIsWorking(true);
    setError(null);

    try {
      const response = await venueFundingApi.submit(flowId, reference.trim());
      setSubmittedFlow(response);
    } catch {
      setError("The wallet transfer reference could not be recorded for venue funding.");
    } finally {
      setIsWorking(false);
    }
  }

  async function handleRefreshStatus() {
    const flowId = submittedFlow?.id ?? preparedFlow?.id;
    if (!flowId) {
      setError("Prepare or submit a wallet funding flow before refreshing status.");
      return;
    }

    setIsWorking(true);
    setError(null);

    try {
      const response = await venueFundingApi.getStatus(flowId);
      setStatusSnapshot(response);
    } catch {
      setError("The latest venue funding status could not be loaded.");
    } finally {
      setIsWorking(false);
    }
  }

  useEffect(() => {
    setConnection(null);
    setEligibility(null);
    setPreparedFlow(null);
    setSubmittedFlow(null);
    setStatusSnapshot(null);
    setVenueConnectionId("");
    setVenueAccountId("");
    setWalletAttestationId("");
    setWalletTransferReference("");
    setError(null);
  }, [selectedVenueKey]);

  if (!wallet && !authLoading) {
    return (
      <PageContainer>
        <PageHeader
          title="Venue Funding"
          description="Create your governed wallet first, then move settled wallet funds into a supported venue."
        />
        <div className="rounded-xl border border-white/[0.06] bg-[#111113] p-8">
          <div className="flex flex-col items-start gap-5">
            <div className="h-12 w-12 rounded-xl bg-[#00FF87]/10 border border-[#00FF87]/20 flex items-center justify-center">
              <Wallet className="h-6 w-6 text-[#00FF87]" />
            </div>
            <div>
              <p className="text-sm text-white/60 leading-relaxed max-w-md">
                Venue funding only starts after your governed wallet is available. Use the deposit
                flow to create or fund the wallet first.
              </p>
            </div>
            <Button
              asChild
              className="bg-[#00FF87] text-[#09090B] font-semibold hover:bg-[#00FF87]/90"
            >
              <Link href="/portal/deposit">Go to wallet deposit</Link>
            </Button>
          </div>
        </div>
      </PageContainer>
    );
  }

  return (
    <PageContainer>
      <PageHeader
        title="Venue Funding"
        description="Keep deposit-to-wallet separate from wallet-to-venue movement while preserving an operator-readable trail for the Hyperliquid USDT pilot."
      />

      <div className="mx-auto max-w-5xl space-y-6">
        <div className="flex items-start gap-3 rounded-lg border border-[#00D4FF]/20 bg-[#00D4FF]/[0.05] p-4">
          <Info className="h-4 w-4 mt-0.5 shrink-0 text-[#00D4FF]" />
          <p className="text-sm text-[#00D4FF]/80">
            This bounded slice uses the verified portal venue-funding API surface and currently
            funds only the Hyperliquid wallet-linked pilot on VN / USDT / Ethereum. Prepare now
            creates a durable transfer draft, so venue IDs, attestation ID, and amount must be
            explicit.
          </p>
        </div>

        <VenueFundingCard
          venues={venues}
          selectedVenueKey={selectedVenueKey}
          amount={amount}
          venueConnectionId={venueConnectionId}
          venueAccountId={venueAccountId}
          walletAttestationId={walletAttestationId}
          walletTransferReference={walletTransferReference}
          eligibility={eligibility}
          preparedFlow={preparedFlow}
          submittedFlow={submittedFlow}
          statusSnapshot={statusSnapshot}
          isLoading={isLoading}
          isWorking={isWorking}
          error={error}
          onSelectVenue={setSelectedVenueKey}
          onConnect={handleConnect}
          onCheckEligibility={handleCheckEligibility}
          onPrepare={handlePrepare}
          onSubmit={handleSubmit}
          onRefreshStatus={handleRefreshStatus}
          onAmountChange={setAmount}
          onVenueConnectionIdChange={setVenueConnectionId}
          onVenueAccountIdChange={setVenueAccountId}
          onWalletAttestationIdChange={setWalletAttestationId}
          onWalletTransferReferenceChange={setWalletTransferReference}
        />
      </div>
    </PageContainer>
  );
}
