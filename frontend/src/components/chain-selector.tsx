"use client";

import * as React from "react";
import { useState, useEffect, useMemo, useCallback } from "react";
import {
  Check,
  ChevronDown,
  Search,
  Clock,
  Fuel,
  Zap,
  Wallet,
  AlertCircle,
  CheckCircle,
  XCircle,
  Loader2,
  ExternalLink,
  RefreshCw,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";

// Types
export type ChainId =
  | "ethereum"
  | "arbitrum"
  | "base"
  | "optimism"
  | "polygon"
  | "avalanche"
  | "bsc"
  | "zksync";

export type NetworkHealth = "healthy" | "degraded" | "down";

export interface ChainInfo {
  id: ChainId;
  name: string;
  shortName: string;
  chainId: number;
  nativeCurrency: {
    name: string;
    symbol: string;
    decimals: number;
  };
  rpcUrls: string[];
  blockExplorerUrl: string;
  iconUrl?: string;
  color: string;
  isTestnet: boolean;
  isL2: boolean;
}

export interface ChainGasEstimate {
  chainId: ChainId;
  gasPrice: string;
  gasPriceGwei: number;
  estimatedCostUsd: number;
  lastUpdated: string;
}

export interface ChainTransactionTime {
  chainId: ChainId;
  avgBlockTime: number;
  estimatedConfirmationTime: number;
  congestionLevel: "low" | "medium" | "high";
}

export interface ChainBalance {
  chainId: ChainId;
  nativeBalance: string;
  nativeBalanceUsd: number;
  tokens: {
    symbol: string;
    balance: string;
    balanceUsd: number;
  }[];
}

export interface ChainStatus {
  chainId: ChainId;
  health: NetworkHealth;
  latency: number;
  lastBlock: number;
  lastBlockTime: string;
}

export interface ChainSelectorProps {
  selectedChain: ChainId;
  onChainChange: (chainId: ChainId) => void;
  chains?: ChainId[];
  showGasEstimates?: boolean;
  showTransactionTime?: boolean;
  showBalances?: boolean;
  showNetworkHealth?: boolean;
  disabled?: boolean;
  className?: string;
  placeholder?: string;
  size?: "sm" | "md" | "lg";
  variant?: "default" | "outline" | "ghost";
}

// Chain configuration data
const CHAIN_CONFIG: Record<ChainId, ChainInfo> = {
  ethereum: {
    id: "ethereum",
    name: "Ethereum",
    shortName: "ETH",
    chainId: 1,
    nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
    rpcUrls: ["https://eth.llamarpc.com"],
    blockExplorerUrl: "https://etherscan.io",
    color: "#627EEA",
    isTestnet: false,
    isL2: false,
  },
  arbitrum: {
    id: "arbitrum",
    name: "Arbitrum One",
    shortName: "ARB",
    chainId: 42161,
    nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
    rpcUrls: ["https://arb1.arbitrum.io/rpc"],
    blockExplorerUrl: "https://arbiscan.io",
    color: "#28A0F0",
    isTestnet: false,
    isL2: true,
  },
  base: {
    id: "base",
    name: "Base",
    shortName: "BASE",
    chainId: 8453,
    nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
    rpcUrls: ["https://mainnet.base.org"],
    blockExplorerUrl: "https://basescan.org",
    color: "#0052FF",
    isTestnet: false,
    isL2: true,
  },
  optimism: {
    id: "optimism",
    name: "Optimism",
    shortName: "OP",
    chainId: 10,
    nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
    rpcUrls: ["https://mainnet.optimism.io"],
    blockExplorerUrl: "https://optimistic.etherscan.io",
    color: "#FF0420",
    isTestnet: false,
    isL2: true,
  },
  polygon: {
    id: "polygon",
    name: "Polygon",
    shortName: "MATIC",
    chainId: 137,
    nativeCurrency: { name: "MATIC", symbol: "MATIC", decimals: 18 },
    rpcUrls: ["https://polygon-rpc.com"],
    blockExplorerUrl: "https://polygonscan.com",
    color: "#8247E5",
    isTestnet: false,
    isL2: false,
  },
  avalanche: {
    id: "avalanche",
    name: "Avalanche",
    shortName: "AVAX",
    chainId: 43114,
    nativeCurrency: { name: "Avalanche", symbol: "AVAX", decimals: 18 },
    rpcUrls: ["https://api.avax.network/ext/bc/C/rpc"],
    blockExplorerUrl: "https://snowtrace.io",
    color: "#E84142",
    isTestnet: false,
    isL2: false,
  },
  bsc: {
    id: "bsc",
    name: "BNB Chain",
    shortName: "BNB",
    chainId: 56,
    nativeCurrency: { name: "BNB", symbol: "BNB", decimals: 18 },
    rpcUrls: ["https://bsc-dataseed.binance.org"],
    blockExplorerUrl: "https://bscscan.com",
    color: "#F0B90B",
    isTestnet: false,
    isL2: false,
  },
  zksync: {
    id: "zksync",
    name: "zkSync Era",
    shortName: "ZK",
    chainId: 324,
    nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
    rpcUrls: ["https://mainnet.era.zksync.io"],
    blockExplorerUrl: "https://explorer.zksync.io",
    color: "#8C8DFC",
    isTestnet: false,
    isL2: true,
  },
};

// Default chains to show
const DEFAULT_CHAINS: ChainId[] = [
  "ethereum",
  "arbitrum",
  "base",
  "optimism",
  "polygon",
];

// Local storage key for recently used chains
const RECENT_CHAINS_KEY = "ramp-recent-chains";
const MAX_RECENT_CHAINS = 3;

// Mock data generators (replace with actual API calls)
function generateMockGasEstimates(): Record<ChainId, ChainGasEstimate> {
  return {
    ethereum: {
      chainId: "ethereum",
      gasPrice: "25000000000",
      gasPriceGwei: 25,
      estimatedCostUsd: 3.5,
      lastUpdated: new Date().toISOString(),
    },
    arbitrum: {
      chainId: "arbitrum",
      gasPrice: "100000000",
      gasPriceGwei: 0.1,
      estimatedCostUsd: 0.15,
      lastUpdated: new Date().toISOString(),
    },
    base: {
      chainId: "base",
      gasPrice: "50000000",
      gasPriceGwei: 0.05,
      estimatedCostUsd: 0.08,
      lastUpdated: new Date().toISOString(),
    },
    optimism: {
      chainId: "optimism",
      gasPrice: "1000000",
      gasPriceGwei: 0.001,
      estimatedCostUsd: 0.05,
      lastUpdated: new Date().toISOString(),
    },
    polygon: {
      chainId: "polygon",
      gasPrice: "50000000000",
      gasPriceGwei: 50,
      estimatedCostUsd: 0.02,
      lastUpdated: new Date().toISOString(),
    },
    avalanche: {
      chainId: "avalanche",
      gasPrice: "25000000000",
      gasPriceGwei: 25,
      estimatedCostUsd: 0.5,
      lastUpdated: new Date().toISOString(),
    },
    bsc: {
      chainId: "bsc",
      gasPrice: "3000000000",
      gasPriceGwei: 3,
      estimatedCostUsd: 0.1,
      lastUpdated: new Date().toISOString(),
    },
    zksync: {
      chainId: "zksync",
      gasPrice: "250000000",
      gasPriceGwei: 0.25,
      estimatedCostUsd: 0.12,
      lastUpdated: new Date().toISOString(),
    },
  };
}

function generateMockTransactionTimes(): Record<ChainId, ChainTransactionTime> {
  return {
    ethereum: {
      chainId: "ethereum",
      avgBlockTime: 12,
      estimatedConfirmationTime: 180,
      congestionLevel: "medium",
    },
    arbitrum: {
      chainId: "arbitrum",
      avgBlockTime: 0.25,
      estimatedConfirmationTime: 2,
      congestionLevel: "low",
    },
    base: {
      chainId: "base",
      avgBlockTime: 2,
      estimatedConfirmationTime: 10,
      congestionLevel: "low",
    },
    optimism: {
      chainId: "optimism",
      avgBlockTime: 2,
      estimatedConfirmationTime: 10,
      congestionLevel: "low",
    },
    polygon: {
      chainId: "polygon",
      avgBlockTime: 2,
      estimatedConfirmationTime: 30,
      congestionLevel: "medium",
    },
    avalanche: {
      chainId: "avalanche",
      avgBlockTime: 2,
      estimatedConfirmationTime: 5,
      congestionLevel: "low",
    },
    bsc: {
      chainId: "bsc",
      avgBlockTime: 3,
      estimatedConfirmationTime: 45,
      congestionLevel: "medium",
    },
    zksync: {
      chainId: "zksync",
      avgBlockTime: 1,
      estimatedConfirmationTime: 60,
      congestionLevel: "low",
    },
  };
}

function generateMockBalances(): Record<ChainId, ChainBalance> {
  return {
    ethereum: {
      chainId: "ethereum",
      nativeBalance: "1.5432",
      nativeBalanceUsd: 4856.78,
      tokens: [
        { symbol: "USDC", balance: "10000.00", balanceUsd: 10000 },
        { symbol: "USDT", balance: "5000.00", balanceUsd: 5000 },
      ],
    },
    arbitrum: {
      chainId: "arbitrum",
      nativeBalance: "0.8765",
      nativeBalanceUsd: 2761.23,
      tokens: [
        { symbol: "USDC", balance: "25000.00", balanceUsd: 25000 },
      ],
    },
    base: {
      chainId: "base",
      nativeBalance: "0.2345",
      nativeBalanceUsd: 738.67,
      tokens: [
        { symbol: "USDC", balance: "8000.00", balanceUsd: 8000 },
      ],
    },
    optimism: {
      chainId: "optimism",
      nativeBalance: "0.5678",
      nativeBalanceUsd: 1788.45,
      tokens: [],
    },
    polygon: {
      chainId: "polygon",
      nativeBalance: "150.00",
      nativeBalanceUsd: 120.0,
      tokens: [
        { symbol: "USDC", balance: "3000.00", balanceUsd: 3000 },
      ],
    },
    avalanche: {
      chainId: "avalanche",
      nativeBalance: "10.00",
      nativeBalanceUsd: 350.0,
      tokens: [],
    },
    bsc: {
      chainId: "bsc",
      nativeBalance: "5.00",
      nativeBalanceUsd: 1500.0,
      tokens: [],
    },
    zksync: {
      chainId: "zksync",
      nativeBalance: "0.1234",
      nativeBalanceUsd: 388.67,
      tokens: [],
    },
  };
}

function generateMockNetworkStatus(): Record<ChainId, ChainStatus> {
  return {
    ethereum: {
      chainId: "ethereum",
      health: "healthy",
      latency: 120,
      lastBlock: 19234567,
      lastBlockTime: new Date().toISOString(),
    },
    arbitrum: {
      chainId: "arbitrum",
      health: "healthy",
      latency: 45,
      lastBlock: 182345678,
      lastBlockTime: new Date().toISOString(),
    },
    base: {
      chainId: "base",
      health: "healthy",
      latency: 60,
      lastBlock: 10234567,
      lastBlockTime: new Date().toISOString(),
    },
    optimism: {
      chainId: "optimism",
      health: "healthy",
      latency: 55,
      lastBlock: 115234567,
      lastBlockTime: new Date().toISOString(),
    },
    polygon: {
      chainId: "polygon",
      health: "degraded",
      latency: 250,
      lastBlock: 54234567,
      lastBlockTime: new Date().toISOString(),
    },
    avalanche: {
      chainId: "avalanche",
      health: "healthy",
      latency: 80,
      lastBlock: 42234567,
      lastBlockTime: new Date().toISOString(),
    },
    bsc: {
      chainId: "bsc",
      health: "healthy",
      latency: 100,
      lastBlock: 36234567,
      lastBlockTime: new Date().toISOString(),
    },
    zksync: {
      chainId: "zksync",
      health: "healthy",
      latency: 70,
      lastBlock: 28234567,
      lastBlockTime: new Date().toISOString(),
    },
  };
}

// Utility functions
function formatCurrency(value: number): string {
  if (value < 0.01) return "<$0.01";
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

function formatTime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
  return `${Math.round(seconds / 3600)}h`;
}

function getHealthIcon(health: NetworkHealth) {
  switch (health) {
    case "healthy":
      return <CheckCircle className="h-3 w-3 text-green-500" />;
    case "degraded":
      return <AlertCircle className="h-3 w-3 text-yellow-500" />;
    case "down":
      return <XCircle className="h-3 w-3 text-red-500" />;
  }
}

function getCongestionColor(level: "low" | "medium" | "high"): string {
  switch (level) {
    case "low":
      return "text-green-600 dark:text-green-400";
    case "medium":
      return "text-yellow-600 dark:text-yellow-400";
    case "high":
      return "text-red-600 dark:text-red-400";
  }
}

// Chain Logo Component
function ChainLogo({
  chainId,
  size = "md",
  className,
}: {
  chainId: ChainId;
  size?: "sm" | "md" | "lg";
  className?: string;
}) {
  const chain = CHAIN_CONFIG[chainId];
  const sizeClasses = {
    sm: "w-4 h-4 text-[8px]",
    md: "w-6 h-6 text-[10px]",
    lg: "w-8 h-8 text-xs",
  };

  return (
    <div
      className={cn(
        "rounded-full flex items-center justify-center font-bold text-white",
        sizeClasses[size],
        className
      )}
      style={{ backgroundColor: chain.color }}
    >
      {chain.shortName.slice(0, 2)}
    </div>
  );
}

// Chain Option Component
function ChainOption({
  chainId,
  isSelected,
  gasEstimate,
  transactionTime,
  balance,
  networkStatus,
  showGasEstimates,
  showTransactionTime,
  showBalances,
  showNetworkHealth,
  onClick,
}: {
  chainId: ChainId;
  isSelected: boolean;
  gasEstimate?: ChainGasEstimate;
  transactionTime?: ChainTransactionTime;
  balance?: ChainBalance;
  networkStatus?: ChainStatus;
  showGasEstimates?: boolean;
  showTransactionTime?: boolean;
  showBalances?: boolean;
  showNetworkHealth?: boolean;
  onClick: () => void;
}) {
  const chain = CHAIN_CONFIG[chainId];

  return (
    <DropdownMenuItem
      className={cn(
        "flex items-start gap-3 p-3 cursor-pointer",
        isSelected && "bg-accent"
      )}
      onClick={onClick}
    >
      <ChainLogo chainId={chainId} size="md" />

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-medium">{chain.name}</span>
          {chain.isL2 && (
            <Badge variant="outline" className="text-[10px] px-1 py-0">
              L2
            </Badge>
          )}
          {showNetworkHealth && networkStatus && (
            <span className="ml-auto">{getHealthIcon(networkStatus.health)}</span>
          )}
          {isSelected && <Check className="h-4 w-4 ml-auto text-primary" />}
        </div>

        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 mt-1 text-xs text-muted-foreground">
          {showGasEstimates && gasEstimate && (
            <span className="flex items-center gap-1">
              <Fuel className="h-3 w-3" />
              {formatCurrency(gasEstimate.estimatedCostUsd)}
            </span>
          )}

          {showTransactionTime && transactionTime && (
            <span
              className={cn(
                "flex items-center gap-1",
                getCongestionColor(transactionTime.congestionLevel)
              )}
            >
              <Clock className="h-3 w-3" />
              ~{formatTime(transactionTime.estimatedConfirmationTime)}
            </span>
          )}

          {showBalances && balance && (
            <span className="flex items-center gap-1">
              <Wallet className="h-3 w-3" />
              {formatCurrency(balance.nativeBalanceUsd)}
            </span>
          )}
        </div>
      </div>
    </DropdownMenuItem>
  );
}

// Main Chain Selector Component
export function ChainSelector({
  selectedChain,
  onChainChange,
  chains = DEFAULT_CHAINS,
  showGasEstimates = true,
  showTransactionTime = true,
  showBalances = true,
  showNetworkHealth = true,
  disabled = false,
  className,
  placeholder = "Select chain",
  size = "md",
  variant = "outline",
}: ChainSelectorProps) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [recentChains, setRecentChains] = useState<ChainId[]>([]);

  // Mock data state (replace with actual API hooks)
  const [gasEstimates, setGasEstimates] = useState<Record<ChainId, ChainGasEstimate>>(
    {} as Record<ChainId, ChainGasEstimate>
  );
  const [transactionTimes, setTransactionTimes] = useState<Record<ChainId, ChainTransactionTime>>(
    {} as Record<ChainId, ChainTransactionTime>
  );
  const [balances, setBalances] = useState<Record<ChainId, ChainBalance>>(
    {} as Record<ChainId, ChainBalance>
  );
  const [networkStatus, setNetworkStatus] = useState<Record<ChainId, ChainStatus>>(
    {} as Record<ChainId, ChainStatus>
  );

  // Load recent chains from localStorage
  useEffect(() => {
    try {
      const stored = localStorage.getItem(RECENT_CHAINS_KEY);
      if (stored) {
        const parsed = JSON.parse(stored) as ChainId[];
        setRecentChains(parsed.filter((c) => chains.includes(c)));
      }
    } catch {
      // Ignore localStorage errors
    }
  }, [chains]);

  // Load mock data
  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      // Simulate API call
      await new Promise((resolve) => setTimeout(resolve, 300));

      setGasEstimates(generateMockGasEstimates());
      setTransactionTimes(generateMockTransactionTimes());
      setBalances(generateMockBalances());
      setNetworkStatus(generateMockNetworkStatus());
      setLoading(false);
    };

    loadData();
  }, []);

  // Save to recent chains
  const saveToRecent = useCallback((chainId: ChainId) => {
    setRecentChains((prev) => {
      const filtered = prev.filter((c) => c !== chainId);
      const updated = [chainId, ...filtered].slice(0, MAX_RECENT_CHAINS);
      try {
        localStorage.setItem(RECENT_CHAINS_KEY, JSON.stringify(updated));
      } catch {
        // Ignore localStorage errors
      }
      return updated;
    });
  }, []);

  // Handle chain selection
  const handleSelectChain = useCallback(
    (chainId: ChainId) => {
      onChainChange(chainId);
      saveToRecent(chainId);
      setOpen(false);
      setSearch("");
    },
    [onChainChange, saveToRecent]
  );

  // Filter chains based on search
  const filteredChains = useMemo(() => {
    if (!search) return chains;
    const searchLower = search.toLowerCase();
    return chains.filter((chainId) => {
      const chain = CHAIN_CONFIG[chainId];
      return (
        chain.name.toLowerCase().includes(searchLower) ||
        chain.shortName.toLowerCase().includes(searchLower)
      );
    });
  }, [chains, search]);

  // Get selected chain info
  const selectedChainInfo = CHAIN_CONFIG[selectedChain];

  // Size classes
  const sizeClasses = {
    sm: "h-8 text-sm",
    md: "h-10",
    lg: "h-12 text-lg",
  };

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild disabled={disabled}>
        <Button
          variant={variant}
          className={cn(
            "w-full justify-between",
            sizeClasses[size],
            className
          )}
        >
          {selectedChainInfo ? (
            <div className="flex items-center gap-2">
              <ChainLogo chainId={selectedChain} size={size === "lg" ? "md" : "sm"} />
              <span>{selectedChainInfo.name}</span>
              {showNetworkHealth && networkStatus[selectedChain] && (
                <span className="ml-1">
                  {getHealthIcon(networkStatus[selectedChain].health)}
                </span>
              )}
            </div>
          ) : (
            <span className="text-muted-foreground">{placeholder}</span>
          )}
          <ChevronDown className="h-4 w-4 opacity-50" />
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent className="w-80" align="start">
        {/* Search */}
        <div className="p-2">
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search chains..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-8 h-9"
            />
          </div>
        </div>

        <DropdownMenuSeparator />

        {loading ? (
          <div className="p-4 space-y-3">
            {[1, 2, 3].map((i) => (
              <div key={i} className="flex items-center gap-3">
                <Skeleton className="h-6 w-6 rounded-full" />
                <div className="space-y-1.5 flex-1">
                  <Skeleton className="h-4 w-24" />
                  <Skeleton className="h-3 w-32" />
                </div>
              </div>
            ))}
          </div>
        ) : (
          <>
            {/* Recently Used */}
            {recentChains.length > 0 && !search && (
              <>
                <DropdownMenuLabel className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Clock className="h-3 w-3" />
                  Recently Used
                </DropdownMenuLabel>
                {recentChains
                  .filter((c) => chains.includes(c))
                  .map((chainId) => (
                    <ChainOption
                      key={`recent-${chainId}`}
                      chainId={chainId}
                      isSelected={chainId === selectedChain}
                      gasEstimate={gasEstimates[chainId]}
                      transactionTime={transactionTimes[chainId]}
                      balance={balances[chainId]}
                      networkStatus={networkStatus[chainId]}
                      showGasEstimates={showGasEstimates}
                      showTransactionTime={showTransactionTime}
                      showBalances={showBalances}
                      showNetworkHealth={showNetworkHealth}
                      onClick={() => handleSelectChain(chainId)}
                    />
                  ))}
                <DropdownMenuSeparator />
              </>
            )}

            {/* All Chains */}
            <DropdownMenuLabel className="text-xs text-muted-foreground">
              {search ? `Results (${filteredChains.length})` : "All Chains"}
            </DropdownMenuLabel>

            {filteredChains.length === 0 ? (
              <div className="p-4 text-center text-sm text-muted-foreground">
                No chains found matching "{search}"
              </div>
            ) : (
              <div className="max-h-[300px] overflow-y-auto">
                {filteredChains.map((chainId) => (
                  <ChainOption
                    key={chainId}
                    chainId={chainId}
                    isSelected={chainId === selectedChain}
                    gasEstimate={gasEstimates[chainId]}
                    transactionTime={transactionTimes[chainId]}
                    balance={balances[chainId]}
                    networkStatus={networkStatus[chainId]}
                    showGasEstimates={showGasEstimates}
                    showTransactionTime={showTransactionTime}
                    showBalances={showBalances}
                    showNetworkHealth={showNetworkHealth}
                    onClick={() => handleSelectChain(chainId)}
                  />
                ))}
              </div>
            )}
          </>
        )}

        <DropdownMenuSeparator />

        {/* Footer with refresh */}
        <div className="p-2 flex items-center justify-between text-xs text-muted-foreground">
          <span>Gas prices update every 15s</span>
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-2"
            onClick={() => {
              setGasEstimates(generateMockGasEstimates());
              setTransactionTimes(generateMockTransactionTimes());
            }}
          >
            <RefreshCw className="h-3 w-3 mr-1" />
            Refresh
          </Button>
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

