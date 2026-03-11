"use client";

import { useEffect, useState } from "react";
import { BadgeCheck, Loader2, RefreshCw, Scale, ShieldAlert } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

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

function tone(value: string): string {
  switch (value.toLowerCase()) {
    case "high":
    case "pending_approval":
      return "text-red-600";
    case "medium":
    case "draft":
      return "text-amber-600";
    default:
      return "text-emerald-600";
  }
}

function approvalReviewSummary(snapshot: SettlementSnapshot | undefined): string {
  if (!snapshot) return "0 proposals need review inside 30 min";
  const attentionCount = snapshot.proposals.filter((proposal) => proposal.approvalRequired).length;
  return `${attentionCount} proposal${attentionCount === 1 ? "" : "s"} needs review inside 30 min`;
}

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
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Settlement Workbench</h1>
          <p className="text-muted-foreground">
            Review bilateral settlement proposals and keep execution approval-gated before any
            release of funds.
          </p>
        </div>
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
      </div>

      <div className="flex gap-2">
        <Button
          variant={scenario === "active" ? "default" : "outline"}
          onClick={() => {
            setScenario("active");
            void load("active");
          }}
        >
          Active Queue
        </Button>
        <Button
          variant={scenario === "approval_pending" ? "default" : "outline"}
          onClick={() => {
            setScenario("approval_pending");
            void load("approval_pending");
          }}
        >
          Approval Pending
        </Button>
        <Button
          variant={scenario === "clean" ? "default" : "outline"}
          onClick={() => {
            setScenario("clean");
            void load("clean");
          }}
        >
          Clean Control
        </Button>
      </div>

      {error ? (
        <Card>
          <CardHeader>
            <CardTitle role="heading" aria-level={2}>
              Settlement workbench unavailable
            </CardTitle>
            <CardDescription>
              {error === "Settlement workbench unavailable"
                ? "Retry the bounded settlement workbench request or switch to a control scenario."
                : error}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={() => void load(scenario)}>Reload workbench</Button>
          </CardContent>
        </Card>
      ) : null}

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Action Mode</CardDescription>
            <CardTitle className="text-lg">Approval gated</CardTitle>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            Bilateral proposals only. No multilateral netting.
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Approval Mode</CardDescription>
            <CardTitle className="text-lg">{data?.approvalMode ?? "manual_approval"}</CardTitle>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            Operators approve before any execution.
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Proposals</CardDescription>
            <CardTitle className="text-lg">{data?.proposalCount ?? 0}</CardTitle>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            Same-counterparty packages only.
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Alerts</CardDescription>
            <CardTitle className="text-lg">{snapshot?.alerts.length ?? 0}</CardTitle>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            Approval queue and payable pressure.
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.1fr_0.9fr]">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2" role="heading" aria-level={2}>
              <Scale className="h-5 w-5" />
              Bilateral proposals
            </CardTitle>
            <CardDescription>
              Settlement packages are grouped by one counterparty and one asset only.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {loading && !snapshot ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                Loading settlement proposals...
              </div>
            ) : (
              snapshot?.proposals.map((proposal) => (
                <div key={proposal.id} className="rounded-lg border p-4">
                  <div className="flex items-start justify-between gap-4">
                    <div>
                      <div className="font-medium">{proposal.counterpartyId}</div>
                      <p className="mt-1 text-sm text-muted-foreground">{proposal.summary}</p>
                    </div>
                    <div className={`text-sm font-medium ${tone(proposal.status)}`}>
                      {proposal.status}
                    </div>
                  </div>
                  <div className="mt-3 grid gap-2 text-sm md:grid-cols-3">
                    <div>Asset: {proposal.asset}</div>
                    <div>Net amount: {proposal.netAmount}</div>
                    <div>Direction: {proposal.direction}</div>
                  </div>
                  <div className="mt-2 text-xs text-muted-foreground">
                    Settlements: {proposal.settlementIds.join(", ")}
                  </div>
                </div>
              ))
            )}
          </CardContent>
        </Card>

        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ShieldAlert className="h-5 w-5" />
                SLA guardian
              </CardTitle>
              <CardDescription>{approvalReviewSummary(snapshot)}</CardDescription>
            </CardHeader>
            <CardContent className="text-sm text-muted-foreground">
              Recommend treasury approval review before any release.
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ShieldAlert className="h-5 w-5" />
                Alerts
              </CardTitle>
              <CardDescription>Execution stays approval-gated even when pressure rises.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {snapshot?.alerts.map((alert) => (
                <div key={alert.id} className="rounded-lg border p-4">
                  <div className={`font-medium ${tone(alert.severity)}`}>{alert.title}</div>
                  <p className="mt-1 text-sm text-muted-foreground">{alert.summary}</p>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <BadgeCheck className="h-5 w-5" />
                Guardrail
              </CardTitle>
              <CardDescription>
                This wave computes bilateral packages and approval status only.
              </CardDescription>
            </CardHeader>
            <CardContent className="text-sm text-muted-foreground">
              No multilateral netting, no auto execution, and no second accounting engine.
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
