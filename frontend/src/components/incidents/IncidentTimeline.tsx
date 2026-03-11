"use client";

import {
  AlertTriangle,
  Clock3,
  FileWarning,
  Radar,
  ShieldAlert,
  Waves,
} from "lucide-react";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

export type IncidentSearchResult = {
  incidentId: string;
  matchedBy: string[];
  relatedReferenceIds: string[];
  entryCount: number;
  recommendationCount: number;
  latestStatus?: string | null;
  latestOccurredAt?: string | null;
};

export type IncidentTimelineEntry = {
  sequence: number;
  sourceKind: string;
  sourceReferenceId: string;
  occurredAt: string;
  label: string;
  status: string;
  confidence: string;
  relatedReferenceIds: string[];
  details: Record<string, unknown>;
};

export type IncidentRecommendation = {
  code: string;
  title: string;
  summary: string;
  confidence: string;
  priority: string;
  mode: string;
  relatedEntryIds: string[];
};

export type IncidentTimelineResponse = {
  incidentId: string;
  generatedAt: string;
  actionMode: string;
  entries: IncidentTimelineEntry[];
  recommendations: IncidentRecommendation[];
};

interface IncidentTimelineProps {
  summary: IncidentSearchResult;
  timeline: IncidentTimelineResponse;
}

