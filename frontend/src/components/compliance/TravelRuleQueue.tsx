"use client";

import { AlertTriangle, RotateCcw, ShieldCheck, Waves } from "lucide-react";

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

export type TravelRuleRegistryRow = {
  vaspCode: string;
  legalName: string;
  jurisdictionCode?: string | null;
  transportProfile?: string | null;
  endpointUri?: string | null;
  review: {
    status: string;
  };
  interoperability: {
    status: string;
  };
  supportsInbound: boolean;
  supportsOutbound: boolean;
};

export type TravelRuleDisclosureRow = {
  disclosureId: string;
  direction: string;
  stage: string;
  queueStatus?: string | null;
  failureCount: number;
  maxFailuresBeforeException: number;
  attemptCount: number;
  transportProfile?: string | null;
  matchedPolicyCode?: string | null;
  action?: string | null;
  retryRecommended: boolean;
  terminal: boolean;
  updatedAt: string;
};

export type TravelRuleExceptionRow = {
  exceptionId: string;
  disclosureId: string;
  status: string;
  reasonCode: string;
  resolutionNote?: string | null;
  resolvedBy?: string | null;
  updatedAt: string;
};

type Props = {
  registry: TravelRuleRegistryRow[];
  disclosures: TravelRuleDisclosureRow[];
  exceptions: TravelRuleExceptionRow[];
  loading: boolean;
  error: string | null;
  refreshing: boolean;
  retryingId: string | null;
  resolvingId: string | null;
  notice: { type: "success" | "error"; message: string } | null;
  onRefresh: () => void;
  onRetryDisclosure: (disclosureId: string) => void;
  onResolveException: (exceptionId: string) => void;
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

export default function TravelRuleQueue({
  registry,
  disclosures,
  exceptions,
  loading,
  error,
  refreshing,
  retryingId,
  resolvingId,
  notice,
  onRefresh,
  onRetryDisclosure,
  onResolveException,
}: Props) {
  return (
    <div className="space-y-6" data-testid="travel-rule-queue">
      {notice && (
        <Alert variant={notice.type === "success" ? "success" : "destructive"}>
          <AlertTitle>{notice.type === "success" ? "Travel Rule updated" : "Request failed"}</AlertTitle>
          <AlertDescription>{notice.message}</AlertDescription>
        </Alert>
      )}

      {error && (
        <Alert variant="destructive">
          <AlertTitle>Travel Rule admin data failed to load</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Registry records</CardDescription>
            <CardTitle>{loading ? "..." : registry.length}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Disclosure queue</CardDescription>
            <CardTitle>{loading ? "..." : disclosures.length}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Open exceptions</CardDescription>
            <CardTitle>
              {loading
                ? "..."
                : exceptions.filter((row) => row.status === "OPEN").length}
            </CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Operator actions</CardDescription>
            <CardTitle className="flex items-center gap-2">
              Live
              <ShieldCheck className="h-4 w-4 text-muted-foreground" />
            </CardTitle>
          </CardHeader>
        </Card>
      </div>

      <Card className="border-dashed">
        <CardHeader className="flex flex-row items-start justify-between gap-4 space-y-0">
          <div className="space-y-1">
            <CardTitle className="text-base">Audit context</CardTitle>
            <CardDescription>
              Retry and resolve actions stay tenant-scoped and reflect the bounded admin API
              currently available in W5.
            </CardDescription>
          </div>
          <Button
            variant="outline"
            onClick={onRefresh}
            disabled={refreshing}
            aria-label="Refresh travel rule page"
          >
            <RotateCcw className={`mr-2 h-4 w-4 ${refreshing ? "animate-spin" : ""}`} />
            {refreshing ? "Refreshing..." : "Refresh"}
          </Button>
        </CardHeader>
      </Card>

      <div className="grid gap-6 xl:grid-cols-[1.05fr,1fr]">
        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Waves className="h-4 w-4 text-muted-foreground" />
              <CardTitle>VASP registry</CardTitle>
            </div>
            <CardDescription>
              Review interoperability posture and transport readiness for counterparties.
            </CardDescription>
          </CardHeader>
          <CardContent>
            {loading ? (
              renderSkeletonRows()
            ) : registry.length === 0 ? (
              <div className="rounded-lg border border-dashed px-4 py-8 text-sm text-muted-foreground">
                No Travel Rule VASP records are available for this tenant yet.
              </div>
            ) : (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader sticky>
                    <TableRow>
                      <TableHead>VASP</TableHead>
                      <TableHead>Review</TableHead>
                      <TableHead>Interop</TableHead>
                      <TableHead>Profile</TableHead>
                      <TableHead>Endpoint</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {registry.map((row) => (
                      <TableRow key={row.vaspCode}>
                        <TableCell>
                          <div className="font-medium">{row.vaspCode}</div>
                          <div className="text-xs text-muted-foreground">{row.legalName}</div>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary" shape="pill">
                            {row.review.status}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary" shape="pill">
                            {row.interoperability.status}
                          </Badge>
                        </TableCell>
                        <TableCell>{row.transportProfile ?? "N/A"}</TableCell>
                        <TableCell className="max-w-[220px] whitespace-normal break-all text-xs">
                          {row.endpointUri ?? "N/A"}
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
              <CardTitle>Exception queue</CardTitle>
            </div>
            <CardDescription>
              Resolve queue items when transport retries need explicit operator action.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {loading ? (
              renderSkeletonRows()
            ) : exceptions.length === 0 ? (
              <div className="rounded-lg border border-dashed px-4 py-8 text-sm text-muted-foreground">
                No Travel Rule exceptions are open right now.
              </div>
            ) : (
              exceptions.map((row) => (
                <div key={row.exceptionId} className="rounded-xl border p-4">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="space-y-1">
                      <div className="font-medium">{row.exceptionId}</div>
                      <div className="text-sm text-muted-foreground">
                        Disclosure {row.disclosureId} · Reason {row.reasonCode}
                      </div>
                    </div>
                    <Badge variant="secondary" shape="pill">
                      {row.status}
                    </Badge>
                  </div>
                  <div className="mt-3 flex items-center justify-between gap-3 text-sm">
                    <span className="text-muted-foreground">
                      Updated {formatTimestamp(row.updatedAt)}
                    </span>
                    <Button
                      size="sm"
                      onClick={() => onResolveException(row.exceptionId)}
                      disabled={resolvingId === row.exceptionId || row.status === "RESOLVED"}
                    >
                      {resolvingId === row.exceptionId ? "Resolving..." : "Resolve"}
                    </Button>
                  </div>
                </div>
              ))
            )}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Disclosure queue</CardTitle>
          <CardDescription>
            Retry disclosures from the operator console and monitor retry/error state.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            renderSkeletonRows()
          ) : disclosures.length === 0 ? (
            <div className="rounded-lg border border-dashed px-4 py-8 text-sm text-muted-foreground">
              No Travel Rule disclosures are currently queued for this tenant.
            </div>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader sticky>
                  <TableRow>
                    <TableHead>Disclosure</TableHead>
                    <TableHead>Stage</TableHead>
                    <TableHead>Queue</TableHead>
                    <TableHead>Attempts</TableHead>
                    <TableHead>Transport</TableHead>
                    <TableHead>Policy</TableHead>
                    <TableHead>Updated</TableHead>
                    <TableHead>Action</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {disclosures.map((row) => (
                    <TableRow key={row.disclosureId}>
                      <TableCell>
                        <div className="font-medium">{row.disclosureId}</div>
                        <div className="text-xs text-muted-foreground">{row.direction}</div>
                      </TableCell>
                      <TableCell>{row.stage}</TableCell>
                      <TableCell>{row.queueStatus ?? "N/A"}</TableCell>
                      <TableCell>
                        {row.attemptCount}/{row.maxFailuresBeforeException}
                      </TableCell>
                      <TableCell>{row.transportProfile ?? "Missing"}</TableCell>
                      <TableCell>{row.matchedPolicyCode ?? row.action ?? "N/A"}</TableCell>
                      <TableCell>{formatTimestamp(row.updatedAt)}</TableCell>
                      <TableCell>
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => onRetryDisclosure(row.disclosureId)}
                          disabled={retryingId === row.disclosureId || row.terminal}
                        >
                          {retryingId === row.disclosureId ? "Retrying..." : "Retry"}
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
    </div>
  );
}
