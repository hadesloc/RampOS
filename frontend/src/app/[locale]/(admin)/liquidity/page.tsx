"use client";

import { useEffect, useState } from "react";
import { Loader2, RefreshCw, ShieldCheck, SlidersHorizontal } from "lucide-react";

import LpScorecard, {
  type LiquidityFilters,
  type LiquidityPolicyCompareResponse,
  type LiquidityScorecardRow,
} from "@/components/liquidity/LpScorecard";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardHeader,
  CardDescription,
  CardTitle,
} from "@/components/ui/card";

const DEFAULT_FILTERS: LiquidityFilters = {
  lpId: "",
  direction: "",
  windowKind: "",
};

type ActivationResponse = {
  status: string;
  version: string;
  direction: string;
  fallbackBehavior: string;
};

async function apiRequest<T>(endpoint: string, init?: RequestInit): Promise<T> {
  const url = `/api/proxy${endpoint}`;
  const options = init
    ? {
        ...init,
        headers: {
          "Content-Type": "application/json",
          ...init.headers,
        },
      }
    : undefined;

  const response = options ? await fetch(url, options) : await fetch(url);

  if (!response.ok) {
    let message = "Request failed";
    try {
      const payload = (await response.json()) as {
        message?: string;
        error?: { message?: string };
      };
      message = payload.message ?? payload.error?.message ?? message;
    } catch {
      // Keep the default message when the body is not JSON.
    }
    throw new Error(message);
  }

  return response.json() as Promise<T>;
}

function buildScorecardQuery(filters: LiquidityFilters): string {
  const params = new URLSearchParams();
  if (filters.lpId.trim()) params.set("lpId", filters.lpId.trim());
  if (filters.direction) params.set("direction", filters.direction);
  if (filters.windowKind) params.set("windowKind", filters.windowKind);
  params.set("limit", "20");
  return params.toString();
}

function getPolicyDirection(direction: string): string {
  return direction || "OFFRAMP";
}

