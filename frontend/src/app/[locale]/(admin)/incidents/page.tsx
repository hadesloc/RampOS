"use client";

import { useState } from "react";
import { Loader2, Radar, Search, Waves, ActivitySquare } from "lucide-react";

import IncidentTimeline, {
  IncidentSearchResult,
  IncidentTimelineResponse,
} from "@/components/incidents/IncidentTimeline";

import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  EmptyState,
  ErrorState,
} from "@/components/shared";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

// ── API helper ─────────────────────────────────────────────────────────────────

type LookupState = {
  intentId: string;
  bankReference: string;
  webhookId: string;
  rfqId: string;
};

async function apiRequest<T>(endpoint: string): Promise<T> {
  const response = await fetch(`/api/proxy${endpoint}`);
  if (!response.ok) {
    let message = "Request failed";
    try {
      const payload = (await response.json()) as {
        error?: { message?: string };
      };
      message = payload.error?.message ?? message;
    } catch {
      /* keep default */
    }
    throw new Error(message);
  }
  return response.json() as Promise<T>;
}

function buildQueryString(lookup: LookupState): string {
  const params = new URLSearchParams();
  if (lookup.intentId) params.set("intentId", lookup.intentId);
  if (lookup.bankReference) params.set("bankReference", lookup.bankReference);
  if (lookup.webhookId) params.set("webhookId", lookup.webhookId);
  if (lookup.rfqId) params.set("rfqId", lookup.rfqId);
  return params.toString();
}

// ── Page ───────────────────────────────────────────────────────────────────────

export default function IncidentsPage() {
  const [lookup, setLookup] = useState<LookupState>({
    intentId: "intent_incident_001",
    bankReference: "",
    webhookId: "",
    rfqId: "",
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [summary, setSummary] = useState<IncidentSearchResult | null>(null);
  const [timeline, setTimeline] = useState<IncidentTimelineResponse | null>(null);

  const handleLookup = async () => {
    const queryString = buildQueryString(lookup);
    if (!queryString) {
      setError("Provide at least one lookup value before loading incidents.");
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const [searchPayload, timelinePayload] = await Promise.all([
        apiRequest<{ data: IncidentSearchResult[] }>(
          `/v1/admin/incidents/search?${queryString}`
        ),
        apiRequest<IncidentTimelineResponse>(
          `/v1/admin/incidents/timeline?${queryString}`
        ),
      ]);

      setSummary(searchPayload.data[0] ?? null);
      setTimeline(timelinePayload);
    } catch (requestError) {
      setError(
        requestError instanceof Error
          ? requestError.message
          : "Failed to load incident data"
      );
    } finally {
      setLoading(false);
    }
  };

  const hasResult = summary !== null && timeline !== null;

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Incident Timeline"
        description="Search correlated webhook, settlement, RFQ, and reconciliation evidence without leaving the bounded operator surface."
        breadcrumb={[
          { label: "Admin", href: "/admin" },
          { label: "Incidents" },
        ]}
        actions={
          <div className="flex items-center gap-2 rounded-full border border-white/[0.08] bg-[#00D4FF]/5 px-3 py-1.5 text-xs text-[#00D4FF]">
            <Waves className="h-3.5 w-3.5" />
            <span>Realtime path active</span>
          </div>
        }
      />

      {/* Context stats */}
      <StatGrid cols={3}>
        <StatCard
          title="Lookup Lanes"
          value="4"
          icon={<Radar className="h-4 w-4" />}
          accentColor="cyan"
          subtitle="intent · bank ref · webhook · RFQ"
        />
        <StatCard
          title="Loaded Incident"
          value={summary?.incidentId ?? "Not loaded"}
          icon={<ActivitySquare className="h-4 w-4" />}
          accentColor={hasResult ? "green" : "violet"}
          subtitle={hasResult ? `${summary?.entryCount} entries` : "Run a search first"}
        />
        <StatCard
          title="Action Mode"
          value={timeline?.actionMode ?? "Recommendation-only"}
          icon={<Waves className="h-4 w-4" />}
          accentColor="amber"
        />
      </StatGrid>

      {/* Search panel */}
      <Panel
        header={{
          title: "Incident Search",
          description:
            "Query by intent ID, bank reference, webhook ID, or RFQ ID. Multiple fields can be used together for tighter correlation.",
        }}
      >
        <div className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <div className="space-y-2">
              <Label htmlFor="intent-id" className="text-xs text-muted-foreground">
                Intent ID
              </Label>
              <Input
                id="intent-id"
                aria-label="Intent ID"
                value={lookup.intentId}
                onChange={(e) =>
                  setLookup((c) => ({ ...c, intentId: e.target.value }))
                }
                placeholder="intent_..."
                className="border-white/[0.08] bg-white/[0.02] focus:border-[#00D4FF]/40"
              />
            </div>
            <div className="space-y-2">
              <Label
                htmlFor="bank-reference"
                className="text-xs text-muted-foreground"
              >
                Bank Reference
              </Label>
              <Input
                id="bank-reference"
                aria-label="Bank reference"
                value={lookup.bankReference}
                onChange={(e) =>
                  setLookup((c) => ({ ...c, bankReference: e.target.value }))
                }
                placeholder="RAMP-..."
                className="border-white/[0.08] bg-white/[0.02] focus:border-[#00D4FF]/40"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="webhook-id" className="text-xs text-muted-foreground">
                Webhook ID
              </Label>
              <Input
                id="webhook-id"
                aria-label="Webhook ID"
                value={lookup.webhookId}
                onChange={(e) =>
                  setLookup((c) => ({ ...c, webhookId: e.target.value }))
                }
                placeholder="evt_..."
                className="border-white/[0.08] bg-white/[0.02] focus:border-[#00D4FF]/40"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="rfq-id" className="text-xs text-muted-foreground">
                RFQ ID
              </Label>
              <Input
                id="rfq-id"
                aria-label="RFQ ID"
                value={lookup.rfqId}
                onChange={(e) =>
                  setLookup((c) => ({ ...c, rfqId: e.target.value }))
                }
                placeholder="rfq_..."
                className="border-white/[0.08] bg-white/[0.02] focus:border-[#00D4FF]/40"
              />
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <Button
              onClick={handleLookup}
              disabled={loading}
              className="bg-[#00FF87] text-black font-semibold hover:bg-[#00FF87]/90 min-w-[140px]"
            >
              {loading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Loading…
                </>
              ) : (
                <>
                  <Search className="mr-2 h-4 w-4" />
                  Load Incident
                </>
              )}
            </Button>
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Radar className="h-3.5 w-3.5 text-[#7B61FF]" />
              Recommendation visibility stays audited and non-destructive.
            </div>
          </div>

          {error && (
            <ErrorState
              title="Lookup failed"
              message={error}
              retry={handleLookup}
            />
          )}
        </div>
      </Panel>

      {/* Results */}
      {hasResult ? (
        <IncidentTimeline summary={summary} timeline={timeline} />
      ) : (
        <Panel>
          <EmptyState
            icon={<ActivitySquare className="h-10 w-10" />}
            title="Awaiting search"
            description="Load an incident to render summary, recommendations, and the correlated event timeline."
          />
        </Panel>
      )}
    </main>
  );
}
