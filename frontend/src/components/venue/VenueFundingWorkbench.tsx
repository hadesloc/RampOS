"use client";

import { useCallback, useMemo, useState } from "react";
import { Loader2, RefreshCw, ShieldAlert } from "lucide-react";

import { PageHeader } from "@/components/layout/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

type Metadata = Record<string, unknown>;

type VenueConnection = {
  connectionId: string;
  venueKey: string;
  status: string;
  connectionMode?: string | null;
  metadata?: Metadata | null;
};

type VenueAccount = {
  accountId: string;
  venueKey?: string | null;
  status: string;
  metadata?: Metadata | null;
};

type WalletAttestation = {
  attestationId: string;
  walletAddress?: string | null;
  network?: string | null;
  attestationStatus: string;
  proofArtifactUri?: string | null;
  metadata?: Metadata | null;
};

type SourceOfFundsPackage = {
  packageId: string;
  reviewStatus: string;
  packageUri?: string | null;
  metadata?: Metadata | null;
};

type VenueTransfer = {
  transferId: string;
  transferDirection?: string | null;
  assetSymbol?: string | null;
  network?: string | null;
  amount?: string | null;
  status: string;
  metadata?: Metadata | null;
};

type VenueSubjectSnapshot = {
  source: string;
  subjectType: string;
  subjectId: string;
  connections: VenueConnection[];
  accounts: VenueAccount[];
  beneficiaryProfiles: unknown[];
  walletAttestations: WalletAttestation[];
  sourceOfFundsPackages: SourceOfFundsPackage[];
};

type VenueTransferDetail = {
  source: string;
  transfer: VenueTransfer;
  connection?: VenueConnection | null;
  account?: VenueAccount | null;
  beneficiaryProfile?: unknown;
  sourceOfFundsPackages: SourceOfFundsPackage[];
};

type LighterConnectorRequirement = {
  code: string;
  status: string;
  message: string;
};

type LighterConnectorReadiness = {
  connectorKey: string;
  status: string;
  connectionId?: string | null;
  accountId?: string | null;
  publicPoolMode: string;
  proofAnchorMode: string;
  operatorLinkageStatus: string;
  institutionalEvidenceStatus: string;
  requirements: LighterConnectorRequirement[];
};

type CexConnectorRequirement = {
  code: string;
  status: string;
  message: string;
};

type CexConnectorReadiness = {
  connectorKey: string;
  status: string;
  connectionId?: string | null;
  accountId?: string | null;
  apiKeyMode?: string | null;
  subaccountMode?: string | null;
  withdrawalAllowlistStatus?: string | null;
  custodyBoundaryMode?: string | null;
  requirements: CexConnectorRequirement[];
};

type VenueTrustDirectionSummary = {
  transferCount: number;
  totalAmount: string;
  statusCounts: Record<string, number>;
  assetSymbols: string[];
  networks: string[];
};

type VenueTrustEvidenceReference = {
  kind: string;
  referenceId: string;
  source: string;
};

type VenueTrustReport = {
  source: string;
  subjectType: string;
  subjectId: string;
  cashIn: VenueTrustDirectionSummary;
  cashOut: VenueTrustDirectionSummary;
  evidenceReferences: VenueTrustEvidenceReference[];
};

type VenueTrustEvidenceExportArtifact = {
  fileName: string;
  contentType: string;
  contents: {
    subjectType?: string;
    subjectId?: string;
    evidenceReferences?: VenueTrustEvidenceReference[];
  };
};

async function adminRequest<T>(endpoint: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`/api/proxy${endpoint}`, init);
  if (!response.ok) {
    let message = "Venue review request failed";
    try {
      const payload = (await response.json()) as { message?: string };
      message = payload.message ?? message;
    } catch {}
    throw new Error(message);
  }
  return response.json() as Promise<T>;
}

function readReviewReason(value: string): string {
  return value.trim() || "admin_workbench_review";
}

