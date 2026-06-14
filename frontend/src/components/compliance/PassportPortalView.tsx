"use client";

import { useEffect, useState } from "react";
import { KeyRound, ShieldCheck, ShieldOff } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  EmptyState,
  ErrorState,
  Panel,
  StatCard,
  StatGrid,
  StatusBadge,
  TableSkeleton,
} from "@/components/shared";
import { kycApi } from "@/lib/portal-api";
import { formatDateTime, formatNumber, toLabel } from "@/lib/format";

type PassportSummary = {
  packageId: string;
  sourceTenantId: string;
  status: string;
  consentStatus: string;
  destinationTenantId?: string | null;
  fieldsShared: string[];
  expiresAt?: string | null;
  revokedAt?: string | null;
  reuseAllowed: boolean;
};

type KycStatus = {
  status: string;
  tier: number;
  passportSummary?: PassportSummary | null;
};

function humanize(value: string): string {
  return toLabel(value.toLowerCase());
}

function safeDate(value?: string | null): string {
  if (!value) return "N/A";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : formatDateTime(date);
}

export default function PassportPortalView() {
  const [data, setData] = useState<KycStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = async () => {
      try {
        setData(await kycApi.getStatus());
      } catch (requestError) {
        setError(requestError instanceof Error ? requestError.message : "Passport status unavailable");
      } finally {
        setLoading(false);
      }
    };

    void load();
  }, []);

  const passport = data?.passportSummary;

  return (
    <div className="flex flex-col gap-section" data-testid="passport-portal-view">
      <StatGrid cols={3}>
        <StatCard title="KYC status" value={data?.status ? humanize(data.status) : "N/A"} icon={<ShieldCheck className="h-4 w-4" />} accentColor="green" loading={loading} />
        <StatCard title="KYC tier" value={data ? `Level ${formatNumber(data.tier)}` : "N/A"} icon={<KeyRound className="h-4 w-4" />} accentColor="cyan" loading={loading} />
        <StatCard title="Shared fields" value={passport ? formatNumber(passport.fieldsShared.length) : "N/A"} icon={<ShieldOff className="h-4 w-4" />} accentColor="violet" loading={loading} />
      </StatGrid>

      <Panel
        header={{
          title: "Reusable KYC passport",
          description: "Review vault availability, consent state, and whether verification can be reused.",
        }}
      >
        {loading ? (
          <TableSkeleton rows={4} columns={3} />
        ) : error ? (
          <ErrorState title="Passport status unavailable" message={error} />
        ) : passport ? (
          <div className="space-y-5">
            <div className="flex flex-wrap items-start justify-between gap-3 rounded-xl border border-[#00FF87]/20 bg-[#00FF87]/5 p-4">
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <ShieldCheck className="h-4 w-4 text-[#00FF87]" />
                  <span className="font-semibold text-foreground">{passport.packageId}</span>
                </div>
                <p className="text-sm text-muted-foreground">Vault-backed passport package</p>
              </div>
              <StatusBadge status={humanize(passport.status)} />
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">
                <div className="text-xs text-muted-foreground">Consent</div>
                <StatusBadge className="mt-2" status={humanize(passport.consentStatus)} />
              </div>
              <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">
                <div className="text-xs text-muted-foreground">Reuse allowed</div>
                <StatusBadge className="mt-2" status={passport.reuseAllowed ? "Yes" : "No"} severity={passport.reuseAllowed ? "success" : "warning"} />
              </div>
              <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">
                <div className="text-xs text-muted-foreground">Source tenant</div>
                <div className="mt-1 font-medium text-foreground">{passport.sourceTenantId}</div>
              </div>
              <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">
                <div className="text-xs text-muted-foreground">Destination</div>
                <div className="mt-1 font-medium text-foreground">{passport.destinationTenantId ?? "Not scoped"}</div>
              </div>
              <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">
                <div className="text-xs text-muted-foreground">Expires</div>
                <div className="mt-1 font-medium text-foreground">{safeDate(passport.expiresAt)}</div>
              </div>
              <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">
                <div className="text-xs text-muted-foreground">Revoked</div>
                <div className="mt-1 font-medium text-foreground">{safeDate(passport.revokedAt)}</div>
              </div>
            </div>

            <div>
              <div className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-[#00D4FF]">
                Shared fields
              </div>
              {passport.fieldsShared.length > 0 ? (
                <div className="flex flex-wrap gap-1.5">
                  {passport.fieldsShared.map((field) => (
                    <StatusBadge key={field} status={humanize(field)} severity="info" dot={false} />
                  ))}
                </div>
              ) : (
                <p className="text-sm text-muted-foreground">No shared fields reported.</p>
              )}
            </div>

            <Button variant="outline" disabled className="border-white/[0.1]">
              Share / Revoke coming next
            </Button>
          </div>
        ) : (
          <EmptyState
            icon={<ShieldOff className="h-8 w-8" />}
            title="No reusable passport"
            description="No reusable passport package is available yet."
          />
        )}
      </Panel>
    </div>
  );
}
