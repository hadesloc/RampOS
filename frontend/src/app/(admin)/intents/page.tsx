"use client";

import { useState } from "react";
import Link from "next/link";
import { PageHeader } from "@/components/layout/page-header";
import { StatusBadge } from "@/components/dashboard/status-badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Filter, Search } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

interface Intent {
  id: string;
  intentType: string;
  state: string;
  amount: string;
  currency: string;
  createdAt: string;
  referenceCode?: string;
}

// Mock data
const mockIntents: Intent[] = [
  {
    id: "intent_01HYJ2KM3N4P5Q6R7S8T9U0V",
    intentType: "PAYIN",
    state: "BANK_CONFIRMED",
    amount: "5000000",
    currency: "VND",
    createdAt: "2026-01-23T10:30:00Z",
    referenceCode: "PAY123456",
  },
  {
    id: "intent_01HYJ2KM3N4P5Q6R7S8T9U1W",
    intentType: "PAYOUT",
    state: "PENDING_RAILS",
    amount: "2000000",
    currency: "VND",
    createdAt: "2026-01-23T10:25:00Z",
    referenceCode: "POT789012",
  },
  {
    id: "intent_01HYJ2KM3N4P5Q6R7S8T9U2X",
    intentType: "TRADE",
    state: "COMPLETED",
    amount: "10000000",
    currency: "VND",
    createdAt: "2026-01-23T10:20:00Z",
  },
  {
    id: "intent_01HYJ2KM3N4P5Q6R7S8T9U3Y",
    intentType: "PAYIN",
    state: "PENDING_BANK",
    amount: "3500000",
    currency: "VND",
    createdAt: "2026-01-23T10:15:00Z",
    referenceCode: "PAY345678",
  },
  {
    id: "intent_01HYJ2KM3N4P5Q6R7S8T9U4Z",
    intentType: "PAYOUT",
    state: "RAILS_FAILED",
    amount: "1000000",
    currency: "VND",
    createdAt: "2026-01-23T10:10:00Z",
    referenceCode: "POT901234",
  },
];

function formatAmount(amount: string, currency: string): string {
  const num = parseInt(amount, 10);
  if (currency === "VND") {
    return new Intl.NumberFormat("vi-VN", {
      style: "currency",
      currency: "VND",
      maximumFractionDigits: 0,
    }).format(num);
  }
  return `${amount} ${currency}`;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function getTypeColor(type: string): string {
  switch (type) {
    case "PAYIN":
      return "bg-blue-100 text-blue-800 dark:bg-blue-500/15 dark:text-blue-400 border-transparent hover:bg-blue-100/80";
    case "PAYOUT":
      return "bg-purple-100 text-purple-800 dark:bg-purple-500/15 dark:text-purple-400 border-transparent hover:bg-purple-100/80";
    case "TRADE":
      return "bg-orange-100 text-orange-800 dark:bg-orange-500/15 dark:text-orange-400 border-transparent hover:bg-orange-100/80";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300 border-transparent";
  }
}

export default function IntentsPage() {
  const [intents] = useState<Intent[]>(mockIntents);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState({
    type: "",
    state: "",
  });

  const filteredIntents = intents.filter((intent) => {
    if (filter.type && intent.intentType !== filter.type) return false;
    if (filter.state && intent.state !== filter.state) return false;
    if (search && !intent.id.toLowerCase().includes(search.toLowerCase()) && !intent.referenceCode?.toLowerCase().includes(search.toLowerCase())) return false;
    return true;
  });

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title="Intents"
        description="View and manage payment intents"
        breadcrumbs={[
          { label: "Dashboard", href: "/" },
          { label: "Intents" }
        ]}
      />

      <Card>
        <CardContent className="p-4 space-y-4">
          {/* Filters */}
          <div className="flex flex-col sm:flex-row gap-4 justify-between">
            <div className="relative w-full sm:w-96">
              <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Search by ID or reference..."
                className="pl-9"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>
            <div className="flex gap-2 w-full sm:w-auto">
              <div className="relative flex-1 sm:flex-none">
                <select
                  className="h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                  value={filter.type}
                  onChange={(e) => setFilter({ ...filter, type: e.target.value })}
                >
                  <option value="">All Types</option>
                  <option value="PAYIN">Pay-in</option>
                  <option value="PAYOUT">Pay-out</option>
                  <option value="TRADE">Trade</option>
                </select>
              </div>

              <div className="relative flex-1 sm:flex-none">
                <select
                  className="h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                  value={filter.state}
                  onChange={(e) => setFilter({ ...filter, state: e.target.value })}
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
              <Button variant="outline" size="icon">
                <Filter className="h-4 w-4" />
              </Button>
            </div>
          </div>

          {/* Table */}
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[100px]">ID</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>State</TableHead>
                  <TableHead className="text-right">Amount</TableHead>
                  <TableHead>Reference</TableHead>
                  <TableHead>Created</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredIntents.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={6} className="h-24 text-center text-muted-foreground">
                      No intents found matching the filters.
                    </TableCell>
                  </TableRow>
                ) : (
                  filteredIntents.map((intent) => (
                    <TableRow key={intent.id}>
                      <TableCell>
                        <Link
                          href={`/intents/${intent.id}`}
                          className="font-mono text-xs text-primary hover:underline"
                          title={intent.id}
                        >
                          {intent.id.substring(0, 8)}...
                        </Link>
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline" className={getTypeColor(intent.intentType)}>
                          {intent.intentType}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <StatusBadge status={intent.state} />
                      </TableCell>
                      <TableCell className="text-right font-mono">
                        {formatAmount(intent.amount, intent.currency)}
                      </TableCell>
                      <TableCell className="font-mono text-xs text-muted-foreground">
                        {intent.referenceCode || "-"}
                      </TableCell>
                      <TableCell className="text-muted-foreground text-sm">
                        {formatDate(intent.createdAt)}
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
