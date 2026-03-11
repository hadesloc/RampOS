"use client";

import { useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  ArrowLeftRight,
  BrainCircuit,
  GitCompareArrows,
  Loader2,
  RefreshCw,
  Sparkles,
} from "lucide-react";

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

function decisionVariant(decision?: string | null) {
  switch ((decision ?? "").toUpperCase()) {
    case "ALLOW":
      return "success" as const;
    case "BLOCK":
      return "destructive" as const;
    case "REVIEW":
      return "warning" as const;
    default:
      return "outline" as const;
  }
}

function formatDecision(decision?: string | null): string {
  if (!decision) return "Awaiting replay";
  return decision.charAt(0).toUpperCase() + decision.slice(1).toLowerCase();
}

function formatDelta(value?: number | null): string {
  if (value === null || value === undefined) return "No challenger";
  return value > 0 ? `+${value}` : `${value}`;
}

function formatFeatureValue(value: number): string {
  return Number.isInteger(value) ? `${value}` : value.toFixed(2);
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
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Risk Lab</h1>
          <p className="text-muted-foreground">
            Compare primary scoring against a bounded shadow lane, replay feature snapshots, and
            keep explainability visible for every operator decision.
          </p>
        </div>
        <Button
          variant="outline"
          size="icon"
          onClick={() => void loadCatalog()}
          disabled={catalogLoading}
          aria-label="Refresh risk lab catalog"
        >
          {catalogLoading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <RefreshCw className="h-4 w-4" />
          )}
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Catalog entries</CardDescription>
            <CardTitle>{catalogLoading ? "Loading..." : `${catalogEntries.length}`}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Shadow compare</CardDescription>
            <CardTitle className="text-lg">
              {catalogLoading ? "Loading..." : selectedShadowEntry ? "Configured" : "Not configured"}
            </CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Latest primary outcome</CardDescription>
            <CardTitle>{formatDecision(lastDecision)}</CardTitle>
          </CardHeader>
        </Card>
      </div>

      <Card className="border-dashed">
        <CardHeader className="flex flex-row items-start justify-between gap-3 space-y-0">
          <div className="space-y-1">
            <CardTitle className="text-base">Bounded replay surface</CardTitle>
            <CardDescription>
              The workbench stays inside the published admin routes and avoids any broader risk
              operations until the backend exposes them.
            </CardDescription>
          </div>
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <GitCompareArrows className="h-4 w-4" />
            Compare, replay, explain
          </div>
        </CardHeader>
      </Card>

      <div className="grid gap-6 xl:grid-cols-[1.05fr,0.95fr]">
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Replay scenarios</CardTitle>
              <CardDescription>
                Start from a bounded scenario and then tune a few high-signal inputs instead of
                opening a full product shell.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {SCENARIOS.map((scenario) => {
                const isActive = scenario.id === selectedScenarioId;

                return (
                  <button
                    key={scenario.id}
                    type="button"
                    onClick={() => applyScenario(scenario.id)}
                    className={`w-full rounded-xl border p-4 text-left transition ${
                      isActive
                        ? "border-primary bg-primary/5 shadow-sm"
                        : "border-border hover:border-primary/40 hover:bg-muted/30"
                    }`}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <div className="font-medium">{scenario.label}</div>
                        <p className="mt-1 text-sm text-muted-foreground">{scenario.description}</p>
                      </div>
                      {isActive ? (
                        <Badge variant="info" shape="pill">
                          Active
                        </Badge>
                      ) : null}
                    </div>
                  </button>
                );
              })}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Replay setup</CardTitle>
              <CardDescription>
                Tune only the inputs that change the operator readout most, then run a new replay.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="risk-lab-replay-id">Replay ID</Label>
                  <Input
                    id="risk-lab-replay-id"
                    value={replayId}
                    onChange={(event) => setReplayId(event.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="risk-lab-rule-version">Rule version</Label>
                  <Input
                    id="risk-lab-rule-version"
                    value={ruleVersionId}
                    onChange={(event) => setRuleVersionId(event.target.value)}
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
                    />
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Compare lane</CardTitle>
              <CardDescription>
                Primary scoring remains rule-based. Shadow compare stays opt-in and catalog-backed.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-xl border bg-muted/20 p-4">
                  <div className="text-sm text-muted-foreground">Primary lane</div>
                  <div className="mt-2 font-medium">Rule-based scorer</div>
                  <div className="mt-2 text-sm text-muted-foreground">
                    Live operator baseline for replay and explanation output.
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="risk-lab-shadow-compare">Shadow compare lane</Label>
                  <select
                    id="risk-lab-shadow-compare"
                    aria-label="Shadow compare lane"
                    className="w-full rounded-md border bg-background px-3 py-2 text-sm"
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
                  <div className="rounded-md border bg-muted/20 px-3 py-2 text-sm text-muted-foreground">
                    Safe fallback: {selectedShadowEntry?.safeFallback ?? "Not available"}
                  </div>
                </div>
              </div>

              <div className="flex flex-wrap items-center gap-3">
                <Button
                  onClick={handleRunReplay}
                  disabled={catalogLoading || !!catalogError || replayLoading}
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
                  <Sparkles className="h-4 w-4" />
                  Explainability stays attached to the same replay request.
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        <div className="space-y-6">
          {catalogLoading ? (
            <Card>
              <CardHeader>
                <CardTitle>Loading compare surface</CardTitle>
                <CardDescription>Loading risk lab catalog...</CardDescription>
              </CardHeader>
              <CardContent className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                Waiting for the backend-published scorer catalog.
              </CardContent>
            </Card>
          ) : catalogError ? (
            <Card className="border-destructive/40">
              <CardHeader>
                <CardTitle>Catalog unavailable</CardTitle>
                <CardDescription>
                  The workbench stays bounded until the catalog can be fetched again.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                  {catalogError}
                </div>
                <div className="rounded-lg border border-dashed px-4 py-3 text-sm text-muted-foreground">
                  Catalog data defines the bounded compare surface exposed by the backend.
                </div>
                <Button variant="outline" onClick={() => void loadCatalog()}>
                  Retry catalog
                </Button>
              </CardContent>
            </Card>
          ) : replayLoading ? (
            <Card>
              <CardHeader>
                <CardTitle>Replay in progress</CardTitle>
                <CardDescription>
                  Preparing compare and explanation surfaces for the selected replay.
                </CardDescription>
              </CardHeader>
              <CardContent className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                Waiting for replay scoring, challenger comparison, and graph assembly.
              </CardContent>
            </Card>
          ) : replayError ? (
            <Card className="border-destructive/40">
              <CardHeader>
                <CardTitle>Replay request failed</CardTitle>
                <CardDescription>
                  The compare lane remains intact so the operator can adjust and rerun.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                  {replayError}
                </div>
                <div className="rounded-lg border border-dashed px-4 py-3 text-sm text-muted-foreground">
                  Adjust the feature snapshot or compare lane and rerun.
                </div>
              </CardContent>
            </Card>
          ) : replayResult ? (
            <div className="space-y-6">
              <Card>
                <CardHeader>
                  <CardTitle>{replayResult.replayId}</CardTitle>
                  <CardDescription>
                    Primary and challenger outcomes stay side-by-side for operator review.
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="grid gap-4 md:grid-cols-3">
                    <div className="rounded-xl border bg-muted/20 p-4">
                      <div className="text-sm text-muted-foreground">Primary decision</div>
                      <div className="mt-2 flex items-center gap-2">
                        <span className="text-2xl font-semibold">
                          {formatDecision(replayResult.primaryDecision.decision)}
                        </span>
                        <Badge
                          variant={decisionVariant(replayResult.primaryDecision.decision)}
                          shape="pill"
                        >
                          {replayResult.primaryScore.riskScore.score}
                        </Badge>
                      </div>
                    </div>
                    <div className="rounded-xl border bg-muted/20 p-4">
                      <div className="text-sm text-muted-foreground">Challenger decision</div>
                      <div className="mt-2 flex items-center gap-2">
                        <span className="text-2xl font-semibold">
                          {formatDecision(replayResult.challengerDecision?.decision)}
                        </span>
                        {replayResult.challengerScore ? (
                          <Badge
                            variant={decisionVariant(replayResult.challengerDecision?.decision)}
                            shape="pill"
                          >
                            {replayResult.challengerScore.riskScore.score}
                          </Badge>
                        ) : null}
                      </div>
                    </div>
                    <div className="rounded-xl border bg-muted/20 p-4">
                      <div className="text-sm text-muted-foreground">Score delta</div>
                      <div className="mt-2 text-2xl font-semibold">
                        {formatDelta(replayResult.scoreDelta)}
                      </div>
                    </div>
                  </div>

                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="rounded-xl border p-4">
                      <div className="mb-3 flex items-center gap-2">
                        <BrainCircuit className="h-4 w-4 text-muted-foreground" />
                        <span className="font-medium">Primary factors</span>
                      </div>
                      <div className="space-y-3">
                        {replayResult.primaryScore.metadata.topRiskFactors.map((factor) => (
                          <div key={factor.ruleName} className="rounded-lg border bg-muted/20 p-3">
                            <div className="flex items-center justify-between gap-3">
                              <div className="font-medium">{factor.ruleName}</div>
                              <Badge variant="outline" shape="pill">
                                +{factor.contribution}
                              </Badge>
                            </div>
                            <p className="mt-2 text-sm text-muted-foreground">
                              {factor.description}
                            </p>
                          </div>
                        ))}
                      </div>
                    </div>

                    <div className="rounded-xl border p-4">
                      <div className="mb-3 flex items-center gap-2">
                        <AlertTriangle className="h-4 w-4 text-muted-foreground" />
                        <span className="font-medium">Challenger factors</span>
                      </div>
                      <div className="space-y-3">
                        {(replayResult.challengerScore?.metadata.topRiskFactors ?? []).length > 0 ? (
                          replayResult.challengerScore?.metadata.topRiskFactors.map((factor) => (
                            <div key={factor.ruleName} className="rounded-lg border bg-muted/20 p-3">
                              <div className="flex items-center justify-between gap-3">
                                <div className="font-medium">{factor.ruleName}</div>
                                <Badge variant="outline" shape="pill">
                                  +{factor.contribution}
                                </Badge>
                              </div>
                              <p className="mt-2 text-sm text-muted-foreground">
                                {factor.description}
                              </p>
                            </div>
                          ))
                        ) : (
                          <div className="rounded-lg border border-dashed px-4 py-3 text-sm text-muted-foreground">
                            Challenger compare is disabled for this replay.
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
              <div className="grid gap-4 md:grid-cols-2">
                <Card>
                  <CardHeader className="pb-2">
                    <CardDescription>Graph nodes</CardDescription>
                    <CardTitle>{replayResult.graph.nodes.length}</CardTitle>
                  </CardHeader>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <CardDescription>Graph edges</CardDescription>
                    <CardTitle>{replayResult.graph.edges.length}</CardTitle>
                  </CardHeader>
                </Card>
              </div>

              <Card>
                <CardHeader>
                  <CardTitle>Explainability graph</CardTitle>
                  <CardDescription>
                    Replay, decision, and factor nodes remain inspectable without leaving the admin
                    workbench.
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="grid gap-3">
                    {replayResult.graph.nodes.map((node) => (
                      <div
                        key={node.id}
                        className="flex items-center justify-between gap-3 rounded-lg border bg-muted/20 px-4 py-3"
                      >
                        <div>
                          <div className="font-medium">
                            {node.kind === "REPLAY"
                              ? "Replay root"
                              : node.kind === "RULE_FACTOR"
                                ? `Factor: ${node.label}`
                                : node.label}
                          </div>
                          <div className="text-sm text-muted-foreground">{node.kind}</div>
                        </div>
                        <div className="text-sm text-muted-foreground">
                          {node.weight === null ? "No weight" : `Weight ${node.weight}`}
                        </div>
                      </div>
                    ))}
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Feature snapshot</CardTitle>
                  <CardDescription>
                    The snapshot below mirrors the replay request that produced the visible outcome.
                  </CardDescription>
                </CardHeader>
                <CardContent className="grid gap-3 md:grid-cols-2">
                  {Object.entries(replayResult.primaryScore.metadata.featureSnapshot).map(
                    ([key, value]) => (
                      <div
                        key={key}
                        className="rounded-lg border bg-muted/20 px-4 py-3 text-sm"
                      >
                        <div className="text-muted-foreground">{key}</div>
                        <div className="mt-1 font-medium">{formatFeatureValue(value)}</div>
                      </div>
                    ),
                  )}
                </CardContent>
              </Card>
            </div>
          ) : (
            <Card className="border-dashed">
              <CardHeader>
                <CardTitle>Awaiting replay</CardTitle>
                <CardDescription>
                  Load a replay to compare the primary scorer against the shadow lane, inspect the
                  decision delta, and keep explainability attached to the same payload.
                </CardDescription>
              </CardHeader>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
