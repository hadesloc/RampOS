"use client";

import { useCallback, useEffect, useState } from "react";
import { ColumnDef } from "@tanstack/react-table";
import {
  MoreHorizontal,
  Plus,
  RefreshCw,
  Users,
  UserCheck,
  Clock,
  ShieldAlert,
} from "lucide-react";
import { useTranslations, useFormatter } from "next-intl";

import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  DataTable,
  Toolbar,
  StatusBadge,
  EmptyState,
  ErrorState,
} from "@/components/shared";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { usersApi, type User } from "@/lib/api";
import { useToast } from "@/components/ui/use-toast";

// ── Helpers ────────────────────────────────────────────────────────────────────

function getInitials(id: string): string {
  return id.substring(0, 2).toUpperCase() || "US";
}

function getTierLabel(tier: number): string {
  switch (tier) {
    case 0: return "Tier 0 — Basic";
    case 1: return "Tier 1 — Phone";
    case 2: return "Tier 2 — ID";
    case 3: return "Tier 3 — Full";
    default: return `Tier ${tier}`;
  }
}

function kycSeverity(status: string): "success" | "warning" | "danger" | "neutral" {
  if (status === "APPROVED") return "success";
  if (status === "PENDING") return "warning";
  if (status === "REJECTED") return "danger";
  return "neutral";
}

function userStatusSeverity(status: string): "success" | "danger" | "neutral" {
  if (status === "ACTIVE") return "success";
  if (status === "SUSPENDED") return "danger";
  return "neutral";
}

// ── Page ───────────────────────────────────────────────────────────────────────

