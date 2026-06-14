"use client";

import { useEffect, useState } from "react";
import { BadgeCheck, Loader2, RefreshCw, Scale, ShieldAlert } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";
import { toLabel } from "@/lib/format";

type SettlementProposal = {
  id: string;
  counterpartyId: string;
  asset: string;
  settlementIds: string[];
  grossIn: string;
  grossOut: string;
  netAmount: string;
  direction: string;
  status: string;
  approvalRequired: boolean;
  summary: string;
};

type SettlementAlert = {
  id: string;
  severity: string;
  title: string;
  summary: string;
};

type SettlementSnapshot = {
  generatedAt: string;
  approvalMode: string;
  actionMode: string;
  proposals: SettlementProposal[];
  alerts: SettlementAlert[];
};

type SettlementWorkbenchResponse = {
  snapshot: SettlementSnapshot;
  actionMode: string;
  approvalMode: string;
  proposalCount: number;
  exportFormats: string[];
};

async function apiRequest<T>(endpoint: string): Promise<T> {
  const response = await fetch(`/api/proxy${endpoint}`);
  if (!response.ok) {
    let message = "Failed to load settlement workbench";
    try {
      const payload = (await response.json()) as { message?: string; error?: { message?: string } };
      message = payload.message ?? payload.error?.message ?? message;
    } catch {
      // Keep fallback.
    }
    throw new Error(message);
  }
  return response.json() as Promise<T>;
}

function approvalReviewSummary(snapshot: SettlementSnapshot | undefined): string {
  if (!snapshot) return "0 proposals need review inside 30 min";
  const attentionCount = snapshot.proposals.filter((proposal) => proposal.approvalRequired).length;
  return `${attentionCount} proposal${attentionCount === 1 ? "" : "s"} needs review inside 30 min`;
}

const SCENARIOS: { id: "active" | "clean" | "approval_pending"; label: string }[] = [
  { id: "active", label: "Active Queue" },
  { id: "approval_pending", label: "Approval Pending" },
  { id: "clean", label: "Clean Control" },
];

