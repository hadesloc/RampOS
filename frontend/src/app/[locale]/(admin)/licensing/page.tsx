"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import type { ColumnDef } from "@tanstack/react-table";
import {
  licensingApi,
  type LicenseStatus,
  type LicenseRequirement,
  type LicenseSubmission,
  type LicenseDeadline,
  type LicenseDashboardStats,
} from "@/lib/api";
import {
  AlertTriangle,
  Calendar,
  CheckCircle2,
  ChevronRight,
  Clock,
  Download,
  FileCheck,
  Filter,
  Loader2,
  RefreshCw,
  Shield,
  Upload,
  XCircle,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import { Checkbox } from "@/components/ui/checkbox";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  CardGridSkeleton,
  DataTable,
  EmptyState,
  ErrorState,
  PageHeader,
  Panel,
  SectionCard,
  StatCard,
  StatGrid,
  StatusBadge,
  TableSkeleton,
} from "@/components/shared";
import { formatDate, formatDateTime, formatNumber, toLabel } from "@/lib/format";
import { cn } from "@/lib/utils";

function getRequirementSeverity(status: string) {
  switch (status) {
    case "APPROVED":
      return "success" as const;
    case "SUBMITTED":
      return "info" as const;
    case "IN_PROGRESS":
      return "pending" as const;
    case "PENDING":
      return "warning" as const;
    case "REJECTED":
      return "danger" as const;
    default:
      return "neutral" as const;
  }
}

function getPriorityColor(priority: string): string {
  switch (priority) {
    case "CRITICAL":
      return "text-red-400";
    case "HIGH":
      return "text-orange-400";
    case "MEDIUM":
      return "text-[#FFB800]";
    case "LOW":
      return "text-[#00FF87]";
    default:
      return "text-muted-foreground";
  }
}

function getDeadlineUrgency(daysRemaining: number): string {
  if (daysRemaining < 0) return "text-red-400 font-semibold";
  if (daysRemaining <= 7) return "text-orange-400 font-medium";
  if (daysRemaining <= 30) return "text-[#FFB800]";
  return "text-muted-foreground";
}

function LicenseStatusCard({
  license,
  selected,
  onClick,
}: {
  license: LicenseStatus;
  selected?: boolean;
  onClick?: () => void;
}) {
  const progress = license.requirements_total > 0
    ? (license.requirements_completed / license.requirements_total) * 100
    : 0;

  return (
    <button
      type="button"
      className={cn(
        "rounded-xl border bg-[#111113]/80 p-5 text-left backdrop-blur-sm transition-all duration-300 hover:border-[#00D4FF]/35 hover:bg-[#00D4FF]/5",
        selected && "border-[#00FF87]/40 bg-[#00FF87]/10 shadow-[0_0_28px_rgba(0,255,135,0.08)]",
        license.status === "EXPIRED" && "border-red-400/25",
      )}
      onClick={onClick}
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="font-semibold text-foreground">{license.license_type}</h3>
          <p className="mt-1 text-sm text-muted-foreground">{license.jurisdiction}</p>
        </div>
        <StatusBadge status={license.status} />
      </div>
      <div className="mt-5 space-y-2">
        <div className="flex justify-between text-sm">
          <span className="text-muted-foreground">Requirements</span>
          <span className="font-medium tabular-nums">
            {formatNumber(license.requirements_completed)}/{formatNumber(license.requirements_total)}
          </span>
        </div>
        <div className="h-2 overflow-hidden rounded-full bg-white/[0.06]">
          <div
            className="h-full rounded-full bg-gradient-to-r from-[#00FF87] to-[#00D4FF]"
            style={{ width: `${progress}%` }}
          />
        </div>
        <div className="flex flex-wrap justify-between gap-2 text-xs text-muted-foreground">
          {license.issue_date && <span>Issued: {formatDate(license.issue_date)}</span>}
          {license.expiry_date && <span>Expires: {formatDate(license.expiry_date)}</span>}
        </div>
      </div>
    </button>
  );
}

