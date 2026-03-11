"use client";

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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

export type LiquidityScorecardRow = {
  lpId: string;
  direction: string;
  windowKind: string;
  snapshotVersion: string;
  quoteCount: number;
  fillCount: number;
  rejectCount: number;
  settlementCount: number;
  disputeCount: number;
  fillRate: string;
  rejectRate: string;
  disputeRate: string;
  avgSlippageBps: string;
  p95SettlementLatencySeconds: number;
  reliabilityScore: string | null;
  updatedAt: string;
};

export type LiquidityPolicyWeights = {
  priceWeight: string;
  reliabilityWeight: string;
  fillRateWeight: string;
  rejectRateWeight: string;
  disputeRateWeight: string;
  slippageWeight: string;
  settlementLatencyWeight: string;
};

export type LiquidityPolicyDescriptor = {
  version: string;
  direction: string;
  reliabilityWindowKind: string;
  minReliabilityObservations: number;
  fallbackBehavior: string;
  weights: LiquidityPolicyWeights;
};

export type LiquidityPolicyCompareResponse = {
  activeVersion: string;
  requestedDirection: string;
  policies: LiquidityPolicyDescriptor[];
};

export type LiquidityFilters = {
  lpId: string;
  direction: string;
  windowKind: string;
};

type LpScorecardProps = {
  filters: LiquidityFilters;
  onFilterChange: (field: keyof LiquidityFilters, value: string) => void;
  onApplyFilters: () => void;
  onResetFilters: () => void;
  scorecardRows: LiquidityScorecardRow[];
  scorecardLoading: boolean;
  scorecardError: string | null;
  policyCompare: LiquidityPolicyCompareResponse | null;
  policyLoading: boolean;
  policyError: string | null;
  activatingVersion: string | null;
  activationNotice:
    | {
        type: "success" | "error";
        message: string;
      }
    | null;
  onActivatePolicy: (version: string, direction: string) => void;
};

const WINDOW_KIND_OPTIONS = [
  { label: "All windows", value: "" },
  { label: "Rolling 24h", value: "ROLLING_24H" },
  { label: "Rolling 7d", value: "ROLLING_7D" },
  { label: "Rolling 30d", value: "ROLLING_30D" },
  { label: "Calendar day", value: "CALENDAR_DAY" },
];

function formatPercent(value?: string | null): string {
  const numeric = Number(value ?? "");
  if (!Number.isFinite(numeric)) return "N/A";
  return `${(numeric * 100).toFixed(2)}%`;
}

function formatScore(value?: string | null): string {
  const numeric = Number(value ?? "");
  if (!Number.isFinite(numeric)) return "N/A";
  return numeric.toFixed(4);
}

