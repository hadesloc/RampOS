"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Input } from "@/components/ui/input";
import { X, Check, ArrowRight } from "lucide-react";
import type { OfframpIntent } from "@/hooks/use-admin-offramp";

interface OfframpDetailProps {
  intent: OfframpIntent;
  onApprove?: (id: string) => void;
  onReject?: (id: string, reason: string) => void;
  onClose?: () => void;
  approving?: boolean;
  rejecting?: boolean;
}

function formatVND(amount: string): string {
  const num = parseInt(amount, 10);
  if (isNaN(num)) return amount;
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
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

const STATUS_TIMELINE: string[] = [
  "QUOTE_CREATED",
  "CRYPTO_PENDING",
  "CRYPTO_RECEIVED",
  "VND_TRANSFERRING",
  "COMPLETED",
];

export function OfframpDetail({
  intent,
  onApprove,
  onReject,
  onClose,
  approving = false,
  rejecting = false,
}: OfframpDetailProps) {
  const [rejectReason, setRejectReason] = useState("");
  const [showRejectInput, setShowRejectInput] = useState(false);

  const displayState = intent.state;
  const userId = intent.userId ?? "-";
  const cryptoAmount = intent.cryptoAmount ?? "";
  const cryptoAsset = intent.cryptoAsset ?? "";
  const vndAmount = intent.netVndAmount ?? "";
  const exchangeRate = intent.exchangeRate ?? "";
  const txHash = intent.txHash;
  const createdAt = intent.createdAt ?? "";
  const completedAt = displayState === "COMPLETED" ? intent.updatedAt : undefined;
  const canApprove = displayState === "CRYPTO_RECEIVED";
  const canReject = displayState === "CRYPTO_RECEIVED";

  const currentStepIndex = STATUS_TIMELINE.indexOf(displayState);

  return (
    <div className="space-y-6" data-testid="offramp-detail">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-bold">Off-Ramp Intent Detail</h2>
        {onClose && (
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-4 w-4" />
          </Button>
        )}
      </div>

      {/* Intent Metadata */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Intent Information</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-muted-foreground">ID</span>
              <p className="font-mono">{intent.id}</p>
            </div>
            <div>
              <span className="text-muted-foreground">User</span>
              <p className="font-mono">{userId}</p>
            </div>
            <div>
              <span className="text-muted-foreground">Status</span>
              <div className="mt-1">
                <StatusBadge status={displayState} showDot />
              </div>
            </div>
            <div>
              <span className="text-muted-foreground">Created</span>
              <p>{formatDate(createdAt)}</p>
            </div>
            {completedAt && (
              <div>
                <span className="text-muted-foreground">Completed</span>
                <p>{formatDate(completedAt)}</p>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Transaction Details */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Transaction Details</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-muted-foreground">Crypto Amount</span>
              <p className="font-mono font-bold">
                {cryptoAmount} {cryptoAsset}
              </p>
            </div>
            <div>
              <span className="text-muted-foreground">VND Amount</span>
              <p className="font-mono font-bold">{formatVND(vndAmount)}</p>
            </div>
            <div>
              <span className="text-muted-foreground">Exchange Rate</span>
              <p className="font-mono">{exchangeRate}</p>
            </div>
            {txHash && (
              <div className="col-span-2">
                <span className="text-muted-foreground">Tx Hash</span>
                <p className="font-mono text-xs break-all">{txHash}</p>
              </div>
            )}
            {intent.linkedRfqId && (
              <div>
                <span className="text-muted-foreground">Linked RFQ</span>
                <p className="font-mono text-xs">{intent.linkedRfqId}</p>
              </div>
            )}
            {intent.winningLpId && (
              <div>
                <span className="text-muted-foreground">Winning LP</span>
                <p className="font-mono text-xs">{intent.winningLpId}</p>
              </div>
            )}
            {intent.matchedRate && (
              <div>
                <span className="text-muted-foreground">Matched Rate</span>
                <p className="font-mono text-xs">{intent.matchedRate}</p>
              </div>
            )}
            {intent.settlementId && (
              <div>
                <span className="text-muted-foreground">Settlement</span>
                <p className="font-mono text-xs">{intent.settlementId}</p>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Status Timeline */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Status Timeline</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-2" data-testid="status-timeline">
            {STATUS_TIMELINE.map((step, index) => {
              const isActive = index <= currentStepIndex && currentStepIndex >= 0;
              const isCurrent = step === displayState;
              return (
                <div key={step} className="flex items-center gap-2">
                  <div
                    className={`flex items-center gap-1 px-2 py-1 rounded text-xs ${
                      isCurrent
                        ? "bg-primary text-primary-foreground font-bold"
                        : isActive
                        ? "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400"
                        : "bg-muted text-muted-foreground"
                    }`}
                  >
                    {isActive && <Check className="h-3 w-3" />}
                    {step.replace(/_/g, " ")}
                  </div>
                  {index < STATUS_TIMELINE.length - 1 && (
                    <ArrowRight className="h-3 w-3 text-muted-foreground" />
                  )}
                </div>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* Action Buttons */}
      {(canApprove || canReject) && (
        <div className="flex gap-3" data-testid="offramp-actions">
          {canApprove && (
            <Button
              onClick={() => onApprove?.(intent.id)}
              disabled={approving}
              data-testid="approve-btn"
            >
              <Check className="mr-2 h-4 w-4" />
              {approving ? "Approving..." : "Approve"}
            </Button>
          )}
          {canReject && !showRejectInput && (
            <Button
              variant="destructive"
              onClick={() => setShowRejectInput(true)}
              data-testid="reject-btn"
            >
              <X className="mr-2 h-4 w-4" />
              Reject
            </Button>
          )}
          {showRejectInput && (
            <div className="flex gap-2 flex-1">
              <Input
                placeholder="Enter rejection reason..."
                value={rejectReason}
                onChange={(e) => setRejectReason(e.target.value)}
                data-testid="reject-reason-input"
              />
              <Button
                variant="destructive"
                onClick={() => {
                  if (rejectReason.trim()) {
                    onReject?.(intent.id, rejectReason.trim());
                    setShowRejectInput(false);
                    setRejectReason("");
                  }
                }}
                disabled={rejecting || !rejectReason.trim()}
                data-testid="confirm-reject-btn"
              >
                {rejecting ? "Rejecting..." : "Confirm Reject"}
              </Button>
              <Button
                variant="ghost"
                onClick={() => {
                  setShowRejectInput(false);
                  setRejectReason("");
                }}
              >
                Cancel
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
