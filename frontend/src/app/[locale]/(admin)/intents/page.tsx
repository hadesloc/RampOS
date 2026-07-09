"use client";

import { useState, useMemo, useEffect, useCallback } from "react";
import Link from "next/link";
import { ColumnDef, PaginationState } from "@tanstack/react-table";
import { Search, RefreshCw } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { intentsApi, type Intent as ApiIntent } from "@/lib/api";
import { useToast } from "@/components/ui/use-toast";
import { useTranslations, useFormatter } from "next-intl";
import { useRouter, useSearchParams } from "next/navigation";
import {
  PageHeader,
  Panel,
  DataTable,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";

interface Intent {
  id: string;
  intentType: string;
  state: string;
  amount: string;
  currency: string;
  createdAt: string;
  referenceCode?: string;
}

function getTypeStyle(type: string): string {
  switch (type) {
    case "PAYIN":
      return "bg-[#00D4FF]/10 text-[#00D4FF]";
    case "PAYOUT":
      return "bg-[#7B61FF]/10 text-[#7B61FF]";
    case "TRADE":
      return "bg-[#FFB800]/10 text-[#FFB800]";
    default:
      return "bg-white/5 text-muted-foreground";
  }
}

function mapApiIntentToLocal(apiIntent: ApiIntent): Intent {
  let intentType: string = apiIntent.intent_type ?? "";
  if (intentType.startsWith("PAYIN")) intentType = "PAYIN";
  else if (intentType.startsWith("PAYOUT")) intentType = "PAYOUT";
  else if (intentType.startsWith("TRADE")) intentType = "TRADE";
  else if (intentType.startsWith("DEPOSIT")) intentType = "DEPOSIT";
  else if (intentType.startsWith("WITHDRAW")) intentType = "WITHDRAW";

  return {
    id: apiIntent.id,
    intentType,
    state: apiIntent.state,
    amount: apiIntent.amount,
    currency: apiIntent.currency,
    createdAt: apiIntent.created_at,
    referenceCode: apiIntent.reference_code,
  };
}

export default function IntentsPage() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const pageParam = searchParams?.get("page");
  const perPageParam = searchParams?.get("per_page");

  const [intents, setIntents] = useState<Intent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { toast } = useToast();
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState({ type: "", state: "" });

  const [{ pageIndex, pageSize }, setPagination] = useState<PaginationState>({
    pageIndex: pageParam ? parseInt(pageParam) - 1 : 0,
    pageSize: perPageParam ? parseInt(perPageParam) : 10,
  });
  const [pageCount, setPageCount] = useState(0);

  const tCommon = useTranslations("Common");
  const format = useFormatter();

  const formatAmount = (amount: string, currency: string) => {
    const num = parseInt(amount, 10);
    if (currency === "VND") {
      return format.number(num, {
        style: "currency",
        currency: "VND",
        maximumFractionDigits: 0,
      });
    }
    return `${amount} ${currency}`;
  };

  const formatDate = (dateStr: string) =>
    format.dateTime(new Date(dateStr), {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });

  const columns: ColumnDef<Intent>[] = useMemo(
    () => [
      {
        accessorKey: "id",
        header: "ID",
        cell: ({ row }) => {
          const id = row.getValue("id") as string;
          return (
            <Link
              href={`/intents/${id}`}
              className="font-mono text-xs text-[#00D4FF] hover:underline"
              title={id}
            >
              {id.substring(0, 8)}...
            </Link>
          );
        },
      },
      {
        accessorKey: "intentType",
        header: "Type",
        cell: ({ row }) => {
          const type = row.getValue("intentType") as string;
          return (
            <span
              className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${getTypeStyle(type)}`}
            >
              {type}
            </span>
          );
        },
      },
      {
        accessorKey: "state",
        header: "State",
        cell: ({ row }) => <StatusBadge status={row.getValue("state") as string} />,
      },
      {
        accessorKey: "amount",
        header: () => <div className="text-right">{tCommon("amount")}</div>,
        cell: ({ row }) => (
          <div className="text-right font-mono tabular-nums text-sm">
            {formatAmount(row.getValue("amount") as string, row.original.currency)}
          </div>
        ),
        sortingFn: (rowA, rowB) =>
          parseInt(rowA.getValue("amount") as string, 10) -
          parseInt(rowB.getValue("amount") as string, 10),
      },
      {
        accessorKey: "referenceCode",
        header: "Reference",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground">
            {(row.getValue("referenceCode") as string | undefined) || "-"}
          </span>
        ),
      },
      {
        accessorKey: "createdAt",
        header: "Created",
        cell: ({ row }) => (
          <span className="text-muted-foreground text-sm whitespace-nowrap">
            {formatDate(row.getValue("createdAt") as string)}
          </span>
        ),
        sortingFn: (rowA, rowB) =>
          new Date(rowA.getValue("createdAt") as string).getTime() -
          new Date(rowB.getValue("createdAt") as string).getTime(),
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [tCommon]
  );

  const fetchIntents = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // Admin ops view: pull the full working set in one bounded request (the
      // backend caps the scan and clamps per_page to 200) and let the DataTable
      // paginate client-side. This keeps the total count and page navigation
      // correct — server-side paging here showed only the first page as the
      // whole dataset because the table isn't wired for manual pagination.
      const response = await intentsApi.list({
        page: 1,
        per_page: 200,
        status: filter.state || undefined,
        intent_type: filter.type || undefined,
      });
      setIntents(response.data.map(mapApiIntentToLocal));
      setPageCount(Math.max(1, Math.ceil(response.data.length / pageSize)));
    } catch (err: any) {
      console.error("Failed to fetch intents:", err);
      const message = err.message || "Failed to load intents";
      setError(message);
      toast({ variant: "destructive", title: tCommon("error"), description: message });
    } finally {
      setLoading(false);
    }
  }, [pageSize, filter.state, filter.type, toast, tCommon]);

  useEffect(() => {
    fetchIntents();
  }, [fetchIntents]);

  const filteredIntents = useMemo(
    () =>
      intents.filter((intent) => {
        if (
          search &&
          !intent.id.toLowerCase().includes(search.toLowerCase()) &&
          !intent.referenceCode?.toLowerCase().includes(search.toLowerCase())
        )
          return false;
        return true;
      }),
    [intents, search]
  );

  const filterControls = (
    <div className="flex flex-wrap gap-2">
      <div className="relative">
        <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground pointer-events-none" />
        <Input
          placeholder="Search by ID or reference..."
          className="pl-9 w-64 border-white/[0.08] bg-[#111113] text-sm"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>
      <select
        className="rounded-md border border-white/[0.08] bg-background px-3 py-1.5 text-sm"
        value={filter.type}
        onChange={(e) => {
          setFilter({ ...filter, type: e.target.value });
          setPagination((p) => ({ ...p, pageIndex: 0 }));
        }}
      >
        <option value="">All Types</option>
        <option value="PAYIN">Pay-in</option>
        <option value="PAYOUT">Pay-out</option>
        <option value="TRADE">Trade</option>
      </select>
      <select
        className="rounded-md border border-white/[0.08] bg-background px-3 py-1.5 text-sm"
        value={filter.state}
        onChange={(e) => {
          setFilter({ ...filter, state: e.target.value });
          setPagination((p) => ({ ...p, pageIndex: 0 }));
        }}
      >
        <option value="">All States</option>
        <option value="PENDING_BANK">Pending Bank</option>
        <option value="BANK_CONFIRMED">Bank Confirmed</option>
        <option value="PENDING_RAILS">Pending Rails</option>
        <option value="COMPLETED">Completed</option>
        <option value="RAILS_FAILED">Failed</option>
        <option value="EXPIRED">Expired</option>
      </select>
    </div>
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Intents"
        description="View and manage payment intents"
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={fetchIntents}
            disabled={loading}
          >
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      {error ? (
        <ErrorState message={error} retry={fetchIntents} />
      ) : (
        <Panel header={{ title: "Payment Intents", actions: filterControls }}>
          <DataTable
            columns={columns}
            data={filteredIntents}
            loading={loading}
            pagination
            pageSize={pageSize}
            emptyState={
              <EmptyState
                title="No intents found"
                description="No payment intents match the current filters."
              />
            }
          />
        </Panel>
      )}
    </main>
  );
}
