"use client";

import { AlertTriangle, RefreshCw, ShieldAlert, ShieldCheck } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

export type RescreeningRunRow = {
  userId: string;
  status: string;
  kycStatus: string;
  nextRunAt: string;
  triggerKind: string;
  priority: string;
  restrictionStatus: string;
  alertCodes: string[];
};

type Props = {
  runs: RescreeningRunRow[];
  loading: boolean;
  error: string | null;
  refreshing: boolean;
  restrictingUserId: string | null;
  notice: { type: "success" | "error"; message: string } | null;
  onRefresh: () => void;
  onApplyRestriction: (userId: string) => void;
};

function formatTimestamp(value?: string | null): string {
  if (!value) return "N/A";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("en-US", {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function renderSkeletonRows() {
  return (
    <div className="space-y-3">
      <Skeleton className="h-10 w-full" />
      <Skeleton className="h-24 w-full" />
    </div>
  );
}

function priorityVariant(priority: string): "success" | "warning" | "destructive" | "outline" {
  switch (priority.toLowerCase()) {
    case "critical":
      return "destructive";
    case "high":
      return "warning";
    case "medium":
      return "outline";
    default:
      return "success";
  }
}

function restrictionVariant(
  status: string,
): "success" | "warning" | "destructive" | "outline" {
  switch (status.toLowerCase()) {
    case "restricted":
      return "destructive";
    case "review_required":
      return "warning";
    default:
      return "success";
  }
}

function humanize(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join(" ");
}

export default function RescreeningQueue({
  runs,
  loading,
  error,
  refreshing,
  restrictingUserId,
  notice,
  onRefresh,
  onApplyRestriction,
}: Props) {
  const reviewRequiredCount = runs.filter((row) => row.restrictionStatus === "REVIEW_REQUIRED").length;
  const restrictedCount = runs.filter((row) => row.restrictionStatus === "RESTRICTED").length;

  return (
    <div className="space-y-6" data-testid="rescreening-queue">
      {notice && (
        <Alert variant={notice.type === "success" ? "success" : "destructive"}>
          <AlertTitle>{notice.type === "success" ? "Rescreening updated" : "Request failed"}</AlertTitle>
          <AlertDescription>{notice.message}</AlertDescription>
        </Alert>
      )}

      {error && (
        <Alert variant="destructive">
          <AlertTitle>Rescreening data failed to load</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Due runs</CardDescription>
            <CardTitle>{loading ? "..." : runs.length}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Review required</CardDescription>
            <CardTitle>{loading ? "..." : reviewRequiredCount}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Restricted</CardDescription>
            <CardTitle>{loading ? "..." : restrictedCount}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Mode</CardDescription>
            <CardTitle className="flex items-center gap-2">
              Recommendation-first
              <ShieldCheck className="h-4 w-4 text-muted-foreground" />
            </CardTitle>
          </CardHeader>
        </Card>
      </div>

      <Card className="border-dashed">
        <CardHeader className="flex flex-row items-start justify-between gap-4 space-y-0">
          <div className="space-y-1">
            <CardTitle className="text-base">Scheduler, alerts, restrictions</CardTitle>
            <CardDescription>
              This surface starts with scheduled due-runs, alert visibility, and bounded
              restriction actions. It does not try to become a broad media platform.
            </CardDescription>
          </div>
          <Button
            variant="outline"
            onClick={onRefresh}
            disabled={refreshing}
            aria-label="Refresh rescreening page"
          >
            <RefreshCw className={`mr-2 h-4 w-4 ${refreshing ? "animate-spin" : ""}`} />
            {refreshing ? "Refreshing..." : "Refresh"}
          </Button>
        </CardHeader>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <ShieldAlert className="h-4 w-4 text-muted-foreground" />
            <CardTitle>Rescreening queue</CardTitle>
          </div>
          <CardDescription>
            Review due users, alert codes, and current restriction state before applying bounded
            actions.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            renderSkeletonRows()
          ) : runs.length === 0 ? (
            <div className="rounded-lg border border-dashed px-4 py-8 text-sm text-muted-foreground">
              No users are currently due for continuous rescreening.
            </div>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader sticky>
                  <TableRow>
                    <TableHead>User</TableHead>
                    <TableHead>Trigger</TableHead>
                    <TableHead>Priority</TableHead>
                    <TableHead>Restriction</TableHead>
                    <TableHead>Alerts</TableHead>
                    <TableHead>Next run</TableHead>
                    <TableHead>Action</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {runs.map((row) => (
                    <TableRow key={row.userId}>
                      <TableCell>
                        <div className="font-medium">{row.userId}</div>
                        <div className="text-xs text-muted-foreground">{row.kycStatus}</div>
                      </TableCell>
                      <TableCell>{humanize(row.triggerKind)}</TableCell>
                      <TableCell>
                        <Badge variant={priorityVariant(row.priority)} shape="pill">
                          {humanize(row.priority)}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <Badge variant={restrictionVariant(row.restrictionStatus)} shape="pill">
                          {humanize(row.restrictionStatus)}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-wrap gap-1">
                          {row.alertCodes.map((code) => (
                            <Badge key={`${row.userId}-${code}`} variant="outline" shape="pill">
                              {humanize(code)}
                            </Badge>
                          ))}
                        </div>
                      </TableCell>
                      <TableCell>{formatTimestamp(row.nextRunAt)}</TableCell>
                      <TableCell>
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => onApplyRestriction(row.userId)}
                          disabled={restrictingUserId === row.userId || row.restrictionStatus === "RESTRICTED"}
                        >
                          {restrictingUserId === row.userId ? "Applying..." : "Apply restriction"}
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <AlertTriangle className="h-4 w-4 text-muted-foreground" />
            <CardTitle>Restriction guardrail</CardTitle>
          </div>
          <CardDescription>
            Restriction writes only update the bounded rescreening status and audit trail. Broader
            remediation and enrichment stay outside this wave.
          </CardDescription>
        </CardHeader>
      </Card>
    </div>
  );
}
