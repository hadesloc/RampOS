"use client";

import {
  AlertTriangle,
  Clock3,
  FileWarning,
  Radar,
  ShieldAlert,
  Waves,
  Shield,
  Clock,
} from "lucide-react";

import { Panel, StatusBadge, EmptyState } from "@/components/shared";
import { formatDateTime } from "@/lib/format";

// ── Types (exported for page use) ─────────────────────────────────────────────

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

// ── Helpers ────────────────────────────────────────────────────────────────────

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
      return <Waves className="h-4 w-4 text-[#00D4FF]" />;
    case "settlement":
      return <ShieldAlert className="h-4 w-4 text-[#7B61FF]" />;
    case "reconciliation":
      return <FileWarning className="h-4 w-4 text-[#FFB800]" />;
    case "rfq":
      return <Radar className="h-4 w-4 text-[#00FF87]" />;
    default:
      return <Clock3 className="h-4 w-4 text-muted-foreground" />;
  }
}

function prioritySeverity(
  priority: string
): "danger" | "warning" | "info" | "neutral" {
  const p = priority.toLowerCase();
  if (p === "high") return "danger";
  if (p === "medium") return "warning";
  if (p === "low") return "info";
  return "neutral";
}

function deriveIncidentGuardian(
  summary: IncidentSearchResult,
  timeline: IncidentTimelineResponse
) {
  const hasHighPriorityRecommendation = timeline.recommendations.some(
    (r) => r.priority.toLowerCase() === "high"
  );
  const hasWebhookFailure = timeline.entries.some(
    (e) =>
      e.sourceKind === "webhook" && e.status.toLowerCase() === "failed"
  );

  if (hasWebhookFailure || hasHighPriorityRecommendation) {
    return {
      urgency: "high" as const,
      headline: "Review within 15 minutes",
      recommendation: "Recommend escalation to webhook operations",
      rationale: `Correlated incident has ${summary.recommendationCount} recommendation signal(s) and ${summary.entryCount} timeline entr${summary.entryCount === 1 ? "y" : "ies"}.`,
    };
  }

  return {
    urgency: "normal" as const,
    headline: "Review inside the next 60 minutes",
    recommendation: "Recommend operator follow-up on the next scheduled queue sweep",
    rationale: "Signals remain informational and can stay in the bounded operator lane.",
  };
}

// ── Component ──────────────────────────────────────────────────────────────────

