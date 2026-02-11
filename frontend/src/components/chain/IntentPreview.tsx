"use client";

import * as React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { type IntentRoute } from "@/hooks/use-intent-builder";

interface IntentPreviewProps {
  route: IntentRoute | null;
  isLoading?: boolean;
}

function StepTypeBadge({ type }: { type: string }) {
  const variant =
    type === "swap" ? "info" : type === "bridge" ? "warning" : "secondary";
  return (
    <Badge variant={variant} shape="pill">
      {type}
    </Badge>
  );
}

export function IntentPreview({ route, isLoading = false }: IntentPreviewProps) {
  if (isLoading) {
    return (
      <Card data-testid="intent-preview">
        <CardHeader>
          <CardTitle className="text-sm">Route Preview</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-8 w-3/4" />
          <Skeleton className="h-4 w-1/2" />
        </CardContent>
      </Card>
    );
  }

  if (!route) {
    return (
      <Card data-testid="intent-preview">
        <CardHeader>
          <CardTitle className="text-sm">Route Preview</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Configure your intent to see the execution route.
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card data-testid="intent-preview">
      <CardHeader>
        <CardTitle className="text-sm">Route Preview</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          {route.steps.map((step, idx) => (
            <div
              key={idx}
              className="flex items-center gap-2 rounded-md border p-2 text-sm"
              data-testid="route-step"
            >
              <span className="text-muted-foreground">{idx + 1}.</span>
              <StepTypeBadge type={step.type} />
              <span>
                {step.fromToken} ({step.fromChain})
              </span>
              <span className="text-muted-foreground">-&gt;</span>
              <span>
                {step.toToken} ({step.toChain})
              </span>
              <span className="ml-auto text-xs text-muted-foreground">
                ~{step.estimatedTime}s via {step.protocol}
              </span>
            </div>
          ))}
        </div>

        <div className="flex items-center justify-between border-t pt-3">
          <div className="text-sm">
            <span className="text-muted-foreground">Est. Time: </span>
            <span data-testid="estimated-time">
              {Math.ceil(route.estimatedTotalTime / 60)} min
            </span>
          </div>
          <div className="text-sm">
            <span className="text-muted-foreground">Total Fees: </span>
            <span data-testid="total-fees">
              {route.estimatedFees.total} {route.estimatedFees.currency}
            </span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
