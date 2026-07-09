"use client";

import { useEffect, useMemo, useState } from "react";
import { Loader2, PlayCircle, RefreshCw, RotateCcw, Rocket, Database, Tag, Activity } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  SectionCard,
  EmptyState,
  ErrorState,
} from "@/components/shared";
import { formatDateTime } from "@/lib/format";

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
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  if (!response.ok) {
    let message = "Request failed";
    try {
      const payload = (await response.json()) as { message?: string };
      message = payload.message ?? message;
    } catch {
      // keep default
    }
    throw new Error(message);
  }
  return response.json() as Promise<T>;
}

const selectCls =
  "w-full rounded-md border border-white/[0.08] bg-[#09090B] px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-[#00FF87]/40 transition-colors";

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
    () => presets.find((p) => p.code === selectedPresetCode) ?? null,
    [presets, selectedPresetCode]
  );

  useEffect(() => {
    const loadPresets = async () => {
      setLoadingPresets(true);
      setPresetError(null);
      try {
        const data = await apiRequest<SandboxPreset[]>("/v1/admin/sandbox");
        setPresets(data);
        if (data.length > 0) {
          setSelectedPresetCode((c) => c || data[0].code);
          setScenarioCode((c) => c || data[0].defaultScenarios[0] || "");
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
        `/v1/admin/sandbox/replay/${encodeURIComponent(journeyId)}`
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
        `/v1/admin/sandbox/replay/${encodeURIComponent(journeyId)}/export`
      );
      setReplayExport(exportPayload);
    } catch (error) {
      setReplayError(error instanceof Error ? error.message : "Failed to export replay bundle");
    } finally {
      setLoadingReplay(false);
    }
  };

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Sandbox Control Room"
        description="Seed bounded sandbox tenants, inspect replay bundles, and prepare operator drills."
        actions={
          <Button variant="outline" size="icon" onClick={() => window.location.reload()}>
            <RefreshCw className="h-4 w-4" />
          </Button>
        }
      />

      <StatGrid cols={3}>
        <StatCard
          title="Preset Catalog"
          value={loadingPresets ? "…" : presets.length}
          icon={<Database className="h-4 w-4" />}
          accentColor="cyan"
          loading={loadingPresets}
        />
        <StatCard
          title="Seeded Tenant"
          value={seedResult?.tenantId ?? "Not seeded"}
          icon={<Tag className="h-4 w-4" />}
          accentColor="violet"
        />
        <StatCard
          title="Replay Status"
          value={replayBundle?.entries[0]?.status ?? "Awaiting"}
          icon={<Activity className="h-4 w-4" />}
          accentColor="green"
        />
      </StatGrid>

      <div className="grid gap-section xl:grid-cols-[1.1fr,0.9fr]">
        {/* Left column */}
        <div className="flex flex-col gap-section">
          {/* Preset Selection */}
          <Panel
            header={{
              title: "Preset Selection",
              description: "Pick a sandbox preset and keep the scenario scope bounded to the current contract.",
            }}
            variant="glass"
          >
            <div className="space-y-4">
              {loadingPresets ? (
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Loading sandbox presets...
                </div>
              ) : presetError ? (
                <ErrorState message={presetError} retry={() => window.location.reload()} />
              ) : presets.length === 0 ? (
                <EmptyState title="No presets" description="No sandbox presets are available." />
              ) : (
                <>
                  <div className="space-y-2">
                    <Label htmlFor="preset-code">Preset</Label>
                    <select
                      id="preset-code"
                      className={selectCls}
                      value={selectedPresetCode}
                      onChange={(e) => setSelectedPresetCode(e.target.value)}
                    >
                      {presets.map((preset) => (
                        <option key={preset.code} value={preset.code}>
                          {preset.name}
                        </option>
                      ))}
                    </select>
                  </div>

                  {selectedPreset && (
                    <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-4 text-sm">
                      <div className="font-medium">{selectedPreset.name}</div>
                      <div className="mt-1 text-muted-foreground text-xs">
                        Package {selectedPreset.seedPackageVersion} — {selectedPreset.resetStrategy}
                      </div>
                      <div className="mt-3 flex flex-wrap gap-2">
                        {selectedPreset.defaultScenarios.map((scenario) => (
                          <span
                            key={scenario}
                            className="rounded-full border border-white/[0.08] px-2 py-0.5 text-xs font-medium text-muted-foreground"
                          >
                            {scenario}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>
          </Panel>

          {/* Seed Tenant */}
          <Panel
            header={{
              title: "Seed Tenant",
              description: "This path is live. Reset workflow stays bounded until a later slice lands.",
            }}
            variant="glass"
          >
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="tenant-name">Tenant name</Label>
                <Input
                  id="tenant-name"
                  value={tenantName}
                  onChange={(e) => setTenantName(e.target.value)}
                  className="border-white/[0.08] bg-[#09090B]"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="scenario-code">Scenario</Label>
                <select
                  id="scenario-code"
                  className={selectCls}
                  value={scenarioCode}
                  onChange={(e) => setScenarioCode(e.target.value)}
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
                <p className="rounded-md border border-red-500/20 bg-red-500/10 px-3 py-2 text-sm text-red-400">
                  {seedError}
                </p>
              )}

              {resetState && (
                <p className="rounded-md border border-white/[0.06] px-3 py-2 text-sm text-muted-foreground">
                  {resetState}
                </p>
              )}

              {seedResult && (
                <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-4 text-sm">
                  <div className="font-medium text-[#00FF87]">{seedResult.tenantName}</div>
                  <div className="mt-2 grid gap-2 sm:grid-cols-2 text-xs">
                    <div>
                      <span className="text-muted-foreground">Tenant ID</span>
                      <div className="font-mono mt-0.5">{seedResult.tenantId}</div>
                    </div>
                    <div>
                      <span className="text-muted-foreground">Created</span>
                      <div className="mt-0.5 tabular-nums">{formatDateTime(seedResult.createdAt)}</div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </Panel>
        </div>

        {/* Right column */}
        <div className="flex flex-col gap-section">
          {/* Scenario Execution */}
          <Panel
            header={{
              title: "Scenario Execution",
              description: "Scenario execution lands in a later slice — keep operators on seed and replay until then.",
            }}
            variant="glass"
          >
            <div>
              <Button variant="outline" disabled>
                <PlayCircle className="mr-2 h-4 w-4" />
                Run scenario
              </Button>
            </div>
          </Panel>

          {/* Replay Launch */}
          <Panel
            header={{
              title: "Replay Launch",
              description: "Load or export the redacted replay bundle the backend exposes today.",
            }}
            variant="glass"
          >
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="journey-id">Journey ID</Label>
                <Input
                  id="journey-id"
                  value={journeyId}
                  onChange={(e) => setJourneyId(e.target.value)}
                  className="border-white/[0.08] bg-[#09090B] font-mono"
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
                <p className="rounded-md border border-red-500/20 bg-red-500/10 px-3 py-2 text-sm text-red-400">
                  {replayError}
                </p>
              )}

              {replayBundle && (
                <div className="space-y-3 rounded-lg border border-white/[0.06] p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="font-medium text-sm font-mono">{replayBundle.journeyId}</div>
                      <div className="text-xs text-muted-foreground tabular-nums mt-0.5">
                        Generated {formatDateTime(replayBundle.generatedAt)}
                      </div>
                    </div>
                    <span className="rounded-full border border-white/[0.08] px-2 py-0.5 text-xs font-medium text-muted-foreground whitespace-nowrap">
                      {replayBundle.redactionApplied ? "Redacted" : "Raw payload"}
                    </span>
                  </div>

                  {replayBundle.entries.map((entry) => (
                    <div
                      key={`${entry.referenceId}-${entry.sequence}`}
                      className="rounded-md border border-white/[0.06] bg-white/[0.02] p-3"
                    >
                      <div className="flex items-center justify-between gap-3">
                        <div className="font-medium text-sm">{entry.label}</div>
                        <div className="text-xs uppercase text-muted-foreground">{entry.status}</div>
                      </div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        {entry.source} — {entry.referenceId} — {formatDateTime(entry.occurredAt)}
                      </div>
                      <pre className="mt-3 overflow-x-auto rounded-md bg-black/60 p-3 text-xs text-slate-300 border border-white/[0.04]">
                        {JSON.stringify(entry.payload, null, 2)}
                      </pre>
                    </div>
                  ))}
                </div>
              )}

              {replayExport && (
                <div className="rounded-md border border-white/[0.06] bg-white/[0.02] p-3 text-sm">
                  <div className="font-medium font-mono">{replayExport.fileName}</div>
                  <div className="text-xs text-muted-foreground mt-0.5">
                    {replayExport.contentType} — {replayExport.redactionApplied ? "redacted" : "raw"}
                  </div>
                </div>
              )}
            </div>
          </Panel>
        </div>
      </div>
    </main>
  );
}
