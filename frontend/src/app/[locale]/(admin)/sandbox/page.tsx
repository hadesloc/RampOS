"use client";

import { useEffect, useMemo, useState } from "react";
import { Loader2, PlayCircle, RefreshCw, RotateCcw, Rocket } from "lucide-react";

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

type SandboxPreset = {
  code: string;
  name: string;
  seedPackageVersion: string;
  defaultScenarios: string[];
  metadata: Record<string, unknown>;
  resetStrategy: string;
  resetSemantics: Record<string, unknown>;
};

type SandboxSeedResponse = {
  tenantId: string;
  tenantName: string;
  tenantStatus: string;
  presetCode: string;
  scenarioCode?: string | null;
  createdAt: string;
};

type SandboxReplayEntry = {
  sequence: number;
  source: string;
  referenceId: string;
  occurredAt: string;
  label: string;
  status: string;
  payload: Record<string, unknown>;
};

type SandboxReplayBundle = {
  journeyId: string;
  generatedAt: string;
  redactionApplied: boolean;
  entries: SandboxReplayEntry[];
};

type SandboxReplayExport = {
  format: string;
  fileName: string;
  contentType: string;
  redactionApplied: boolean;
  bundle: Record<string, unknown>;
};

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
      // Keep the default message when the response body is not JSON.
    }
    throw new Error(message);
  }

  return response.json() as Promise<T>;
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