export function VenueFundingWorkbench() {
  const cexConnectorKey = "binance";
  const [subjectId, setSubjectId] = useState("");
  const [transferId, setTransferId] = useState("");
  const [reviewReason, setReviewReason] = useState("admin_workbench_review");
  const [snapshot, setSnapshot] = useState<VenueSubjectSnapshot | null>(null);
  const [transferDetail, setTransferDetail] = useState<VenueTransferDetail | null>(null);
  const [lighterReadiness, setLighterReadiness] = useState<LighterConnectorReadiness | null>(null);
  const [cexReadiness, setCexReadiness] = useState<CexConnectorReadiness | null>(null);
  const [trustReport, setTrustReport] = useState<VenueTrustReport | null>(null);
  const [evidenceExport, setEvidenceExport] = useState<VenueTrustEvidenceExportArtifact | null>(null);
  const [loadingSubject, setLoadingSubject] = useState(false);
  const [loadingTransfer, setLoadingTransfer] = useState(false);
  const [loadingExport, setLoadingExport] = useState(false);
  const [actionKey, setActionKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  const activeReason = useMemo(() => readReviewReason(reviewReason), [reviewReason]);

  const loadSnapshot = useCallback(async () => {
    if (!subjectId.trim()) {
      setError("Subject id is required to inspect venue funding records.");
      return;
    }
    setLoadingSubject(true);
    setError(null);
    setStatusMessage(null);
    try {
      const encodedSubjectId = encodeURIComponent(subjectId.trim());
      const [snapshotPayload, readinessPayload, cexReadinessPayload, reportPayload] =
        await Promise.all([
        adminRequest<VenueSubjectSnapshot>(`/v1/admin/venue-trust/subjects/user/${encodedSubjectId}`),
        adminRequest<LighterConnectorReadiness>(
          `/v1/admin/venue-trust/connectors/lighter/readiness/user/${encodedSubjectId}`,
        ),
        adminRequest<CexConnectorReadiness>(
          `/v1/admin/venue-trust/connectors/cex/${cexConnectorKey}/readiness/user/${encodedSubjectId}`,
        ),
        adminRequest<VenueTrustReport>(
          `/v1/admin/venue-trust/reports/user/${encodedSubjectId}`,
        ),
      ]);
      setSnapshot(snapshotPayload);
      setLighterReadiness(readinessPayload);
      setCexReadiness(cexReadinessPayload);
      setTrustReport(reportPayload);
      setEvidenceExport(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load venue trust subject.");
    } finally {
      setLoadingSubject(false);
    }
  }, [subjectId]);

  const loadEvidenceExport = useCallback(async () => {
    if (!subjectId.trim()) {
      setError("Subject id is required to export venue trust evidence.");
      return;
    }
    setLoadingExport(true);
    setError(null);
    setStatusMessage(null);
    try {
      const payload = await adminRequest<VenueTrustEvidenceExportArtifact>(
        `/v1/admin/venue-trust/reports/user/${encodeURIComponent(subjectId.trim())}/export`,
      );
      setEvidenceExport(payload);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to export venue trust evidence.");
    } finally {
      setLoadingExport(false);
    }
  }, [subjectId]);

  const loadTransferDetail = useCallback(async () => {
    if (!transferId.trim()) {
      setError("Transfer id is required to inspect venue funding transfer detail.");
      return;
    }
    setLoadingTransfer(true);
    setError(null);
    setStatusMessage(null);
    try {
      const payload = await adminRequest<VenueTransferDetail>(
        `/v1/admin/venue-trust/transfers/${encodeURIComponent(transferId.trim())}`,
      );
      setTransferDetail(payload);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load venue funding transfer.");
    } finally {
      setLoadingTransfer(false);
    }
  }, [transferId]);

  const postReview = useCallback(
    async <T,>(
      endpoint: string,
      status: string,
      onSuccess: (payload: T) => void,
      successMessage: string,
      key: string,
    ) => {
      setActionKey(key);
      setError(null);
      setStatusMessage(null);
      try {
        const payload = await adminRequest<T>(endpoint, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            status,
            reviewReason: activeReason,
          }),
        });
        onSuccess(payload);
        setStatusMessage(successMessage);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Venue review action failed.");
      } finally {
        setActionKey(null);
      }
    },
    [activeReason],
  );

  const handleApproveConnection = useCallback(
    async (connection: VenueConnection) => {
      await postReview<VenueConnection>(
        `/v1/admin/venue-trust/connections/${encodeURIComponent(connection.connectionId)}/review`,
        "active",
        (payload) => {
          setSnapshot((current) =>
            current
              ? {
                  ...current,
                  connections: current.connections.map((item) =>
                    item.connectionId === payload.connectionId ? payload : item,
                  ),
                }
              : current,
          );
          setTransferDetail((current) =>
            current && current.connection?.connectionId === payload.connectionId
              ? { ...current, connection: payload }
              : current,
          );
        },
        "Connection updated to active",
        `connection:${connection.connectionId}`,
      );
    },
    [postReview],
  );

  const handleVerifyWallet = useCallback(
    async (attestation: WalletAttestation) => {
      await postReview<WalletAttestation>(
        `/v1/admin/venue-trust/wallet-attestations/${encodeURIComponent(attestation.attestationId)}/review`,
        "verified",
        (payload) => {
          setSnapshot((current) =>
            current
              ? {
                  ...current,
                  walletAttestations: current.walletAttestations.map((item) =>
                    item.attestationId === payload.attestationId ? payload : item,
                  ),
                }
              : current,
          );
        },
        "Wallet attestation updated to verified",
        `wallet:${attestation.attestationId}`,
      );
    },
    [postReview],
  );

  const handleApproveSourceOfFunds = useCallback(
    async (pkg: SourceOfFundsPackage) => {
      await postReview<SourceOfFundsPackage>(
        `/v1/admin/venue-trust/source-of-funds-packages/${encodeURIComponent(pkg.packageId)}/review`,
        "approved",
        (payload) => {
          setSnapshot((current) =>
            current
              ? {
                  ...current,
                  sourceOfFundsPackages: current.sourceOfFundsPackages.map((item) =>
                    item.packageId === payload.packageId ? payload : item,
                  ),
                }
              : current,
          );
          setTransferDetail((current) =>
            current
              ? {
                  ...current,
                  sourceOfFundsPackages: current.sourceOfFundsPackages.map((item) =>
                    item.packageId === payload.packageId ? payload : item,
                  ),
                }
              : current,
          );
        },
        "Source-of-funds package updated to approved",
        `sof:${pkg.packageId}`,
      );
    },
    [postReview],
  );

  const handleMarkTransferSubmitted = useCallback(
    async (transfer: VenueTransfer) => {
      await postReview<VenueTransfer>(
        `/v1/admin/venue-trust/transfers/${encodeURIComponent(transfer.transferId)}/review`,
        "submitted",
        (payload) => {
          setTransferDetail((current) => (current ? { ...current, transfer: payload } : current));
        },
        "Transfer updated to submitted",
        `transfer:${transfer.transferId}`,
      );
    },
    [postReview],
  );

  return (
    <div className="space-y-6" data-testid="venue-funding-workbench">
      <PageHeader
        title="Venue Funding Review"
        description="Review venue trust records and operator transitions without inventing a new backend queue."
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={() => {
              void loadSnapshot();
              void loadTransferDetail();
            }}
            disabled={loadingSubject || loadingTransfer}
            aria-label="Refresh venue review data"
          >
            <RefreshCw
              className={`h-4 w-4 ${loadingSubject || loadingTransfer ? "animate-spin" : ""}`}
            />
          </Button>
        }
      />

      <Card>
        <CardHeader>
          <CardTitle>Lookup controls</CardTitle>
          <CardDescription>
            Load a subject snapshot and transfer detail using the existing venue-trust admin seams.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 md:grid-cols-3">
          <div className="space-y-2">
            <Label htmlFor="venue-subject-id">Subject id</Label>
            <Input
              id="venue-subject-id"
              value={subjectId}
              onChange={(event) => setSubjectId(event.target.value)}
              placeholder="user-venue-1"
              aria-label="Subject id"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="venue-transfer-id">Transfer id</Label>
            <Input
              id="venue-transfer-id"
              value={transferId}
              onChange={(event) => setTransferId(event.target.value)}
              placeholder="transfer-venue-1"
              aria-label="Transfer id"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="venue-review-reason">Review reason</Label>
            <Input
              id="venue-review-reason"
              value={reviewReason}
              onChange={(event) => setReviewReason(event.target.value)}
              placeholder="admin_workbench_review"
              aria-label="Review reason"
            />
          </div>
          <div className="md:col-span-3 flex flex-wrap gap-3">
            <Button onClick={() => void loadSnapshot()} disabled={loadingSubject}>
              {loadingSubject && <Loader2 className="h-4 w-4 animate-spin" />}
              Load subject snapshot
            </Button>
            <Button variant="secondary" onClick={() => void loadTransferDetail()} disabled={loadingTransfer}>
              {loadingTransfer && <Loader2 className="h-4 w-4 animate-spin" />}
              Load transfer detail
            </Button>
          </div>
        </CardContent>
      </Card>

      {error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {statusMessage && (
        <div className="rounded-md border border-emerald-500/30 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-700 dark:text-emerald-300">
          {statusMessage}
        </div>
      )}

      <div className="grid gap-6 xl:grid-cols-[1.2fr_1fr]">
        <Card>
          <CardHeader>
            <CardTitle>Subject snapshot</CardTitle>
            <CardDescription>
              Connections, wallet attestations, and source-of-funds records tied to the requested user.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            {!snapshot ? (
              <div className="flex items-center gap-2 rounded-md border border-dashed border-white/10 px-4 py-8 text-sm text-muted-foreground">
                <ShieldAlert className="h-4 w-4" />
                Load a subject snapshot to review venue funding readiness.
              </div>
            ) : (
              <>
                <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
                  <Badge variant="outline">{snapshot.source}</Badge>
                  <span>{snapshot.subjectType}</span>
                  <span className="font-mono text-foreground">{snapshot.subjectId}</span>
                </div>

                <div className="space-y-3">
                  <h3 className="text-sm font-semibold text-foreground">Lighter readiness</h3>
                  {!lighterReadiness ? (
                    <p className="text-sm text-muted-foreground">
                      No Lighter connector readiness snapshot loaded for this subject.
                    </p>
                  ) : (
                    <div className="space-y-4 rounded-lg border border-white/10 p-4">
                      <div className="flex flex-wrap items-center gap-3 text-sm">
                        <Badge variant="outline">{lighterReadiness.connectorKey}</Badge>
                        <Badge variant={lighterReadiness.status === "ready" ? "default" : "secondary"}>
                          {lighterReadiness.status}
                        </Badge>
                      </div>
                      <div className="grid gap-3 sm:grid-cols-2">
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Connection id
                          </div>
                          <div className="font-mono text-sm">{lighterReadiness.connectionId ?? "-"}</div>
                        </div>
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Account id
                          </div>
                          <div className="font-mono text-sm">{lighterReadiness.accountId ?? "-"}</div>
                        </div>
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Public pool mode
                          </div>
                          <div className="text-sm">{lighterReadiness.publicPoolMode}</div>
                        </div>
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Proof anchor mode
                          </div>
                          <div className="text-sm">{lighterReadiness.proofAnchorMode}</div>
                        </div>
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Operator linkage
                          </div>
                          <div className="text-sm">{lighterReadiness.operatorLinkageStatus}</div>
                        </div>
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Institutional evidence
                          </div>
                          <div className="text-sm">{lighterReadiness.institutionalEvidenceStatus}</div>
                        </div>
                      </div>
                      <div className="space-y-3">
                        <h4 className="text-sm font-medium text-foreground">Requirements</h4>
                        {lighterReadiness.requirements.length === 0 ? (
                          <p className="text-sm text-muted-foreground">
                            No connector readiness requirements were returned.
                          </p>
                        ) : (
                          <Table>
                            <TableHeader>
                              <TableRow>
                                <TableHead>Requirement</TableHead>
                                <TableHead>Status</TableHead>
                                <TableHead>Message</TableHead>
                              </TableRow>
                            </TableHeader>
                            <TableBody>
                              {lighterReadiness.requirements.map((requirement) => (
                                <TableRow key={requirement.code}>
                                  <TableCell className="font-mono text-xs">{requirement.code}</TableCell>
                                  <TableCell>{requirement.status}</TableCell>
                                  <TableCell>{requirement.message}</TableCell>
                                </TableRow>
                              ))}
                            </TableBody>
                          </Table>
                        )}
                      </div>
                    </div>
                  )}
                </div>

                <div className="space-y-3">
                  <h3 className="text-sm font-semibold text-foreground">CEX readiness</h3>
                  {!cexReadiness ? (
                    <p className="text-sm text-muted-foreground">
                      No generic CEX connector readiness snapshot loaded for this subject.
                    </p>
                  ) : (
                    <div className="space-y-4 rounded-lg border border-white/10 p-4">
                      <div className="flex flex-wrap items-center gap-3 text-sm">
                        <Badge variant="outline">{cexReadiness.connectorKey}</Badge>
                        <Badge variant={cexReadiness.status === "ready" ? "default" : "secondary"}>
                          {cexReadiness.status}
                        </Badge>
                      </div>
                      <div className="grid gap-3 sm:grid-cols-2">
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Connection id
                          </div>
                          <div className="font-mono text-sm">{cexReadiness.connectionId ?? "-"}</div>
                        </div>
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Account id
                          </div>
                          <div className="font-mono text-sm">{cexReadiness.accountId ?? "-"}</div>
                        </div>
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            API key mode
                          </div>
                          <div className="text-sm">{cexReadiness.apiKeyMode ?? "-"}</div>
                        </div>
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Subaccount mode
                          </div>
                          <div className="text-sm">{cexReadiness.subaccountMode ?? "-"}</div>
                        </div>
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Withdrawal allowlist
                          </div>
                          <div className="text-sm">
                            {cexReadiness.withdrawalAllowlistStatus ?? "-"}
                          </div>
                        </div>
                        <div>
                          <div className="text-xs uppercase tracking-wide text-muted-foreground">
                            Custody boundary
                          </div>
                          <div className="text-sm">{cexReadiness.custodyBoundaryMode ?? "-"}</div>
                        </div>
                      </div>
                      <div className="space-y-3">
                        <h4 className="text-sm font-medium text-foreground">Requirements</h4>
                        {cexReadiness.requirements.length === 0 ? (
                          <p className="text-sm text-muted-foreground">
                            No generic CEX readiness requirements were returned.
                          </p>
                        ) : (
                          <Table>
                            <TableHeader>
                              <TableRow>
                                <TableHead>Requirement</TableHead>
                                <TableHead>Status</TableHead>
                                <TableHead>Message</TableHead>
                              </TableRow>
                            </TableHeader>
                            <TableBody>
                              {cexReadiness.requirements.map((requirement) => (
                                <TableRow key={requirement.code}>
                                  <TableCell className="font-mono text-xs">{requirement.code}</TableCell>
                                  <TableCell>{requirement.status}</TableCell>
                                  <TableCell>{requirement.message}</TableCell>
                                </TableRow>
                              ))}
                            </TableBody>
                          </Table>
                        )}
                      </div>
                    </div>
                  )}
                </div>

                <div className="space-y-3">
                  <div className="flex items-center justify-between gap-3">
                    <h3 className="text-sm font-semibold text-foreground">Trust report</h3>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => void loadEvidenceExport()}
                      disabled={loadingExport}
                    >
                      {loadingExport && <Loader2 className="h-4 w-4 animate-spin" />}
                      Load evidence export
                    </Button>
                  </div>
                  {!trustReport ? (
                    <p className="text-sm text-muted-foreground">
                      No trust report snapshot loaded for this subject.
                    </p>
                  ) : (
                    <div className="space-y-4 rounded-lg border border-white/10 p-4">
                      <div className="grid gap-3 sm:grid-cols-2">
                        <div className="space-y-2 rounded-md border border-white/10 p-3">
                          <div className="text-sm font-medium text-foreground">Cash in</div>
                          <div className="text-sm text-muted-foreground">
                            Transfers: {trustReport.cashIn.transferCount}
                          </div>
                          <div className="text-sm text-muted-foreground">
                            Total amount: {trustReport.cashIn.totalAmount}
                          </div>
                          <div className="text-sm text-muted-foreground">
                            Assets: {trustReport.cashIn.assetSymbols.join(", ") || "-"}
                          </div>
                          <div className="text-sm text-muted-foreground">
                            Networks: {trustReport.cashIn.networks.join(", ") || "-"}
                          </div>
                        </div>
                        <div className="space-y-2 rounded-md border border-white/10 p-3">
                          <div className="text-sm font-medium text-foreground">Cash out</div>
                          <div className="text-sm text-muted-foreground">
                            Transfers: {trustReport.cashOut.transferCount}
                          </div>
                          <div className="text-sm text-muted-foreground">
                            Total amount: {trustReport.cashOut.totalAmount}
                          </div>
                          <div className="text-sm text-muted-foreground">
                            Assets: {trustReport.cashOut.assetSymbols.join(", ") || "-"}
                          </div>
                          <div className="text-sm text-muted-foreground">
                            Networks: {trustReport.cashOut.networks.join(", ") || "-"}
                          </div>
                        </div>
                      </div>
                      <div className="space-y-3">
                        <h4 className="text-sm font-medium text-foreground">Evidence references</h4>
                        {trustReport.evidenceReferences.length === 0 ? (
                          <p className="text-sm text-muted-foreground">
                            No evidence references were returned for this subject.
                          </p>
                        ) : (
                          <Table>
                            <TableHeader>
                              <TableRow>
                                <TableHead>Kind</TableHead>
                                <TableHead>Reference</TableHead>
                                <TableHead>Source</TableHead>
                              </TableRow>
                            </TableHeader>
                            <TableBody>
                              {trustReport.evidenceReferences.map((reference) => (
                                <TableRow key={`${reference.kind}:${reference.referenceId}`}>
                                  <TableCell>{reference.kind}</TableCell>
                                  <TableCell className="font-mono text-xs">
                                    {reference.referenceId}
                                  </TableCell>
                                  <TableCell>{reference.source}</TableCell>
                                </TableRow>
                              ))}
                            </TableBody>
                          </Table>
                        )}
                      </div>
                      {evidenceExport && (
                        <div className="space-y-2 rounded-md border border-white/10 p-3">
                          <div className="text-sm font-medium text-foreground">
                            Evidence export artifact
                          </div>
                          <div className="font-mono text-xs">{evidenceExport.fileName}</div>
                          <div className="text-sm text-muted-foreground">
                            Content type: {evidenceExport.contentType}
                          </div>
                          <div className="text-sm text-muted-foreground">
                            Export references:{" "}
                            {evidenceExport.contents.evidenceReferences?.length ?? 0}
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </div>

                <div className="space-y-3">
                  <h3 className="text-sm font-semibold text-foreground">Connections</h3>
                  {snapshot.connections.length === 0 ? (
                    <p className="text-sm text-muted-foreground">No venue connections found.</p>
                  ) : (
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>ID</TableHead>
                          <TableHead>Venue</TableHead>
                          <TableHead>Status</TableHead>
                          <TableHead>Mode</TableHead>
                          <TableHead className="text-right">Action</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {snapshot.connections.map((connection) => (
                          <TableRow key={connection.connectionId}>
                            <TableCell className="font-mono text-xs">{connection.connectionId}</TableCell>
                            <TableCell>{connection.venueKey}</TableCell>
                            <TableCell>{connection.status}</TableCell>
                            <TableCell>{connection.connectionMode ?? "-"}</TableCell>
                            <TableCell className="text-right">
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => void handleApproveConnection(connection)}
                                disabled={actionKey === `connection:${connection.connectionId}`}
                              >
                                Approve connection
                              </Button>
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  )}
                </div>

                <div className="space-y-3">
                  <h3 className="text-sm font-semibold text-foreground">Wallet attestation</h3>
                  {snapshot.walletAttestations.length === 0 ? (
                    <p className="text-sm text-muted-foreground">No wallet attestations found.</p>
                  ) : (
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Wallet</TableHead>
                          <TableHead>Network</TableHead>
                          <TableHead>Status</TableHead>
                          <TableHead>Proof</TableHead>
                          <TableHead className="text-right">Action</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {snapshot.walletAttestations.map((attestation) => (
                          <TableRow key={attestation.attestationId}>
                            <TableCell className="font-mono text-xs">
                              {attestation.walletAddress ?? attestation.attestationId}
                            </TableCell>
                            <TableCell>{attestation.network ?? "-"}</TableCell>
                            <TableCell>{attestation.attestationStatus}</TableCell>
                            <TableCell className="break-all text-xs">
                              {attestation.proofArtifactUri ?? "-"}
                            </TableCell>
                            <TableCell className="text-right">
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => void handleVerifyWallet(attestation)}
                                disabled={actionKey === `wallet:${attestation.attestationId}`}
                              >
                                Verify wallet
                              </Button>
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  )}
                </div>
              </>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Transfer detail</CardTitle>
            <CardDescription>
              Transfer-level review for wallet-to-venue progression and source-of-funds evidence.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            {!transferDetail ? (
              <div className="flex items-center gap-2 rounded-md border border-dashed border-white/10 px-4 py-8 text-sm text-muted-foreground">
                <ShieldAlert className="h-4 w-4" />
                Load a transfer id to inspect operator review state.
              </div>
            ) : (
              <>
                <div className="grid gap-3 rounded-lg border border-white/10 p-4 sm:grid-cols-2">
                  <div>
                    <div className="text-xs uppercase tracking-wide text-muted-foreground">Transfer id</div>
                    <div className="font-mono text-sm">{transferDetail.transfer.transferId}</div>
                  </div>
                  <div>
                    <div className="text-xs uppercase tracking-wide text-muted-foreground">Status</div>
                    <div className="text-sm">{transferDetail.transfer.status}</div>
                  </div>
                  <div>
                    <div className="text-xs uppercase tracking-wide text-muted-foreground">Direction</div>
                    <div className="text-sm">{transferDetail.transfer.transferDirection ?? "-"}</div>
                  </div>
                  <div>
                    <div className="text-xs uppercase tracking-wide text-muted-foreground">Asset</div>
                    <div className="text-sm">
                      {transferDetail.transfer.assetSymbol ?? "-"} / {transferDetail.transfer.network ?? "-"}
                    </div>
                  </div>
                  <div>
                    <div className="text-xs uppercase tracking-wide text-muted-foreground">Amount</div>
                    <div className="text-sm">{transferDetail.transfer.amount ?? "-"}</div>
                  </div>
                  <div>
                    <div className="text-xs uppercase tracking-wide text-muted-foreground">Venue</div>
                    <div className="text-sm">{transferDetail.connection?.venueKey ?? "-"}</div>
                  </div>
                </div>

                <div className="flex flex-wrap gap-3">
                  <Button
                    variant="outline"
                    onClick={() => void handleMarkTransferSubmitted(transferDetail.transfer)}
                    disabled={actionKey === `transfer:${transferDetail.transfer.transferId}`}
                  >
                    Mark transfer submitted
                  </Button>
                </div>

                <div className="space-y-3">
                  <h3 className="text-sm font-semibold text-foreground">Source-of-funds packages</h3>
                  {transferDetail.sourceOfFundsPackages.length === 0 ? (
                    <p className="text-sm text-muted-foreground">No source-of-funds packages linked.</p>
                  ) : (
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Package</TableHead>
                          <TableHead>Status</TableHead>
                          <TableHead className="text-right">Action</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {transferDetail.sourceOfFundsPackages.map((pkg) => (
                          <TableRow key={pkg.packageId}>
                            <TableCell className="font-mono text-xs">{pkg.packageId}</TableCell>
                            <TableCell>{pkg.reviewStatus}</TableCell>
                            <TableCell className="text-right">
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => void handleApproveSourceOfFunds(pkg)}
                                disabled={actionKey === `sof:${pkg.packageId}`}
                              >
                                Approve source of funds
                              </Button>
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  )}
                </div>
              </>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