export default function IncidentTimeline({
  summary,
  timeline,
}: IncidentTimelineProps) {
  const guardian = deriveIncidentGuardian(summary, timeline);

  const urgencyColor =
    guardian.urgency === "high"
      ? "border-red-400/30 bg-red-400/5"
      : "border-[#FFB800]/30 bg-[#FFB800]/5";

  const urgencyTextColor =
    guardian.urgency === "high" ? "text-red-400" : "text-[#FFB800]";

  return (
    <div className="flex flex-col gap-6" data-testid="incident-timeline">
      {/* Summary KPI strip */}
      <div className="grid gap-4 md:grid-cols-4">
        {[
          { label: "Incident ID", value: summary.incidentId, mono: true },
          { label: "Latest Status", value: summary.latestStatus ?? "Unknown", mono: false },
          { label: "Timeline Entries", value: summary.entryCount, mono: false },
          { label: "Recommendations", value: summary.recommendationCount, mono: false },
        ].map(({ label, value, mono }) => (
          <div
            key={label}
            className="rounded-xl border border-white/[0.06] bg-[#111113]/80 px-4 py-3"
          >
            <div className="text-xs text-muted-foreground mb-1">{label}</div>
            <div
              className={`text-sm font-semibold truncate ${mono ? "font-mono text-[#00D4FF]" : ""}`}
            >
              {String(value)}
            </div>
          </div>
        ))}
      </div>

      {/* Guardrail notice */}
      <div className="rounded-xl border border-[#FFB800]/20 bg-[#FFB800]/5 px-5 py-4">
        <div className="flex items-center gap-2 text-[#FFB800] mb-1">
          <AlertTriangle className="h-4 w-4" />
          <span className="text-sm font-semibold">Guardrail: recommendation-only</span>
        </div>
        <p className="text-xs text-muted-foreground">
          Incident actions stay operator-audited. This page only surfaces
          correlation, confidence, and recommended next checks.
        </p>
      </div>

      {/* SLA guardian */}
      <div className={`rounded-xl border px-5 py-4 ${urgencyColor}`}>
        <div className={`flex items-center gap-2 mb-1 ${urgencyTextColor}`}>
          <Clock className="h-4 w-4" />
          <span className="text-sm font-semibold">
            SLA Guardian — {guardian.headline}
          </span>
        </div>
        <p className="text-xs text-muted-foreground mb-1">
          {guardian.recommendation}.
        </p>
        <p className="text-xs text-muted-foreground">{guardian.rationale}</p>
      </div>

      {/* Summary + Recommendations grid */}
      <div className="grid gap-6 xl:grid-cols-[0.9fr,1.1fr]">
        <Panel header={{ title: "Summary", description: `Matched by ${summary.matchedBy.join(", ")} · Updated ${summary.latestOccurredAt ? formatDateTime(summary.latestOccurredAt) : "N/A"}` }}>
          <div className="space-y-4">
            <div>
              <div className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground mb-3">
                Related References
              </div>
              <div className="flex flex-wrap gap-2">
                {summary.relatedReferenceIds.length === 0 ? (
                  <span className="text-xs text-muted-foreground">None</span>
                ) : (
                  summary.relatedReferenceIds.map((ref) => (
                    <span
                      key={ref}
                      className="rounded-full border border-white/[0.08] bg-white/[0.03] px-3 py-1 text-xs font-mono font-medium"
                    >
                      {ref}
                    </span>
                  ))
                )}
              </div>
            </div>

            <div className="rounded-lg border border-white/[0.06] bg-[#09090B] p-4 text-sm">
              <div className="font-medium text-[#00D4FF] flex items-center gap-2">
                <Waves className="h-3.5 w-3.5" />
                Realtime path
              </div>
              <p className="mt-2 text-xs text-muted-foreground">
                Timeline refreshes reuse the existing WebSocket event envelope
                and keep tenant scope intact.
              </p>
            </div>
          </div>
        </Panel>

        <Panel
          header={{
            title: "Recommended Checks",
            description:
              "Derived from current timeline signals and in-memory metrics only.",
          }}
        >
          {timeline.recommendations.length === 0 ? (
            <EmptyState
              icon={<Shield className="h-8 w-8" />}
              title="No recommendations"
              description="No recommendations were raised for this incident snapshot."
            />
          ) : (
            <div className="space-y-3">
              {timeline.recommendations.map((rec) => (
                <div
                  key={rec.code}
                  className="rounded-xl border border-white/[0.06] bg-[#09090B] p-4"
                  data-testid="incident-recommendation"
                >
                  <div className="flex flex-wrap items-center gap-2 mb-2">
                    <span className="font-medium text-sm">{rec.title}</span>
                    <StatusBadge
                      status={prettyLabel(rec.priority)}
                      severity={prioritySeverity(rec.priority)}
                    />
                    <StatusBadge
                      status={prettyLabel(rec.confidence)}
                      severity="neutral"
                      dot={false}
                    />
                  </div>
                  <p className="text-xs text-muted-foreground">{rec.summary}</p>
                  {rec.relatedEntryIds.length > 0 && (
                    <div className="mt-3 flex flex-wrap gap-2">
                      {rec.relatedEntryIds.map((ref) => (
                        <span
                          key={ref}
                          className="rounded-full bg-white/[0.04] px-2 py-1 text-[11px] font-mono font-medium border border-white/[0.06]"
                        >
                          {ref}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </Panel>
      </div>

      {/* Timeline entries */}
      <Panel
        header={{
          title: "Timeline",
          description: `Generated ${timeline.generatedAt ? formatDateTime(timeline.generatedAt) : "N/A"} · Mode ${prettyLabel(timeline.actionMode)}`,
        }}
      >
        {timeline.entries.length === 0 ? (
          <EmptyState
            icon={<Clock3 className="h-8 w-8" />}
            title="No timeline entries"
            description="No correlated events were found for this incident."
          />
        ) : (
          <div className="space-y-4">
            {timeline.entries.map((entry) => (
              <div
                key={`${entry.sourceKind}-${entry.sourceReferenceId}`}
                className="rounded-xl border border-white/[0.06] bg-[#09090B] p-4"
                data-testid="incident-entry"
              >
                <div className="flex flex-wrap items-start justify-between gap-4">
                  <div className="space-y-2">
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      {sourceIcon(entry.sourceKind)}
                      <span>{prettyLabel(entry.sourceKind)}</span>
                      <span className="text-muted-foreground/50">
                        #{entry.sequence}
                      </span>
                    </div>
                    <div className="font-semibold text-sm">{entry.label}</div>
                    <div className="flex flex-wrap gap-2">
                      <StatusBadge status={entry.status} />
                      <StatusBadge
                        status={prettyLabel(entry.confidence)}
                        severity="neutral"
                        dot={false}
                      />
                      <span className="rounded-full border border-white/[0.06] px-2 py-0.5 text-xs font-mono text-muted-foreground">
                        {entry.sourceReferenceId}
                      </span>
                    </div>
                  </div>
                  <span className="text-xs text-muted-foreground tabular-nums whitespace-nowrap">
                    {entry.occurredAt ? formatDateTime(entry.occurredAt) : "N/A"}
                  </span>
                </div>

                {entry.relatedReferenceIds.length > 0 && (
                  <div className="mt-3 flex flex-wrap gap-2">
                    {entry.relatedReferenceIds.map((ref) => (
                      <span
                        key={ref}
                        className="rounded-full border border-white/[0.06] bg-white/[0.03] px-2 py-1 text-[11px] font-mono font-medium"
                      >
                        {ref}
                      </span>
                    ))}
                  </div>
                )}

                <pre className="mt-4 overflow-x-auto rounded-lg bg-black/60 p-4 text-xs text-slate-100 border border-white/[0.06]">
                  {JSON.stringify(entry.details, null, 2)}
                </pre>
              </div>
            ))}
          </div>
        )}
      </Panel>
    </div>
  );
}
