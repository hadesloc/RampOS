"use client";

import { useState } from "react";
import Link from "next/link";

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

function getStateColor(state: string): string {
  switch (state) {
    case "COMPLETED":
    case "BANK_CONFIRMED":
      return "bg-green-100 text-green-800";
    case "PENDING_BANK":
    case "PENDING_RAILS":
      return "bg-yellow-100 text-yellow-800";
    case "RAILS_FAILED":
    case "EXPIRED":
      return "bg-red-100 text-red-800";
    default:
      return "bg-gray-100 text-gray-800";
  }
}

function getTypeColor(type: string): string {
  switch (type) {
    case "PAYIN":
      return "bg-blue-100 text-blue-800";
    case "PAYOUT":
      return "bg-purple-100 text-purple-800";
    case "TRADE":
      return "bg-orange-100 text-orange-800";
    default:
      return "bg-gray-100 text-gray-800";
  }
}

export default function IntentsPage() {
  const [intents] = useState<Intent[]>(mockIntents);
  const [filter, setFilter] = useState({
    type: "",
    state: "",
  });

  const filteredIntents = intents.filter((intent) => {
    if (filter.type && intent.intentType !== filter.type) return false;
    if (filter.state && intent.state !== filter.state) return false;
    return true;
  });

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Intents</h1>
        <p className="text-muted-foreground">
          View and manage payment intents
        </p>
      </div>

      {/* Filters */}
      <div className="flex gap-4">
        <select
          className="rounded-md border px-3 py-2 text-sm"
          value={filter.type}
          onChange={(e) => setFilter({ ...filter, type: e.target.value })}
        >
          <option value="">All Types</option>
          <option value="PAYIN">Pay-in</option>
          <option value="PAYOUT">Pay-out</option>
          <option value="TRADE">Trade</option>
        </select>

        <select
          className="rounded-md border px-3 py-2 text-sm"
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

      {/* Table */}
      <div className="rounded-md border">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="px-4 py-3 text-left font-medium">ID</th>
              <th className="px-4 py-3 text-left font-medium">Type</th>
              <th className="px-4 py-3 text-left font-medium">State</th>
              <th className="px-4 py-3 text-right font-medium">Amount</th>
              <th className="px-4 py-3 text-left font-medium">Reference</th>
              <th className="px-4 py-3 text-left font-medium">Created</th>
            </tr>
          </thead>
          <tbody>
            {filteredIntents.map((intent) => (
              <tr key={intent.id} className="border-t hover:bg-muted/30">
                <td className="px-4 py-3">
                  <Link
                    href={`/intents/${intent.id}`}
                    className="font-mono text-xs text-blue-600 hover:underline"
                  >
                    {intent.id.substring(0, 20)}...
                  </Link>
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getTypeColor(
                      intent.intentType
                    )}`}
                  >
                    {intent.intentType}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getStateColor(
                      intent.state
                    )}`}
                  >
                    {intent.state}
                  </span>
                </td>
                <td className="px-4 py-3 text-right font-mono">
                  {formatAmount(intent.amount, intent.currency)}
                </td>
                <td className="px-4 py-3 font-mono text-xs">
                  {intent.referenceCode || "-"}
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {formatDate(intent.createdAt)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {filteredIntents.length === 0 && (
        <div className="text-center py-8 text-muted-foreground">
          No intents found matching the filters.
        </div>
      )}
    </div>
  );
}