export default function SettlementWorkbench() {
  const [data, setData] = useState<SettlementWorkbenchResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [scenario, setScenario] = useState<"active" | "clean" | "approval_pending">("active");

  const load = async (
    nextScenario: "active" | "clean" | "approval_pending",
    isRefresh = false,
  ) => {
    if (isRefresh) setRefreshing(true);
    else setLoading(true);
    setError(null);

    try {
      const query =
        nextScenario === "active" ? "" : `?scenario=${encodeURIComponent(nextScenario)}`;
      const response = await apiRequest<SettlementWorkbenchResponse>(
        `/v1/admin/settlement/workbench${query}`,
      );
      setData(response);
    } catch (requestError) {
      setData(null);
      setError(
        requestError instanceof Error
          ? requestError.message
          : "Failed to load settlement workbench",
      );
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    void load("active");
  }, []);

  const snapshot = data?.snapshot;

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Settlement Workbench"
        description="Review bilateral settlement proposals and keep execution approval-gated before any release of funds."
        actions={
          <Button
            variant="outline"
            size="icon"
            aria-label="Refresh settlement workbench"
            onClick={() => {
              void load(scenario, true);
            }}
            disabled={loading || refreshing}
          >
            {loading || refreshing ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
          </Button>
        }
      />

      <div className="flex flex-wrap gap-2">
        {SCENARIOS.map((item) => (
          <Button
            key={item.id}
            variant={scenario === item.id ? "default" : "outline"}
            size="sm"
            onClick={() => {
              setScenario(item.id);
              void load(item.id);
            }}
          >
            {item.label}
          </Button>
        ))}
      </div>

      {error ? (
        <ErrorState
          title="Settlement workbench unavailable"
          message={
            error === "Settlement workbench unavailable"
              ? "Retry the bounded settlement workbench request or switch to a control scenario."
              : error
          }
          retry={() => void load(scenario)}
        />
      ) : null}

      <StatGrid cols={4}>
        <StatCard
          title="Action Mode"
          value="Approval gated"
          accentColor="cyan"
          subtitle="Bilateral only — no netting"
          loading={loading && !snapshot}
        />
        <StatCard
          title="Approval Mode"
          value={toLabel(data?.approvalMode ?? "manual_approval")}
          accentColor="violet"
          subtitle="Operator approves execution"
          loading={loading && !snapshot}
        />
        <StatCard
          title="Proposals"
          value={data?.proposalCount ?? 0}
          accentColor="green"
          subtitle="Same-counterparty packages"
          loading={loading && !snapshot}
        />
        <StatCard
          title="Alerts"
          value={snapshot?.alerts.length ?? 0}
          accentColor="amber"
          subtitle="Approval & payable pressure"
          loading={loading && !snapshot}
        />
      </StatGrid>

      <div className="grid gap-6 xl:grid-cols-[1.1fr_0.9fr]">
        <Panel
          header={{
            title: "Bilateral proposals",
            description:
              "Settlement packages are grouped by one counterparty and one asset only.",
          }}
        >
          <div className="flex items-center gap-2 pb-3 text-sm font-medium text-muted-foreground">
            <Scale className="h-4 w-4" />
            Counterparty packages
          </div>
          {loading && !snapshot ? (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <div
                  key={i}
                  className="h-24 rounded-lg border border-white/[0.06] bg-white/[0.02] animate-pulse"
                />
              ))}
            </div>
          ) : !snapshot || snapshot.proposals.length === 0 ? (
            <EmptyState
              title="No bilateral proposals"
              description="No counterparty settlement packages in this scenario."
            />
          ) : (
            <div className="space-y-3">
              {snapshot.proposals.map((proposal) => (
                <div
                  key={proposal.id}
                  className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-4 transition-colors hover:border-white/[0.1]"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0">
                      <div className="font-medium">{proposal.counterpartyId}</div>
                      <p className="mt-1 text-sm text-muted-foreground">{proposal.summary}</p>
                    </div>
                    <StatusBadge status={proposal.status} />
                  </div>
                  <div className="mt-3 grid gap-2 text-sm md:grid-cols-3">
                    <div>
                      <span className="text-muted-foreground">Asset: </span>
                      <span className="font-mono">{proposal.asset}</span>
                    </div>
                    <div className="tabular-nums">
                      <span className="text-muted-foreground">Net: </span>
                      {proposal.netAmount}
                    </div>
                    <div>
                      <span className="text-muted-foreground">Direction: </span>
                      {proposal.direction}
                    </div>
                  </div>
                  <div className="mt-2 text-xs text-muted-foreground">
                    Settlements: {proposal.settlementIds.join(", ")}
                  </div>
                </div>
              ))}
            </div>
          )}
        </Panel>

        <div className="flex flex-col gap-6">
          <Panel
            header={{ title: "SLA guardian", description: approvalReviewSummary(snapshot) }}
          >
            <div className="flex items-start gap-3 text-sm text-muted-foreground">
              <ShieldAlert className="mt-0.5 h-4 w-4 shrink-0 text-[#FFB800]" />
              Recommend treasury approval review before any release.
            </div>
          </Panel>

          <Panel
            header={{
              title: "Alerts",
              description: "Execution stays approval-gated even when pressure rises.",
            }}
          >
            {!snapshot || snapshot.alerts.length === 0 ? (
              <EmptyState title="No active alerts" />
            ) : (
              <div className="space-y-3">
                {snapshot.alerts.map((alert) => (
                  <div
                    key={alert.id}
                    className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-4"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <div className="font-medium">{alert.title}</div>
                      <StatusBadge status={alert.severity} />
                    </div>
                    <p className="mt-1 text-sm text-muted-foreground">{alert.summary}</p>
                  </div>
                ))}
              </div>
            )}
          </Panel>

          <Panel
            header={{
              title: "Guardrail",
              description: "This wave computes bilateral packages and approval status only.",
            }}
          >
            <div className="flex items-start gap-3 text-sm text-muted-foreground">
              <BadgeCheck className="mt-0.5 h-4 w-4 shrink-0 text-[#00FF87]" />
              No multilateral netting, no auto execution, and no second accounting engine.
            </div>
          </Panel>
        </div>
      </div>
    </main>
  );
}
