"use client";

import * as React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { ChainSelector } from "./ChainSelector";
import { TokenSelector } from "./TokenSelector";
import { IntentPreview } from "./IntentPreview";
import { useIntentBuilder } from "@/hooks/use-intent-builder";

export function IntentBuilder() {
  const {
    state,
    chains,
    sourceTokens,
    destTokens,
    setSourceChain,
    setDestChain,
    setSourceToken,
    setDestToken,
    setAmount,
    buildIntent,
    executeIntent,
    canBuild,
    canExecute,
  } = useIntentBuilder();

  const isLoading =
    state.status === "building" ||
    state.status === "estimating" ||
    state.status === "executing";

  return (
    <div className="space-y-4" data-testid="intent-builder">
      <Card>
        <CardHeader>
          <CardTitle>Cross-Chain Intent Builder</CardTitle>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Source section */}
          <div className="space-y-4">
            <h3 className="text-sm font-medium">Source</h3>
            <div className="grid grid-cols-2 gap-4">
              <ChainSelector
                label="Source Chain"
                value={state.sourceChain}
                chains={chains}
                onChange={setSourceChain}
                disabled={isLoading}
              />
              <TokenSelector
                label="Source Token"
                value={state.sourceToken}
                tokens={sourceTokens}
                onChange={setSourceToken}
                disabled={isLoading}
              />
            </div>
          </div>

          {/* Amount */}
          <div className="flex flex-col gap-2">
            <Label htmlFor="amount">Amount</Label>
            <Input
              id="amount"
              type="number"
              placeholder="0.00"
              value={state.amount}
              onChange={(e) => setAmount(e.target.value)}
              disabled={isLoading}
              min="0"
              step="any"
            />
          </div>

          {/* Destination section */}
          <div className="space-y-4">
            <h3 className="text-sm font-medium">Destination</h3>
            <div className="grid grid-cols-2 gap-4">
              <ChainSelector
                label="Destination Chain"
                value={state.destChain}
                chains={chains}
                onChange={setDestChain}
                disabled={isLoading}
              />
              <TokenSelector
                label="Destination Token"
                value={state.destToken}
                tokens={destTokens}
                onChange={setDestToken}
                disabled={isLoading}
              />
            </div>
          </div>

          {/* Status badges */}
          {state.status !== "idle" && (
            <div className="flex items-center gap-2">
              {state.status === "success" && (
                <Badge variant="success" dot>
                  Intent executed successfully
                </Badge>
              )}
              {state.status === "error" && (
                <Badge variant="destructive" dot data-testid="error-badge">
                  {state.error ?? "An error occurred"}
                </Badge>
              )}
              {(state.status === "building" || state.status === "estimating") && (
                <Badge variant="info" dot>
                  Building route...
                </Badge>
              )}
              {state.status === "executing" && (
                <Badge variant="warning" dot>
                  Executing intent...
                </Badge>
              )}
            </div>
          )}

          {/* Action buttons */}
          <div className="flex gap-3">
            <Button
              onClick={() => buildIntent()}
              disabled={!canBuild || isLoading}
              variant="outline"
              isLoading={state.status === "building"}
            >
              Preview Route
            </Button>
            <Button
              onClick={() => executeIntent()}
              disabled={!canExecute || isLoading}
              isLoading={state.status === "executing"}
            >
              Execute Intent
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Route preview */}
      <IntentPreview
        route={state.route}
        isLoading={state.status === "building" || state.status === "estimating"}
      />
    </div>
  );
}