// Compact Chain Selector (for toolbars/headers)
export function CompactChainSelector({
  selectedChain,
  onChainChange,
  chains = DEFAULT_CHAINS,
  disabled = false,
  className,
}: Omit<ChainSelectorProps, "showGasEstimates" | "showTransactionTime" | "showBalances" | "showNetworkHealth" | "size" | "variant" | "placeholder">) {
  const [open, setOpen] = useState(false);
  const selectedChainInfo = CHAIN_CONFIG[selectedChain];

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild disabled={disabled}>
        <Button
          variant="ghost"
          size="sm"
          className={cn("h-8 gap-1.5 px-2", className)}
        >
          <ChainLogo chainId={selectedChain} size="sm" />
          <span className="hidden sm:inline">{selectedChainInfo?.shortName}</span>
          <ChevronDown className="h-3 w-3 opacity-50" />
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent align="end" className="w-48">
        {chains.map((chainId) => {
          const chain = CHAIN_CONFIG[chainId];
          return (
            <DropdownMenuItem
              key={chainId}
              className={cn(
                "flex items-center gap-2",
                chainId === selectedChain && "bg-accent"
              )}
              onClick={() => {
                onChainChange(chainId);
                setOpen(false);
              }}
            >
              <ChainLogo chainId={chainId} size="sm" />
              <span>{chain.name}</span>
              {chainId === selectedChain && (
                <Check className="h-4 w-4 ml-auto" />
              )}
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

// Chain Badge Component (for display only)
export function ChainBadge({
  chainId,
  showName = true,
  size = "md",
  className,
}: {
  chainId: ChainId;
  showName?: boolean;
  size?: "sm" | "md" | "lg";
  className?: string;
}) {
  const chain = CHAIN_CONFIG[chainId];
  const sizeClasses = {
    sm: "text-xs gap-1 px-1.5 py-0.5",
    md: "text-sm gap-1.5 px-2 py-1",
    lg: "text-base gap-2 px-3 py-1.5",
  };

  return (
    <Badge
      variant="outline"
      className={cn("flex items-center", sizeClasses[size], className)}
    >
      <ChainLogo chainId={chainId} size={size === "lg" ? "md" : "sm"} />
      {showName && <span>{chain.name}</span>}
    </Badge>
  );
}

// Export chain config for external use
export { CHAIN_CONFIG, DEFAULT_CHAINS };
