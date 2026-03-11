"use client";

import { useEffect, useState } from "react";
import {
  AlertTriangle,
  Download,
  FileSearch,
  Loader2,
  Radar,
  RefreshCw,
  ShieldAlert,
} from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

export type ReconciliationQueueRow = {
  discrepancyId: string;
  reportId: string;
  ownerLane: string;
  rootCause: string;
  ageBucket: string;
  severity: string;
  settlementId: string | null;
  onChainTx: string | null;
  detectedAt: string;
  summary: string;
  suggestedMatches: Array<{
    settlementId: string;
    confidence: string;
  }>;
};

export type ReconciliationWorkbenchResponse = {
  snapshot: {
    generatedAt: string;
    report: {
      id: string;
      totalDiscrepancies: number;
      criticalCount: number;
      status: string;
    };
    queue: ReconciliationQueueRow[];
  };
  actionMode: string;
  exportFormats: string[];
  incidentLinkHint: string;
};

export type ReconciliationEvidenceResponse = {
  queueItem: {
    discrepancyId: string;
    summary: string;
    severity: string;
    ownerLane: string;
    rootCause: string;
  };
  settlementIds: string[];
  replayEntries: Array<{
    referenceId: string;
    label: string;
    status: string;
  }>;
  incidentEntries: Array<{
    sourceReferenceId: string;
    label: string;
    status: string;
  }>;
};

type ScenarioKey = "ops-demo" | "clean";

const SCENARIO_LABELS: Record<ScenarioKey, string> = {
  "ops-demo": "Ops demo",
  clean: "Clean path",
};

async function apiRequest<T>(endpoint: string, init?: RequestInit): Promise<T> {
  const response = init ? await fetch(`/api/proxy${endpoint}`, init) : await fetch(`/api/proxy${endpoint}`);

  if (!response.ok) {
    let message = "Request failed";
    try {
      const payload = (await response.json()) as {
        message?: string;
        error?: { message?: string };
      };
      message = payload.message ?? payload.error?.message ?? message;
    } catch {
      // Keep default fallback.
    }
    throw new Error(message);
  }

  return response.json() as Promise<T>;
}

function badgeVariantForSeverity(severity: string): "success" | "warning" | "destructive" | "outline" {
  switch (severity.toLowerCase()) {
    case "critical":
      return "destructive";
    case "high":
      return "warning";
    case "medium":
      return "outline";
    default:
      return "success";
  }
}

