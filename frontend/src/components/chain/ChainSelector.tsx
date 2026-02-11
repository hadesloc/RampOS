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
import { type ChainInfo } from "@/hooks/use-intent-builder";

interface ChainSelectorProps {
  label: string;
  value: string;
  chains: ChainInfo[];
  onChange: (chainId: string) => void;
  disabled?: boolean;
}

export function ChainSelector({
  label,
  value,
  chains,
  onChange,
  disabled = false,
}: ChainSelectorProps) {
  return (
    <div className="flex flex-col gap-2">
      <Label>{label}</Label>
      <Select value={value} onValueChange={onChange} disabled={disabled}>
        <SelectTrigger data-testid={`chain-selector-${label.toLowerCase().replace(/\s+/g, "-")}`}>
          <SelectValue placeholder="Select chain" />
        </SelectTrigger>
        <SelectContent>
          {chains.map((chain) => (
            <SelectItem key={chain.id} value={chain.id}>
              <span className="flex items-center gap-2">
                <span className="font-mono text-xs text-muted-foreground">
                  {chain.icon}
                </span>
                <span>{chain.name}</span>
              </span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
