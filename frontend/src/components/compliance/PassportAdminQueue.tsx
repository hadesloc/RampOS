"use client";

import { useEffect, useState } from "react";
import { FileCheck2, ShieldAlert, ShieldCheck, Users } from "lucide-react";

import {
  CardGridSkeleton,
  EmptyState,
  ErrorState,
  Panel,
  StatCard,
  StatGrid,
  StatusBadge,
} from "@/components/shared";
import { formatNumber, toLabel } from "@/lib/format";

type PassportQueueItem = {
  packageId: string;
  userId: string;
  sourceTenantId: string;
  targetTenantId: string;
  status: string;
  consentStatus: string;
  reviewStatus: string;
  fieldsShared: string[];
};

type QueueResponse = {
  queue: PassportQueueItem[];
  actionMode: string;
};

function humanize(value?: string | null): string {
  if (!value) return "—";
  return toLabel(value.toLowerCase());
}

export default function PassportAdminQueue() {
  const [data, setData] = useState<QueueResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = async () => {
      try {
        const response = await fetch("/api/proxy/v1/admin/passport/queue");
        if (!response.ok) {
          throw new Error("Passport queue failed to load");
        }
        const payload = await response.json();
        setData(payload);
      } catch (requestError) {
        setError(requestError instanceof Error ? requestError.message : "Passport queue unavailable");
      } finally {
        setLoading(false);
      }
    };

    void load();
  }, []);

  const queue = data?.queue ?? [];
  const consented = queue.filter((item) => item.consentStatus === "GRANTED" || item.consentStatus === "ACTIVE").length;
  const reviewRequired = queue.filter((item) => item.reviewStatus === "REVIEW_REQUIRED" || item.reviewStatus === "PENDING").length;
  const fieldCount = queue.reduce((total, item) => total + (item.fieldsShared?.length ?? 0), 0);

  return (
    <div className="flex flex-col gap-section" data-testid="passport-admin-queue">
      <StatGrid cols={4}>
        <StatCard title="Passport packages" value={formatNumber(queue.length)} icon={<ShieldAlert className="h-4 w-4" />} accentColor="cyan" loading={loading} />
        <StatCard title="Consented" value={formatNumber(consented)} icon={<ShieldCheck className="h-4 w-4" />} accentColor="green" loading={loading} />
        <StatCard title="Review required" value={formatNumber(reviewRequired)} icon={<Users className="h-4 w-4" />} accentColor="amber" loading={loading} />
        <StatCard title="Shared fields" value={formatNumber(fieldCount)} icon={<FileCheck2 className="h-4 w-4" />} accentColor="violet" loading={loading} subtitle={data?.actionMode ? humanize(data.actionMode) : undefined} />
      </StatGrid>

      <Panel
        header={{
          title: "Passport queue",
          description: "Review shared-vault packages, consent state, destination tenant, and freshness.",
        }}
      >
        {loading ? (
          <CardGridSkeleton cards={4} />
        ) : error ? (
          <ErrorState title="Passport queue unavailable" message={error} />
        ) : queue.length === 0 ? (
          <EmptyState
            icon={<ShieldAlert className="h-8 w-8" />}
            title="No passport packages queued"
            description="No reusable KYC passport packages are available for admin review."
          />
        ) : (
          <div className="grid gap-4 lg:grid-cols-2">
            {queue.map((item) => (
              <div key={item.packageId} className="rounded-xl border border-white/[0.06] bg-[#09090B]/60 p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="space-y-1">
                    <div className="font-semibold text-foreground">{item.packageId}</div>
                    <div className="text-xs text-muted-foreground">User {item.userId}</div>
                  </div>
                  <StatusBadge status={humanize(item.status)} />
                </div>

                <div className="mt-4 grid gap-3 text-sm md:grid-cols-2">
                  <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">
                    <div className="text-xs text-muted-foreground">Consent</div>
                    <StatusBadge className="mt-2" status={humanize(item.consentStatus)} />
                  </div>
                  <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">
                    <div className="text-xs text-muted-foreground">Review</div>
                    <StatusBadge className="mt-2" status={humanize(item.reviewStatus)} />
                  </div>
                  <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">
                    <div className="text-xs text-muted-foreground">Source tenant</div>
                    <div className="mt-1 font-medium text-foreground">{item.sourceTenantId}</div>
                  </div>
                  <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3">
                    <div className="text-xs text-muted-foreground">Destination</div>
                    <div className="mt-1 font-medium text-foreground">{item.targetTenantId}</div>
                  </div>
                </div>

                <div className="mt-4">
                  <div className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-[#00D4FF]">
                    Shared fields
                  </div>
                  {(item.fieldsShared?.length ?? 0) > 0 ? (
                    <div className="flex flex-wrap gap-1.5">
                      {item.fieldsShared.map((field) => (
                        <StatusBadge key={field} status={humanize(field)} severity="info" dot={false} />
                      ))}
                    </div>
                  ) : (
                    <p className="text-xs text-muted-foreground">No shared fields reported.</p>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </Panel>
    </div>
  );
}
