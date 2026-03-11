"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Loader2,
  RefreshCw,
  Banknote,
  ArrowUpDown,
  Save,
  Shield,
} from "lucide-react";

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
import { Badge } from "@/components/ui/badge";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type TierLimit = {
  kycTier: number;
  tierName: string;
  dailyPayinLimitVnd: number;
  dailyPayoutLimitVnd: number;
  monthlyPayinLimitVnd: number;
  monthlyPayoutLimitVnd: number;
  singleTransactionMaxVnd: number;
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
    } catch { /* keep default */ }
    throw new Error(message);
  }
  return response.json() as Promise<T>;
}

function formatVnd(value: number): string {
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
    maximumFractionDigits: 0,
  }).format(value);
}

const tierColors: Record<number, string> = {
  1: "border-blue-500/30 bg-blue-50/50 dark:bg-blue-950/20",
  2: "border-emerald-500/30 bg-emerald-50/50 dark:bg-emerald-950/20",
  3: "border-violet-500/30 bg-violet-50/50 dark:bg-violet-950/20",
};

const tierBadgeColors: Record<number, string> = {
  1: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400",
  2: "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-400",
  3: "bg-violet-100 text-violet-800 dark:bg-violet-900/30 dark:text-violet-400",
};

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

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
      setLimits(data);
      const editMap: Record<number, TierLimit> = {};
      data.forEach((l) => { editMap[l.kycTier] = { ...l }; });
      setEditing(editMap);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load limits");
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
      setError(err instanceof Error ? err.message : "Failed to save limits");
    } finally {
      setSaving(null);
    }
  };

  const updateField = (tier: number, field: keyof TierLimit, value: string) => {
    setEditing((prev) => ({
      ...prev,
      [tier]: { ...prev[tier], [field]: parseInt(value.replace(/\D/g, ""), 10) || 0 },
    }));
  };

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Transaction Limits</h1>
          <p className="text-muted-foreground">
            Configure VND transaction limits per KYC tier — daily, monthly, and per-transaction caps.
          </p>
        </div>
        <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </div>

      {error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}
      {successMsg && (
        <div className="rounded-md border border-emerald-500/30 bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:bg-emerald-950/20 dark:text-emerald-400">
          {successMsg}
        </div>
      )}

      {loading ? (
        <div className="flex items-center justify-center gap-2 py-16 text-muted-foreground">
          <Loader2 className="h-6 w-6 animate-spin" />
          Loading limits…
        </div>
      ) : (
        <div className="grid gap-6 xl:grid-cols-3">
          {limits.map((limit) => {
            const ed = editing[limit.kycTier] ?? limit;
            return (
              <Card key={limit.kycTier} className={tierColors[limit.kycTier] ?? ""}>
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <CardTitle className="flex items-center gap-2">
                      <Shield className="h-5 w-5" />
                      {limit.tierName}
                    </CardTitle>
                    <span className={`inline-flex items-center rounded-full px-2.5 py-1 text-xs font-bold ${tierBadgeColors[limit.kycTier] ?? ""}`}>
                      Tier {limit.kycTier}
                    </span>
                  </div>
                  <CardDescription>Current: {formatVnd(limit.dailyPayinLimitVnd)}/day payin</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="grid gap-3 sm:grid-cols-2">
                    <div className="space-y-1">
                      <Label className="text-xs">Daily Pay-in (VND)</Label>
                      <Input
                        type="text"
                        value={ed.dailyPayinLimitVnd.toLocaleString("vi-VN")}
                        onChange={(e) => updateField(limit.kycTier, "dailyPayinLimitVnd", e.target.value)}
                      />
                    </div>
                    <div className="space-y-1">
                      <Label className="text-xs">Daily Pay-out (VND)</Label>
                      <Input
                        type="text"
                        value={ed.dailyPayoutLimitVnd.toLocaleString("vi-VN")}
                        onChange={(e) => updateField(limit.kycTier, "dailyPayoutLimitVnd", e.target.value)}
                      />
                    </div>
                    <div className="space-y-1">
                      <Label className="text-xs">Monthly Pay-in (VND)</Label>
                      <Input
                        type="text"
                        value={ed.monthlyPayinLimitVnd.toLocaleString("vi-VN")}
                        onChange={(e) => updateField(limit.kycTier, "monthlyPayinLimitVnd", e.target.value)}
                      />
                    </div>
                    <div className="space-y-1">
                      <Label className="text-xs">Monthly Pay-out (VND)</Label>
                      <Input
                        type="text"
                        value={ed.monthlyPayoutLimitVnd.toLocaleString("vi-VN")}
                        onChange={(e) => updateField(limit.kycTier, "monthlyPayoutLimitVnd", e.target.value)}
                      />
                    </div>
                  </div>
                  <div className="space-y-1">
                    <Label className="text-xs">Single Transaction Max (VND)</Label>
                    <Input
                      type="text"
                      value={ed.singleTransactionMaxVnd.toLocaleString("vi-VN")}
                      onChange={(e) => updateField(limit.kycTier, "singleTransactionMaxVnd", e.target.value)}
                    />
                  </div>
                  <Button
                    className="w-full"
                    onClick={() => handleSave(limit.kycTier)}
                    disabled={saving === limit.kycTier}
                  >
                    {saving === limit.kycTier ? (
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    ) : (
                      <Save className="mr-2 h-4 w-4" />
                    )}
                    Save Tier {limit.kycTier}
                  </Button>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
