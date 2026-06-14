"use client";

import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  AlertTriangle,
  ArrowLeftRight,
  BrainCircuit,
  GitCompareArrows,
  Loader2,
  Network,
  RefreshCw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  CardGridSkeleton,
  EmptyState,
  ErrorState,
  PageHeader,
  Panel,
  SectionCard,
  StatCard,
  StatGrid,
  StatusBadge,
} from "@/components/shared";
import { formatMoney, formatNumber, formatPercent, toLabel } from "@/lib/format";
import { cn } from "@/lib/utils";

type ScorerKind = "RULE_BASED" | "ONNX_HEURISTIC" | string;

type RiskLabCatalogEntry = {
  scorerKind: ScorerKind;
  label: string;
  supportsShadowCompare: boolean;
  safeFallback: string;
};

type RiskLabCatalogResponse = {
  entries: RiskLabCatalogEntry[];
};

type FeatureVector = {
  amountPercentile: number;
  velocity1h: number;
  velocity24h: number;
  velocity7d: number;
  timeOfDayAnomaly: number;
  amountRoundingPattern: number;
  recipientRecency: number;
  historicalDisputeRate: number;
  accountAgeDays: number;
  amountToAvgRatio: number;
  distinctRecipients24h: number;
  deviceNovelty: number;
  countryRisk: number;
  isCrossBorder: number;
  amountUsd: number;
  failedTxnCount24h: number;
  cumulativeAmount24hUsd: number;
};

type RiskFactor = {
  ruleName: string;
  contribution: number;
  description: string;
};

type DecisionThresholds = {
  allowBelow: number;
  blockAbove: number;
};

type DecisionExplanation = {
  decision: string;
  decisionBasis: string;
  boundaryDistance: number;
  triggeredRules: string[];
  topRiskFactors: string[];
  thresholds: DecisionThresholds;
};

type ExplainedRiskScore = {
  riskScore: {
    score: number;
    riskFactors: RiskFactor[];
  };
  metadata: {
    ruleVersionId?: string | null;
    scorer: string;
    safeFallbackUsed: boolean;
    rawScore: number;
    triggeredRules: string[];
    topRiskFactors: RiskFactor[];
    featureSnapshot: FeatureVector;
  };
};

type RiskGraph = {
  nodes: Array<{
    id: string;
    kind: string;
    label: string;
    weight: number | null;
  }>;
  edges: Array<{
    sourceId: string;
    targetId: string;
    kind: string;
  }>;
};

type RiskLabReplayResponse = {
  replayId: string;
  primaryScore: ExplainedRiskScore;
  primaryDecision: DecisionExplanation;
  challengerScore?: ExplainedRiskScore | null;
  challengerDecision?: DecisionExplanation | null;
  scoreDelta?: number | null;
  graph: RiskGraph;
};

type ScenarioPreset = {
  id: string;
  label: string;
  description: string;
  replayId: string;
  ruleVersionId: string;
  featureVector: FeatureVector;
};

const SCENARIOS: ScenarioPreset[] = [
  {
    id: "baseline-review",
    label: "Baseline review",
    description: "Moderate velocity and one novel signal keep the primary lane in review.",
    replayId: "risk_replay_baseline_review",
    ruleVersionId: "fraud-rules-v4",
    featureVector: {
      amountPercentile: 0.64,
      velocity1h: 4,
      velocity24h: 9,
      velocity7d: 24,
      timeOfDayAnomaly: 0.28,
      amountRoundingPattern: 0.2,
      recipientRecency: 0.7,
      historicalDisputeRate: 0.02,
      accountAgeDays: 45,
      amountToAvgRatio: 2.6,
      distinctRecipients24h: 3,
      deviceNovelty: 0,
      countryRisk: 0.32,
      isCrossBorder: 0,
      amountUsd: 4800,
      failedTxnCount24h: 1,
      cumulativeAmount24hUsd: 9200,
    },
  },
  {
    id: "velocity-spike",
    label: "Velocity spike",
    description: "Shadow compare should drift upward when rapid activity stacks with new-device risk.",
    replayId: "risk_replay_velocity_spike",
    ruleVersionId: "fraud-rules-v4",
    featureVector: {
      amountPercentile: 0.93,
      velocity1h: 8,
      velocity24h: 18,
      velocity7d: 42,
      timeOfDayAnomaly: 0.72,
      amountRoundingPattern: 0.8,
      recipientRecency: 1,
      historicalDisputeRate: 0.08,
      accountAgeDays: 4,
      amountToAvgRatio: 6.2,
      distinctRecipients24h: 7,
      deviceNovelty: 1,
      countryRisk: 0.82,
      isCrossBorder: 1,
      amountUsd: 24000,
      failedTxnCount24h: 4,
      cumulativeAmount24hUsd: 48000,
    },
  },
  {
    id: "cross-border-hold",
    label: "Cross-border hold",
    description: "A high-risk geography and cumulative spend push the replay toward block.",
    replayId: "risk_replay_cross_border_hold",
    ruleVersionId: "fraud-rules-v5",
    featureVector: {
      amountPercentile: 0.99,
      velocity1h: 6,
      velocity24h: 21,
      velocity7d: 54,
      timeOfDayAnomaly: 0.66,
      amountRoundingPattern: 0.6,
      recipientRecency: 1,
      historicalDisputeRate: 0.11,
      accountAgeDays: 2,
      amountToAvgRatio: 8.4,
      distinctRecipients24h: 9,
      deviceNovelty: 1,
      countryRisk: 0.94,
      isCrossBorder: 1,
      amountUsd: 52000,
      failedTxnCount24h: 5,
      cumulativeAmount24hUsd: 86000,
    },
  },
];

