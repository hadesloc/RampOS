"use client";

import { useEffect, useState } from "react";
import { AlertTriangle, Loader2, RefreshCw, ShieldAlert, Wallet } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  PageHeader,
  StatCard,
  Panel,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";

type TreasuryFloatSlice = {
  segment: string;
  asset: string;
  available: string;
  reserved: string;
  utilizationPct: number;
  shortageRisk: string;
};

type TreasuryForecast = {
  asset: string;
  horizonHours: number;
  projectedAvailable: string;
  projectedRequired: string;
  shortageAmount: string;
  confidence: string;
};

type TreasuryExposure = {
  counterpartyType: string;
  counterpartyId: string;
  direction: string;
  pressureScore: string;
  concentration: string;
  reliabilityScore?: string | null;
  p95SettlementLatencySeconds: number;
};

type TreasuryAlert = {
  id: string;
  severity: string;
  title: string;
  summary: string;
  recommendationIds: string[];
};

type TreasuryRecommendation = {
  id: string;
  category: string;
  title: string;
  summary: string;
  asset: string;
  amount: string;
  sourceSegment?: string | null;
  destinationSegment?: string | null;
  confidence: string;
  mode: string;
};

type TreasuryYieldAllocation = {
  protocol: string;
  principalAmount: string;
  currentValue: string;
  accruedYield: string;
  sharePercent: string;
  strategyPosture: string;
};

type TreasuryProvenance = {
  sourceKind: string;
  evidenceImportIds: string[];
  earliestEvidenceAt?: string | null;
  latestEvidenceAt?: string | null;
  freshnessWarning?: string | null;
};

type TreasurySnapshot = {
  generatedAt: string;
  forecastWindowHours: number;
  actionMode: string;
  bufferTargetPercent: number;
  policyHint: string;
  dataSource: string;
  provenance: TreasuryProvenance;
  floatSlices: TreasuryFloatSlice[];
  forecasts: TreasuryForecast[];
  exposures: TreasuryExposure[];
  alerts: TreasuryAlert[];
  recommendations: TreasuryRecommendation[];
  yieldAllocations: TreasuryYieldAllocation[];
};

type TreasuryWorkbenchResponse = {
  snapshot: TreasurySnapshot;
  actionMode: string;
  recommendationCount: number;
  stressAlertCount: number;
};

async function apiRequest<T>(endpoint: string): Promise<T> {
  const response = await fetch(`/api/proxy${endpoint}`);

  if (!response.ok) {
    let message = "Failed to load treasury workbench";
    try {
      const payload = (await response.json()) as {
        message?: string;
        error?: { message?: string };
      };
      message = payload.message ?? payload.error?.message ?? message;
    } catch {
      // Keep fallback message.
    }
    throw new Error(message);
  }

  return response.json() as Promise<T>;
}

type RiskSeverity = "danger" | "warning" | "success";

function riskSeverity(value: string): RiskSeverity {
  switch (value.toLowerCase()) {
    case "high":
    case "critical":
      return "danger";
    case "medium":
      return "warning";
    default:
      return "success";
  }
}

