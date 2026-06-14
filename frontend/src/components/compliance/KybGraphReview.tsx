"use client";

import { useEffect, useState } from "react";
import { AlertTriangle, FileWarning, Network, ShieldCheck } from "lucide-react";

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

type ReviewItem = {
  entityId: string;
  legalName: string;
  reviewStatus: string;
  summary: {
    missingRequirements: string[];
    reviewFlags: string[];
  };
};

type ReviewResponse = {
  queue: ReviewItem[];
  actionMode: string;
};

function humanize(value: string): string {
  return toLabel(value.toLowerCase());
}

export default function KybGraphReview() {
  const [data, setData] = useState<ReviewResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = async () => {
      try {
        const response = await fetch("/api/proxy/v1/admin/kyb/reviews");
        if (!response.ok) {
          throw new Error("KYB review graph failed to load");
        }
        setData(await response.json());
      } catch (requestError) {
        setError(requestError instanceof Error ? requestError.message : "KYB review graph unavailable");
      } finally {
        setLoading(false);
      }
    };

    void load();
  }, []);

  const queue = data?.queue ?? [];
  const missingRequirementCount = queue.reduce(
    (total, item) => total + item.summary.missingRequirements.length,
    0,
  );
  const reviewFlagCount = queue.reduce((total, item) => total + item.summary.reviewFlags.length, 0);

  return (
    <div className="flex flex-col gap-section" data-testid="kyb-graph-review">
      <StatGrid cols={4}>
        <StatCard title="Review entities" value={formatNumber(queue.length)} icon={<Network className="h-4 w-4" />} accentColor="cyan" loading={loading} />
        <StatCard title="Missing requirements" value={formatNumber(missingRequirementCount)} icon={<FileWarning className="h-4 w-4" />} accentColor="amber" loading={loading} />
        <StatCard title="Review flags" value={formatNumber(reviewFlagCount)} icon={<AlertTriangle className="h-4 w-4" />} accentColor="violet" loading={loading} />
        <StatCard title="Action mode" value={data?.actionMode ? humanize(data.actionMode) : "N/A"} icon={<ShieldCheck className="h-4 w-4" />} accentColor="green" loading={loading} />
      </StatGrid>

      <Panel
        header={{
          title: "KYB ownership review",
          description: "Review relational ownership edges, missing licensing documents, and review flags.",
        }}
      >
        {loading ? (
          <CardGridSkeleton cards={4} />
        ) : error ? (
          <ErrorState title="KYB graph unavailable" message={error} />
        ) : queue.length === 0 ? (
          <EmptyState
            icon={<Network className="h-8 w-8" />}
            title="No KYB reviews queued"
            description="There are no ownership review items available for this tenant."
          />
        ) : (
          <div className="grid gap-4 lg:grid-cols-2">
            {queue.map((item) => (
              <div key={item.entityId} className="rounded-xl border border-white/[0.06] bg-[#09090B]/60 p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="space-y-1">
                    <div className="font-semibold text-foreground">{item.legalName}</div>
                    <div className="text-xs text-muted-foreground">{item.entityId}</div>
                  </div>
                  <StatusBadge status={humanize(item.reviewStatus)} />
                </div>

                <div className="mt-4 grid gap-3 md:grid-cols-2">
                  <div className="rounded-lg border border-[#FFB800]/15 bg-[#FFB800]/5 p-3">
                    <div className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-[#FFB800]">
                      Missing
                    </div>
                    {item.summary.missingRequirements.length > 0 ? (
                      <div className="flex flex-wrap gap-1.5">
                        {item.summary.missingRequirements.map((requirement) => (
                          <StatusBadge key={requirement} status={humanize(requirement)} severity="warning" dot={false} />
                        ))}
                      </div>
                    ) : (
                      <p className="text-xs text-muted-foreground">No missing requirements reported.</p>
                    )}
                  </div>

                  <div className="rounded-lg border border-[#7B61FF]/15 bg-[#7B61FF]/5 p-3">
                    <div className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-[#7B61FF]">
                      Flags
                    </div>
                    {item.summary.reviewFlags.length > 0 ? (
                      <div className="flex flex-wrap gap-1.5">
                        {item.summary.reviewFlags.map((flag) => (
                          <StatusBadge key={flag} status={humanize(flag)} severity="pending" dot={false} />
                        ))}
                      </div>
                    ) : (
                      <p className="text-xs text-muted-foreground">No review flags reported.</p>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </Panel>
    </div>
  );
}
