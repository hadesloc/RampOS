"use client";

import { useState, useEffect, useCallback } from "react";
import {
  licensingApi,
  type LicenseStatus,
  type LicenseRequirement,
  type LicenseSubmission,
  type LicenseDeadline,
  type LicenseDashboardStats,
} from "@/lib/api";
import {
  Loader2,
  RefreshCw,
  Shield,
  FileCheck,
  Clock,
  AlertTriangle,
  Upload,
  Calendar,
  CheckCircle2,
  XCircle,
  ChevronRight,
  Download,
  Filter,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import { StatCard } from "@/components/dashboard/stat-card";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Checkbox } from "@/components/ui/checkbox";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
}

function formatDateTime(dateStr: string): string {
  return new Date(dateStr).toLocaleString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function getLicenseStatusColor(status: string): string {
  switch (status) {
    case "ACTIVE":
      return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400";
    case "PENDING":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400";
    case "EXPIRED":
      return "bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400";
    case "SUSPENDED":
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

function getRequirementStatusColor(status: string): string {
  switch (status) {
    case "APPROVED":
      return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400";
    case "SUBMITTED":
      return "bg-blue-100 text-blue-800 dark:bg-blue-500/15 dark:text-blue-400";
    case "IN_PROGRESS":
      return "bg-purple-100 text-purple-800 dark:bg-purple-500/15 dark:text-purple-400";
    case "PENDING":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400";
    case "REJECTED":
      return "bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

function getPriorityColor(priority: string): string {
  switch (priority) {
    case "CRITICAL":
      return "text-red-600 dark:text-red-400";
    case "HIGH":
      return "text-orange-600 dark:text-orange-400";
    case "MEDIUM":
      return "text-yellow-600 dark:text-yellow-400";
    case "LOW":
      return "text-green-600 dark:text-green-400";
    default:
      return "text-gray-600 dark:text-gray-400";
  }
}

function getDeadlineUrgency(daysRemaining: number): string {
  if (daysRemaining < 0) return "text-red-600 dark:text-red-400 font-semibold";
  if (daysRemaining <= 7) return "text-orange-600 dark:text-orange-400 font-medium";
  if (daysRemaining <= 30) return "text-yellow-600 dark:text-yellow-400";
  return "text-muted-foreground";
}

// License Status Card Component
function LicenseStatusCard({
  license,
  onClick,
}: {
  license: LicenseStatus;
  onClick?: () => void;
}) {
  const progress = license.requirements_total > 0
    ? (license.requirements_completed / license.requirements_total) * 100
    : 0;

  return (
    <Card
      className={cn(
        "cursor-pointer transition-all hover:shadow-md",
        license.status === "ACTIVE" && "border-green-200 dark:border-green-800",
        license.status === "EXPIRED" && "border-red-200 dark:border-red-800"
      )}
      onClick={onClick}
    >
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">{license.license_type}</CardTitle>
          <StatusBadge status={license.status} className={getLicenseStatusColor(license.status)} />
        </div>
        <CardDescription>{license.jurisdiction}</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          <div className="flex justify-between text-sm">
            <span className="text-muted-foreground">Requirements</span>
            <span className="font-medium">
              {license.requirements_completed}/{license.requirements_total}
            </span>
          </div>
          <Progress value={progress} className="h-2" />
          <div className="flex justify-between text-xs text-muted-foreground">
            {license.issue_date && (
              <span>Issued: {formatDate(license.issue_date)}</span>
            )}
            {license.expiry_date && (
              <span>Expires: {formatDate(license.expiry_date)}</span>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

// Requirement Checklist Component
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

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Filter className="h-4 w-4 text-muted-foreground" />
        <select
          className="rounded-md border bg-background px-3 py-1.5 text-sm"
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
          <h4 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
            {category.replace(/_/g, " ")}
          </h4>
          <div className="space-y-1">
            {reqs.map((req) => (
              <div
                key={req.id}
                className="flex items-center gap-3 p-3 rounded-lg border bg-card hover:bg-muted/50 transition-colors"
              >
                <Checkbox
                  checked={req.status === "APPROVED"}
                  disabled={req.status === "APPROVED"}
                  onCheckedChange={(checked) => {
                    if (checked && onStatusChange) {
                      onStatusChange(req.id, "APPROVED");
                    }
                  }}
                />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-sm truncate">{req.name}</span>
                    <span className={cn("text-xs", getPriorityColor(req.priority))}>
                      [{req.priority}]
                    </span>
                  </div>
                  <p className="text-xs text-muted-foreground truncate">{req.description}</p>
                </div>
                <StatusBadge
                  status={req.status}
                  className={cn("text-xs", getRequirementStatusColor(req.status))}
                />
                {req.deadline && (
                  <span className="text-xs text-muted-foreground whitespace-nowrap">
                    Due: {formatDate(req.deadline)}
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      ))}

      {filteredRequirements.length === 0 && (
        <p className="text-center text-muted-foreground py-4">No requirements found.</p>
      )}
    </div>
  );
}

// Deadline Calendar Component
function DeadlineCalendar({
  deadlines,
  loading,
}: {
  deadlines: LicenseDeadline[];
  loading: boolean;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const sortedDeadlines = [...deadlines].sort(
    (a, b) => new Date(a.deadline).getTime() - new Date(b.deadline).getTime()
  );

  const overdueDeadlines = sortedDeadlines.filter((d) => d.days_remaining < 0);
  const upcomingDeadlines = sortedDeadlines.filter((d) => d.days_remaining >= 0);

  return (
    <div className="space-y-4">
      {overdueDeadlines.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-sm font-medium text-red-600 dark:text-red-400 flex items-center gap-2">
            <AlertTriangle className="h-4 w-4" />
            Overdue ({overdueDeadlines.length})
          </h4>
          {overdueDeadlines.map((deadline) => (
            <div
              key={deadline.id}
              className="flex items-center gap-3 p-3 rounded-lg border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-500/10"
            >
              <Calendar className="h-4 w-4 text-red-600 dark:text-red-400" />
              <div className="flex-1">
                <p className="text-sm font-medium">{deadline.requirement_name}</p>
                <p className="text-xs text-muted-foreground">{deadline.license_type}</p>
              </div>
              <div className="text-right">
                <p className="text-sm font-medium text-red-600 dark:text-red-400">
                  {Math.abs(deadline.days_remaining)} days overdue
                </p>
                <p className="text-xs text-muted-foreground">{formatDate(deadline.deadline)}</p>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="space-y-2">
        <h4 className="text-sm font-medium text-muted-foreground">
          Upcoming Deadlines ({upcomingDeadlines.length})
        </h4>
        {upcomingDeadlines.map((deadline) => (
          <div
            key={deadline.id}
            className="flex items-center gap-3 p-3 rounded-lg border bg-card hover:bg-muted/50 transition-colors"
          >
            <Calendar className="h-4 w-4 text-muted-foreground" />
            <div className="flex-1">
              <p className="text-sm font-medium">{deadline.requirement_name}</p>
              <p className="text-xs text-muted-foreground">{deadline.license_type}</p>
            </div>
            <div className="text-right">
              <p className={cn("text-sm", getDeadlineUrgency(deadline.days_remaining))}>
                {deadline.days_remaining === 0
                  ? "Due today"
                  : `${deadline.days_remaining} days left`}
              </p>
              <p className="text-xs text-muted-foreground">{formatDate(deadline.deadline)}</p>
            </div>
          </div>
        ))}

        {upcomingDeadlines.length === 0 && (
          <p className="text-center text-muted-foreground py-4">No upcoming deadlines.</p>
        )}
      </div>
    </div>
  );
}

// Submission History Component
function SubmissionHistory({
  submissions,
  loading,
  onExport,
}: {
  submissions: LicenseSubmission[];
  loading: boolean;
  onExport?: () => void;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button variant="outline" size="sm" onClick={onExport}>
          <Download className="h-4 w-4 mr-2" />
          Export
        </Button>
      </div>

      <div className="rounded-md border bg-card">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="px-4 py-3 text-left font-medium">Requirement</th>
              <th className="px-4 py-3 text-left font-medium">Document</th>
              <th className="px-4 py-3 text-left font-medium">Status</th>
              <th className="px-4 py-3 text-left font-medium">Submitted</th>
              <th className="px-4 py-3 text-left font-medium">Reviewed</th>
            </tr>
          </thead>
          <tbody>
            {submissions.length === 0 ? (
              <tr>
                <td colSpan={5} className="h-24 text-center text-muted-foreground">
                  No submissions found.
                </td>
              </tr>
            ) : (
              submissions.map((submission) => (
                <tr key={submission.id} className="border-t hover:bg-muted/30">
                  <td className="px-4 py-3">
                    <span className="font-medium">{submission.requirement_name}</span>
                  </td>
                  <td className="px-4 py-3">
                    {submission.document_name ? (
                      <a
                        href={submission.document_url}
                        className="text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1"
                        target="_blank"
                        rel="noopener noreferrer"
                      >
                        {submission.document_name}
                        <ChevronRight className="h-3 w-3" />
                      </a>
                    ) : (
                      <span className="text-muted-foreground">-</span>
                    )}
                  </td>
                  <td className="px-4 py-3">
                    <div className="flex items-center gap-2">
                      {submission.status === "APPROVED" ? (
                        <CheckCircle2 className="h-4 w-4 text-green-600" />
                      ) : submission.status === "REJECTED" ? (
                        <XCircle className="h-4 w-4 text-red-600" />
                      ) : (
                        <Clock className="h-4 w-4 text-yellow-600" />
                      )}
                      <StatusBadge status={submission.status} />
                    </div>
                  </td>
                  <td className="px-4 py-3 text-muted-foreground">
                    {formatDateTime(submission.submitted_at)}
                  </td>
                  <td className="px-4 py-3 text-muted-foreground">
                    {submission.reviewed_at ? formatDateTime(submission.reviewed_at) : "-"}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// Document Upload Component
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
      toast({
        title: "Success",
        description: "Document uploaded successfully",
      });
      setDialogOpen(false);
      setSelectedFile(null);
      setSelectedRequirement("");
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Failed to upload document";
      toast({
        variant: "destructive",
        title: "Error",
        description: message,
      });
    } finally {
      setUploading(false);
    }
  };

  return (
    <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
      <DialogTrigger asChild>
        <Button>
          <Upload className="h-4 w-4 mr-2" />
          Upload Document
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Upload Document</DialogTitle>
          <DialogDescription>
            Upload a document for a pending requirement.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="requirement">Requirement</Label>
            <select
              id="requirement"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
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
              onChange={(e) => setSelectedFile(e.target.files?.[0] || null)}
              accept=".pdf,.doc,.docx,.xls,.xlsx,.png,.jpg,.jpeg"
            />
            <p className="text-xs text-muted-foreground">
              Supported formats: PDF, DOC, DOCX, XLS, XLSX, PNG, JPG
            </p>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => setDialogOpen(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleUpload}
            disabled={!selectedRequirement || !selectedFile || uploading}
          >
            {uploading ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Uploading...
              </>
            ) : (
              <>
                <Upload className="h-4 w-4 mr-2" />
                Upload
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// Main Licensing Page
export default function LicensingPage() {
  const [stats, setStats] = useState<LicenseDashboardStats | null>(null);
  const [licenses, setLicenses] = useState<LicenseStatus[]>([]);
  const [requirements, setRequirements] = useState<LicenseRequirement[]>([]);
  const [submissions, setSubmissions] = useState<LicenseSubmission[]>([]);
  const [deadlines, setDeadlines] = useState<LicenseDeadline[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedLicense, setSelectedLicense] = useState<string | null>(null);
  const { toast } = useToast();

  const fetchData = useCallback(async () => {
    setLoading(true);
    try {
      const [statsData, licensesData, requirementsData, submissionsData, deadlinesData] =
        await Promise.all([
          licensingApi.getStats(),
          licensingApi.listLicenses(),
          licensingApi.listRequirements(),
          licensingApi.listSubmissions({ per_page: 50 }),
          licensingApi.listDeadlines({ days_ahead: 90, include_overdue: true }),
        ]);

      setStats(statsData);
      setLicenses(licensesData);
      setRequirements(requirementsData);
      setSubmissions(submissionsData.data);
      setDeadlines(deadlinesData);
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Failed to load licensing data";
      console.error("Failed to fetch licensing data:", error);
      toast({
        variant: "destructive",
        title: "Error",
        description: message,
      });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleRequirementStatusChange = async (id: string, status: string) => {
    try {
      await licensingApi.updateRequirement(id, { status });
      toast({
        title: "Success",
        description: "Requirement status updated",
      });
      fetchData();
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Failed to update requirement";
      toast({
        variant: "destructive",
        title: "Error",
        description: message,
      });
    }
  };

  const handleDocumentUpload = async (requirementId: string, file: File) => {
    const result = await licensingApi.uploadDocument(file, requirementId);
    await licensingApi.createSubmission({
      requirement_id: requirementId,
      document_name: result.name,
      document_url: result.url,
    });
    fetchData();
  };

  const handleExportSubmissions = () => {
    const csvContent = [
      ["Requirement", "Document", "Status", "Submitted", "Reviewed"].join(","),
      ...submissions.map((s) =>
        [
          s.requirement_name,
          s.document_name || "",
          s.status,
          s.submitted_at,
          s.reviewed_at || "",
        ].join(",")
      ),
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
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Licensing</h1>
          <p className="text-muted-foreground">
            License status tracking and compliance management
          </p>
        </div>
        <div className="flex gap-2">
          <DocumentUpload requirements={requirements} onUpload={handleDocumentUpload} />
          <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid gap-4 md:grid-cols-4">
        <StatCard
          title="Active Licenses"
          value={stats?.active_licenses ?? 0}
          icon={<Shield className="h-4 w-4" />}
          loading={loading}
          className="border-green-200 dark:border-green-800"
        />
        <StatCard
          title="Pending Licenses"
          value={stats?.pending_licenses ?? 0}
          icon={<Clock className="h-4 w-4" />}
          loading={loading}
          className={stats?.pending_licenses ? "border-yellow-200 dark:border-yellow-800" : ""}
        />
        <StatCard
          title="Requirements Completed"
          value={`${stats?.requirements_completed ?? 0}/${(stats?.requirements_completed ?? 0) + (stats?.requirements_pending ?? 0)}`}
          icon={<FileCheck className="h-4 w-4" />}
          loading={loading}
        />
        <StatCard
          title="Overdue Items"
          value={stats?.overdue_items ?? 0}
          icon={<AlertTriangle className="h-4 w-4" />}
          loading={loading}
          className={stats?.overdue_items ? "border-red-200 dark:border-red-800" : ""}
        />
      </div>

      {/* License Cards */}
      <div>
        <h2 className="text-lg font-semibold mb-3">License Status</h2>
        {loading ? (
          <div className="grid gap-4 md:grid-cols-3">
            {[1, 2, 3].map((i) => (
              <Card key={i} className="animate-pulse">
                <CardHeader className="pb-2">
                  <div className="h-5 bg-muted rounded w-1/3" />
                </CardHeader>
                <CardContent>
                  <div className="h-4 bg-muted rounded w-full mb-2" />
                  <div className="h-2 bg-muted rounded w-full" />
                </CardContent>
              </Card>
            ))}
          </div>
        ) : (
          <div className="grid gap-4 md:grid-cols-3">
            {licenses.map((license) => (
              <LicenseStatusCard
                key={license.id}
                license={license}
                onClick={() =>
                  setSelectedLicense(selectedLicense === license.id ? null : license.id)
                }
              />
            ))}
            {licenses.length === 0 && (
              <p className="text-muted-foreground col-span-3 text-center py-8">
                No licenses found.
              </p>
            )}
          </div>
        )}
      </div>

      {/* Tabs for Requirements, Deadlines, Submissions */}
      <Tabs defaultValue="requirements" className="space-y-4">
        <TabsList>
          <TabsTrigger value="requirements">
            Requirements
            {stats?.requirements_pending ? (
              <span className="ml-2 px-2 py-0.5 text-xs bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400 rounded-full">
                {stats.requirements_pending}
              </span>
            ) : null}
          </TabsTrigger>
          <TabsTrigger value="deadlines">
            Deadlines
            {stats?.overdue_items ? (
              <span className="ml-2 px-2 py-0.5 text-xs bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400 rounded-full">
                {stats.overdue_items}
              </span>
            ) : null}
          </TabsTrigger>
          <TabsTrigger value="submissions">Submission History</TabsTrigger>
        </TabsList>

        <TabsContent value="requirements">
          <Card>
            <CardHeader>
              <CardTitle>Requirement Checklist</CardTitle>
              <CardDescription>
                {selectedLicense
                  ? `Showing requirements for selected license`
                  : "All license requirements"}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <RequirementChecklist
                requirements={filteredRequirements}
                loading={loading}
                onStatusChange={handleRequirementStatusChange}
              />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="deadlines">
          <Card>
            <CardHeader>
              <CardTitle>Deadline Calendar</CardTitle>
              <CardDescription>Upcoming and overdue deadlines</CardDescription>
            </CardHeader>
            <CardContent>
              <DeadlineCalendar deadlines={deadlines} loading={loading} />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="submissions">
          <Card>
            <CardHeader>
              <CardTitle>Submission History</CardTitle>
              <CardDescription>Past document submissions and their review status</CardDescription>
            </CardHeader>
            <CardContent>
              <SubmissionHistory
                submissions={submissions}
                loading={loading}
                onExport={handleExportSubmissions}
              />
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
