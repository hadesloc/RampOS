"use client";

import { useState, useEffect } from "react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { MoreHorizontal, Plus, Settings2, Loader2, RefreshCw } from "lucide-react";
import { usersApi, type User } from "@/lib/api";
import { useToast } from "@/components/ui/use-toast";

function formatVnd(value?: string): string {
  if (!value) return "0 ₫";
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
      return "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400";
    case "PENDING":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400";
    case "REJECTED":
      return "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400";
    case "NOT_STARTED":
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

function getStatusColor(status: string): string {
  switch (status) {
    case "ACTIVE":
      return "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400";
    case "SUSPENDED":
      return "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400";
    case "INACTIVE":
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
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

function getInitials(id: string): string {
    return id.substring(0, 2).toUpperCase() || "US";
}

export default function UsersPage() {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { toast } = useToast();

  const [filter, setFilter] = useState({
    kycTier: "",
    status: "",
  });
  const [search, setSearch] = useState("");
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [visibleColumns, setVisibleColumns] = useState<Record<string, boolean>>({
    avatar: true,
    id: true,
    kycTier: true,
    kycStatus: true,
    status: true,
    payinLimit: true,
    payoutLimit: true,
    created: true,
    actions: true,
  });
  const [newUser, setNewUser] = useState({
      externalId: "",
      kycTier: "0",
  });

  const fetchUsers = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await usersApi.list({
        status: filter.status || undefined,
        // kyc_status: filter.kycStatus || undefined, // Not in filter state yet but supported by API
      });
      setUsers(response.data);
    } catch (err: any) {
      console.error("Failed to fetch users:", err);
      setError(err.message || "Failed to load users");
      toast({
        variant: "destructive",
        title: "Error",
        description: err.message || "Failed to load users",
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchUsers();
  }, [filter.status]); // Refetch when filter changes

  // Client-side filtering for other fields
  const filteredUsers = users.filter((user) => {
    if (filter.kycTier && user.kyc_tier !== parseInt(filter.kycTier)) return false;
    if (search && !user.id.toLowerCase().includes(search.toLowerCase())) return false;
    return true;
  });

  const handleCreateUser = () => {
    // API doesn't have create user yet, keeping as placeholder
    toast({
      title: "Coming Soon",
      description: "User creation API is not yet available.",
    });
    setIsCreateOpen(false);
  };

  const handleRefresh = () => {
    fetchUsers();
  };

  return (
    <div className="space-y-6 p-4">
      <div className="flex justify-between items-center">
        <div>
            <h1 className="text-3xl font-bold tracking-tight">Users</h1>
            <p className="text-muted-foreground">
            Manage users and their KYC status
            </p>
        </div>
        <div className="flex gap-2">
            <Button variant="outline" size="icon" onClick={handleRefresh} disabled={loading}>
                <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
            </Button>
            <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
                <DialogTrigger asChild>
                    <Button>
                        <Plus className="mr-2 h-4 w-4" /> Create User
                    </Button>
                </DialogTrigger>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>Create New User</DialogTitle>
                        <DialogDescription>
                            Add a new user to the system. They will start with Tier 0.
                        </DialogDescription>
                    </DialogHeader>
                    <div className="grid gap-4 py-4">
                        <div className="grid grid-cols-4 items-center gap-4">
                            <Label htmlFor="externalId" className="text-right">
                                External ID
                            </Label>
                            <Input
                                id="externalId"
                                value={newUser.externalId}
                                onChange={(e) => setNewUser({...newUser, externalId: e.target.value})}
                                className="col-span-3"
                            />
                        </div>
                         <div className="grid grid-cols-4 items-center gap-4">
                            <Label htmlFor="kycTier" className="text-right">
                                Initial Tier
                            </Label>
                            <select
                                id="kycTier"
                                className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 col-span-3"
                                value={newUser.kycTier}
                                onChange={(e) => setNewUser({ ...newUser, kycTier: e.target.value })}
                            >
                                <option value="0">Tier 0</option>
                                <option value="1">Tier 1</option>
                                <option value="2">Tier 2</option>
                                <option value="3">Tier 3</option>
                            </select>
                        </div>
                    </div>
                    <DialogFooter>
                        <Button onClick={handleCreateUser}>Create User</Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-4 items-center flex-wrap">
        <Input
          placeholder="Search by ID..."
          className="max-w-xs"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />

        <select
          className="h-10 rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
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
          className="h-10 rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          value={filter.status}
          onChange={(e) => setFilter({ ...filter, status: e.target.value })}
        >
          <option value="">All Statuses</option>
          <option value="ACTIVE">Active</option>
          <option value="SUSPENDED">Suspended</option>
          <option value="INACTIVE">Inactive</option>
        </select>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm" className="ml-auto flex h-10">
              <Settings2 className="mr-2 h-4 w-4" />
              View
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-[150px]">
            <DropdownMenuLabel>Toggle columns</DropdownMenuLabel>
            <DropdownMenuSeparator />
            {Object.keys(visibleColumns).map((column) => (
              <DropdownMenuItem
                key={column}
                className="capitalize"
                onSelect={(e) => {
                    e.preventDefault();
                    setVisibleColumns(prev => ({ ...prev, [column]: !prev[column] }));
                }}
              >
                <input
                  type="checkbox"
                  className="mr-2"
                  checked={visibleColumns[column]}
                  readOnly
                />
                {column.replace(/([A-Z])/g, " $1")}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {/* Table */}
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              {visibleColumns.avatar && <TableHead className="w-[80px]">Avatar</TableHead>}
              {visibleColumns.id && <TableHead>ID</TableHead>}
              {visibleColumns.kycTier && <TableHead>KYC Tier</TableHead>}
              {visibleColumns.kycStatus && <TableHead>KYC Status</TableHead>}
              {visibleColumns.status && <TableHead>Status</TableHead>}
              {visibleColumns.payinLimit && <TableHead className="text-right">Daily Payin Limit</TableHead>}
              {visibleColumns.payoutLimit && <TableHead className="text-right">Daily Payout Limit</TableHead>}
              {visibleColumns.created && <TableHead>Created</TableHead>}
              {visibleColumns.actions && <TableHead className="w-[50px]"></TableHead>}
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
                <TableRow>
                    <TableCell colSpan={9} className="h-24 text-center">
                        <div className="flex justify-center items-center gap-2">
                            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                            <span className="text-muted-foreground">Loading users...</span>
                        </div>
                    </TableCell>
                </TableRow>
            ) : filteredUsers.length === 0 ? (
                <TableRow>
                    <TableCell colSpan={9} className="h-24 text-center text-muted-foreground">
                        No users found matching the filters.
                    </TableCell>
                </TableRow>
            ) : (
                filteredUsers.map((user) => (
              <TableRow key={user.id}>
                {visibleColumns.avatar && (
                  <TableCell>
                    <Avatar>
                      <AvatarImage src={`https://api.dicebear.com/7.x/avataaars/svg?seed=${user.id}`} alt={user.id} />
                      <AvatarFallback>{getInitials(user.id)}</AvatarFallback>
                    </Avatar>
                  </TableCell>
                )}
                {visibleColumns.id && (
                  <TableCell className="font-mono text-xs">{user.id}</TableCell>
                )}
                {visibleColumns.kycTier && (
                  <TableCell>{getTierLabel(user.kyc_tier)}</TableCell>
                )}
                {visibleColumns.kycStatus && (
                  <TableCell>
                    <span
                      className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getKycStatusColor(
                        user.kyc_status
                      )}`}
                    >
                      {user.kyc_status}
                    </span>
                  </TableCell>
                )}
                {visibleColumns.status && (
                  <TableCell>
                    <span
                      className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${getStatusColor(
                        user.status
                      )}`}
                    >
                      {user.status}
                    </span>
                  </TableCell>
                )}
                {visibleColumns.payinLimit && (
                  <TableCell className="text-right font-mono text-sm">
                    {formatVnd(user.daily_payin_limit_vnd)}
                  </TableCell>
                )}
                {visibleColumns.payoutLimit && (
                  <TableCell className="text-right font-mono text-sm">
                    {formatVnd(user.daily_payout_limit_vnd)}
                  </TableCell>
                )}
                {visibleColumns.created && (
                  <TableCell className="text-muted-foreground">
                    {formatDate(user.created_at)}
                  </TableCell>
                )}
                {visibleColumns.actions && (
                  <TableCell>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" className="h-8 w-8 p-0">
                          <span className="sr-only">Open menu</span>
                          <MoreHorizontal className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuLabel>Actions</DropdownMenuLabel>
                        <DropdownMenuItem onClick={() => alert(`View details for ${user.id}`)}>
                          View Details
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={() => alert(`Edit ${user.id}`)}>
                          Edit User
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          className="text-red-600 focus:text-red-600"
                          onClick={() => alert(`Suspend ${user.id}`)}
                        >
                          Suspend User
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                )}
              </TableRow>
            )))}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