export default function UsersPage() {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [tierFilter, setTierFilter] = useState("");
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [newUser, setNewUser] = useState({ externalId: "", kycTier: "0" });

  const { toast } = useToast();
  const t = useTranslations("Navigation");
  const tCommon = useTranslations("Common");
  const format = useFormatter();

  const formatVnd = (value?: string) => {
    if (!value) return "0 ₫";
    const num = parseInt(value, 10);
    return format.number(num, {
      style: "currency",
      currency: "VND",
      maximumFractionDigits: 0,
    });
  };

  const formatDateStr = (dateStr: string) =>
    format.dateTime(new Date(dateStr), {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
    });

  const fetchUsers = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await usersApi.list({
        status: statusFilter || undefined,
      });
      setUsers(response.data);
    } catch (err: any) {
      console.error("Failed to fetch users:", err);
      setError(err.message || tCommon("error"));
      toast({
        variant: "destructive",
        title: tCommon("error"),
        description: err.message || tCommon("error"),
      });
    } finally {
      setLoading(false);
    }
  }, [statusFilter, tCommon, toast]);

  useEffect(() => {
    fetchUsers();
  }, [fetchUsers]);

  // Client-side filtering
  const filteredUsers = users.filter((user) => {
    if (tierFilter && user.kyc_tier !== parseInt(tierFilter)) return false;
    if (
      search &&
      !user.id.toLowerCase().includes(search.toLowerCase())
    )
      return false;
    return true;
  });

  const handleCreateUser = () => {
    toast({
      title: "Coming Soon",
      description: "User creation API is not yet available.",
    });
    setIsCreateOpen(false);
  };

  // Derived stats
  const activeCount = users.filter((u) => u.status === "ACTIVE").length;
  const kycPendingCount = users.filter(
    (u) => u.kyc_status === "PENDING"
  ).length;
  const suspendedCount = users.filter(
    (u) => u.status === "SUSPENDED"
  ).length;

  // ── Columns ──────────────────────────────────────────────────────────────────

  const columns: ColumnDef<User>[] = [
    {
      id: "avatar",
      header: "",
      enableSorting: false,
      cell: ({ row }) => (
        <Avatar className="h-8 w-8">
          <AvatarImage
            src={`https://api.dicebear.com/7.x/avataaars/svg?seed=${row.original.id}`}
            alt={row.original.id}
          />
          <AvatarFallback className="text-xs bg-[#7B61FF]/20 text-[#7B61FF]">
            {getInitials(row.original.id)}
          </AvatarFallback>
        </Avatar>
      ),
    },
    {
      accessorKey: "id",
      header: "User ID",
      cell: ({ row }) => (
        <span className="font-mono text-xs text-muted-foreground">
          {row.original.id}
        </span>
      ),
    },
    {
      accessorKey: "kyc_tier",
      header: "KYC Tier",
      cell: ({ row }) => (
        <span className="text-sm">{getTierLabel(row.original.kyc_tier)}</span>
      ),
    },
    {
      accessorKey: "kyc_status",
      header: "KYC Status",
      cell: ({ row }) => (
        <StatusBadge
          status={row.original.kyc_status}
          severity={kycSeverity(row.original.kyc_status)}
        />
      ),
    },
    {
      accessorKey: "status",
      header: tCommon("status"),
      cell: ({ row }) => (
        <StatusBadge
          status={row.original.status}
          severity={userStatusSeverity(row.original.status)}
        />
      ),
    },
    {
      accessorKey: "daily_payin_limit_vnd",
      header: "Daily Pay-in Limit",
      cell: ({ row }) => (
        <span className="text-sm tabular-nums text-right block">
          {formatVnd(row.original.daily_payin_limit_vnd)}
        </span>
      ),
    },
    {
      accessorKey: "daily_payout_limit_vnd",
      header: "Daily Pay-out Limit",
      cell: ({ row }) => (
        <span className="text-sm tabular-nums text-right block">
          {formatVnd(row.original.daily_payout_limit_vnd)}
        </span>
      ),
    },
    {
      accessorKey: "created_at",
      header: tCommon("date"),
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground tabular-nums">
          {formatDateStr(row.original.created_at)}
        </span>
      ),
    },
    {
      id: "actions",
      header: "",
      enableSorting: false,
      cell: ({ row }) => (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="h-8 w-8 p-0">
              <span className="sr-only">Open menu</span>
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            align="end"
            className="border-white/[0.08] bg-[#111113]"
          >
            <DropdownMenuLabel className="text-xs text-muted-foreground">
              {tCommon("actions")}
            </DropdownMenuLabel>
            <DropdownMenuItem
              onClick={() => alert(`View details for ${row.original.id}`)}
            >
              {tCommon("view")}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => alert(`Edit ${row.original.id}`)}
            >
              {tCommon("edit")}
            </DropdownMenuItem>
            <DropdownMenuSeparator className="bg-white/[0.06]" />
            <DropdownMenuItem
              className="text-red-400 focus:text-red-400"
              onClick={() => alert(`Suspend ${row.original.id}`)}
            >
              Suspend User
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      ),
    },
  ];

  // ── Toolbar filters ───────────────────────────────────────────────────────────

  const filterSlot = (
    <div className="flex flex-wrap items-center gap-2">
      <select
        className="h-9 rounded-md border border-white/[0.08] bg-white/[0.02] px-3 text-sm text-foreground focus:outline-none focus:border-white/[0.2] transition-colors"
        value={tierFilter}
        onChange={(e) => setTierFilter(e.target.value)}
        aria-label="Filter by KYC tier"
      >
        <option value="">All Tiers</option>
        <option value="0">Tier 0</option>
        <option value="1">Tier 1</option>
        <option value="2">Tier 2</option>
        <option value="3">Tier 3</option>
      </select>

      <select
        className="h-9 rounded-md border border-white/[0.08] bg-white/[0.02] px-3 text-sm text-foreground focus:outline-none focus:border-white/[0.2] transition-colors"
        value={statusFilter}
        onChange={(e) => setStatusFilter(e.target.value)}
        aria-label="Filter by status"
      >
        <option value="">All Statuses</option>
        <option value="ACTIVE">Active</option>
        <option value="SUSPENDED">Suspended</option>
        <option value="INACTIVE">Inactive</option>
      </select>
    </div>
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title={t("users")}
        description="Manage users and their KYC status"
        breadcrumb={[
          { label: t("dashboard"), href: "/" },
          { label: t("users") },
        ]}
        actions={
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="icon"
              onClick={fetchUsers}
              disabled={loading}
              className="border-white/[0.08] hover:border-white/[0.16] hover:bg-white/[0.03] h-9 w-9"
            >
              <RefreshCw
                className={`h-4 w-4 ${loading ? "animate-spin" : ""}`}
              />
            </Button>

            <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
              <DialogTrigger asChild>
                <Button className="bg-[#00FF87] text-black font-semibold hover:bg-[#00FF87]/90 h-9">
                  <Plus className="mr-2 h-4 w-4" />
                  Create User
                </Button>
              </DialogTrigger>
              <DialogContent className="border-white/[0.08] bg-[#111113]">
                <DialogHeader>
                  <DialogTitle>Create New User</DialogTitle>
                  <DialogDescription>
                    Add a new user to the system. They will start with Tier 0.
                  </DialogDescription>
                </DialogHeader>
                <div className="grid gap-4 py-4">
                  <div className="grid grid-cols-4 items-center gap-4">
                    <Label htmlFor="externalId" className="text-right text-sm">
                      External ID
                    </Label>
                    <Input
                      id="externalId"
                      value={newUser.externalId}
                      onChange={(e) =>
                        setNewUser({ ...newUser, externalId: e.target.value })
                      }
                      className="col-span-3 border-white/[0.08] bg-white/[0.02]"
                    />
                  </div>
                  <div className="grid grid-cols-4 items-center gap-4">
                    <Label htmlFor="kycTier" className="text-right text-sm">
                      Initial Tier
                    </Label>
                    <select
                      id="kycTier"
                      className="col-span-3 flex h-10 w-full rounded-md border border-white/[0.08] bg-white/[0.02] px-3 py-2 text-sm focus:outline-none"
                      value={newUser.kycTier}
                      onChange={(e) =>
                        setNewUser({ ...newUser, kycTier: e.target.value })
                      }
                    >
                      <option value="0">Tier 0</option>
                      <option value="1">Tier 1</option>
                      <option value="2">Tier 2</option>
                      <option value="3">Tier 3</option>
                    </select>
                  </div>
                </div>
                <DialogFooter>
                  <Button
                    onClick={handleCreateUser}
                    className="bg-[#00FF87] text-black font-semibold hover:bg-[#00FF87]/90"
                  >
                    Create User
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </div>
        }
      />

      {/* KPIs */}
      <StatGrid cols={4}>
        <StatCard
          title="Total Users"
          value={loading ? "—" : users.length.toLocaleString()}
          icon={<Users className="h-4 w-4" />}
          accentColor="cyan"
          loading={loading}
        />
        <StatCard
          title="Active"
          value={loading ? "—" : activeCount.toLocaleString()}
          icon={<UserCheck className="h-4 w-4" />}
          accentColor="green"
          loading={loading}
        />
        <StatCard
          title="KYC Pending"
          value={loading ? "—" : kycPendingCount.toLocaleString()}
          icon={<Clock className="h-4 w-4" />}
          accentColor="amber"
          loading={loading}
        />
        <StatCard
          title="Suspended"
          value={loading ? "—" : suspendedCount.toLocaleString()}
          icon={<ShieldAlert className="h-4 w-4" />}
          accentColor="violet"
          loading={loading}
        />
      </StatGrid>

      {error && (
        <ErrorState
          title="Failed to load users"
          message={error}
          retry={fetchUsers}
        />
      )}

      <Panel header={{ title: "User Registry" }}>
        <Toolbar
          searchValue={search}
          onSearchChange={setSearch}
          searchPlaceholder="Search by ID…"
          filters={filterSlot}
          className="mb-4"
        />

        <DataTable
          columns={columns}
          data={filteredUsers}
          loading={loading}
          skeletonRows={8}
          pagination
          pageSize={10}
          emptyState={
            <EmptyState
              icon={<Users className="h-8 w-8" />}
              title="No users found"
              description={
                search
                  ? "No users match your search query."
                  : "No users have been registered yet."
              }
            />
          }
        />
      </Panel>
    </main>
  );
}
