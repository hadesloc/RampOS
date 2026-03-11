"use client";

import { useEffect, useState } from "react";
import { AlertTriangle, Loader2, RefreshCw, ShieldAlert, Wallet } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

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

type TreasurySnapshot = {
  generatedAt: string;
  forecastWindowHours: number;
  actionMode: string;
  bufferTargetPercent: number;
  policyHint: string;
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

function riskTone(value: string): string {
  switch (value.toLowerCase()) {
    case "high":
    case "critical":
      return "text-red-600";
    case "medium":
      return "text-amber-600";
    default:
      return "text-emerald-600";
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
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Treasury Control Tower</h1>
          <p className="text-muted-foreground">
            Forecast float pressure, inspect LP exposure, and review bounded prefund or rebalance
            suggestions without moving funds automatically.
          </p>
        </div>
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
      </div>

      <div className="flex gap-2">
        <Button
          variant={scenario === "active" ? "default" : "outline"}
          onClick={() => {
            setScenario("active");
            void loadWorkbench("active");
          }}
        >
          Active Pressure
        </Button>
        <Button
          variant={scenario === "stable" ? "default" : "outline"}
          onClick={() => {
            setScenario("stable");
            void loadWorkbench("stable");
          }}
        >
          Stable Control
        </Button>
      </div>

      {error ? (
        <Card>
          <CardHeader>
            <CardTitle role="heading" aria-level={2}>
              Treasury workbench unavailable
            </CardTitle>
            <CardDescription>
              {error === "Treasury workbench unavailable"
                ? "Retry the bounded treasury workbench request or switch to the stable control."
                : error}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button
              onClick={() => {
                void loadWorkbench(scenario);
              }}
            >
              Reload workbench
            </Button>
          </CardContent>
        </Card>
      ) : null}

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Action Mode</CardDescription>
            <CardTitle className="text-lg">Recommendation only</CardTitle>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            {snapshot?.policyHint ?? "Loading treasury posture..."}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Buffer Target</CardDescription>
            <CardTitle className="text-lg">
              {snapshot ? `${snapshot.bufferTargetPercent}%` : "..."}
            </CardTitle>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            Reserve held back before any parking or rebalance recommendation.
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Recommendations</CardDescription>
            <CardTitle className="text-lg">{data?.recommendationCount ?? 0}</CardTitle>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            Operator-reviewed moves only.
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Stress Alerts</CardDescription>
            <CardTitle className="text-lg">{data?.stressAlertCount ?? 0}</CardTitle>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            Forecast window: {snapshot?.forecastWindowHours ?? 24}h
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.3fr_1fr]">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Wallet className="h-5 w-5" />
              Float Pressure
            </CardTitle>
            <CardDescription>
              Bank and chain inventory slices feeding the treasury forecast.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {loading && !snapshot ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                Loading treasury slices...
              </div>
            ) : (
              snapshot?.floatSlices.map((slice) => (
                <div key={slice.segment} className="rounded-lg border p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="font-medium">{slice.segment}</div>
                      <div className="text-sm text-muted-foreground">{slice.asset}</div>
                    </div>
                    <div className={`text-sm font-medium ${riskTone(slice.shortageRisk)}`}>
                      {slice.shortageRisk}
                    </div>
                  </div>
                  <div className="mt-3 grid gap-2 text-sm md:grid-cols-3">
                    <div>Available: {slice.available}</div>
                    <div>Reserved: {slice.reserved}</div>
                    <div>Utilization: {slice.utilizationPct}%</div>
                  </div>
                </div>
              ))
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <AlertTriangle className="h-5 w-5" />
              Stress Alerts
            </CardTitle>
            <CardDescription>Alerts are recommendation-linked and approval-gated.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {snapshot?.alerts.map((alert) => (
              <div key={alert.id} className="rounded-lg border p-4">
                <div className={`font-medium ${riskTone(alert.severity)}`}>{alert.title}</div>
                <p className="mt-1 text-sm text-muted-foreground">{alert.summary}</p>
              </div>
            ))}
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.1fr_0.9fr]">
        <Card>
          <CardHeader>
            <CardTitle>Recommendations</CardTitle>
            <CardDescription>
              Prefund, counterparty, and yield parking suggestions constrained by treasury policy.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {snapshot?.recommendations.map((recommendation) => (
              <div key={recommendation.id} className="rounded-lg border p-4">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <div className="font-medium">{recommendation.title}</div>
                    <p className="mt-1 text-sm text-muted-foreground">
                      {recommendation.summary}
                    </p>
                  </div>
                  <div className={`text-sm font-medium ${riskTone(recommendation.confidence)}`}>
                    {recommendation.confidence}
                  </div>
                </div>
                <div className="mt-3 grid gap-2 text-sm md:grid-cols-3">
                  <div>Category: {recommendation.category}</div>
                  <div>
                    Suggested amount: {recommendation.amount} {recommendation.asset}
                  </div>
                  <div>Mode: {recommendation.mode}</div>
                </div>
              </div>
            ))}
          </CardContent>
        </Card>

        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ShieldAlert className="h-5 w-5" />
                Counterparty Exposure
              </CardTitle>
              <CardDescription>
                LP pressure is derived from reliability and settlement latency, not automatic
                throttling.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {snapshot?.exposures.map((exposure) => (
                <div key={exposure.counterpartyId} className="rounded-lg border p-4 text-sm">
                  <div className="flex items-center justify-between gap-3">
                    <span className="font-medium">{exposure.counterpartyId}</span>
                    <span className={riskTone(exposure.concentration)}>
                      {exposure.concentration}
                    </span>
                  </div>
                  <div className="mt-2 grid gap-1 text-muted-foreground">
                    <div>Pressure score: {exposure.pressureScore}</div>
                    <div>Reliability: {exposure.reliabilityScore ?? "n/a"}</div>
                    <div>p95 settlement latency: {exposure.p95SettlementLatencySeconds}s</div>
                  </div>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Yield Parking Lanes</CardTitle>
              <CardDescription>
                Existing allocations are shown as context for recommendations, not auto-actions.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {snapshot?.yieldAllocations?.map((allocation) => (
                <div key={allocation.protocol} className="rounded-lg border p-4 text-sm">
                  <div className="flex items-center justify-between gap-3">
                    <span className="font-medium">{allocation.protocol}</span>
                    <span>{allocation.sharePercent}%</span>
                  </div>
                  <div className="mt-2 grid gap-1 text-muted-foreground">
                    <div>Current value: {allocation.currentValue}</div>
                    <div>Accrued yield: {allocation.accruedYield}</div>
                    <div>Posture: {allocation.strategyPosture}</div>
                  </div>
                </div>
              ))}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