export default function SandboxPage() {
  const [presets, setPresets] = useState<SandboxPreset[]>([]);
  const [loadingPresets, setLoadingPresets] = useState(true);
  const [presetError, setPresetError] = useState<string | null>(null);

  const [selectedPresetCode, setSelectedPresetCode] = useState("");
  const [tenantName, setTenantName] = useState("Sandbox Tenant");
  const [scenarioCode, setScenarioCode] = useState("");

  const [seeding, setSeeding] = useState(false);
  const [seedResult, setSeedResult] = useState<SandboxSeedResponse | null>(null);
  const [seedError, setSeedError] = useState<string | null>(null);

  const [resetState, setResetState] = useState<string | null>(null);

  const [journeyId, setJourneyId] = useState("intent_sandbox_001");
  const [loadingReplay, setLoadingReplay] = useState(false);
  const [replayBundle, setReplayBundle] = useState<SandboxReplayBundle | null>(null);
  const [replayExport, setReplayExport] = useState<SandboxReplayExport | null>(null);
  const [replayError, setReplayError] = useState<string | null>(null);

  const selectedPreset = useMemo(
    () => presets.find((preset) => preset.code === selectedPresetCode) ?? null,
    [presets, selectedPresetCode],
  );

  useEffect(() => {
    const loadPresets = async () => {
      setLoadingPresets(true);
      setPresetError(null);

      try {
        const data = await apiRequest<SandboxPreset[]>("/v1/admin/sandbox");
        setPresets(data);
        if (data.length > 0) {
          setSelectedPresetCode((current) => current || data[0].code);
          setScenarioCode((current) => current || data[0].defaultScenarios[0] || "");
        }
      } catch (error) {
        setPresetError(error instanceof Error ? error.message : "Failed to load presets");
      } finally {
        setLoadingPresets(false);
      }
    };

    void loadPresets();
  }, []);

  useEffect(() => {
    if (!selectedPreset) return;
    if (!selectedPreset.defaultScenarios.includes(scenarioCode)) {
      setScenarioCode(selectedPreset.defaultScenarios[0] || "");
    }
  }, [scenarioCode, selectedPreset]);

  const handleSeedTenant = async () => {
    if (!selectedPreset) return;

    setSeeding(true);
    setSeedError(null);

    try {
      const result = await apiRequest<SandboxSeedResponse>("/v1/admin/sandbox/seed", {
        method: "POST",
        body: JSON.stringify({
          tenantName,
          presetCode: selectedPreset.code,
          scenarioCode: scenarioCode || undefined,
          configOverrides: {},
        }),
      });
      setSeedResult(result);
      setJourneyId(result.tenantId);
    } catch (error) {
      setSeedError(error instanceof Error ? error.message : "Failed to seed tenant");
    } finally {
      setSeeding(false);
    }
  };

  const handleReset = async () => {
    if (!seedResult) {
      setResetState("Seed a sandbox tenant first, then request a bounded reset.");
      return;
    }

    try {
      await apiRequest("/v1/admin/sandbox/reset", {
        method: "POST",
        body: JSON.stringify({
          tenantId: seedResult.tenantId,
          presetCode: seedResult.presetCode,
          reason: "Admin requested sandbox reset",
        }),
      });
      setResetState("Sandbox reset accepted.");
    } catch (error) {
      setResetState(error instanceof Error ? error.message : "Sandbox reset is unavailable.");
    }
  };

  const handleLoadReplay = async () => {
    setLoadingReplay(true);
    setReplayError(null);

    try {
      const bundle = await apiRequest<SandboxReplayBundle>(
        `/v1/admin/sandbox/replay/${encodeURIComponent(journeyId)}`,
      );
      setReplayBundle(bundle);
    } catch (error) {
      setReplayError(error instanceof Error ? error.message : "Failed to load replay bundle");
    } finally {
      setLoadingReplay(false);
    }
  };

  const handleExportReplay = async () => {
    setLoadingReplay(true);
    setReplayError(null);

    try {
      const exportPayload = await apiRequest<SandboxReplayExport>(
        `/v1/admin/sandbox/replay/${encodeURIComponent(journeyId)}/export`,
      );
      setReplayExport(exportPayload);
    } catch (error) {
      setReplayError(error instanceof Error ? error.message : "Failed to export replay bundle");
    } finally {
      setLoadingReplay(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Sandbox Control Room</h1>
          <p className="text-muted-foreground">
            Seed bounded sandbox tenants, inspect replay bundles, and prepare operator drills.
          </p>
        </div>
        <Button
          variant="outline"
          size="icon"
          onClick={() => window.location.reload()}
          aria-label="Refresh sandbox page"
        >
          <RefreshCw className="h-4 w-4" />
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Preset catalog</CardDescription>
            <CardTitle>{presets.length}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Seeded tenant</CardDescription>
            <CardTitle>{seedResult?.tenantId ?? "Not seeded"}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Replay status</CardDescription>
            <CardTitle>{replayBundle?.entries[0]?.status ?? "Awaiting replay load"}</CardTitle>
          </CardHeader>
        </Card>
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.1fr,0.9fr]">
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Preset selection</CardTitle>
              <CardDescription>
                Pick a sandbox preset and keep the scenario scope bounded to the currently exposed
                contract.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {loadingPresets ? (
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Loading sandbox presets...
                </div>
              ) : presetError ? (
                <p className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                  {presetError}
                </p>
              ) : (
                <>
                  <div className="space-y-2">
                    <Label htmlFor="preset-code">Preset</Label>
                    <select
                      id="preset-code"
                      className="w-full rounded-md border bg-background px-3 py-2 text-sm"
                      value={selectedPresetCode}
                      onChange={(event) => setSelectedPresetCode(event.target.value)}
                    >
                      {presets.map((preset) => (
                        <option key={preset.code} value={preset.code}>
                          {preset.name}
                        </option>
                      ))}
                    </select>
                  </div>

                  {selectedPreset && (
                    <div className="rounded-lg border bg-muted/30 p-4 text-sm">
                      <div className="font-medium">{selectedPreset.name}</div>
                      <div className="mt-2 text-muted-foreground">
                        Package {selectedPreset.seedPackageVersion} - {selectedPreset.resetStrategy}
                      </div>
                      <div className="mt-3 flex flex-wrap gap-2">
                        {selectedPreset.defaultScenarios.map((scenario) => (
                          <span
                            key={scenario}
                            className="rounded-full border px-2 py-1 text-xs font-medium"
                          >
                            {scenario}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                </>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Seed tenant</CardTitle>
              <CardDescription>
                This path is live. Reset workflow stays bounded until a later W1 slice lands.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="tenant-name">Tenant name</Label>
                <Input
                  id="tenant-name"
                  value={tenantName}
                  onChange={(event) => setTenantName(event.target.value)}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="scenario-code">Scenario</Label>
                <select
                  id="scenario-code"
                  className="w-full rounded-md border bg-background px-3 py-2 text-sm"
                  value={scenarioCode}
                  onChange={(event) => setScenarioCode(event.target.value)}
                  disabled={!selectedPreset || selectedPreset.defaultScenarios.length === 0}
                >
                  {(selectedPreset?.defaultScenarios ?? []).map((scenario) => (
                    <option key={scenario} value={scenario}>
                      {scenario}
                    </option>
                  ))}
                </select>
              </div>

              <div className="flex flex-wrap gap-3">
                <Button onClick={handleSeedTenant} disabled={!selectedPreset || seeding}>
                  {seeding ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Seeding...
                    </>
                  ) : (
                    <>
                      <Rocket className="mr-2 h-4 w-4" />
                      Seed tenant
                    </>
                  )}
                </Button>
                <Button variant="outline" onClick={handleReset}>
                  <RotateCcw className="mr-2 h-4 w-4" />
                  Reset tenant
                </Button>
              </div>

              {seedError && (
                <p className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                  {seedError}
                </p>
              )}

              {resetState && (
                <p className="rounded-md border border-muted px-3 py-2 text-sm text-muted-foreground">
                  {resetState}
                </p>
              )}

              {seedResult && (
                <div className="rounded-lg border bg-card p-4 text-sm">
                  <div className="font-medium">{seedResult.tenantName}</div>
                  <div className="mt-2 grid gap-2 md:grid-cols-2">
                    <div>
                      <span className="text-muted-foreground">Tenant ID</span>
                      <div>{seedResult.tenantId}</div>
                    </div>
                    <div>
                      <span className="text-muted-foreground">Created</span>
                      <div>{formatTimestamp(seedResult.createdAt)}</div>
                    </div>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Scenario execution</CardTitle>
              <CardDescription>
                Scenario execution will land in a later W1 slice; keep operators on seed and replay
                until then.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Button variant="outline" disabled>
                <PlayCircle className="mr-2 h-4 w-4" />
                Run scenario
              </Button>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Replay launch</CardTitle>
              <CardDescription>
                Load or export the redacted replay bundle contract that the backend exposes today.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="journey-id">Journey ID</Label>
                <Input
                  id="journey-id"
                  value={journeyId}
                  onChange={(event) => setJourneyId(event.target.value)}
                />
              </div>

              <div className="flex flex-wrap gap-3">
                <Button onClick={handleLoadReplay} disabled={!journeyId || loadingReplay}>
                  {loadingReplay ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Loading...
                    </>
                  ) : (
                    "Load replay"
                  )}
                </Button>
                <Button variant="outline" onClick={handleExportReplay} disabled={!journeyId || loadingReplay}>
                  Export replay
                </Button>
              </div>

              {replayError && (
                <p className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                  {replayError}
                </p>
              )}

              {replayBundle && (
                <div className="space-y-3 rounded-lg border p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="font-medium">{replayBundle.journeyId}</div>
                      <div className="text-sm text-muted-foreground">
                        Generated {formatTimestamp(replayBundle.generatedAt)}
                      </div>
                    </div>
                    <span className="rounded-full border px-2 py-1 text-xs font-medium">
                      {replayBundle.redactionApplied ? "Redaction applied" : "Raw payload"}
                    </span>
                  </div>

                  {replayBundle.entries.map((entry) => (
                    <div key={`${entry.referenceId}-${entry.sequence}`} className="rounded-md border bg-muted/20 p-3">
                      <div className="flex items-center justify-between gap-3">
                        <div className="font-medium">{entry.label}</div>
                        <div className="text-xs uppercase text-muted-foreground">{entry.status}</div>
                      </div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        {entry.source} - {entry.referenceId} - {formatTimestamp(entry.occurredAt)}
                      </div>
                      <pre className="mt-3 overflow-x-auto rounded-md bg-slate-950 p-3 text-xs text-slate-100">
                        {JSON.stringify(entry.payload, null, 2)}
                      </pre>
                    </div>
                  ))}
                </div>
              )}

              {replayExport && (
                <div className="rounded-md border bg-muted/20 p-3 text-sm">
                  <div className="font-medium">{replayExport.fileName}</div>
                  <div className="text-muted-foreground">
                    {replayExport.contentType} - {replayExport.redactionApplied ? "redacted" : "raw"}
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
