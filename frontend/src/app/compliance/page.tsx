"use client";

import { useState } from "react";
import Link from "next/link";

interface Case {
  id: string;
  caseType: string;
  severity: string;
  status: string;
  assignedTo?: string;
  userId?: string;
  intentId?: string;
  createdAt: string;
  updatedAt: string;
}

// Mock data
const mockCases: Case[] = [
  {
    id: "case_01HYJ2KM3N4P5Q6R7S8T9U0A",
    caseType: "VELOCITY",
    severity: "HIGH",
    status: "OPEN",
    assignedTo: "analyst@rampos.io",
    userId: "user_123",
    intentId: "intent_456",
    createdAt: "2026-01-23T09:00:00Z",
    updatedAt: "2026-01-23T09:30:00Z",
  },
  {
    id: "case_01HYJ2KM3N4P5Q6R7S8T9U0B",
    caseType: "LARGE_TRANSACTION",
    severity: "CRITICAL",
    status: "REVIEW",
    assignedTo: "senior@rampos.io",
    userId: "user_456",
    intentId: "intent_789",
    createdAt: "2026-01-23T08:00:00Z",
    updatedAt: "2026-01-23T10:00:00Z",
  },
  {
    id: "case_01HYJ2KM3N4P5Q6R7S8T9U0C",
    caseType: "STRUCTURING",
    severity: "MEDIUM",
    status: "HOLD",
    userId: "user_789",
    createdAt: "2026-01-22T15:00:00Z",
    updatedAt: "2026-01-23T08:00:00Z",
  },
  {
    id: "case_01HYJ2KM3N4P5Q6R7S8T9U0D",
    caseType: "UNUSUAL_PAYOUT",
    severity: "LOW",
    status: "RELEASED",
    assignedTo: "analyst@rampos.io",
    userId: "user_101",
    createdAt: "2026-01-22T10:00:00Z",
    updatedAt: "2026-01-22T16:00:00Z",
  },
];

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function getSeverityColor(severity: string): string {
  switch (severity) {
    case "CRITICAL":
      return "bg-red-100 text-red-800";
    case "HIGH":
      return "bg-orange-100 text-orange-800";
    case "MEDIUM":
      return "bg-yellow-100 text-yellow-800";
    case "LOW":
      return "bg-green-100 text-green-800";
    default:
      return "bg-gray-100 text-gray-800";
  }
}

function getStatusColor(status: string): string {
  switch (status) {
    case "OPEN":
      return "bg-blue-100 text-blue-800";
    case "REVIEW":
      return "bg-purple-100 text-purple-800";
    case "HOLD":
      return "bg-yellow-100 text-yellow-800";
    case "RELEASED":
      return "bg-green-100 text-green-800";
    case "REPORTED":
      return "bg-red-100 text-red-800";
    default:
      return "bg-gray-100 text-gray-800";
  }
}

export default function CompliancePage() {
  const [cases] = useState<Case[]>(mockCases);
  const [filter, setFilter] = useState({
    severity: "",
    status: "",
  });

  const filteredCases = cases.filter((c) => {
    if (filter.severity && c.severity !== filter.severity) return false;
    if (filter.status && c.status !== filter.status) return false;
    return true;
  });

  const stats = {
    total: cases.length,
    open: cases.filter((c) => c.status === "OPEN").length,
    critical: cases.filter((c) => c.severity === "CRITICAL").length,
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Compliance</h1>
        <p className="text-muted-foreground">
          AML case management and monitoring
        </p>
      </div>

      {/* Stats */}
      <div className="grid gap-4 md:grid-cols-3">
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Total Cases</div>
          <div className="text-2xl font-bold">{stats.total}</div>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Open Cases</div>
          <div className="text-2xl font-bold text-blue-600">{stats.open}</div>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="text-sm text-muted-foreground">Critical</div>
          <div className="text-2xl font-bold text-red-600">{stats.critical}</div>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-4">
        <select
          className="rounded-md border px-3 py-2 text-sm"
          value={filter.severity}
          onChange={(e) => setFilter({ ...filter, severity: e.target.value })}
        >
          <option value="">All Severities</option>
          <option value="CRITICAL">Critical</option>
          <option value="HIGH">High</option>
          <option value="MEDIUM">Medium</option>
          <option value="LOW">Low</option>
        </select>

        <select
          className="rounded-md border px-3 py-2 text-sm"
          value={filter.status}
          onChange={(e) => setFilter({ ...filter, status: e.target.value })}
        >
          <option value="">All Statuses</option>
          <option value="OPEN">Open</option>
          <option value="REVIEW">Review</option>
          <option value="HOLD">Hold</option>
          <option value="RELEASED">Released</option>
          <option value="REPORTED">Reported</option>
        </select>
      </div>

      {/* Table */}
      <div className="rounded-md border">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="px-4 py-3 text-left font-medium">Case ID</th>
              <th className="px-4 py-3 text-left font-medium">Type</th>
              <th className="px-4 py-3 text-left font-medium">Severity</th>
              <th className="px-4 py-3 text-left font-medium">Status</th>
              <th className="px-4 py-3 text-left font-medium">Assigned To</th>
              <th className="px-4 py-3 text-left font-medium">Created</th>
              <th className="px-4 py-3 text-left font-medium">Actions</th>
            </tr>
          </thead>
          <tbody>
            {filteredCases.map((c) => (
              <tr key={c.id} className="border-t hover:bg-muted/30">
                <td className="px-4 py-3">
                  <span className="font-mono text-xs">
                    {c.id.substring(0, 20)}...
                  </span>
                </td>
                <td className="px-4 py-3">{c.caseType}</td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getSeverityColor(
                      c.severity
                    )}`}
                  >
                    {c.severity}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getStatusColor(
                      c.status
                    )}`}
                  >
                    {c.status}
                  </span>
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {c.assignedTo || "Unassigned"}
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {formatDate(c.createdAt)}
                </td>
                <td className="px-4 py-3">
                  <button className="text-blue-600 hover:underline text-xs">
                    View
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {filteredCases.length === 0 && (
        <div className="text-center py-8 text-muted-foreground">
          No cases found matching the filters.
        </div>
      )}
    </div>
  );
}
