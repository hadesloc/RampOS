"use client";

import { useEffect, useState } from "react";
import { Loader2, RefreshCw, ShieldCheck, SlidersHorizontal } from "lucide-react";

import LpScorecard, {
  type LiquidityFilters,
  type LiquidityPolicyCompareResponse,
  type LiquidityScorecardRow,
} from "@/components/liquidity/LpScorecard";
import { Button } from "@/components/ui/button";
import { PageHeader, StatGrid, StatCard, Panel } from "@/components/shared";

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

  const activePolicyLabel = policyLoading
    ? "Loading…"
    : policyCompare?.activeVersion ?? "Not loaded";
  const compareDirectionLabel = policyLoading
    ? "Loading…"
    : policyCompare?.requestedDirection ?? getPolicyDirection(filters.direction);

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Liquidity Scorecard"
        description="Review LP reliability snapshots, compare bounded policy versions, and activate the operator-selected catalog entry."
        actions={
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
        }
      />

      <StatGrid cols={3}>
        <StatCard
          title="Visible rows"
          value={scorecardRows.length}
          accentColor="cyan"
          loading={scorecardLoading}
        />
        <StatCard
          title="Active policy"
          value={activePolicyLabel}
          accentColor="green"
          loading={policyLoading}
        />
        <StatCard
          title="Compare direction"
          value={compareDirectionLabel}
          icon={<ShieldCheck className="h-4 w-4" />}
          accentColor="violet"
          loading={policyLoading}
        />
      </StatGrid>

      <Panel
        variant="solid"
        header={{
          title: "Bounded operator surface",
          description:
            "Filtering hits the scorecard endpoint only. Policy compare stays direction-scoped and activation is limited to the backend catalog.",
          actions: (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <SlidersHorizontal className="h-4 w-4" />
              No broad admin refactor
            </div>
          ),
        }}
      >
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
      </Panel>
    </main>
  );
}