function RequirementChecklist({
  requirements,
  loading,
  onStatusChange,
}: {
  requirements: LicenseRequirement[];
  loading: boolean;
  onStatusChange?: (id: string, status: string) => void;
}) {
  const [statusFilter, setStatusFilter] = useState<string>("");

  const filteredRequirements = statusFilter
    ? requirements.filter((r) => r.status === statusFilter)
    : requirements;

  const groupedByCategory = filteredRequirements.reduce((acc, req) => {
    if (!acc[req.category]) acc[req.category] = [];
    acc[req.category].push(req);
    return acc;
  }, {} as Record<string, LicenseRequirement[]>);

  if (loading) return <TableSkeleton rows={6} columns={4} />;

  return (
    <div className="space-y-5">
      <div className="flex items-center gap-2">
        <Filter className="h-4 w-4 text-[#00D4FF]" />
        <select
          className="rounded-md border border-white/[0.08] bg-[#09090B]/70 px-3 py-2 text-sm text-foreground"
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
        >
          <option value="">All Status</option>
          <option value="PENDING">Pending</option>
          <option value="IN_PROGRESS">In Progress</option>
          <option value="SUBMITTED">Submitted</option>
          <option value="APPROVED">Approved</option>
          <option value="REJECTED">Rejected</option>
        </select>
      </div>

      {Object.entries(groupedByCategory).map(([category, reqs]) => (
        <div key={category} className="space-y-2">
          <h4 className="text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            {toLabel(category)}
          </h4>
          <div className="space-y-2">
            {reqs.map((req) => (
              <div
                key={req.id}
                className="flex flex-col gap-3 rounded-xl border border-white/[0.06] bg-[#09090B]/55 p-4 transition-colors hover:bg-white/[0.03] sm:flex-row sm:items-center"
              >
                <Checkbox
                  checked={req.status === "APPROVED"}
                  disabled={req.status === "APPROVED"}
                  onCheckedChange={(checked) => {
                    if (checked && onStatusChange) onStatusChange(req.id, "APPROVED");
                  }}
                />
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-medium text-sm">{req.name}</span>
                    <span className={cn("text-xs font-semibold", getPriorityColor(req.priority))}>
                      [{req.priority}]
                    </span>
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">{req.description}</p>
                </div>
                <div className="flex flex-wrap items-center gap-3">
                  <StatusBadge status={req.status} severity={getRequirementSeverity(req.status)} />
                  {req.deadline && (
                    <span className="text-xs text-muted-foreground whitespace-nowrap">
                      Due: {formatDate(req.deadline)}
                    </span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}

      {filteredRequirements.length === 0 && (
        <EmptyState
          title="No requirements found"
          description="No license requirements match the selected status filter."
        />
      )}
    </div>
  );
}

function DeadlineCalendar({ deadlines, loading }: { deadlines: LicenseDeadline[]; loading: boolean }) {
  if (loading) return <TableSkeleton rows={5} columns={3} />;

  const sortedDeadlines = [...deadlines].sort(
    (a, b) => new Date(a.deadline).getTime() - new Date(b.deadline).getTime()
  );

  const overdueDeadlines = sortedDeadlines.filter((d) => d.days_remaining < 0);
  const upcomingDeadlines = sortedDeadlines.filter((d) => d.days_remaining >= 0);

  return (
    <div className="space-y-5">
      {overdueDeadlines.length > 0 && (
        <div className="space-y-2">
          <h4 className="flex items-center gap-2 text-sm font-semibold text-red-400">
            <AlertTriangle className="h-4 w-4" />
            Overdue ({formatNumber(overdueDeadlines.length)})
          </h4>
          {overdueDeadlines.map((deadline) => (
            <DeadlineRow key={deadline.id} deadline={deadline} overdue />
          ))}
        </div>
      )}

      <div className="space-y-2">
        <h4 className="text-sm font-medium text-muted-foreground">
          Upcoming Deadlines ({formatNumber(upcomingDeadlines.length)})
        </h4>
        {upcomingDeadlines.map((deadline) => (
          <DeadlineRow key={deadline.id} deadline={deadline} />
        ))}

        {upcomingDeadlines.length === 0 && overdueDeadlines.length === 0 && (
          <EmptyState title="No deadlines" description="No upcoming or overdue licensing deadlines were returned." />
        )}
      </div>
    </div>
  );
}

function DeadlineRow({ deadline, overdue = false }: { deadline: LicenseDeadline; overdue?: boolean }) {
  return (
    <div
      className={cn(
        "flex items-center gap-3 rounded-xl border p-4",
        overdue
          ? "border-red-400/25 bg-red-400/10"
          : "border-white/[0.06] bg-[#09090B]/55 hover:bg-white/[0.03]",
      )}
    >
      <Calendar className={cn("h-4 w-4", overdue ? "text-red-400" : "text-[#00D4FF]")} />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{deadline.requirement_name}</p>
        <p className="text-xs text-muted-foreground">{deadline.license_type}</p>
      </div>
      <div className="text-right">
        <p className={cn("text-sm", getDeadlineUrgency(deadline.days_remaining))}>
          {deadline.days_remaining < 0
            ? `${Math.abs(deadline.days_remaining)} days overdue`
            : deadline.days_remaining === 0
              ? "Due today"
              : `${deadline.days_remaining} days left`}
        </p>
        <p className="text-xs text-muted-foreground">{formatDate(deadline.deadline)}</p>
      </div>
    </div>
  );
}

function SubmissionHistory({
  submissions,
  loading,
  onExport,
}: {
  submissions: LicenseSubmission[];
  loading: boolean;
  onExport?: () => void;
}) {
  const columns = useMemo<ColumnDef<LicenseSubmission>[]>(
    () => [
      {
        accessorKey: "requirement_name",
        header: "Requirement",
        cell: ({ row }) => <span className="font-medium">{row.original.requirement_name}</span>,
      },
      {
        accessorKey: "document_name",
        header: "Document",
        cell: ({ row }) =>
          row.original.document_name ? (
            <a
              href={row.original.document_url}
              className="inline-flex items-center gap-1 text-[#00D4FF] hover:underline"
              target="_blank"
              rel="noopener noreferrer"
            >
              {row.original.document_name}
              <ChevronRight className="h-3 w-3" />
            </a>
          ) : (
            <span className="text-muted-foreground">-</span>
          ),
      },
      {
        accessorKey: "status",
        header: "Status",
        cell: ({ row }) => (
          <div className="flex items-center gap-2">
            {row.original.status === "APPROVED" ? (
              <CheckCircle2 className="h-4 w-4 text-[#00FF87]" />
            ) : row.original.status === "REJECTED" ? (
              <XCircle className="h-4 w-4 text-red-400" />
            ) : (
              <Clock className="h-4 w-4 text-[#FFB800]" />
            )}
            <StatusBadge status={row.original.status} severity={getRequirementSeverity(row.original.status)} />
          </div>
        ),
      },
      {
        accessorKey: "submitted_at",
        header: "Submitted",
        cell: ({ row }) => <span className="text-muted-foreground">{formatDateTime(row.original.submitted_at)}</span>,
      },
      {
        accessorKey: "reviewed_at",
        header: "Reviewed",
        cell: ({ row }) => (
          <span className="text-muted-foreground">
            {row.original.reviewed_at ? formatDateTime(row.original.reviewed_at) : "-"}
          </span>
        ),
      },
    ],
    [],
  );

  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button variant="outline" size="sm" onClick={onExport} className="border-[#00D4FF]/25 text-[#00D4FF]">
          <Download className="h-4 w-4 mr-2" />
          Export
        </Button>
      </div>
      <Panel contentClassName="p-0">
        <DataTable
          columns={columns}
          data={submissions}
          loading={loading}
          skeletonRows={6}
          emptyState={<EmptyState title="No submissions found" description="No licensing document submissions were returned." />}
        />
      </Panel>
    </div>
  );
}

function DocumentUpload({
  requirements,
  onUpload,
}: {
  requirements: LicenseRequirement[];
  onUpload: (requirementId: string, file: File) => Promise<void>;
}) {
  const [selectedRequirement, setSelectedRequirement] = useState<string>("");
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const { toast } = useToast();

  const pendingRequirements = requirements.filter(
    (r) => r.status === "PENDING" || r.status === "IN_PROGRESS" || r.status === "REJECTED"
  );

  const handleUpload = async () => {
    if (!selectedRequirement || !selectedFile) return;

    setUploading(true);
    try {
      await onUpload(selectedRequirement, selectedFile);
      toast({ title: "Success", description: "Document uploaded successfully" });
      setDialogOpen(false);
      setSelectedFile(null);
      setSelectedRequirement("");
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Failed to upload document";
      toast({ variant: "destructive", title: "Error", description: message });
    } finally {
      setUploading(false);
    }
  };

  return (
    <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
      <DialogTrigger asChild>
        <Button className="bg-[#00FF87] text-black hover:bg-[#00FF87]/90">
          <Upload className="h-4 w-4 mr-2" />
          Upload Document
        </Button>
      </DialogTrigger>
      <DialogContent className="border-white/[0.08] bg-[#111113]">
        <DialogHeader>
          <DialogTitle>Upload Document</DialogTitle>
          <DialogDescription>Upload a document for a pending requirement.</DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="requirement">Requirement</Label>
            <select
              id="requirement"
              className="w-full rounded-md border border-white/[0.08] bg-[#09090B]/70 px-3 py-2 text-sm text-foreground"
              value={selectedRequirement}
              onChange={(e) => setSelectedRequirement(e.target.value)}
            >
              <option value="">Select a requirement...</option>
              {pendingRequirements.map((req) => (
                <option key={req.id} value={req.id}>
                  {req.name} ({req.category})
                </option>
              ))}
            </select>
          </div>
          <div className="space-y-2">
            <Label htmlFor="file">Document</Label>
            <Input
              id="file"
              type="file"
              className="border-white/[0.08] bg-[#09090B]/70"
              onChange={(e) => setSelectedFile(e.target.files?.[0] || null)}
              accept=".pdf,.doc,.docx,.xls,.xlsx,.png,.jpg,.jpeg"
            />
            <p className="text-xs text-muted-foreground">Supported formats: PDF, DOC, DOCX, XLS, XLSX, PNG, JPG</p>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => setDialogOpen(false)}>Cancel</Button>
          <Button onClick={handleUpload} disabled={!selectedRequirement || !selectedFile || uploading}>
            {uploading ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <Upload className="h-4 w-4 mr-2" />}
            {uploading ? "Uploading..." : "Upload"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default function LicensingPage() {
  const [stats, setStats] = useState<LicenseDashboardStats | null>(null);
  const [licenses, setLicenses] = useState<LicenseStatus[]>([]);
  const [requirements, setRequirements] = useState<LicenseRequirement[]>([]);
  const [submissions, setSubmissions] = useState<LicenseSubmission[]>([]);
  const [deadlines, setDeadlines] = useState<LicenseDeadline[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedLicense, setSelectedLicense] = useState<string | null>(null);
  const { toast } = useToast();

  const fetchData = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    // Each licensing feed is independent; render whatever resolves and leave the
    // rest as clean empty state rather than blocking the whole page on one 404.
    const [statsR, licensesR, requirementsR, submissionsR, deadlinesR] =
      await Promise.allSettled([
        licensingApi.getStats(),
        licensingApi.listLicenses(),
        licensingApi.listRequirements(),
        licensingApi.listSubmissions({ per_page: 50 }),
        licensingApi.listDeadlines({ days_ahead: 90, include_overdue: true }),
      ]);

    if (statsR.status === "fulfilled") setStats(statsR.value);
    if (licensesR.status === "fulfilled") setLicenses(Array.isArray(licensesR.value) ? licensesR.value : []);
    if (requirementsR.status === "fulfilled") setRequirements(Array.isArray(requirementsR.value) ? requirementsR.value : []);
    if (submissionsR.status === "fulfilled") setSubmissions(Array.isArray(submissionsR.value?.data) ? submissionsR.value.data : []);
    if (deadlinesR.status === "fulfilled") setDeadlines(Array.isArray(deadlinesR.value) ? deadlinesR.value : []);
    setLoading(false);
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleRequirementStatusChange = async (id: string, status: string) => {
    try {
      await licensingApi.updateRequirement(id, { status });
      toast({ title: "Success", description: "Requirement status updated" });
      fetchData();
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Failed to update requirement";
      toast({ variant: "destructive", title: "Error", description: message });
    }
  };

  const handleDocumentUpload = async (requirementId: string, file: File) => {
    const result = await licensingApi.uploadDocument(file, requirementId);
    await licensingApi.createSubmission({ requirement_id: requirementId, document_name: result.name, document_url: result.url });
    fetchData();
  };

  const handleExportSubmissions = () => {
    const csvContent = [
      ["Requirement", "Document", "Status", "Submitted", "Reviewed"].join(","),
      ...submissions.map((s) => [s.requirement_name, s.document_name || "", s.status, s.submitted_at, s.reviewed_at || ""].join(",")),
    ].join("\n");

    const blob = new Blob([csvContent], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `license-submissions-${new Date().toISOString().split("T")[0]}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const filteredRequirements = selectedLicense
    ? requirements.filter((r) => r.license_id === selectedLicense)
    : requirements;

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Licensing"
        description="License status tracking and compliance management."
        actions={
          <>
            <DocumentUpload requirements={requirements} onUpload={handleDocumentUpload} />
            <Button variant="outline" size="icon" onClick={fetchData} disabled={loading} aria-label="Refresh licensing data">
              <RefreshCw className={cn("h-4 w-4", loading && "animate-spin")} />
            </Button>
          </>
        }
      />

      {loadError && !loading ? (
        <Panel>
          <ErrorState message={loadError} retry={fetchData} />
        </Panel>
      ) : null}

      <StatGrid>
        <StatCard title="Active Licenses" value={stats?.active_licenses ?? 0} icon={<Shield className="h-4 w-4" />} loading={loading} accentColor="green" />
        <StatCard title="Pending Licenses" value={stats?.pending_licenses ?? 0} icon={<Clock className="h-4 w-4" />} loading={loading} accentColor="amber" />
        <StatCard
          title="Requirements Completed"
          value={`${stats?.requirements_completed ?? 0}/${(stats?.requirements_completed ?? 0) + (stats?.requirements_pending ?? 0)}`}
          icon={<FileCheck className="h-4 w-4" />}
          loading={loading}
          accentColor="cyan"
        />
        <StatCard title="Overdue Items" value={stats?.overdue_items ?? 0} icon={<AlertTriangle className="h-4 w-4" />} loading={loading} accentColor="violet" />
      </StatGrid>

      <SectionCard header={{ title: "License Status", description: "Select a license to filter requirements." }}>
        {loading ? (
          <CardGridSkeleton cards={3} className="lg:grid-cols-3" />
        ) : licenses.length > 0 ? (
          <div className="grid gap-4 md:grid-cols-3">
            {licenses.map((license) => (
              <LicenseStatusCard
                key={license.id}
                license={license}
                selected={selectedLicense === license.id}
                onClick={() => setSelectedLicense(selectedLicense === license.id ? null : license.id)}
              />
            ))}
          </div>
        ) : (
          <EmptyState title="No licenses found" description="No license records were returned by the licensing service." />
        )}
      </SectionCard>

      <Tabs defaultValue="requirements" className="space-y-4">
        <TabsList className="border border-white/[0.06] bg-[#111113]">
          <TabsTrigger value="requirements">
            Requirements
            {stats?.requirements_pending ? <StatusBadge className="ml-2" status={formatNumber(stats.requirements_pending)} severity="warning" dot={false} /> : null}
          </TabsTrigger>
          <TabsTrigger value="deadlines">
            Deadlines
            {stats?.overdue_items ? <StatusBadge className="ml-2" status={formatNumber(stats.overdue_items)} severity="danger" dot={false} /> : null}
          </TabsTrigger>
          <TabsTrigger value="submissions">Submission History</TabsTrigger>
        </TabsList>

        <TabsContent value="requirements">
          <SectionCard
            header={{
              title: "Requirement Checklist",
              description: selectedLicense ? "Showing requirements for selected license." : "All license requirements.",
            }}
          >
            <RequirementChecklist requirements={filteredRequirements} loading={loading} onStatusChange={handleRequirementStatusChange} />
          </SectionCard>
        </TabsContent>

        <TabsContent value="deadlines">
          <SectionCard header={{ title: "Deadline Calendar", description: "Upcoming and overdue deadlines." }}>
            <DeadlineCalendar deadlines={deadlines} loading={loading} />
          </SectionCard>
        </TabsContent>

        <TabsContent value="submissions">
          <SectionCard header={{ title: "Submission History", description: "Past document submissions and their review status." }}>
            <SubmissionHistory submissions={submissions} loading={loading} onExport={handleExportSubmissions} />
          </SectionCard>
        </TabsContent>
      </Tabs>
    </main>
  );
}
