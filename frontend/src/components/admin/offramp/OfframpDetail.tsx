"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Input } from "@/components/ui/input";
import { X, Check, Flag, Clock, ArrowRight } from "lucide-react";
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
  "PENDING",
  "AWAITING_APPROVAL",
  "APPROVED",
  "PROCESSING",
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

  const canApprove = intent.status === "AWAITING_APPROVAL" || intent.status === "PENDING";
  const canReject = intent.status === "AWAITING_APPROVAL" || intent.status === "PENDING";

  const currentStepIndex = STATUS_TIMELINE.indexOf(intent.status);

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
              <p className="font-mono">{intent.user_id}</p>
            </div>
            <div>
              <span className="text-muted-foreground">Status</span>
              <div className="mt-1">
                <StatusBadge status={intent.status} showDot />
              </div>
            </div>
            <div>
              <span className="text-muted-foreground">Created</span>
              <p>{formatDate(intent.created_at)}</p>
            </div>
            {intent.completed_at && (
              <div>
                <span className="text-muted-foreground">Completed</span>
                <p>{formatDate(intent.completed_at)}</p>
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
                {intent.amount_crypto} {intent.crypto_currency}
              </p>
            </div>
            <div>
              <span className="text-muted-foreground">VND Amount</span>
              <p className="font-mono font-bold">{formatVND(intent.amount_vnd)}</p>
            </div>
            <div>
              <span className="text-muted-foreground">Exchange Rate</span>
              <p className="font-mono">{intent.exchange_rate}</p>
            </div>
            <div>
              <span className="text-muted-foreground">Fee</span>
              <p className="font-mono">
                {intent.fee_amount} {intent.fee_currency}
              </p>
            </div>
            {intent.tx_hash && (
              <div className="col-span-2">
                <span className="text-muted-foreground">Tx Hash</span>
                <p className="font-mono text-xs break-all">{intent.tx_hash}</p>
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
              const isCurrent = step === intent.status;
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

      {/* Bank Transfer Details */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Bank Transfer Details</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-muted-foreground">Bank</span>
              <p>{intent.bank_name}</p>
            </div>
            <div>
              <span className="text-muted-foreground">Account Number</span>
              <p className="font-mono">{intent.bank_account_number}</p>
            </div>
            <div>
              <span className="text-muted-foreground">Account Name</span>
              <p>{intent.bank_account_name}</p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Reject Reason */}
      {intent.reject_reason && (
        <Card className="border-red-500/50">
          <CardHeader>
            <CardTitle className="text-sm font-medium text-red-600">Rejection Reason</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm">{intent.reject_reason}</p>
          </CardContent>
        </Card>
      )}

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