export default function TreasuryWorkbench() {
  const [data, setData] = useState<TreasuryWorkbenchResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [scenario, setScenario] = useState<"active" | "stable">("active");

  const loadWorkbench = async (nextScenario: "active" | "stable", isRefresh = false) => {
    if (isRefresh) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }
    setError(null);

    try {
      const query = nextScenario === "stable" ? "?scenario=stable" : "";
      const response = await apiRequest<TreasuryWorkbenchResponse>(
        `/v1/admin/treasury/workbench${query}`,
      );
      setData(response);
    } catch (requestError) {
      setData(null);
      setError(
        requestError instanceof Error
          ? requestError.message
          : "Failed to load treasury workbench",
      );
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    void loadWorkbench("active");
  }, []);

  const snapshot = data?.snapshot;

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Treasury Control Tower"
        description="Forecast float pressure, inspect LP exposure, and review bounded prefund or rebalance suggestions without moving funds automatically."
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={() => {
              void loadWorkbench(scenario, true);
            }}
            disabled={loading || refreshing}
            aria-label="Refresh treasury workbench"
          >
            {loading || refreshing ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
          </Button>
        }
      />

      <div className="flex flex-col gap-2">
        <div className="flex flex-wrap gap-2">
          <Button
            variant={scenario === "active" ? "default" : "outline"}
            size="sm"
            onClick={() => {
              setScenario("active");
              void loadWorkbench("active");
            }}
          >
            Active Scenario
          </Button>
          <Button
            variant={scenario === "stable" ? "default" : "outline"}
            size="sm"
            onClick={() => {
              setScenario("stable");
              void loadWorkbench("stable");
            }}
          >
            Stable Scenario
          </Button>
        </div>
        <p className="text-sm text-muted-foreground">
          Scenario toggles modeled pressure profiles. Source-of-truth is shown separately and may
          be sample-backed.
        </p>
      </div>

      {error ? (
        <ErrorState
          title="Treasury workbench unavailable"
          message={
            error === "Treasury workbench unavailable"
              ? "Retry the bounded treasury workbench request or switch to the stable control."
              : error
          }
          retry={() => void loadWorkbench(scenario)}
        />
      ) : null}

      {snapshot?.provenance.freshnessWarning ? (
        <Panel variant="solid" className="border-[#FFB800]/40">
          <div className="flex items-start gap-3">
            <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-[#FFB800]" />
            <div>
              <div className="font-medium">Sample fallback in use</div>
              <p className="mt-1 text-sm text-muted-foreground">
                {snapshot.provenance.freshnessWarning}
              </p>
            </div>
          </div>
        </Panel>
      ) : null}

      {/* Summary — provenance shown explicitly (sample vs evidence-backed) */}
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-5">
        <StatCard
          title="Action Mode"
          value="Recommendation only"
          accentColor="cyan"
          subtitle={snapshot?.policyHint ?? "Loading posture…"}
          loading={loading && !snapshot}
        />
        <StatCard
          title="Buffer Target"
          value={snapshot ? `${snapshot.bufferTargetPercent}%` : "—"}
          accentColor="violet"
          subtitle="Reserve before any move"
          loading={loading && !snapshot}
        />
        <StatCard
          title="Recommendations"
          value={data?.recommendationCount ?? 0}
          accentColor="green"
          subtitle="Operator-reviewed only"
          loading={loading && !snapshot}
        />
        <StatCard
          title="Stress Alerts"
          value={data?.stressAlertCount ?? 0}
          accentColor="amber"
          subtitle={`Forecast ${snapshot?.forecastWindowHours ?? 24}h`}
          loading={loading && !snapshot}
        />
        <StatCard
          title="Source Of Truth"
          value={snapshot?.dataSource ?? "—"}
          accentColor="cyan"
          subtitle={`${snapshot?.provenance.evidenceImportIds.length ?? 0} evidence imports`}
          loading={loading && !snapshot}
        />
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.3fr_1fr]">
        <Panel
          header={{
            title: "Float Pressure",
            description: "Bank and chain inventory slices feeding the treasury forecast.",
          }}
        >
          <div className="mb-3 flex items-center gap-2 text-sm font-medium text-muted-foreground">
            <Wallet className="h-4 w-4" />
            Inventory slices
          </div>
          {loading && !snapshot ? (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <div
                  key={i}
                  className="h-20 rounded-lg border border-white/[0.06] bg-white/[0.02] animate-pulse"
                />
              ))}
            </div>
          ) : !snapshot || snapshot.floatSlices.length === 0 ? (
            <EmptyState title="No float slices" description="No inventory data in this scenario." />
          ) : (
            <div className="space-y-3">
              {snapshot.floatSlices.map((slice) => (
                <div
                  key={slice.segment}
                  className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-4 transition-colors hover:border-white/[0.1]"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="font-medium">{slice.segment}</div>
                      <div className="text-sm text-muted-foreground">{slice.asset}</div>
                    </div>
                    <StatusBadge
                      status={slice.shortageRisk}
                      severity={riskSeverity(slice.shortageRisk)}
                    />
                  </div>
                  <div className="mt-3 grid gap-2 text-sm md:grid-cols-3">
                    <div className="tabular-nums">
                      <span className="text-muted-foreground">Available: </span>
                      {slice.available}
                    </div>
                    <div className="tabular-nums">
                      <span className="text-muted-foreground">Reserved: </span>
                      {slice.reserved}
                    </div>
                    <div className="tabular-nums">
                      <span className="text-muted-foreground">Utilization: </span>
                      {slice.utilizationPct}%
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Panel>

        <Panel
          header={{
            title: "Stress Alerts",
            description: "Alerts are recommendation-linked and approval-gated.",
          }}
        >
          {!snapshot || snapshot.alerts.length === 0 ? (
            <EmptyState
              icon={<AlertTriangle className="h-6 w-6" />}
              title="No stress alerts"
            />
          ) : (
            <div className="space-y-3">
              {snapshot.alerts.map((alert) => (
                <div
                  key={alert.id}
                  className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-4"
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="font-medium">{alert.title}</div>
                    <StatusBadge status={alert.severity} severity={riskSeverity(alert.severity)} />
                  </div>
                  <p className="mt-1 text-sm text-muted-foreground">{alert.summary}</p>
                </div>
              ))}
            </div>
          )}
        </Panel>
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.1fr_0.9fr]">
        <Panel
          header={{
            title: "Recommendations",
            description:
              "Prefund, counterparty, and yield parking suggestions constrained by treasury policy.",
          }}
        >
          {!snapshot || snapshot.recommendations.length === 0 ? (
            <EmptyState title="No recommendations" description="No bounded moves suggested." />
          ) : (
            <div className="space-y-3">
              {snapshot.recommendations.map((recommendation) => (
                <div
                  key={recommendation.id}
                  className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-4"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0">
                      <div className="font-medium">{recommendation.title}</div>
                      <p className="mt-1 text-sm text-muted-foreground">{recommendation.summary}</p>
                    </div>
                    <StatusBadge
                      status={recommendation.confidence}
                      severity={riskSeverity(recommendation.confidence)}
                    />
                  </div>
                  <div className="mt-3 grid gap-2 text-sm md:grid-cols-3">
                    <div>
                      <span className="text-muted-foreground">Category: </span>
                      {recommendation.category}
                    </div>
                    <div className="tabular-nums">
                      <span className="text-muted-foreground">Amount: </span>
                      {recommendation.amount} {recommendation.asset}
                    </div>
                    <div>
                      <span className="text-muted-foreground">Mode: </span>
                      {recommendation.mode}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Panel>

        <div className="flex flex-col gap-6">
          <Panel
            header={{
              title: "Counterparty Exposure",
              description:
                "LP pressure is derived from reliability and settlement latency, not automatic throttling.",
            }}
          >
            {!snapshot || snapshot.exposures.length === 0 ? (
              <EmptyState title="No counterparty exposure" />
            ) : (
              <div className="space-y-3">
                {snapshot.exposures.map((exposure) => (
                  <div
                    key={exposure.counterpartyId}
                    className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-4 text-sm"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <span className="font-medium">{exposure.counterpartyId}</span>
                      <StatusBadge
                        status={exposure.concentration}
                        severity={riskSeverity(exposure.concentration)}
                      />
                    </div>
                    <div className="mt-2 grid gap-1 text-muted-foreground">
                      <div>Pressure score: {exposure.pressureScore}</div>
                      <div>Reliability: {exposure.reliabilityScore ?? "n/a"}</div>
                      <div className="tabular-nums">
                        p95 settlement latency: {exposure.p95SettlementLatencySeconds}s
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </Panel>

          <Panel
            header={{
              title: "Yield Parking Lanes",
              description:
                "Existing allocations are shown as context for recommendations, not auto-actions.",
            }}
          >
            {!snapshot || !snapshot.yieldAllocations || snapshot.yieldAllocations.length === 0 ? (
              <EmptyState title="No yield allocations" />
            ) : (
              <div className="space-y-3">
                {snapshot.yieldAllocations.map((allocation) => (
                  <div
                    key={allocation.protocol}
                    className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-4 text-sm"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <span className="font-medium">{allocation.protocol}</span>
                      <span className="tabular-nums text-[#00FF87]">{allocation.sharePercent}%</span>
                    </div>
                    <div className="mt-2 grid gap-1 text-muted-foreground tabular-nums">
                      <div>Current value: {allocation.currentValue}</div>
                      <div>Accrued yield: {allocation.accruedYield}</div>
                      <div>Posture: {allocation.strategyPosture}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </Panel>
        </div>
      </div>
    </main>
  );
}
