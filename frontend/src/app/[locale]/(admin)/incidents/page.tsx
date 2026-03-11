"use client";

import { useState } from "react";
import { Loader2, Radar, Search, Waves } from "lucide-react";

import IncidentTimeline, {
  IncidentSearchResult,
  IncidentTimelineResponse,
} from "@/components/incidents/IncidentTimeline";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

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
      const payload = (await response.json()) as { error?: { message?: string } };
      message = payload.error?.message ?? message;
    } catch {
      // Keep the default message when the body is not JSON.
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
        apiRequest<{ data: IncidentSearchResult[] }>(`/v1/admin/incidents/search?${queryString}`),
        apiRequest<IncidentTimelineResponse>(`/v1/admin/incidents/timeline?${queryString}`),
      ]);

      setSummary(searchPayload.data[0] ?? null);
      setTimeline(timelinePayload);
    } catch (requestError) {
      setError(
        requestError instanceof Error ? requestError.message : "Failed to load incident data",
      );
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Incident Timeline</h1>
          <p className="text-muted-foreground">
            Search correlated webhook, settlement, RFQ, and reconciliation evidence without leaving
            the bounded operator surface.
          </p>
        </div>
        <div className="flex items-center gap-2 rounded-full border bg-muted/30 px-3 py-1 text-sm text-muted-foreground">
          <Waves className="h-4 w-4" />
          Realtime path active
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Lookup lanes</CardDescription>
            <CardTitle>4</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Loaded incident</CardDescription>
            <CardTitle>{summary?.incidentId ?? "Not loaded"}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Action mode</CardDescription>
            <CardTitle>{timeline?.actionMode ?? "Recommendation-only"}</CardTitle>
          </CardHeader>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Incident search</CardTitle>
          <CardDescription>
            Query by intent ID, bank reference, webhook ID, or RFQ ID. Multiple fields can be used
            together for tighter correlation.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <div className="space-y-2">
              <Label htmlFor="intent-id">Intent ID</Label>
              <Input
                id="intent-id"
                aria-label="Intent ID"
                value={lookup.intentId}
                onChange={(event) => setLookup((current) => ({ ...current, intentId: event.target.value }))}
                placeholder="intent_..."
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="bank-reference">Bank reference</Label>
              <Input
                id="bank-reference"
                aria-label="Bank reference"
                value={lookup.bankReference}
                onChange={(event) =>
                  setLookup((current) => ({ ...current, bankReference: event.target.value }))
                }
                placeholder="RAMP-..."
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="webhook-id">Webhook ID</Label>
              <Input
                id="webhook-id"
                aria-label="Webhook ID"
                value={lookup.webhookId}
                onChange={(event) =>
                  setLookup((current) => ({ ...current, webhookId: event.target.value }))
                }
                placeholder="evt_..."
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="rfq-id">RFQ ID</Label>
              <Input
                id="rfq-id"
                aria-label="RFQ ID"
                value={lookup.rfqId}
                onChange={(event) => setLookup((current) => ({ ...current, rfqId: event.target.value }))}
                placeholder="rfq_..."
              />
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <Button onClick={handleLookup} disabled={loading}>
              {loading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Loading incidents
                </>
              ) : (
                <>
                  <Search className="mr-2 h-4 w-4" />
                  Load incident
                </>
              )}
            </Button>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Radar className="h-4 w-4" />
              Recommendation visibility stays audited and non-destructive.
            </div>
          </div>

          {error && (
            <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              {error}
            </div>
          )}
        </CardContent>
      </Card>

      {summary && timeline ? (
        <IncidentTimeline summary={summary} timeline={timeline} />
      ) : (
        <Card className="border-dashed">
          <CardHeader>
            <CardTitle>Awaiting search</CardTitle>
            <CardDescription>
              Load an incident to render summary, recommendations, and the correlated event
              timeline.
            </CardDescription>
          </CardHeader>
        </Card>
      )}
    </div>
  );
}
