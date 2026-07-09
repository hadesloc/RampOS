"use client";

import { useEffect, useState } from "react";
import { CheckCircle2, LockKeyhole, PlugZap, Power, Puzzle, ShieldAlert, SlidersHorizontal } from "lucide-react";

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
import { formatNumber, toLabel } from "@/lib/format";

type ExtensionsResponse = {
  actionMode: string;
  actions: Array<{
    actionId: string;
    label: string;
    description: string;
    enabled: boolean;
    approvalRequired?: boolean;
    source?: string;
    rolloutScope?: Record<string, unknown>;
  }>;
};

function stringifyScopeValue(value: unknown) {
  if (value === null || value === undefined) return "n/a";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}

export default function ExtensionsRegistry() {
  const [data, setData] = useState<ExtensionsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch("/api/proxy/v1/admin/extensions");
      if (!response.ok) {
        throw new Error("Failed to load extension registry.");
      }
      setData(await response.json());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load extension registry.");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  const actions = data?.actions ?? [];
  const enabledCount = actions.filter((action) => action.enabled).length;
  const approvalRequiredCount = actions.filter((action) => action.approvalRequired === true).length;
  const unknownApprovalCount = actions.filter((action) => action.approvalRequired === undefined).length;

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Extensions Registry"
        description="Review governed extension actions only. This remains a whitelisted control surface, not a plugin runtime."
      />

      {error ? (
        <ErrorState message={error} retry={load} />
      ) : loading ? (
        <div className="space-y-6">
          <CardGridSkeleton cards={4} />
          <Panel
            variant="glass"
            header={{
              title: "Governed Actions",
              description: "Loading whitelisted extension action metadata.",
            }}
          >
            <CardGridSkeleton cards={4} className="lg:grid-cols-2" />
          </Panel>
        </div>
      ) : !data ? (
        <EmptyState
          title="No extension registry available"
          description="No governed extension action registry could be resolved."
        />
      ) : (
        <div className="space-y-6">
          <StatGrid cols={4}>
            <StatCard
              title="Registered Actions"
              value={formatNumber(actions.length)}
              subtitle="Whitelisted controls"
              icon={<Puzzle className="h-4 w-4" />}
              accentColor="green"
            />
            <StatCard
              title="Enabled"
              value={formatNumber(enabledCount)}
              subtitle="Currently callable"
              icon={<Power className="h-4 w-4" />}
              accentColor="cyan"
            />
            <StatCard
              title="Approval Required"
              value={formatNumber(approvalRequiredCount)}
              subtitle="Guarded actions"
              icon={<LockKeyhole className="h-4 w-4" />}
              accentColor="violet"
            />
            <StatCard
              title="Action Mode"
              value={toLabel(data.actionMode)}
              subtitle={unknownApprovalCount ? `${formatNumber(unknownApprovalCount)} unknown approval flags` : "Governance mode"}
              icon={<SlidersHorizontal className="h-4 w-4" />}
              accentColor="amber"
            />
          </StatGrid>

          <Panel
            variant="glass"
            className="overflow-hidden"
            header={{
              title: "Governed Actions",
              description: "Registry entries are rendered exactly as returned by the server; no plugin capability is inferred.",
              actions: <StatusBadge status={toLabel(data.actionMode)} severity="info" />,
            }}
          >
            {actions.length > 0 ? (
              <div className="grid gap-4 xl:grid-cols-2">
                {actions.map((action) => {
                  const rolloutEntries = action.rolloutScope ? Object.entries(action.rolloutScope) : [];

                  return (
                    <article
                      key={action.actionId}
                      className="relative overflow-hidden rounded-xl border border-white/[0.06] bg-[#09090B] p-5 transition-colors hover:border-[#00D4FF]/25"
                    >
                      <div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-[#00FF87]/0 via-[#00D4FF]/60 to-[#7B61FF]/0" />
                      <div className="flex items-start justify-between gap-4">
                        <div className="min-w-0">
                          <div className="flex flex-wrap items-center gap-2">
                            <h2 className="text-base font-semibold text-foreground">{action.label}</h2>
                            <StatusBadge
                              status={action.enabled ? "Enabled" : "Disabled"}
                              severity={action.enabled ? "success" : "neutral"}
                            />
                            <StatusBadge
                              status={
                                action.approvalRequired === undefined
                                  ? "Approval unknown"
                                  : action.approvalRequired
                                    ? "Approval required"
                                    : "No approval flag"
                              }
                              severity={
                                action.approvalRequired === undefined
                                  ? "warning"
                                  : action.approvalRequired
                                    ? "pending"
                                    : "neutral"
                              }
                            />
                          </div>
                          <p className="mt-2 text-sm leading-6 text-muted-foreground">{action.description}</p>
                        </div>
                        <div className="rounded-lg border border-[#00FF87]/20 bg-[#00FF87]/10 p-2 text-[#00FF87]">
                          {action.enabled ? <PlugZap className="h-5 w-5" /> : <ShieldAlert className="h-5 w-5" />}
                        </div>
                      </div>

                      <dl className="mt-5 grid gap-3 sm:grid-cols-2">
                        <div className="rounded-lg border border-white/[0.06] bg-white/[0.025] p-3">
                          <dt className="text-xs text-muted-foreground">Action ID</dt>
                          <dd className="mt-1 break-all font-mono text-xs text-foreground">{action.actionId}</dd>
                        </div>
                        <div className="rounded-lg border border-white/[0.06] bg-white/[0.025] p-3">
                          <dt className="text-xs text-muted-foreground">Source</dt>
                          <dd className="mt-1 text-sm font-medium text-foreground">{action.source ?? "unknown"}</dd>
                        </div>
                      </dl>

                      <div className="mt-4 rounded-lg border border-white/[0.06] bg-white/[0.025] p-3">
                        <div className="mb-3 flex items-center justify-between gap-3">
                          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">Rollout Scope</p>
                          {rolloutEntries.length > 0 ? (
                            <CheckCircle2 className="h-4 w-4 text-[#00FF87]" />
                          ) : null}
                        </div>
                        {rolloutEntries.length > 0 ? (
                          <div className="grid gap-2">
                            {rolloutEntries.map(([key, value]) => (
                              <div key={key} className="flex flex-wrap items-center justify-between gap-2 text-sm">
                                <span className="text-muted-foreground">{toLabel(key)}</span>
                                <span className="break-words text-right font-mono text-foreground">{stringifyScopeValue(value)}</span>
                              </div>
                            ))}
                          </div>
                        ) : (
                          <p className="text-sm text-muted-foreground">No rollout scope metadata supplied.</p>
                        )}
                      </div>
                    </article>
                  );
                })}
              </div>
            ) : (
              <EmptyState
                title="No governed extension actions"
                description="The registry did not return any whitelisted extension actions."
              />
            )}
          </Panel>
        </div>
      )}
    </main>
  );
}
