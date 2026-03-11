"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Loader2,
  RefreshCw,
  Gavel,
  TrendingUp,
  Clock,
  CheckCircle2,
  ArrowUpDown,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

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

function directionBadge(direction: RfqRequest["direction"]) {
  return direction === "ONRAMP" ? (
    <Badge variant="outline" className="border-emerald-500/40 text-emerald-600 dark:text-emerald-400">
      VND to USDT
    </Badge>
  ) : (
    <Badge variant="outline" className="border-violet-500/40 text-violet-600 dark:text-violet-400">
      USDT to VND
    </Badge>
  );
}

function statusBadge(status: RfqRequest["state"]) {
  const colors: Record<RfqRequest["state"], string> = {
    OPEN: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400",
    MATCHED: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400",
    EXPIRED: "bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400",
    CANCELLED: "bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400",
  };

  return (
    <span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${colors[status]}`}>
      {status}
    </span>
  );
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
      await apiRequest(`/v1/admin/rfq/${rfqId}/finalize`, { method: "POST" });
      await fetchData();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Finalize failed");
    } finally {
      setFinalizing(null);
    }
  };

  const openCount = requests.filter((request) => request.state === "OPEN").length;
  const avgBids =
    requests.length > 0
      ? (requests.reduce((sum, request) => sum + request.bidCount, 0) / requests.length).toFixed(1)
      : "0";
  const bestRate = requests.reduce((highest, request) => {
    const current = toNumber(request.bestRate);
    return current !== null ? Math.max(highest, current) : highest;
  }, 0);
  const expiringSoon = requests.filter((request) => {
    const expiresAt = new Date(request.expiresAt).getTime();
    return Number.isFinite(expiresAt) && expiresAt - Date.now() <= 5 * 60 * 1000;
  }).length;

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">RFQ Auctions</h1>
          <p className="text-muted-foreground">
            Monitor active RFQ auctions and manually finalize an open request when needed.
          </p>
        </div>
        <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Open Auctions</CardDescription>
            <Gavel className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{openCount}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Avg Bids / Request</CardDescription>
            <ArrowUpDown className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{avgBids}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Best Visible Rate</CardDescription>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {bestRate > 0 ? bestRate.toLocaleString("vi-VN") : "-"}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardDescription>Expiring in 5m</CardDescription>
            <Clock className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{expiringSoon}</div>
          </CardContent>
        </Card>
      </div>

      {error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Auction Queue</CardTitle>
          <CardDescription>
            Active RFQ requests returned by the admin API.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="flex items-center justify-center gap-2 py-12 text-muted-foreground">
              <Loader2 className="h-5 w-5 animate-spin" />
              Loading auctions...
            </div>
          ) : requests.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground">
              No open RFQ requests found.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>ID</TableHead>
                  <TableHead>User</TableHead>
                  <TableHead>Direction</TableHead>
                  <TableHead>Asset</TableHead>
                  <TableHead className="text-right">Crypto Amount</TableHead>
                  <TableHead className="text-right">Budget VND</TableHead>
                  <TableHead className="text-center">Bids</TableHead>
                  <TableHead className="text-right">Best Rate</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Expires</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {requests.map((request) => (
                  <TableRow key={request.id}>
                    <TableCell className="font-mono text-xs">
                      {request.id.slice(0, 16)}...
                    </TableCell>
                    <TableCell className="font-mono text-xs">
                      {request.userId.slice(0, 12)}...
                    </TableCell>
                    <TableCell>{directionBadge(request.direction)}</TableCell>
                    <TableCell>{request.cryptoAsset}</TableCell>
                    <TableCell className="text-right font-medium">
                      {toNumber(request.cryptoAmount)?.toLocaleString("en-US") ?? request.cryptoAmount}
                    </TableCell>
                    <TableCell className="text-right">
                      {toNumber(request.vndAmount)?.toLocaleString("vi-VN") ?? "-"}
                    </TableCell>
                    <TableCell className="text-center">
                      <Badge variant="secondary">{request.bidCount}</Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      {toNumber(request.bestRate)?.toLocaleString("vi-VN") ?? "-"}
                    </TableCell>
                    <TableCell>{statusBadge(request.state)}</TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {formatTimestamp(request.expiresAt)}
                    </TableCell>
                    <TableCell className="text-right">
                      {request.state === "OPEN" && (
                        <Button
                          size="sm"
                          variant="outline"
                          disabled={finalizing === request.id}
                          onClick={() => handleFinalize(request.id)}
                        >
                          {finalizing === request.id ? (
                            <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                          ) : (
                            <CheckCircle2 className="mr-1 h-3 w-3" />
                          )}
                          Finalize
                        </Button>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
