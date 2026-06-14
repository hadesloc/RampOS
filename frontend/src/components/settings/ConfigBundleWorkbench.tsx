"use client";

import { useEffect, useState } from "react";
import { CheckCircle2, Download, FileArchive, GitBranch, Layers3, ShieldCheck } from "lucide-react";

import {
  CardGridSkeleton,
  EmptyState,
  ErrorState,
  PageHeader,
  Panel,
  StatCard,
  StatGrid,
  StatusBadge,
} from "@/components/shared";
import { capitalize, formatNumber, toLabel } from "@/lib/format";

type BundleResponse = {
  bundle: {
    bundleId: string;
    tenantName: string;
    actionMode: string;
    sections: string[];
    approvalStatus?: string;
    source?: string;
    rolloutScope?: Record<string, unknown>;
  };
};

function stringifyScopeValue(value: unknown) {
  if (value === null || value === undefined) return "n/a";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}

export default function ConfigBundleWorkbench() {
  const [data, setData] = useState<BundleResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch("/api/proxy/v1/admin/config-bundles/export");
      if (!response.ok) {
        throw new Error("Failed to load config bundle.");
      }
      setData(await response.json());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load config bundle.");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  const bundle = data?.bundle;
  const sections = bundle?.sections ?? [];
  const rolloutEntries = bundle?.rolloutScope ? Object.entries(bundle.rolloutScope) : [];

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Config Bundles"
        description="Export approved tenant configuration sections and review import-safe bundle contents."
      />

      {error ? (
        <ErrorState message={error} retry={load} />
      ) : loading ? (
        <div className="space-y-6">
          <CardGridSkeleton cards={4} />
          <Panel
            variant="glass"
            header={{
              title: "Bundle Manifest",
              description: "Loading tenant configuration bundle metadata.",
            }}
          >
            <CardGridSkeleton cards={3} className="lg:grid-cols-3" />
          </Panel>
        </div>
      ) : !bundle ? (
        <EmptyState
          title="No config bundle available"
          description="No approved tenant configuration bundle could be resolved for export."
        />
      ) : (
        <div className="space-y-6">
          <StatGrid cols={4}>
            <StatCard
              title="Bundle ID"
              value={bundle.bundleId}
              subtitle="Export manifest"
              icon={<FileArchive className="h-4 w-4" />}
              accentColor="green"
            />
            <StatCard
              title="Tenant"
              value={bundle.tenantName}
              subtitle="Server-resolved scope"
              icon={<ShieldCheck className="h-4 w-4" />}
              accentColor="cyan"
            />
            <StatCard
              title="Sections"
              value={formatNumber(sections.length)}
              subtitle="Approved config surfaces"
              icon={<Layers3 className="h-4 w-4" />}
              accentColor="violet"
            />
            <StatCard
              title="Action Mode"
              value={toLabel(bundle.actionMode)}
              subtitle="Import guardrail"
              icon={<GitBranch className="h-4 w-4" />}
              accentColor="amber"
            />
          </StatGrid>

          <Panel
            variant="glass"
            className="overflow-hidden"
            header={{
              title: "Bundle Manifest",
              description: "A read-only command surface for the exact configuration payload approved for export.",
              actions: (
                <StatusBadge
                  status={bundle.approvalStatus ? toLabel(bundle.approvalStatus) : "Approval n/a"}
                  severity={bundle.approvalStatus ? undefined : "neutral"}
                />
              ),
            }}
          >
            <div className="grid gap-4 lg:grid-cols-[1.05fr_0.95fr]">
              <div className="relative overflow-hidden rounded-xl border border-[#00FF87]/15 bg-[#09090B] p-5 shadow-[0_0_40px_rgba(0,255,135,0.05)]">
                <div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-[#00FF87]/70 to-transparent" />
                <div className="mb-5 flex items-start justify-between gap-4">
                  <div>
                    <p className="text-xs font-semibold uppercase tracking-[0.28em] text-[#00FF87]">Export Packet</p>
                    <h2 className="mt-2 font-mono text-lg font-semibold text-foreground break-all">{bundle.bundleId}</h2>
                  </div>
                  <div className="rounded-lg border border-[#00D4FF]/20 bg-[#00D4FF]/10 p-2 text-[#00D4FF]">
                    <Download className="h-5 w-5" />
                  </div>
                </div>

                <dl className="grid gap-3 sm:grid-cols-2">
                  <div className="rounded-lg border border-white/[0.06] bg-white/[0.025] p-3">
                    <dt className="text-xs text-muted-foreground">Tenant</dt>
                    <dd className="mt-1 text-sm font-medium text-foreground">{bundle.tenantName}</dd>
                  </div>
                  <div className="rounded-lg border border-white/[0.06] bg-white/[0.025] p-3">
                    <dt className="text-xs text-muted-foreground">Source</dt>
                    <dd className="mt-1 text-sm font-medium text-foreground">{bundle.source ?? "unknown"}</dd>
                  </div>
                  <div className="rounded-lg border border-white/[0.06] bg-white/[0.025] p-3">
                    <dt className="text-xs text-muted-foreground">Mode</dt>
                    <dd className="mt-1 text-sm font-medium text-foreground">{toLabel(bundle.actionMode)}</dd>
                  </div>
                  <div className="rounded-lg border border-white/[0.06] bg-white/[0.025] p-3">
                    <dt className="text-xs text-muted-foreground">Approval</dt>
                    <dd className="mt-1 text-sm font-medium text-foreground">
                      {bundle.approvalStatus ? capitalize(toLabel(bundle.approvalStatus)) : "n/a"}
                    </dd>
                  </div>
                </dl>
              </div>

              <div className="rounded-xl border border-white/[0.06] bg-[#09090B] p-5">
                <div className="mb-4 flex items-center justify-between gap-3">
                  <div>
                    <p className="text-sm font-semibold text-foreground">Configuration Sections</p>
                    <p className="text-xs text-muted-foreground">Only server-approved sections are listed.</p>
                  </div>
                  <StatusBadge status={`${formatNumber(sections.length)} sections`} severity="info" dot={false} />
                </div>

                {sections.length > 0 ? (
                  <div className="grid gap-2">
                    {sections.map((section) => (
                      <div
                        key={section}
                        className="flex items-center justify-between rounded-lg border border-white/[0.06] bg-white/[0.025] px-3 py-2"
                      >
                        <span className="font-mono text-sm text-foreground">{section}</span>
                        <CheckCircle2 className="h-4 w-4 text-[#00FF87]" />
                      </div>
                    ))}
                  </div>
                ) : (
                  <EmptyState
                    title="No sections in bundle"
                    description="This export does not include any approved configuration sections."
                  />
                )}
              </div>
            </div>
          </Panel>

          <Panel
            header={{
              title: "Rollout Scope",
              description: "Scope metadata returned by the server. Empty scope means no rollout metadata was supplied.",
            }}
          >
            {rolloutEntries.length > 0 ? (
              <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
                {rolloutEntries.map(([key, value]) => (
                  <div key={key} className="rounded-lg border border-white/[0.06] bg-[#09090B] p-4">
                    <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">{toLabel(key)}</p>
                    <p className="mt-2 break-words font-mono text-sm text-foreground">{stringifyScopeValue(value)}</p>
                  </div>
                ))}
              </div>
            ) : (
              <EmptyState
                title="No rollout scope supplied"
                description="The bundle response did not include rollout scope metadata."
              />
            )}
          </Panel>
        </div>
      )}
    </main>
  );
}
