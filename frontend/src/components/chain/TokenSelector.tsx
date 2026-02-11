"use client";

import * as React from "react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { type TokenInfo } from "@/hooks/use-intent-builder";

interface TokenSelectorProps {
  label: string;
  value: string;
  tokens: TokenInfo[];
  onChange: (tokenAddress: string) => void;
  disabled?: boolean;
}

export function TokenSelector({
  label,
  value,
  tokens,
  onChange,
  disabled = false,
}: TokenSelectorProps) {
  return (
    <div className="flex flex-col gap-2">
      <Label>{label}</Label>
      <Select
        value={value}
        onValueChange={onChange}
        disabled={disabled || tokens.length === 0}
      >
        <SelectTrigger data-testid={`token-selector-${label.toLowerCase().replace(/\s+/g, "-")}`}>
          <SelectValue placeholder={tokens.length === 0 ? "Select chain first" : "Select token"} />
        </SelectTrigger>
        <SelectContent>
          {tokens.map((token) => (
            <SelectItem key={token.address} value={token.address}>
              <span className="flex items-center gap-2">
                <span className="font-semibold">{token.symbol}</span>
                <span className="text-xs text-muted-foreground">{token.name}</span>
                {token.balance && (
                  <span className="ml-auto text-xs text-muted-foreground">
                    {token.balance}
                  </span>
                )}
              </span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