function formatLabel(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function formatTimestamp(value?: string | null): string {
  if (!value) return "N/A";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("en-US", {
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function responseTargetForSeverity(severity: string): string {
  switch (severity.toLowerCase()) {
    case "critical":
      return "Review within 15 minutes";
    case "high":
      return "Review within 30 minutes";
    case "medium":
      return "Review within 2 hours";
    default:
      return "Review during the next queue sweep";
  }
}

export function ReconciliationWorkbench() {
  const [scenario, setScenario] = useState<ScenarioKey>("ops-demo");
  const [workbench, setWorkbench] = useState<ReconciliationWorkbenchResponse | null>(null);
  const [evidence, setEvidence] = useState<ReconciliationEvidenceResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [evidenceLoading, setEvidenceLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [activeDiscrepancyId, setActiveDiscrepancyId] = useState<string | null>(null);
  const [pendingDiscrepancyId, setPendingDiscrepancyId] = useState<string | null>(null);

  const loadWorkbench = async (nextScenario: ScenarioKey) => {
    setLoading(true);
    setError(null);
    setNotice(null);

    try {
      const suffix = nextScenario === "clean" ? "?scenario=clean" : "";
      const response = await apiRequest<ReconciliationWorkbenchResponse>(
        `/v1/admin/reconciliation/workbench${suffix}`,
      );
      setWorkbench(response);
      setEvidence(null);
      setActiveDiscrepancyId(null);
    } catch (requestError) {
      setWorkbench(null);
      setEvidence(null);
      setActiveDiscrepancyId(null);
      setError(
        requestError instanceof Error ? requestError.message : "Failed to load reconciliation workbench",
      );
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadWorkbench(scenario);
  }, [scenario]);

  const handleLoadEvidence = async (discrepancyId: string) => {
    const suffix = scenario === "clean" ? "?scenario=clean" : "";
    setEvidenceLoading(true);
    setNotice(null);
    setEvidence(null);
    setPendingDiscrepancyId(discrepancyId);

    try {
      const response = await apiRequest<ReconciliationEvidenceResponse>(
        `/v1/admin/reconciliation/evidence/${discrepancyId}${suffix}`,
      );
      setEvidence(response);
      setActiveDiscrepancyId(discrepancyId);
    } catch (requestError) {
      setNotice(
        requestError instanceof Error ? requestError.message : "Failed to load discrepancy evidence",
      );
    } finally {
      setEvidenceLoading(false);
      setPendingDiscrepancyId(null);
    }
  };

  const handleExportQueue = async (format: "csv" | "json") => {
    setNotice(null);
    const suffix = scenario === "clean" ? "&scenario=clean" : "";
    try {
      await downloadAttachment(`/v1/admin/reconciliation/export?format=${format}${suffix}`);
      setNotice(`Queue export ready in ${format.toUpperCase()} format.`);
    } catch (requestError) {
      setNotice(requestError instanceof Error ? requestError.message : "Queue export failed");
    }
  };

  const handleExportEvidence = async () => {
    if (!activeDiscrepancyId) return;

    setNotice(null);
    const suffix = scenario === "clean" ? "?scenario=clean" : "";
    try {
      await downloadAttachment(
        `/v1/admin/reconciliation/evidence/${activeDiscrepancyId}/export${suffix}`,
      );
      setNotice("Evidence pack export ready in JSON format.");
    } catch (requestError) {
      setNotice(requestError instanceof Error ? requestError.message : "Evidence export failed");
    }
  };

  const queue = workbench?.snapshot.queue ?? [];
  const activeQueueItem = queue.find((item) => item.discrepancyId === activeDiscrepancyId) ?? null;
  const urgentQueueCount = queue.filter(
    (item) =>
      item.ageBucket.toLowerCase() === "aging" &&
      ["critical", "high"].includes(item.severity.toLowerCase()),
  ).length;
  const evidenceResponseTarget = evidence ? responseTargetForSeverity(evidence.queueItem.severity) : null;

  return (
    <div className="grid gap-6 xl:grid-cols-[1.3fr,0.9fr]">
      <div className="space-y-6">
        <Card className="overflow-hidden border-none bg-[linear-gradient(135deg,rgba(9,32,63,1)_0%,rgba(83,120,149,0.96)_52%,rgba(235,248,255,0.98)_100%)] text-white shadow-xl">
          <CardHeader className="gap-3">
            <div className="flex items-start justify-between gap-4">
              <div>
                <CardTitle className="text-2xl">Reconciliation Ops Workbench</CardTitle>
                <CardDescription className="max-w-2xl text-slate-100/85">
                  Triage breaks by severity, aging, owner lane, and linked evidence without leaving
                  the bounded operator surface.
                </CardDescription>
              </div>
              <div className="rounded-full border border-white/25 bg-white/10 p-3">
                <Radar className="h-5 w-5" />
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-3 text-sm text-slate-100/85">
              <span>Action mode: {workbench?.actionMode ?? "recommendation_only"}</span>
              <span>Incident link: {workbench?.incidentLinkHint ?? "/v1/admin/incidents/timeline"}</span>
            </div>
          </CardHeader>
        </Card>

        <div className="grid gap-4 md:grid-cols-3">
          <Card>
            <CardHeader className="pb-2">
              <CardDescription>Total discrepancies</CardDescription>
              <CardTitle>{loading ? "..." : workbench?.snapshot.report.totalDiscrepancies ?? 0}</CardTitle>
            </CardHeader>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardDescription>Critical count</CardDescription>
              <CardTitle>{loading ? "..." : workbench?.snapshot.report.criticalCount ?? 0}</CardTitle>
            </CardHeader>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardDescription>Report status</CardDescription>
              <CardTitle>{loading ? "..." : workbench?.snapshot.report.status ?? "N/A"}</CardTitle>
            </CardHeader>
          </Card>
        </div>

        <Card className="border-blue-200 bg-blue-50/70">
          <CardHeader>
            <CardTitle className="text-base text-blue-950">SLA guardian</CardTitle>
            <CardDescription className="text-blue-900/80">
              {urgentQueueCount} needs attention within 15 min.
            </CardDescription>
          </CardHeader>
          <CardContent className="text-sm text-blue-950/85">
            Recommendation-only guidance prioritizes aging high-severity discrepancies without
            introducing automatic settlement decisions.
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Workbench controls</CardTitle>
            <CardDescription>
              Switch between the active ops demo and a clean control case, then export the queue or
              inspect discrepancy evidence.
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-wrap items-center gap-3">
            <label className="flex items-center gap-2 text-sm text-muted-foreground">
              Scenario
              <select
                aria-label="Scenario"
                className="h-10 rounded-md border bg-background px-3 py-2 text-sm text-foreground"
                value={scenario}
                onChange={(event) => setScenario(event.target.value as ScenarioKey)}
              >
                {Object.entries(SCENARIO_LABELS).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            <Button variant="outline" onClick={() => void loadWorkbench(scenario)} disabled={loading}>
              {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <RefreshCw className="mr-2 h-4 w-4" />}
              Reload workbench
            </Button>
            <Button variant="outline" onClick={() => void handleExportQueue("csv")} disabled={loading}>
              <Download className="mr-2 h-4 w-4" />
              Export queue CSV
            </Button>
            <Button variant="outline" onClick={() => void handleExportQueue("json")} disabled={loading}>
              <Download className="mr-2 h-4 w-4" />
              Export snapshot JSON
            </Button>
          </CardContent>
        </Card>

        {error && (
          <Alert variant="destructive">
            <AlertTitle>Workbench request failed</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {notice && (
          <Alert variant="success">
            <AlertTitle>Workbench update</AlertTitle>
            <AlertDescription>{notice}</AlertDescription>
          </Alert>
        )}

        <Card>
          <CardHeader>
            <CardTitle>Break queue</CardTitle>
            <CardDescription>
              Owner lane, root cause, and fuzzy-match hints stay visible in one place for fast
              triage.
            </CardDescription>
          </CardHeader>
          <CardContent>
            {loading && !workbench ? (
              <div className="space-y-3">
                <Skeleton className="h-10 w-full" />
                <Skeleton className="h-48 w-full" />
              </div>
            ) : queue.length === 0 ? (
              <div className="rounded-lg border border-dashed px-4 py-8 text-sm text-muted-foreground">
                No discrepancies are active for this scenario. Switch back to Ops demo to inspect
                owner assignment and evidence pack flow.
              </div>
            ) : (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader sticky>
                    <TableRow>
                      <TableHead>Severity</TableHead>
                      <TableHead>Root cause</TableHead>
                      <TableHead>Owner lane</TableHead>
                      <TableHead>Aging</TableHead>
                      <TableHead>Suggested matches</TableHead>
                      <TableHead>Detected</TableHead>
                      <TableHead>Action</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {queue.map((item) => (
                      <TableRow key={item.discrepancyId}>
                        <TableCell>
                          <Badge variant={badgeVariantForSeverity(item.severity)} shape="pill">
                            {formatLabel(item.severity)}
                          </Badge>
                        </TableCell>
                        <TableCell className="min-w-[220px]">
                          <div className="font-medium">{formatLabel(item.rootCause)}</div>
                          <div className="text-xs text-muted-foreground">{item.summary}</div>
                        </TableCell>
                        <TableCell>{formatLabel(item.ownerLane)}</TableCell>
                        <TableCell>{formatLabel(item.ageBucket)}</TableCell>
                        <TableCell>{item.suggestedMatches.length}</TableCell>
                        <TableCell>{formatTimestamp(item.detectedAt)}</TableCell>
                        <TableCell>
                          <Button
                            size="sm"
                            variant={activeDiscrepancyId === item.discrepancyId ? "default" : "outline"}
                            onClick={() => void handleLoadEvidence(item.discrepancyId)}
                            disabled={evidenceLoading && pendingDiscrepancyId === item.discrepancyId}
                          >
                            {evidenceLoading && pendingDiscrepancyId === item.discrepancyId ? (
                              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                            ) : (
                              <FileSearch className="mr-2 h-4 w-4" />
                            )}
                            View evidence
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="space-y-6">
        <Card className="border-dashed">
          <CardHeader className="space-y-2">
            <CardTitle className="flex items-center gap-2 text-base">
              <ShieldAlert className="h-4 w-4 text-muted-foreground" />
              Resolution guardrail
            </CardTitle>
            <CardDescription>
              This surface stays recommendation-oriented. It exposes evidence, owner assignment,
              and exports without introducing a parallel accounting engine.
            </CardDescription>
          </CardHeader>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-start justify-between gap-3 space-y-0">
            <div>
              <CardTitle>Evidence pack</CardTitle>
              <CardDescription>
                Linked replay and incident entries for the selected discrepancy.
              </CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => void handleExportEvidence()}
              disabled={!activeDiscrepancyId || evidenceLoading}
            >
              <Download className="mr-2 h-4 w-4" />
              Export evidence
            </Button>
          </CardHeader>
          <CardContent className="space-y-4">
            {evidenceLoading ? (
              <div className="space-y-3">
                <Skeleton className="h-8 w-full" />
                <Skeleton className="h-32 w-full" />
              </div>
            ) : !evidence ? (
              <div className="rounded-lg border border-dashed px-4 py-8 text-sm text-muted-foreground">
                Select a discrepancy from the queue to load its linked evidence pack.
              </div>
            ) : (
              <>
                <div className="space-y-2 rounded-xl border bg-muted/30 p-4">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant={badgeVariantForSeverity(evidence.queueItem.severity)} shape="pill">
                      {formatLabel(evidence.queueItem.severity)}
                    </Badge>
                    <Badge variant="outline" shape="pill">
                      {formatLabel(evidence.queueItem.ownerLane)}
                    </Badge>
                    <Badge variant="outline" shape="pill">
                      {formatLabel(evidence.queueItem.rootCause)}
                    </Badge>
                  </div>
                  <div className="font-medium">{evidence.queueItem.summary}</div>
                  <div className="text-sm text-muted-foreground">
                    Settlement IDs: {evidence.settlementIds.length > 0 ? evidence.settlementIds.join(", ") : "None linked"}
                  </div>
                </div>

                {activeQueueItem && activeQueueItem.suggestedMatches.length > 0 && (
                  <div className="space-y-2 rounded-xl border border-dashed p-4">
                    <div className="text-sm font-medium">Suggested matches</div>
                    <div className="flex flex-wrap gap-2">
                      {activeQueueItem.suggestedMatches.map((match) => (
                        <Badge key={`${activeQueueItem.discrepancyId}-${match.settlementId}`} variant="outline" shape="pill">
                          {match.settlementId} · {formatLabel(match.confidence)}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}

                <div className="space-y-2 rounded-xl border border-blue-200 bg-blue-50/70 p-4">
                  <div className="text-sm font-medium text-blue-950">Recommended response target</div>
                  <div className="text-sm text-blue-900/85">{evidenceResponseTarget}</div>
                  <div className="text-sm text-blue-900/85">
                    Page banking partner and incident commander.
                  </div>
                </div>

                <div className="grid gap-4 md:grid-cols-2">
                  <Card>
                    <CardHeader className="pb-2">
                      <CardTitle className="text-base">Replay trail</CardTitle>
                      <CardDescription>{evidence.replayEntries.length} linked entries</CardDescription>
                    </CardHeader>
                    <CardContent className="max-h-72 space-y-3 overflow-y-auto pr-1">
                      {evidence.replayEntries.map((entry) => (
                        <div key={entry.referenceId} className="rounded-lg border p-3">
                          <div className="font-medium">{entry.label}</div>
                          <div className="text-sm text-muted-foreground">{entry.referenceId}</div>
                          <div className="mt-2 text-xs uppercase tracking-wide text-muted-foreground">
                            {entry.status}
                          </div>
                        </div>
                      ))}
                    </CardContent>
                  </Card>

                  <Card>
                    <CardHeader className="pb-2">
                      <CardTitle className="text-base">Incident links</CardTitle>
                      <CardDescription>{evidence.incidentEntries.length} correlated items</CardDescription>
                    </CardHeader>
                    <CardContent className="max-h-72 space-y-3 overflow-y-auto pr-1">
                      {evidence.incidentEntries.map((entry) => (
                        <div key={entry.sourceReferenceId} className="rounded-lg border p-3">
                          <div className="font-medium">{entry.label}</div>
                          <div className="text-sm text-muted-foreground">{entry.sourceReferenceId}</div>
                          <div className="mt-2 text-xs uppercase tracking-wide text-muted-foreground">
                            {entry.status}
                          </div>
                        </div>
                      ))}
                    </CardContent>
                  </Card>
                </div>
              </>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <AlertTriangle className="h-4 w-4 text-muted-foreground" />
              Triage cues
            </CardTitle>
            <CardDescription>
              Breaks with settlement IDs usually have the richest evidence packs. Missing-settlement
              cases rely more on match hints and downstream intent history.
            </CardDescription>
          </CardHeader>
        </Card>
      </div>
    </div>
  );
}

export default ReconciliationWorkbench;

async function downloadAttachment(endpoint: string): Promise<void> {
  const response = await fetch(`/api/proxy${endpoint}`);
  if (!response.ok) {
    throw new Error("Export request failed");
  }

  const contents = await response.text();
  const contentType = response.headers?.get?.("content-type") ?? "application/octet-stream";
  const disposition = response.headers?.get?.("content-disposition") ?? "";
  const filenameMatch = disposition.match(/filename=\"([^\"]+)\"/i);
  const fileName = filenameMatch?.[1] ?? "reconciliation-export";
  const isJsdom =
    typeof window !== "undefined" && /jsdom/i.test(window.navigator.userAgent);

  if (
    typeof window !== "undefined" &&
    typeof URL.createObjectURL === "function" &&
    !isJsdom
  ) {
    const blob = new Blob([contents], { type: contentType });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = fileName;
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
  }
}
