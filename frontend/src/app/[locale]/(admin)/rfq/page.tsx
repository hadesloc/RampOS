"use client";

import { useEffect, useState, useCallback, useMemo } from "react";
import {
  Loader2,
  RefreshCw,
  Gavel,
  TrendingUp,
  Clock,
  CheckCircle2,
  ArrowUpDown,
} from "lucide-react";
import type { ColumnDef } from "@tanstack/react-table";
import { Button } from "@/components/ui/button";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  DataTable,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";
import { truncateMiddle } from "@/lib/format";

type RfqRequest = {
  id: string;
  userId: string;
  direction: "ONRAMP" | "OFFRAMP";
  cryptoAsset: string;
  cryptoAmount: string;
  vndAmount: string | null;
  state: "OPEN" | "MATCHED" | "EXPIRED" | "CANCELLED";
  bidCount: number;
  bestRate: string | null;
  expiresAt: string;
  createdAt: string;
};

type ListOpenRfqResponse = {
  data: RfqRequest[];
  total: number;
  limit: number;
  offset: number;
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
    } catch {}
    throw new Error(message);
  }
  return response.json() as Promise<T>;
}

function formatTimestamp(value?: string | null): string {
  if (!value) return "-";
  return new Date(value).toLocaleString("vi-VN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function toNumber(value: string | null): number | null {
  if (!value) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

export default function RfqAdminPage() {
  const [requests, setRequests] = useState<RfqRequest[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [finalizing, setFinalizing] = useState<string | null>(null);
  const [finalizeResult, setFinalizeResult] = useState<{
    rfqId: string;
    state: string;
    winningLpId: string;
    finalRate: string;
  } | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const payload = await apiRequest<ListOpenRfqResponse>("/v1/admin/rfq/open");
      setRequests(payload.data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load RFQ data");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleFinalize = async (rfqId: string) => {
    setFinalizing(rfqId);
    try {
      const result = await apiRequest<{
        rfqId: string;
        state: string;
        winningLpId: string;
        finalRate: string;
      }>(`/v1/admin/rfq/${rfqId}/finalize`, { method: "POST" });
      setFinalizeResult(result);
      await fetchData();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Finalize failed");
    } finally {
      setFinalizing(null);
    }
  };

  const openCount = requests.filter((r) => r.state === "OPEN").length;
  const avgBids =
    requests.length > 0
      ? (requests.reduce((sum, r) => sum + r.bidCount, 0) / requests.length).toFixed(1)
      : "0";
  const bestRate = requests.reduce((highest, r) => {
    const current = toNumber(r.bestRate);
    return current !== null ? Math.max(highest, current) : highest;
  }, 0);
  const expiringSoon = requests.filter((r) => {
    const expiresAt = new Date(r.expiresAt).getTime();
    return Number.isFinite(expiresAt) && expiresAt - Date.now() <= 5 * 60 * 1000;
  }).length;

  const columns = useMemo<ColumnDef<RfqRequest>[]>(
    () => [
      {
        accessorKey: "id",
        header: "ID",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground" title={row.original.id}>
            {truncateMiddle(row.original.id, 8, 6)}
          </span>
        ),
      },
      {
        accessorKey: "userId",
        header: "User",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground" title={row.original.userId}>
            {truncateMiddle(row.original.userId, 8, 6)}
          </span>
        ),
      },
      {
        accessorKey: "direction",
        header: "Direction",
        cell: ({ row }) => {
          const dir = row.original.direction;
          return (
            <span
              className={
                dir === "ONRAMP"
                  ? "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-[#00FF87]/10 text-[#00FF87]"
                  : "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-[#7B61FF]/10 text-[#7B61FF]"
              }
            >
              {dir === "ONRAMP" ? "VND → USDT" : "USDT → VND"}
            </span>
          );
        },
      },
      {
        accessorKey: "cryptoAsset",
        header: "Asset",
        cell: ({ row }) => <span className="font-mono text-sm">{row.original.cryptoAsset}</span>,
      },
      {
        accessorKey: "cryptoAmount",
        header: () => <div className="text-right">Crypto Amt</div>,
        cell: ({ row }) => (
          <div className="text-right font-mono tabular-nums text-sm">
            {toNumber(row.original.cryptoAmount)?.toLocaleString("en-US") ?? row.original.cryptoAmount}
          </div>
        ),
      },
      {
        accessorKey: "vndAmount",
        header: () => <div className="text-right">Budget VND</div>,
        cell: ({ row }) => (
          <div className="text-right font-mono tabular-nums text-sm">
            {toNumber(row.original.vndAmount)?.toLocaleString("vi-VN") ?? "-"}
          </div>
        ),
      },
      {
        accessorKey: "bidCount",
        header: () => <div className="text-center">Bids</div>,
        cell: ({ row }) => (
          <div className="text-center">
            <span className="inline-flex items-center justify-center rounded-full bg-white/5 px-2 py-0.5 text-xs font-medium">
              {row.original.bidCount}
            </span>
          </div>
        ),
      },
      {
        accessorKey: "bestRate",
        header: () => <div className="text-right">Best Rate</div>,
        cell: ({ row }) => (
          <div className="text-right font-mono tabular-nums text-[#00FF87]">
            {toNumber(row.original.bestRate)?.toLocaleString("vi-VN") ?? "-"}
          </div>
        ),
      },
      {
        accessorKey: "state",
        header: "Status",
        cell: ({ row }) => <StatusBadge status={row.original.state} />,
      },
      {
        accessorKey: "expiresAt",
        header: "Expires",
        cell: ({ row }) => (
          <span className="text-xs text-muted-foreground whitespace-nowrap">
            {formatTimestamp(row.original.expiresAt)}
          </span>
        ),
      },
      {
        id: "actions",
        header: () => <div className="text-right">Actions</div>,
        cell: ({ row }) => (
          <div className="text-right">
            {row.original.state === "OPEN" && (
              <Button
                size="sm"
                variant="outline"
                disabled={finalizing === row.original.id}
                onClick={() => handleFinalize(row.original.id)}
                className="border-[#00FF87]/30 text-[#00FF87] hover:bg-[#00FF87]/10"
              >
                {finalizing === row.original.id ? (
                  <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                ) : (
                  <CheckCircle2 className="mr-1 h-3 w-3" />
                )}
                Finalize
              </Button>
            )}
          </div>
        ),
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [finalizing]
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="RFQ Auctions"
        description="Monitor active RFQ auctions and manually finalize an open request when needed."
        actions={
          <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      <StatGrid cols={4}>
        <StatCard
          title="Open Auctions"
          value={loading ? "-" : openCount}
          icon={<Gavel className="h-4 w-4" />}
          accentColor="cyan"
          loading={loading}
        />
        <StatCard
          title="Avg Bids / Request"
          value={loading ? "-" : avgBids}
          icon={<ArrowUpDown className="h-4 w-4" />}
          accentColor="violet"
          loading={loading}
        />
        <StatCard
          title="Best Visible Rate"
          value={loading ? "-" : bestRate > 0 ? bestRate.toLocaleString("vi-VN") : "-"}
          icon={<TrendingUp className="h-4 w-4" />}
          accentColor="green"
          loading={loading}
        />
        <StatCard
          title="Expiring in 5m"
          value={loading ? "-" : expiringSoon}
          icon={<Clock className="h-4 w-4" />}
          accentColor="amber"
          loading={loading}
        />
      </StatGrid>

      {finalizeResult && (
        <div className="rounded-md border border-[#00FF87]/30 bg-[#00FF87]/10 px-4 py-3 text-sm text-[#00FF87]">
          <span className="font-medium">RFQ matched:</span>{" "}
          {finalizeResult.rfqId} · LP {finalizeResult.winningLpId} · rate{" "}
          {Number(finalizeResult.finalRate).toLocaleString("vi-VN")}
        </div>
      )}

      {error ? (
        <ErrorState message={error} retry={fetchData} />
      ) : (
        <Panel header={{ title: "Auction Queue", description: "Active RFQ requests returned by the admin API." }}>
          <DataTable
            columns={columns}
            data={requests}
            loading={loading}
            pagination
            pageSize={15}
            emptyState={
              <EmptyState
                title="No open RFQ requests"
                description="There are currently no open RFQ auction requests."
              />
            }
          />
        </Panel>
      )}
    </main>
  );
}