const PRIMARY_SCORER_KIND = "RULE_BASED";
const FEATURE_FIELDS: Array<{
  key: keyof FeatureVector;
  label: string;
  step?: string;
}> = [
  { key: "amountUsd", label: "Amount USD", step: "100" },
  { key: "velocity1h", label: "Velocity 1h", step: "1" },
  { key: "velocity24h", label: "Velocity 24h", step: "1" },
  { key: "accountAgeDays", label: "Account age days", step: "1" },
  { key: "deviceNovelty", label: "Device novelty", step: "0.1" },
  { key: "countryRisk", label: "Country risk", step: "0.01" },
  { key: "historicalDisputeRate", label: "Historical dispute rate", step: "0.01" },
  { key: "isCrossBorder", label: "Cross-border flag", step: "1" },
];

function getScenarioById(id: string): ScenarioPreset {
  return SCENARIOS.find((scenario) => scenario.id === id) ?? SCENARIOS[1];
}

async function apiRequest<T>(endpoint: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`/api/proxy${endpoint}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...init?.headers,
    },
  });

  if (!response.ok) {
    let message = "Request failed";
    try {
      const payload = (await response.json()) as { message?: string };
      message = payload.message ?? message;
    } catch {
      // Keep default message when the payload is not JSON.
    }
    throw new Error(message);
  }

  return response.json() as Promise<T>;
}

function decisionSeverity(decision?: string | null) {
  switch ((decision ?? "").toUpperCase()) {
    case "ALLOW":
      return "success" as const;
    case "BLOCK":
      return "danger" as const;
    case "REVIEW":
      return "warning" as const;
    default:
      return "neutral" as const;
  }
}

function formatDecision(decision?: string | null): string {
  if (!decision) return "Awaiting replay";
  return decision.charAt(0).toUpperCase() + decision.slice(1).toLowerCase();
}

function formatDelta(value?: number | null): string {
  if (value === null || value === undefined) return "No challenger";
  return value > 0 ? `+${formatNumber(value)}` : formatNumber(value);
}

function formatFeatureValue(key: string, value: number): string {
  if (key.toLowerCase().includes("usd")) return formatMoney(value, "USD");
  if (key.toLowerCase().includes("rate") || key.toLowerCase().includes("risk") || key.toLowerCase().includes("percentile")) {
    return formatPercent(value);
  }
  return Number.isInteger(value) ? formatNumber(value) : value.toFixed(2);
}

function neonInputClassName() {
  return "border-white/[0.08] bg-[#09090B]/70 text-foreground shadow-inner shadow-black/20 focus-visible:ring-[#00D4FF]/30";
}

export default function RiskLabWorkbench() {
  const defaultScenario = getScenarioById("velocity-spike");
  const [selectedScenarioId, setSelectedScenarioId] = useState(defaultScenario.id);
  const [replayId, setReplayId] = useState(defaultScenario.replayId);
  const [ruleVersionId, setRuleVersionId] = useState(defaultScenario.ruleVersionId);
  const [featureVector, setFeatureVector] = useState<FeatureVector>(defaultScenario.featureVector);

  const [catalogEntries, setCatalogEntries] = useState<RiskLabCatalogEntry[]>([]);
  const [catalogLoading, setCatalogLoading] = useState(true);
  const [catalogError, setCatalogError] = useState<string | null>(null);

  const [selectedChallengerKind, setSelectedChallengerKind] = useState("");
  const [replayLoading, setReplayLoading] = useState(false);
  const [replayError, setReplayError] = useState<string | null>(null);
  const [replayResult, setReplayResult] = useState<RiskLabReplayResponse | null>(null);

  const shadowCompareOptions = useMemo(
    () =>
      catalogEntries.filter(
        (entry) => entry.supportsShadowCompare && entry.scorerKind !== PRIMARY_SCORER_KIND,
      ),
    [catalogEntries],
  );

  const selectedShadowEntry = useMemo(
    () =>
      shadowCompareOptions.find((entry) => entry.scorerKind === selectedChallengerKind) ?? null,
    [selectedChallengerKind, shadowCompareOptions],
  );

  const loadCatalog = async () => {
    setCatalogLoading(true);
    setCatalogError(null);

    try {
      const response = await apiRequest<RiskLabCatalogResponse>("/v1/admin/risk-lab/catalog");
      setCatalogEntries(response.entries);
      setSelectedChallengerKind((current) => {
        if (
          current &&
          response.entries.some((entry) => entry.scorerKind === current && entry.supportsShadowCompare)
        ) {
          return current;
        }

        const fallback = response.entries.find(
          (entry) => entry.supportsShadowCompare && entry.scorerKind !== PRIMARY_SCORER_KIND,
        );
        return fallback?.scorerKind ?? "";
      });
    } catch (error) {
      setCatalogEntries([]);
      setCatalogError(error instanceof Error ? error.message : "Failed to load risk lab catalog");
    } finally {
      setCatalogLoading(false);
    }
  };

  useEffect(() => {
    void loadCatalog();
  }, []);

  const applyScenario = (scenarioId: string) => {
    const scenario = getScenarioById(scenarioId);
    setSelectedScenarioId(scenario.id);
    setReplayId(scenario.replayId);
    setRuleVersionId(scenario.ruleVersionId);
    setFeatureVector(scenario.featureVector);
    setReplayError(null);
  };

  const handleFeatureChange = (field: keyof FeatureVector, value: string) => {
    const nextValue = Number.parseFloat(value);

    setFeatureVector((current) => ({
      ...current,
      [field]: Number.isFinite(nextValue) ? nextValue : 0,
    }));
  };

  const handleRunReplay = async () => {
    setReplayLoading(true);
    setReplayError(null);

    try {
      const payload = {
        replayId,
        featureVector,
        ruleVersionId: ruleVersionId.trim() || undefined,
        challenger: selectedChallengerKind
          ? {
              scorerKind: selectedChallengerKind,
            }
          : undefined,
      };

      const response = await apiRequest<RiskLabReplayResponse>("/v1/admin/risk-lab/replay", {
        method: "POST",
        body: JSON.stringify(payload),
      });
      setReplayResult(response);
    } catch (error) {
      setReplayResult(null);
      setReplayError(error instanceof Error ? error.message : "Failed to replay risk lab request");
    } finally {
      setReplayLoading(false);
    }
  };

  const lastDecision = replayResult?.primaryDecision?.decision ?? null;

  return (
    <div className="space-y-6">
      <PageHeader
        title="Risk Lab"
        description="Compare primary scoring against a bounded shadow lane, replay feature snapshots, and keep explainability visible for every operator decision."
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={() => void loadCatalog()}
            disabled={catalogLoading}
            aria-label="Refresh risk lab catalog"
            className="border-[#00D4FF]/25 bg-[#00D4FF]/10 text-[#00D4FF] hover:bg-[#00D4FF]/15"
          >
            {catalogLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
          </Button>
        }
      />

      <StatGrid>
        <StatCard
          title="Catalog entries"
          value={catalogLoading ? "Loading" : formatNumber(catalogEntries.length)}
          icon={<ShieldCheck className="h-4 w-4" />}
          loading={catalogLoading}
          subtitle="Backend-published scorers"
          accentColor="green"
        />
        <StatCard
          title="Shadow compare"
          value={catalogLoading ? "Loading" : selectedShadowEntry ? "Configured" : "Not configured"}
          icon={<GitCompareArrows className="h-4 w-4" />}
          loading={catalogLoading}
          subtitle={selectedShadowEntry?.label ?? "Catalog controlled"}
          accentColor="violet"
        />
        <StatCard
          title="Primary outcome"
          value={formatDecision(lastDecision)}
          icon={<BrainCircuit className="h-4 w-4" />}
          subtitle="Latest replay decision"
          accentColor="cyan"
        />
        <StatCard
          title="Scenario amount"
          value={formatMoney(featureVector.amountUsd, "USD")}
          icon={<Sparkles className="h-4 w-4" />}
          subtitle={`${formatNumber(featureVector.velocity24h)} tx / 24h`}
          accentColor="amber"
        />
      </StatGrid>

      <Panel
        variant="glass"
        className="border-[#00D4FF]/15 bg-[#09090B]/70"
        contentClassName="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between"
      >
        <div className="space-y-1">
          <div className="flex items-center gap-2 text-sm font-semibold text-[#00D4FF]">
            <Network className="h-4 w-4" />
            Bounded replay surface
          </div>
          <p className="max-w-3xl text-sm text-muted-foreground">
            The workbench stays inside published admin routes and avoids broader risk operations until
            the backend exposes them.
          </p>
        </div>
        <StatusBadge status="Compare, replay, explain" severity="info" />
      </Panel>

      <div className="grid gap-6 xl:grid-cols-[1.05fr,0.95fr]">
        <div className="space-y-6">
          <SectionCard
            header={{
              title: "Replay scenarios",
              description:
                "Start from a bounded scenario and then tune high-signal inputs instead of opening a full product shell.",
            }}
          >
            <div className="grid gap-3">
              {SCENARIOS.map((scenario) => {
                const isActive = scenario.id === selectedScenarioId;

                return (
                  <button
                    key={scenario.id}
                    type="button"
                    onClick={() => applyScenario(scenario.id)}
                    className={cn(
                      "w-full rounded-xl border p-4 text-left transition duration-300",
                      isActive
                        ? "border-[#00FF87]/40 bg-[#00FF87]/10 shadow-[0_0_28px_rgba(0,255,135,0.08)]"
                        : "border-white/[0.06] bg-[#09090B]/50 hover:border-[#00D4FF]/35 hover:bg-[#00D4FF]/5",
                    )}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <div className="font-medium text-foreground">{scenario.label}</div>
                        <p className="mt-1 text-sm text-muted-foreground">{scenario.description}</p>
                      </div>
                      {isActive ? <StatusBadge status="Active" severity="success" /> : null}
                    </div>
                  </button>
                );
              })}
            </div>
          </SectionCard>

          <SectionCard
            header={{
              title: "Replay setup",
              description: "Tune only inputs that change the operator readout most, then run a new replay.",
            }}
          >
            <div className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="risk-lab-replay-id">Replay ID</Label>
                  <Input
                    id="risk-lab-replay-id"
                    value={replayId}
                    onChange={(event) => setReplayId(event.target.value)}
                    className={neonInputClassName()}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="risk-lab-rule-version">Rule version</Label>
                  <Input
                    id="risk-lab-rule-version"
                    value={ruleVersionId}
                    onChange={(event) => setRuleVersionId(event.target.value)}
                    className={neonInputClassName()}
                  />
                </div>
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                {FEATURE_FIELDS.map((field) => (
                  <div key={field.key} className="space-y-2">
                    <Label htmlFor={`risk-lab-${field.key}`}>{field.label}</Label>
                    <Input
                      id={`risk-lab-${field.key}`}
                      type="number"
                      step={field.step}
                      value={featureVector[field.key]}
                      onChange={(event) => handleFeatureChange(field.key, event.target.value)}
                      className={neonInputClassName()}
                    />
                  </div>
                ))}
              </div>
            </div>
          </SectionCard>

          <SectionCard
            header={{
              title: "Compare lane",
              description: "Primary scoring remains rule-based. Shadow compare stays opt-in and catalog-backed.",
            }}
            footer={
              <div className="flex flex-wrap items-center gap-3">
                <Button
                  onClick={handleRunReplay}
                  disabled={catalogLoading || !!catalogError || replayLoading}
                  className="bg-[#00FF87] text-black hover:bg-[#00FF87]/90"
                >
                  {replayLoading ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Running replay
                    </>
                  ) : (
                    <>
                      <ArrowLeftRight className="mr-2 h-4 w-4" />
                      Run replay
                    </>
                  )}
                </Button>
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Sparkles className="h-4 w-4 text-[#FFB800]" />
                  Explainability stays attached to the same replay request.
                </div>
              </div>
            }
          >
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-xl border border-[#00FF87]/15 bg-[#00FF87]/5 p-4">
                <div className="text-sm text-muted-foreground">Primary lane</div>
                <div className="mt-2 font-medium text-[#00FF87]">Rule-based scorer</div>
                <div className="mt-2 text-sm text-muted-foreground">
                  Live operator baseline for replay and explanation output.
                </div>
              </div>
              <div className="space-y-2">
                <Label htmlFor="risk-lab-shadow-compare">Shadow compare lane</Label>
                <select
                  id="risk-lab-shadow-compare"
                  aria-label="Shadow compare lane"
                  className="w-full rounded-md border border-white/[0.08] bg-[#09090B]/70 px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#7B61FF]/30"
                  value={selectedChallengerKind}
                  onChange={(event) => setSelectedChallengerKind(event.target.value)}
                  disabled={catalogLoading || shadowCompareOptions.length === 0}
                >
                  {shadowCompareOptions.length === 0 ? (
                    <option value="">No compare lanes published</option>
                  ) : (
                    shadowCompareOptions.map((entry) => (
                      <option key={entry.scorerKind} value={entry.scorerKind}>
                        {entry.label}
                      </option>
                    ))
                  )}
                </select>
                <div className="rounded-md border border-white/[0.06] bg-[#7B61FF]/10 px-3 py-2 text-sm text-muted-foreground">
                  Safe fallback: {selectedShadowEntry?.safeFallback ?? "Not available"}
                </div>
              </div>
            </div>
          </SectionCard>
        </div>

        <div className="space-y-6">
          {catalogLoading ? (
            <Panel header={{ title: "Loading compare surface", description: "Loading risk lab catalog..." }}>
              <CardGridSkeleton cards={2} className="lg:grid-cols-2" />
            </Panel>
          ) : catalogError ? (
            <Panel header={{ title: "Catalog unavailable", description: "The workbench stays bounded until the catalog can be fetched again." }}>
              <ErrorState message={catalogError} retry={() => void loadCatalog()} />
            </Panel>
          ) : replayLoading ? (
            <Panel header={{ title: "Replay in progress", description: "Preparing compare and explanation surfaces for the selected replay." }}>
              <EmptyState
                icon={<Loader2 className="h-8 w-8 animate-spin text-[#00D4FF]" />}
                title="Running replay"
                description="Waiting for replay scoring, challenger comparison, and graph assembly."
              />
            </Panel>
          ) : replayError ? (
            <Panel header={{ title: "Replay request failed", description: "The compare lane remains intact so the operator can adjust and rerun." }}>
              <ErrorState message={replayError} />
            </Panel>
          ) : replayResult ? (
            <div className="space-y-6">
              <SectionCard
                header={{
                  title: replayResult.replayId,
                  description: "Primary and challenger outcomes stay side-by-side for operator review.",
                }}
              >
                <div className="space-y-4">
                  <div className="grid gap-4 md:grid-cols-3">
                    <div className="rounded-xl border border-white/[0.06] bg-[#09090B]/60 p-4">
                      <div className="text-sm text-muted-foreground">Primary decision</div>
                      <div className="mt-2 flex items-center gap-2">
                        <span className="text-2xl font-semibold">
                          {formatDecision(replayResult.primaryDecision.decision)}
                        </span>
                        <StatusBadge
                          status={formatNumber(replayResult.primaryScore.riskScore.score)}
                          severity={decisionSeverity(replayResult.primaryDecision.decision)}
                        />
                      </div>
                    </div>
                    <div className="rounded-xl border border-white/[0.06] bg-[#09090B]/60 p-4">
                      <div className="text-sm text-muted-foreground">Challenger decision</div>
                      <div className="mt-2 flex items-center gap-2">
                        <span className="text-2xl font-semibold">
                          {formatDecision(replayResult.challengerDecision?.decision)}
                        </span>
                        {replayResult.challengerScore ? (
                          <StatusBadge
                            status={formatNumber(replayResult.challengerScore.riskScore.score)}
                            severity={decisionSeverity(replayResult.challengerDecision?.decision)}
                          />
                        ) : null}
                      </div>
                    </div>
                    <div className="rounded-xl border border-white/[0.06] bg-[#09090B]/60 p-4">
                      <div className="text-sm text-muted-foreground">Score delta</div>
                      <div className="mt-2 text-2xl font-semibold text-[#00D4FF]">
                        {formatDelta(replayResult.scoreDelta)}
                      </div>
                    </div>
                  </div>

                  <div className="grid gap-4 md:grid-cols-2">
                    <RiskFactorsPanel
                      title="Primary factors"
                      icon={<BrainCircuit className="h-4 w-4 text-[#00FF87]" />}
                      factors={replayResult.primaryScore.metadata.topRiskFactors}
                    />
                    <RiskFactorsPanel
                      title="Challenger factors"
                      icon={<AlertTriangle className="h-4 w-4 text-[#FFB800]" />}
                      factors={replayResult.challengerScore?.metadata.topRiskFactors ?? []}
                      emptyDescription="Challenger compare is disabled for this replay."
                    />
                  </div>
                </div>
              </SectionCard>

              <StatGrid>
                <StatCard
                  title="Graph nodes"
                  value={formatNumber(replayResult.graph.nodes.length)}
                  icon={<Network className="h-4 w-4" />}
                  accentColor="violet"
                />
                <StatCard
                  title="Graph edges"
                  value={formatNumber(replayResult.graph.edges.length)}
                  icon={<GitCompareArrows className="h-4 w-4" />}
                  accentColor="cyan"
                />
              </StatGrid>

              <SectionCard
                header={{
                  title: "Explainability graph",
                  description: "Replay, decision, and factor nodes remain inspectable without leaving the admin workbench.",
                }}
              >
                <div className="grid gap-3">
                  {replayResult.graph.nodes.map((node) => (
                    <div
                      key={node.id}
                      className="flex items-center justify-between gap-3 rounded-lg border border-white/[0.06] bg-[#09090B]/60 px-4 py-3"
                    >
                      <div>
                        <div className="font-medium">
                          {node.kind === "REPLAY"
                            ? "Replay root"
                            : node.kind === "RULE_FACTOR"
                              ? `Factor: ${node.label}`
                              : node.label}
                        </div>
                        <div className="text-sm text-muted-foreground">{toLabel(node.kind)}</div>
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {node.weight === null ? "No weight" : `Weight ${formatNumber(node.weight)}`}
                      </div>
                    </div>
                  ))}
                </div>
              </SectionCard>

              <SectionCard
                header={{
                  title: "Feature snapshot",
                  description: "The snapshot below mirrors the replay request that produced the visible outcome.",
                }}
              >
                <div className="grid gap-3 md:grid-cols-2">
                  {Object.entries(replayResult.primaryScore.metadata.featureSnapshot).map(([key, value]) => (
                    <div
                      key={key}
                      className="rounded-lg border border-white/[0.06] bg-[#09090B]/60 px-4 py-3 text-sm"
                    >
                      <div className="text-muted-foreground">{toLabel(key)}</div>
                      <div className="mt-1 font-medium">{formatFeatureValue(key, value)}</div>
                    </div>
                  ))}
                </div>
              </SectionCard>
            </div>
          ) : (
            <Panel className="border-dashed" header={{ title: "Awaiting replay" }}>
              <EmptyState
                icon={<ArrowLeftRight className="h-8 w-8" />}
                title="No replay loaded"
                description="Run a replay to compare the primary scorer against the shadow lane, inspect the decision delta, and keep explainability attached to the same payload."
              />
            </Panel>
          )}
        </div>
      </div>
    </div>
  );
}

function RiskFactorsPanel({
  title,
  icon,
  factors,
  emptyDescription = "No top factors returned for this lane.",
}: {
  title: string;
  icon: ReactNode;
  factors: RiskFactor[];
  emptyDescription?: string;
}) {
  return (
    <div className="rounded-xl border border-white/[0.06] bg-[#09090B]/50 p-4">
      <div className="mb-3 flex items-center gap-2">
        {icon}
        <span className="font-medium">{title}</span>
      </div>
      <div className="space-y-3">
        {factors.length > 0 ? (
          factors.map((factor) => (
            <div key={factor.ruleName} className="rounded-lg border border-white/[0.06] bg-[#111113]/80 p-3">
              <div className="flex items-center justify-between gap-3">
                <div className="font-medium">{factor.ruleName}</div>
                <StatusBadge status={`+${formatNumber(factor.contribution)}`} severity="info" />
              </div>
              <p className="mt-2 text-sm text-muted-foreground">{factor.description}</p>
            </div>
          ))
        ) : (
          <EmptyState title="No factors" description={emptyDescription} className="py-8" />
        )}
      </div>
    </div>
  );
}