export default function LiquidityPage() {
  const [filters, setFilters] = useState<LiquidityFilters>(DEFAULT_FILTERS);
  const [scorecardRows, setScorecardRows] = useState<LiquidityScorecardRow[]>([]);
  const [scorecardLoading, setScorecardLoading] = useState(true);
  const [scorecardError, setScorecardError] = useState<string | null>(null);
  const [policyCompare, setPolicyCompare] = useState<LiquidityPolicyCompareResponse | null>(null);
  const [policyLoading, setPolicyLoading] = useState(true);
  const [policyError, setPolicyError] = useState<string | null>(null);
  const [activatingVersion, setActivatingVersion] = useState<string | null>(null);
  const [activationNotice, setActivationNotice] = useState<{
    type: "success" | "error";
    message: string;
  } | null>(null);

  const loadLiquidity = async (nextFilters: LiquidityFilters) => {
    const policyDirection = getPolicyDirection(nextFilters.direction);
    const scorecardEndpoint = `/v1/admin/liquidity/scorecard?${buildScorecardQuery(nextFilters)}`;
    const policyEndpoint = `/v1/admin/liquidity/policies/compare?direction=${policyDirection}`;

    setScorecardLoading(true);
    setPolicyLoading(true);
    setScorecardError(null);
    setPolicyError(null);

    const [scorecardResult, policyResult] = await Promise.allSettled([
      apiRequest<LiquidityScorecardRow[]>(scorecardEndpoint),
      apiRequest<LiquidityPolicyCompareResponse>(policyEndpoint),
    ]);

    if (scorecardResult.status === "fulfilled") {
      setScorecardRows(scorecardResult.value);
    } else {
      setScorecardRows([]);
      setScorecardError(scorecardResult.reason instanceof Error ? scorecardResult.reason.message : "Failed to load scorecard");
    }

    if (policyResult.status === "fulfilled") {
      setPolicyCompare(policyResult.value);
    } else {
      setPolicyCompare(null);
      setPolicyError(policyResult.reason instanceof Error ? policyResult.reason.message : "Failed to load policy catalog");
    }

    setScorecardLoading(false);
    setPolicyLoading(false);
  };

  useEffect(() => {
    void loadLiquidity(DEFAULT_FILTERS);
  }, []);

  const handleFilterChange = (field: keyof LiquidityFilters, value: string) => {
    setFilters((current) => ({
      ...current,
      [field]: value,
    }));
  };

  const handleApplyFilters = () => {
    setActivationNotice(null);
    void loadLiquidity(filters);
  };

  const handleResetFilters = () => {
    setFilters(DEFAULT_FILTERS);
    setActivationNotice(null);
    void loadLiquidity(DEFAULT_FILTERS);
  };

  const handleRefresh = () => {
    setActivationNotice(null);
    void loadLiquidity(filters);
  };

  const handleActivatePolicy = async (version: string, direction: string) => {
    setActivatingVersion(version);
    setActivationNotice(null);

    try {
      const response = await apiRequest<ActivationResponse>(
        "/v1/admin/liquidity/policies/activate",
        {
          method: "POST",
          body: JSON.stringify({ version, direction }),
        },
      );

      setPolicyCompare((current) =>
        current
          ? {
              ...current,
              activeVersion: response.version,
              requestedDirection: response.direction,
            }
          : current,
      );
      setActivationNotice({
        type: "success",
        message: `Activated ${response.version} for ${response.direction}.`,
      });
    } catch (error) {
      setActivationNotice({
        type: "error",
        message: error instanceof Error ? error.message : "Failed to activate policy",
      });
    } finally {
      setActivatingVersion(null);
    }
  };

  const rowCountLabel = scorecardLoading ? "Loading..." : `${scorecardRows.length}`;
  const activePolicyLabel = policyLoading
    ? "Loading..."
    : policyCompare?.activeVersion ?? "Not loaded";
  const compareDirectionLabel = policyLoading
    ? "Loading..."
    : policyCompare?.requestedDirection ?? getPolicyDirection(filters.direction);

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Liquidity Scorecard</h1>
          <p className="text-muted-foreground">
            Review LP reliability snapshots, compare bounded policy versions, and activate the
            operator-selected catalog entry.
          </p>
        </div>
        <Button
          variant="outline"
          size="icon"
          onClick={handleRefresh}
          disabled={scorecardLoading || policyLoading}
          aria-label="Refresh liquidity page"
        >
          {scorecardLoading || policyLoading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <RefreshCw className="h-4 w-4" />
          )}
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Visible rows</CardDescription>
            <CardTitle>{rowCountLabel}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Active policy</CardDescription>
            <CardTitle className="break-all">{activePolicyLabel}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Compare direction</CardDescription>
            <CardTitle className="flex items-center gap-2">
              {compareDirectionLabel}
              <ShieldCheck className="h-4 w-4 text-muted-foreground" />
            </CardTitle>
          </CardHeader>
        </Card>
      </div>

      <Card className="border-dashed">
        <CardHeader className="flex flex-row items-start justify-between gap-3 space-y-0">
          <div className="space-y-1">
            <CardTitle className="text-base">Bounded operator surface</CardTitle>
            <CardDescription>
              Filtering hits the scorecard endpoint only. Policy compare stays direction-scoped and
              activation is limited to the backend catalog.
            </CardDescription>
          </div>
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <SlidersHorizontal className="h-4 w-4" />
            No broad admin refactor
          </div>
        </CardHeader>
      </Card>

      <LpScorecard
        filters={filters}
        onFilterChange={handleFilterChange}
        onApplyFilters={handleApplyFilters}
        onResetFilters={handleResetFilters}
        scorecardRows={scorecardRows}
        scorecardLoading={scorecardLoading}
        scorecardError={scorecardError}
        policyCompare={policyCompare}
        policyLoading={policyLoading}
        policyError={policyError}
        activatingVersion={activatingVersion}
        activationNotice={activationNotice}
        onActivatePolicy={handleActivatePolicy}
      />
    </div>
  );
}