function formatTimestamp(value?: string | null): string {
  if (!value) return "N/A";
  return new Date(value).toLocaleString("vi-VN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function prettyLabel(value: string): string {
  return value
    .replaceAll("_", " ")
    .replaceAll(".", " ")
    .toLowerCase()
    .replace(/(^\w|\s\w)/g, (match) => match.toUpperCase());
}

function sourceIcon(sourceKind: string) {
  switch (sourceKind) {
    case "webhook":
      return <Waves className="h-4 w-4" />;
    case "settlement":
      return <ShieldAlert className="h-4 w-4" />;
    case "reconciliation":
      return <FileWarning className="h-4 w-4" />;
    case "rfq":
      return <Radar className="h-4 w-4" />;
    default:
      return <Clock3 className="h-4 w-4" />;
  }
}

function deriveIncidentGuardian(summary: IncidentSearchResult, timeline: IncidentTimelineResponse) {
  const hasHighPriorityRecommendation = timeline.recommendations.some(
    (recommendation) => recommendation.priority.toLowerCase() === "high",
  );
  const hasWebhookFailure = timeline.entries.some(
    (entry) => entry.sourceKind === "webhook" && entry.status.toLowerCase() === "failed",
  );

  if (hasWebhookFailure || hasHighPriorityRecommendation) {
    return {
      headline: "Review within 15 minutes",
      recommendation: "Recommend escalation to webhook operations",
      rationale: `Correlated incident has ${summary.recommendationCount} recommendation signal(s) and ${
        summary.entryCount
      } timeline entr${summary.entryCount === 1 ? "y" : "ies"}.`,
    };
  }

  return {
    headline: "Review inside the next 60 minutes",
    recommendation: "Recommend operator follow-up on the next scheduled queue sweep",
    rationale: "Signals remain informational and can stay in the bounded operator lane.",
  };
}

export default function IncidentTimeline({ summary, timeline }: IncidentTimelineProps) {
  const guardian = deriveIncidentGuardian(summary, timeline);

  return (
    <div className="space-y-6" data-testid="incident-timeline">
      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Incident</CardDescription>
            <CardTitle className="text-base font-mono">{summary.incidentId}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Latest status</CardDescription>
            <CardTitle>{summary.latestStatus ?? "Unknown"}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Timeline entries</CardDescription>
            <CardTitle>{summary.entryCount}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Recommendations</CardDescription>
            <CardTitle>{summary.recommendationCount}</CardTitle>
          </CardHeader>
        </Card>
      </div>

      <Card className="border-amber-200 bg-amber-50/60">
        <CardHeader>
          <div className="flex items-center gap-2 text-amber-900">
            <AlertTriangle className="h-4 w-4" />
            <CardTitle className="text-base">Guardrail: recommendation-only</CardTitle>
          </div>
          <CardDescription className="text-amber-900/80">
            Incident actions stay operator-audited. This page only surfaces correlation, confidence,
            and recommended next checks.
          </CardDescription>
        </CardHeader>
      </Card>

      <Card className="border-blue-200 bg-blue-50/70">
        <CardHeader>
          <CardTitle className="text-base text-blue-950">SLA guardian</CardTitle>
          <CardDescription className="text-blue-900/80">
            {guardian.headline}. {guardian.recommendation}.
          </CardDescription>
        </CardHeader>
        <CardContent className="text-sm text-blue-950/85">
          {guardian.rationale}
        </CardContent>
      </Card>

      <div className="grid gap-6 xl:grid-cols-[0.9fr,1.1fr]">
        <Card>
          <CardHeader>
            <CardTitle>Summary</CardTitle>
            <CardDescription>
              Matched by {summary.matchedBy.join(", ")} · Updated{" "}
              {formatTimestamp(summary.latestOccurredAt)}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <div className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                Related references
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {summary.relatedReferenceIds.map((reference) => (
                  <span
                    key={reference}
                    className="rounded-full border bg-muted/40 px-3 py-1 text-xs font-medium"
                  >
                    {reference}
                  </span>
                ))}
              </div>
            </div>

            <div className="rounded-lg border bg-muted/20 p-4 text-sm">
              <div className="font-medium">Realtime path</div>
              <p className="mt-2 text-muted-foreground">
                Timeline refreshes reuse the existing WebSocket event envelope and keep tenant scope
                intact.
              </p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Recommended checks</CardTitle>
            <CardDescription>
              Derived from current timeline signals and in-memory metrics only.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {timeline.recommendations.length === 0 ? (
              <div className="rounded-lg border border-dashed p-4 text-sm text-muted-foreground">
                No recommendations were raised for this incident snapshot.
              </div>
            ) : (
              timeline.recommendations.map((recommendation) => (
                <div
                  key={recommendation.code}
                  className="rounded-lg border bg-background p-4"
                  data-testid="incident-recommendation"
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-medium">{recommendation.title}</span>
                    <span className="rounded-full border px-2 py-0.5 text-[11px] uppercase tracking-wide text-muted-foreground">
                      {prettyLabel(recommendation.priority)}
                    </span>
                    <span className="rounded-full border px-2 py-0.5 text-[11px] uppercase tracking-wide text-muted-foreground">
                      {prettyLabel(recommendation.confidence)}
                    </span>
                  </div>
                  <p className="mt-2 text-sm text-muted-foreground">{recommendation.summary}</p>
                  {recommendation.relatedEntryIds.length > 0 && (
                    <div className="mt-3 flex flex-wrap gap-2">
                      {recommendation.relatedEntryIds.map((reference) => (
                        <span
                          key={reference}
                          className="rounded-full bg-muted px-2 py-1 text-[11px] font-medium"
                        >
                          {reference}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              ))
            )}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Timeline</CardTitle>
          <CardDescription>
            Generated {formatTimestamp(timeline.generatedAt)} · Mode{" "}
            {prettyLabel(timeline.actionMode)}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {timeline.entries.map((entry) => (
            <div
              key={`${entry.sourceKind}-${entry.sourceReferenceId}`}
              className="rounded-xl border bg-background p-4"
              data-testid="incident-entry"
            >
              <div className="flex flex-wrap items-start justify-between gap-4">
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    {sourceIcon(entry.sourceKind)}
                    <span>{prettyLabel(entry.sourceKind)}</span>
                    <span>#{entry.sequence}</span>
                  </div>
                  <div className="font-semibold">{entry.label}</div>
                  <div className="flex flex-wrap gap-2 text-xs">
                    <span className="rounded-full border px-2 py-1 uppercase tracking-wide">
                      {entry.status}
                    </span>
                    <span className="rounded-full border px-2 py-1 uppercase tracking-wide text-muted-foreground">
                      {prettyLabel(entry.confidence)}
                    </span>
                    <span className="rounded-full border px-2 py-1 font-mono">
                      {entry.sourceReferenceId}
                    </span>
                  </div>
                </div>
                <div className="text-sm text-muted-foreground">{formatTimestamp(entry.occurredAt)}</div>
              </div>

              {entry.relatedReferenceIds.length > 0 && (
                <div className="mt-3 flex flex-wrap gap-2">
                  {entry.relatedReferenceIds.map((reference) => (
                    <span
                      key={reference}
                      className="rounded-full border bg-muted/40 px-2 py-1 text-[11px] font-medium"
                    >
                      {reference}
                    </span>
                  ))}
                </div>
              )}

              <pre className="mt-4 overflow-x-auto rounded-lg bg-slate-950 p-4 text-xs text-slate-100">
                {JSON.stringify(entry.details, null, 2)}
              </pre>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
