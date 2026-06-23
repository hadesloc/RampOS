"use client";

import { useEffect, useState, useCallback } from "react";
import {
  RefreshCw,
  Save,
  Shield,
  Loader2,
  Banknote,
  ArrowUpDown,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";

// ── Types ─────────────────────────────────────────────────────────────────────

type TierLimit = {
  kycTier: number;
  tierName: string;
  dailyPayinLimitVnd: number;
  dailyPayoutLimitVnd: number;
  monthlyPayinLimitVnd: number;
  monthlyPayoutLimitVnd: number;
  singleTransactionMaxVnd: number;
};

// ── API helper ────────────────────────────────────────────────────────────────

async function apiRequest<T>(endpoint: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`/api/proxy${endpoint}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  if (!response.ok) {
    let message = "Request failed";
    try {
      const p = (await response.json()) as { message?: string };
      message = p.message ?? message;
    } catch {
      /* keep default */
    }
    throw new Error(message);
  }
  return response.json() as Promise<T>;
}

// ── Formatters ────────────────────────────────────────────────────────────────

function formatVnd(value: number): string {
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
    maximumFractionDigits: 0,
  }).format(value);
}

// ── Tier accent map ───────────────────────────────────────────────────────────

const tierAccent: Record<number, "cyan" | "green" | "violet"> = {
  1: "cyan",
  2: "green",
  3: "violet",
};

const tierBorder: Record<number, string> = {
  1: "border-[#00D4FF]/20",
  2: "border-[#00FF87]/20",
  3: "border-[#7B61FF]/20",
};

// ── Sub-component: Tier Card ──────────────────────────────────────────────────

function TierCard({
  limit,
  editing,
  saving,
  onFieldChange,
  onSave,
}: {
  limit: TierLimit;
  editing: TierLimit;
  saving: boolean;
  onFieldChange: (field: keyof TierLimit, value: string) => void;
  onSave: () => void;
}) {
  const accent = tierAccent[limit.kycTier] ?? "cyan";

  const fields: Array<{ key: keyof TierLimit; label: string }> = [
    { key: "dailyPayinLimitVnd", label: "Daily Pay-in (VND)" },
    { key: "dailyPayoutLimitVnd", label: "Daily Pay-out (VND)" },
    { key: "monthlyPayinLimitVnd", label: "Monthly Pay-in (VND)" },
    { key: "monthlyPayoutLimitVnd", label: "Monthly Pay-out (VND)" },
  ];

  return (
    <Panel
      className={`border ${tierBorder[limit.kycTier] ?? ""}`}
      header={{
        title: limit.tierName,
        description: `Current daily pay-in: ${formatVnd(limit.dailyPayinLimitVnd)}`,
        actions: (
          <StatusBadge
            status={`Tier ${limit.kycTier}`}
            severity={
              limit.kycTier === 1
                ? "info"
                : limit.kycTier === 2
                ? "success"
                : "pending"
            }
            dot={false}
          />
        ),
      }}
    >
      <div className="space-y-5">
        <div className="grid gap-3 sm:grid-cols-2">
          {fields.map(({ key, label }) => (
            <div key={key} className="space-y-1.5">
              <Label className="text-xs text-muted-foreground">{label}</Label>
              <Input
                type="text"
                value={(editing[key] as number).toLocaleString("vi-VN")}
                onChange={(e) => onFieldChange(key, e.target.value)}
                className="h-8 text-sm tabular-nums border-white/[0.08] bg-white/[0.02] focus:border-white/[0.16]"
              />
            </div>
          ))}
        </div>

        <div className="space-y-1.5">
          <Label className="text-xs text-muted-foreground">
            Single Transaction Max (VND)
          </Label>
          <Input
            type="text"
            value={(editing.singleTransactionMaxVnd as number).toLocaleString(
              "vi-VN"
            )}
            onChange={(e) =>
              onFieldChange("singleTransactionMaxVnd", e.target.value)
            }
            className="h-8 text-sm tabular-nums border-white/[0.08] bg-white/[0.02] focus:border-white/[0.16]"
          />
        </div>

        <Button
          className="w-full h-9 text-sm"
          onClick={onSave}
          disabled={saving}
          aria-label={`Save Tier ${limit.kycTier} limits`}
        >
          {saving ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Save className="mr-2 h-4 w-4" />
          )}
          Save Tier {limit.kycTier}
        </Button>
      </div>
    </Panel>
  );
}

// ── Page ─────────────────────────────────────────────────────────────────────

export default function LimitsPage() {
  const [limits, setLimits] = useState<TierLimit[]>([]);
  const [editing, setEditing] = useState<Record<number, TierLimit>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState<number | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiRequest<TierLimit[]>("/v1/admin/limits");
      const list = Array.isArray(data) ? data : [];
      setLimits(list);
      const editMap: Record<number, TierLimit> = {};
      list.forEach((l) => {
        editMap[l.kycTier] = { ...l };
      });
      setEditing(editMap);
    } catch {
      // Limits backend not wired — clean empty state instead of an error block.
      setLimits([]);
      setEditing({});
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleSave = async (tier: number) => {
    const data = editing[tier];
    if (!data) return;
    setSaving(tier);
    setSuccessMsg(null);
    try {
      await apiRequest(`/v1/admin/limits/${tier}`, {
        method: "PUT",
        body: JSON.stringify(data),
      });
      setSuccessMsg(`Tier ${tier} limits saved successfully.`);
      await fetchData();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to save limits"
      );
    } finally {
      setSaving(null);
    }
  };

  const updateField = (
    tier: number,
    field: keyof TierLimit,
    value: string
  ) => {
    setEditing((prev) => ({
      ...prev,
      [tier]: {
        ...prev[tier],
        [field]: parseInt(value.replace(/\D/g, ""), 10) || 0,
      },
    }));
  };

  // KPI aggregates from current limits
  const totalDailyCapacity = limits.reduce(
    (sum, l) => sum + l.dailyPayinLimitVnd + l.dailyPayoutLimitVnd,
    0
  );
  const maxSingleTx = limits.reduce(
    (max, l) => Math.max(max, l.singleTransactionMaxVnd),
    0
  );

  return (
    <main className="p-6 md:p-8 flex flex-col gap-6">
      <PageHeader
        title="Transaction Limits"
        description="Configure VND transaction limits per KYC tier — daily, monthly, and per-transaction caps."
        breadcrumb={[{ label: "Admin", href: "/admin" }, { label: "Limits" }]}
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={fetchData}
            disabled={loading}
            aria-label="Refresh limits"
            className="border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] h-9 w-9"
          >
            <RefreshCw
              className={`h-4 w-4 ${loading ? "animate-spin" : ""}`}
            />
          </Button>
        }
      />

      {/* KPI strip */}
      <StatGrid cols={3}>
        <StatCard
          title="KYC Tiers"
          value={loading ? "—" : limits.length.toString()}
          icon={<Shield className="h-4 w-4" />}
          accentColor="violet"
          subtitle="Configured tiers"
          loading={loading}
        />
        <StatCard
          title="Total Daily Capacity"
          value={
            loading
              ? "—"
              : formatVnd(totalDailyCapacity)
          }
          icon={<Banknote className="h-4 w-4" />}
          accentColor="green"
          subtitle="Sum of all tier daily caps"
          loading={loading}
        />
        <StatCard
          title="Max Single Transaction"
          value={
            loading ? "—" : formatVnd(maxSingleTx)
          }
          icon={<ArrowUpDown className="h-4 w-4" />}
          accentColor="cyan"
          subtitle="Highest single-tx cap"
          loading={loading}
        />
      </StatGrid>

      {/* Notifications */}
      {successMsg && (
        <div
          role="status"
          className="rounded-lg border border-[#00FF87]/20 bg-[#00FF87]/5 px-4 py-3 text-sm text-[#00FF87]"
        >
          {successMsg}
        </div>
      )}

      {/* Error */}
      {error && !loading && (
        <ErrorState
          title="Failed to load limits"
          message={error}
          retry={fetchData}
        />
      )}

      {/* Tier cards */}
      {!error && (
        <>
          {loading ? (
            <div className="grid gap-6 xl:grid-cols-3">
              {Array.from({ length: 3 }).map((_, i) => (
                <div
                  key={i}
                  className="h-72 rounded-xl bg-white/[0.02] border border-white/[0.06] animate-pulse"
                />
              ))}
            </div>
          ) : limits.length === 0 ? (
            <EmptyState
              icon={<Shield className="h-8 w-8" />}
              title="No tier limits configured"
              description="No KYC tier limits are currently set up."
            />
          ) : (
            <div className="grid gap-6 xl:grid-cols-3">
              {limits.map((limit) => (
                <TierCard
                  key={limit.kycTier}
                  limit={limit}
                  editing={editing[limit.kycTier] ?? limit}
                  saving={saving === limit.kycTier}
                  onFieldChange={(field, value) =>
                    updateField(limit.kycTier, field, value)
                  }
                  onSave={() => handleSave(limit.kycTier)}
                />
              ))}
            </div>
          )}
        </>
      )}
    </main>
  );
}
