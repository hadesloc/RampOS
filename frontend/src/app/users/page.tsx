"use client";

import { useState } from "react";

interface User {
  id: string;
  externalId: string;
  kycTier: number;
  kycStatus: string;
  status: string;
  dailyPayinLimitVnd: string;
  dailyPayoutLimitVnd: string;
  createdAt: string;
}

// Mock data
const mockUsers: User[] = [
  {
    id: "user_01HYJ2KM3N4P5Q6R7S8T9U0A",
    externalId: "EXT_USER_001",
    kycTier: 2,
    kycStatus: "APPROVED",
    status: "ACTIVE",
    dailyPayinLimitVnd: "500000000",
    dailyPayoutLimitVnd: "200000000",
    createdAt: "2026-01-15T10:00:00Z",
  },
  {
    id: "user_01HYJ2KM3N4P5Q6R7S8T9U0B",
    externalId: "EXT_USER_002",
    kycTier: 1,
    kycStatus: "PENDING",
    status: "ACTIVE",
    dailyPayinLimitVnd: "100000000",
    dailyPayoutLimitVnd: "50000000",
    createdAt: "2026-01-20T14:30:00Z",
  },
  {
    id: "user_01HYJ2KM3N4P5Q6R7S8T9U0C",
    externalId: "EXT_USER_003",
    kycTier: 0,
    kycStatus: "NOT_STARTED",
    status: "ACTIVE",
    dailyPayinLimitVnd: "10000000",
    dailyPayoutLimitVnd: "5000000",
    createdAt: "2026-01-22T09:15:00Z",
  },
  {
    id: "user_01HYJ2KM3N4P5Q6R7S8T9U0D",
    externalId: "EXT_USER_004",
    kycTier: 3,
    kycStatus: "APPROVED",
    status: "SUSPENDED",
    dailyPayinLimitVnd: "1000000000",
    dailyPayoutLimitVnd: "500000000",
    createdAt: "2026-01-10T08:00:00Z",
  },
];

function formatVnd(value: string): string {
  const num = parseInt(value, 10);
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
    maximumFractionDigits: 0,
  }).format(num);
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
}

function getKycStatusColor(status: string): string {
  switch (status) {
    case "APPROVED":
      return "bg-green-100 text-green-800";
    case "PENDING":
      return "bg-yellow-100 text-yellow-800";
    case "REJECTED":
      return "bg-red-100 text-red-800";
    case "NOT_STARTED":
      return "bg-gray-100 text-gray-800";
    default:
      return "bg-gray-100 text-gray-800";
  }
}

function getStatusColor(status: string): string {
  switch (status) {
    case "ACTIVE":
      return "bg-green-100 text-green-800";
    case "SUSPENDED":
      return "bg-red-100 text-red-800";
    case "INACTIVE":
      return "bg-gray-100 text-gray-800";
    default:
      return "bg-gray-100 text-gray-800";
  }
}

function getTierLabel(tier: number): string {
  switch (tier) {
    case 0:
      return "Tier 0 (Basic)";
    case 1:
      return "Tier 1 (Phone)";
    case 2:
      return "Tier 2 (ID)";
    case 3:
      return "Tier 3 (Full)";
    default:
      return `Tier ${tier}`;
  }
}

export default function UsersPage() {
  const [users] = useState<User[]>(mockUsers);
  const [filter, setFilter] = useState({
    kycTier: "",
    status: "",
  });
  const [search, setSearch] = useState("");

  const filteredUsers = users.filter((user) => {
    if (filter.kycTier && user.kycTier !== parseInt(filter.kycTier)) return false;
    if (filter.status && user.status !== filter.status) return false;
    if (
      search &&
      !user.externalId.toLowerCase().includes(search.toLowerCase()) &&
      !user.id.toLowerCase().includes(search.toLowerCase())
    )
      return false;
    return true;
  });

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Users</h1>
        <p className="text-muted-foreground">
          Manage users and their KYC status
        </p>
      </div>

      {/* Filters */}
      <div className="flex gap-4">
        <input
          type="text"
          placeholder="Search by ID..."
          className="rounded-md border px-3 py-2 text-sm w-64"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />

        <select
          className="rounded-md border px-3 py-2 text-sm"
          value={filter.kycTier}
          onChange={(e) => setFilter({ ...filter, kycTier: e.target.value })}
        >
          <option value="">All Tiers</option>
          <option value="0">Tier 0</option>
          <option value="1">Tier 1</option>
          <option value="2">Tier 2</option>
          <option value="3">Tier 3</option>
        </select>

        <select
          className="rounded-md border px-3 py-2 text-sm"
          value={filter.status}
          onChange={(e) => setFilter({ ...filter, status: e.target.value })}
        >
          <option value="">All Statuses</option>
          <option value="ACTIVE">Active</option>
          <option value="SUSPENDED">Suspended</option>
          <option value="INACTIVE">Inactive</option>
        </select>
      </div>

      {/* Table */}
      <div className="rounded-md border">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="px-4 py-3 text-left font-medium">External ID</th>
              <th className="px-4 py-3 text-left font-medium">KYC Tier</th>
              <th className="px-4 py-3 text-left font-medium">KYC Status</th>
              <th className="px-4 py-3 text-left font-medium">Status</th>
              <th className="px-4 py-3 text-right font-medium">Daily Payin Limit</th>
              <th className="px-4 py-3 text-right font-medium">Daily Payout Limit</th>
              <th className="px-4 py-3 text-left font-medium">Created</th>
            </tr>
          </thead>
          <tbody>
            {filteredUsers.map((user) => (
              <tr key={user.id} className="border-t hover:bg-muted/30">
                <td className="px-4 py-3 font-mono text-sm">{user.externalId}</td>
                <td className="px-4 py-3">{getTierLabel(user.kycTier)}</td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getKycStatusColor(
                      user.kycStatus
                    )}`}
                  >
                    {user.kycStatus}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getStatusColor(
                      user.status
                    )}`}
                  >
                    {user.status}
                  </span>
                </td>
                <td className="px-4 py-3 text-right font-mono text-sm">
                  {formatVnd(user.dailyPayinLimitVnd)}
                </td>
                <td className="px-4 py-3 text-right font-mono text-sm">
                  {formatVnd(user.dailyPayoutLimitVnd)}
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {formatDate(user.createdAt)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {filteredUsers.length === 0 && (
        <div className="text-center py-8 text-muted-foreground">
          No users found matching the filters.
        </div>
      )}
    </div>
  );
}