function formatTimestamp(value?: string | null): string {
  if (!value) return "N/A";

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;

  return date.toLocaleString("en-US", {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function WeightBadge({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <Badge variant="secondary" shape="pill" className="font-mono">
      {label}: {value}
    </Badge>
  );
}

export function LpScorecard({
  filters,
  onFilterChange,
  onApplyFilters,
  onResetFilters,
  scorecardRows,
  scorecardLoading,
  scorecardError,
  policyCompare,
  policyLoading,
  policyError,
  activatingVersion,
  activationNotice,
  onActivatePolicy,
}: LpScorecardProps) {
  const hasPolicies = (policyCompare?.policies.length ?? 0) > 0;

  return (
    <div className="grid gap-6 xl:grid-cols-[1.35fr,0.95fr]">
      <div className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Scorecard filters</CardTitle>
            <CardDescription>
              Filter by LP, direction, and reliability window before reloading the scorecard.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-4 md:grid-cols-3">
              <div className="space-y-2">
                <Label htmlFor="liquidity-lp-id">LP ID</Label>
                <Input
                  id="liquidity-lp-id"
                  aria-label="LP ID"
                  value={filters.lpId}
                  onChange={(event) => onFilterChange("lpId", event.target.value)}
                  placeholder="lp_..."
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="liquidity-direction">Direction</Label>
                <select
                  id="liquidity-direction"
                  aria-label="Direction"
                  className="h-10 w-full rounded-md border bg-background px-3 py-2 text-sm"
                  value={filters.direction}
                  onChange={(event) => onFilterChange("direction", event.target.value)}
                >
                  <option value="">All directions</option>
                  <option value="OFFRAMP">OFFRAMP</option>
                  <option value="ONRAMP">ONRAMP</option>
                </select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="liquidity-window-kind">Window kind</Label>
                <select
                  id="liquidity-window-kind"
                  aria-label="Window kind"
                  className="h-10 w-full rounded-md border bg-background px-3 py-2 text-sm"
                  value={filters.windowKind}
                  onChange={(event) => onFilterChange("windowKind", event.target.value)}
                >
                  {WINDOW_KIND_OPTIONS.map((option) => (
                    <option key={option.value || "all"} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="flex flex-wrap gap-3">
              <Button onClick={onApplyFilters} disabled={scorecardLoading || policyLoading}>
                Apply filters
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={onResetFilters}
                disabled={scorecardLoading || policyLoading}
              >
                Clear filters
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>LP scorecard</CardTitle>
            <CardDescription>
              Reliability snapshots from the bounded admin endpoint, ordered by freshest updates.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {scorecardError && (
              <Alert variant="destructive">
                <AlertTitle>Scorecard request failed</AlertTitle>
                <AlertDescription>{scorecardError}</AlertDescription>
              </Alert>
            )}

            {scorecardLoading && scorecardRows.length === 0 ? (
              <div className="space-y-3">
                <Skeleton className="h-10 w-full" />
                <Skeleton className="h-48 w-full" />
              </div>
            ) : scorecardRows.length === 0 ? (
              <div className="rounded-lg border border-dashed px-4 py-8 text-sm text-muted-foreground">
                No scorecard rows match the current filters. Clear filters or switch direction to
                widen the view.
              </div>
            ) : (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader sticky>
                    <TableRow>
                      <TableHead>LP</TableHead>
                      <TableHead>Direction</TableHead>
                      <TableHead>Window</TableHead>
                      <TableHead>Reliability</TableHead>
                      <TableHead>Fill rate</TableHead>
                      <TableHead>Reject rate</TableHead>
                      <TableHead>Dispute rate</TableHead>
                      <TableHead>Slippage</TableHead>
                      <TableHead>P95 latency</TableHead>
                      <TableHead>Updated</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {scorecardRows.map((row) => (
                      <TableRow key={`${row.lpId}-${row.direction}-${row.windowKind}`}>
                        <TableCell className="font-medium">{row.lpId}</TableCell>
                        <TableCell>{row.direction}</TableCell>
                        <TableCell>{row.windowKind}</TableCell>
                        <TableCell>{formatScore(row.reliabilityScore)}</TableCell>
                        <TableCell>{formatPercent(row.fillRate)}</TableCell>
                        <TableCell>{formatPercent(row.rejectRate)}</TableCell>
                        <TableCell>{formatPercent(row.disputeRate)}</TableCell>
                        <TableCell>{row.avgSlippageBps} bps</TableCell>
                        <TableCell>{row.p95SettlementLatencySeconds}s</TableCell>
                        <TableCell>{formatTimestamp(row.updatedAt)}</TableCell>
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
        <Card>
          <CardHeader>
            <CardTitle>Policy catalog</CardTitle>
            <CardDescription>
              Compare policy versions for the active direction and activate a bounded catalog entry.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {activationNotice && (
              <Alert variant={activationNotice.type === "success" ? "success" : "destructive"}>
                <AlertTitle>
                  {activationNotice.type === "success" ? "Policy activated" : "Activation failed"}
                </AlertTitle>
                <AlertDescription>{activationNotice.message}</AlertDescription>
              </Alert>
            )}

            {policyError && (
              <Alert variant="destructive">
                <AlertTitle>Policy compare failed</AlertTitle>
                <AlertDescription>{policyError}</AlertDescription>
              </Alert>
            )}

            {policyLoading && !hasPolicies ? (
              <div className="space-y-3">
                <Skeleton className="h-24 w-full" />
                <Skeleton className="h-24 w-full" />
              </div>
            ) : !hasPolicies ? (
              <div className="rounded-lg border border-dashed px-4 py-8 text-sm text-muted-foreground">
                Policy catalog will appear once compare data loads.
              </div>
            ) : (
              <div className="space-y-4">
                {policyCompare?.policies.map((policy) => {
                  const isActive = policyCompare.activeVersion === policy.version;
                  const isActivating = activatingVersion === policy.version;

                  return (
                    <div key={policy.version} className="rounded-xl border p-4">
                      <div className="flex items-start justify-between gap-3">
                        <div className="space-y-1">
                          <div className="flex flex-wrap items-center gap-2">
                            <div className="font-medium">{policy.version}</div>
                            {isActive ? (
                              <Badge variant="success" shape="pill">
                                Active
                              </Badge>
                            ) : null}
                            <Badge variant="outline" shape="pill">
                              {policy.direction}
                            </Badge>
                          </div>
                          <div className="text-sm text-muted-foreground">
                            Window {policy.reliabilityWindowKind} with min{" "}
                            {policy.minReliabilityObservations} observations
                          </div>
                        </div>
                        <Button
                          type="button"
                          variant={isActive ? "outline" : "default"}
                          onClick={() => onActivatePolicy(policy.version, policy.direction)}
                          disabled={isActive || isActivating}
                        >
                          {isActivating
                            ? "Activating..."
                            : isActive
                              ? "Active policy"
                              : `Activate ${policy.version}`}
                        </Button>
                      </div>

                      <div className="mt-4 flex flex-wrap gap-2">
                        <WeightBadge label="price" value={policy.weights.priceWeight} />
                        <WeightBadge label="reliability" value={policy.weights.reliabilityWeight} />
                        <WeightBadge label="fill" value={policy.weights.fillRateWeight} />
                        <WeightBadge label="reject" value={policy.weights.rejectRateWeight} />
                        <WeightBadge label="dispute" value={policy.weights.disputeRateWeight} />
                        <WeightBadge label="slippage" value={policy.weights.slippageWeight} />
                        <WeightBadge
                          label="latency"
                          value={policy.weights.settlementLatencyWeight}
                        />
                      </div>

                      <div className="mt-4 text-sm text-muted-foreground">
                        Fallback: {policy.fallbackBehavior}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export default LpScorecard;
