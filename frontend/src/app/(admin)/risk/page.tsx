"use client";

import { useState, useEffect, useCallback } from "react";
import {
  Loader2,
  RefreshCw,
  Shield,
  AlertTriangle,
  TrendingDown,
  Activity,
  Bell,
  CheckCircle,
  XCircle,
  AlertCircle,
  Zap,
  PieChart,
  BarChart3,
  Clock,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import { StatCard } from "@/components/dashboard/stat-card";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

// Types
type RiskLevel = "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
type AlertSeverity = "INFO" | "WARNING" | "CRITICAL";
type StablecoinSymbol = "USDT" | "USDC" | "DAI" | "VNST";
type ChainId = "ethereum" | "arbitrum" | "base" | "optimism";
type YieldProtocol = "aave" | "compound" | "morpho" | "yearn";

interface DepegRiskIndicator {
  token: StablecoinSymbol;
  current_price: number;
  peg_price: number;
  deviation_percent: number;
  deviation_24h: number;
  risk_level: RiskLevel;
  last_updated: string;
  price_history: { timestamp: string; price: number }[];
}

interface ProtocolExposure {
  protocol: YieldProtocol;
  total_deposited_usd: number;
  percentage: number;
  health_factor: number;
  risk_level: RiskLevel;
  positions_count: number;
}

interface ConcentrationRisk {
  type: "token" | "chain";
  identifier: string;
  value_usd: number;
  percentage: number;
  limit: number;
  status: "OK" | "WARNING" | "EXCEEDED";
}

interface RiskAlert {
  id: string;
  severity: AlertSeverity;
  title: string;
  description: string;
  source: string;
  created_at: string;
  acknowledged: boolean;
  resolved: boolean;
  resolved_at?: string;
}

interface HealthFactorMonitor {
  protocol: YieldProtocol;
  chain: ChainId;
  health_factor: number;
  liquidation_threshold: number;
  warning_threshold: number;
  status: RiskLevel;
  position_value_usd: number;
}

interface RiskDashboardData {
  overall_risk_score: number;
  overall_risk_level: RiskLevel;
  depeg_indicators: DepegRiskIndicator[];
  protocol_exposure: ProtocolExposure[];
  concentration_by_token: ConcentrationRisk[];
  concentration_by_chain: ConcentrationRisk[];
  health_factors: HealthFactorMonitor[];
  recent_alerts: RiskAlert[];
  stats: {
    total_exposure_usd: number;
    avg_health_factor: number;
    alerts_24h: number;
    critical_alerts: number;
  };
}

// Mock data for development
function generateMockData(): RiskDashboardData {
  return {
    overall_risk_score: 32,
    overall_risk_level: "LOW",
    depeg_indicators: [
      {
        token: "USDT",
        current_price: 0.9998,
        peg_price: 1.0,
        deviation_percent: -0.02,
        deviation_24h: 0.01,
        risk_level: "LOW",
        last_updated: new Date().toISOString(),
        price_history: Array.from({ length: 24 }, (_, i) => ({
          timestamp: new Date(Date.now() - i * 3600000).toISOString(),
          price: 0.999 + Math.random() * 0.002,
        })),
      },
      {
        token: "USDC",
        current_price: 1.0001,
        peg_price: 1.0,
        deviation_percent: 0.01,
        deviation_24h: -0.005,
        risk_level: "LOW",
        last_updated: new Date().toISOString(),
        price_history: Array.from({ length: 24 }, (_, i) => ({
          timestamp: new Date(Date.now() - i * 3600000).toISOString(),
          price: 0.9995 + Math.random() * 0.001,
        })),
      },
      {
        token: "DAI",
        current_price: 0.9995,
        peg_price: 1.0,
        deviation_percent: -0.05,
        deviation_24h: 0.02,
        risk_level: "LOW",
        last_updated: new Date().toISOString(),
        price_history: Array.from({ length: 24 }, (_, i) => ({
          timestamp: new Date(Date.now() - i * 3600000).toISOString(),
          price: 0.998 + Math.random() * 0.003,
        })),
      },
      {
        token: "VNST",
        current_price: 0.9980,
        peg_price: 1.0,
        deviation_percent: -0.20,
        deviation_24h: -0.15,
        risk_level: "MEDIUM",
        last_updated: new Date().toISOString(),
        price_history: Array.from({ length: 24 }, (_, i) => ({
          timestamp: new Date(Date.now() - i * 3600000).toISOString(),
          price: 0.996 + Math.random() * 0.004,
        })),
      },
    ],
    protocol_exposure: [
      {
        protocol: "aave",
        total_deposited_usd: 2500000,
        percentage: 45,
        health_factor: 2.8,
        risk_level: "LOW",
        positions_count: 3,
      },
      {
        protocol: "compound",
        total_deposited_usd: 1800000,
        percentage: 32,
        health_factor: 2.1,
        risk_level: "LOW",
        positions_count: 2,
      },
      {
        protocol: "morpho",
        total_deposited_usd: 800000,
        percentage: 14,
        health_factor: 1.8,
        risk_level: "MEDIUM",
        positions_count: 1,
      },
      {
        protocol: "yearn",
        total_deposited_usd: 500000,
        percentage: 9,
        health_factor: 0,
        risk_level: "LOW",
        positions_count: 1,
      },
    ],
    concentration_by_token: [
      { type: "token", identifier: "USDC", value_usd: 3000000, percentage: 53, limit: 60, status: "OK" },
      { type: "token", identifier: "USDT", value_usd: 1500000, percentage: 27, limit: 60, status: "OK" },
      { type: "token", identifier: "DAI", value_usd: 800000, percentage: 14, limit: 40, status: "OK" },
      { type: "token", identifier: "VNST", value_usd: 300000, percentage: 6, limit: 20, status: "OK" },
    ],
    concentration_by_chain: [
      { type: "chain", identifier: "ethereum", value_usd: 3500000, percentage: 62, limit: 70, status: "OK" },
      { type: "chain", identifier: "arbitrum", value_usd: 1200000, percentage: 22, limit: 40, status: "OK" },
      { type: "chain", identifier: "base", value_usd: 600000, percentage: 11, limit: 30, status: "OK" },
      { type: "chain", identifier: "optimism", value_usd: 300000, percentage: 5, limit: 30, status: "OK" },
    ],
    health_factors: [
      {
        protocol: "aave",
        chain: "ethereum",
        health_factor: 2.8,
        liquidation_threshold: 1.0,
        warning_threshold: 1.5,
        status: "LOW",
        position_value_usd: 1500000,
      },
      {
        protocol: "aave",
        chain: "arbitrum",
        health_factor: 2.5,
        liquidation_threshold: 1.0,
        warning_threshold: 1.5,
        status: "LOW",
        position_value_usd: 1000000,
      },
      {
        protocol: "compound",
        chain: "ethereum",
        health_factor: 2.1,
        liquidation_threshold: 1.0,
        warning_threshold: 1.5,
        status: "LOW",
        position_value_usd: 1800000,
      },
      {
        protocol: "morpho",
        chain: "ethereum",
        health_factor: 1.8,
        liquidation_threshold: 1.0,
        warning_threshold: 1.5,
        status: "MEDIUM",
        position_value_usd: 800000,
      },
    ],
    recent_alerts: [
      {
        id: "1",
        severity: "INFO",
        title: "Yield rate updated",
        description: "Aave V3 USDC yield rate changed from 4.2% to 4.5%",
        source: "yield-monitor",
        created_at: new Date(Date.now() - 3600000).toISOString(),
        acknowledged: true,
        resolved: true,
        resolved_at: new Date(Date.now() - 1800000).toISOString(),
      },
      {
        id: "2",
        severity: "WARNING",
        title: "Health factor approaching threshold",
        description: "Morpho position health factor dropped to 1.8",
        source: "health-monitor",
        created_at: new Date(Date.now() - 7200000).toISOString(),
        acknowledged: true,
        resolved: false,
      },
      {
        id: "3",
        severity: "INFO",
        title: "Rebalance completed",
        description: "Successfully bridged 100,000 USDC from Ethereum to Arbitrum",
        source: "treasury",
        created_at: new Date(Date.now() - 14400000).toISOString(),
        acknowledged: true,
        resolved: true,
        resolved_at: new Date(Date.now() - 14000000).toISOString(),
      },
      {
        id: "4",
        severity: "WARNING",
        title: "VNST slight depeg detected",
        description: "VNST trading at $0.998, 0.2% below peg",
        source: "depeg-monitor",
        created_at: new Date(Date.now() - 28800000).toISOString(),
        acknowledged: false,
        resolved: false,
      },
    ],
    stats: {
      total_exposure_usd: 5600000,
      avg_health_factor: 2.3,
      alerts_24h: 4,
      critical_alerts: 0,
    },
  };
}

// Utility functions
function formatCurrency(value: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(value);
}

function formatPercent(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

function formatDateTime(dateStr: string): string {
  return new Date(dateStr).toLocaleString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function getTimeAgo(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const hours = Math.floor(diff / 3600000);
  if (hours < 1) return `${Math.floor(diff / 60000)}m ago`;
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

const PROTOCOL_NAMES: Record<YieldProtocol, string> = {
  aave: "Aave V3",
  compound: "Compound V3",
  morpho: "Morpho",
  yearn: "Yearn",
};

const CHAIN_NAMES: Record<ChainId, string> = {
  ethereum: "Ethereum",
  arbitrum: "Arbitrum",
  base: "Base",
  optimism: "Optimism",
};

const TOKEN_COLORS: Record<StablecoinSymbol, string> = {
  USDT: "bg-green-500",
  USDC: "bg-blue-500",
  DAI: "bg-yellow-500",
  VNST: "bg-purple-500",
};

const CHAIN_COLORS: Record<ChainId, string> = {
  ethereum: "bg-blue-500",
  arbitrum: "bg-orange-500",
  base: "bg-blue-600",
  optimism: "bg-red-500",
};

function getRiskLevelColor(level: RiskLevel): string {
  switch (level) {
    case "LOW":
      return "bg-green-100 text-green-800 dark:bg-green-500/15 dark:text-green-400";
    case "MEDIUM":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-500/15 dark:text-yellow-400";
    case "HIGH":
      return "bg-orange-100 text-orange-800 dark:bg-orange-500/15 dark:text-orange-400";
    case "CRITICAL":
      return "bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-400";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
  }
}

function getAlertSeverityIcon(severity: AlertSeverity) {
  switch (severity) {
    case "INFO":
      return <AlertCircle className="h-4 w-4 text-blue-500" />;
    case "WARNING":
      return <AlertTriangle className="h-4 w-4 text-yellow-500" />;
    case "CRITICAL":
      return <XCircle className="h-4 w-4 text-red-500" />;
  }
}

function getConcentrationStatusColor(status: string): string {
  switch (status) {
    case "OK":
      return "text-green-600 dark:text-green-400";
    case "WARNING":
      return "text-yellow-600 dark:text-yellow-400";
    case "EXCEEDED":
      return "text-red-600 dark:text-red-400";
    default:
      return "text-gray-600 dark:text-gray-400";
  }
}

// Risk Overview Component
function RiskOverview({
  data,
  loading,
}: {
  data: RiskDashboardData | null;
  loading: boolean;
}) {
  if (loading || !data) {
    return (
      <div className="grid gap-4 md:grid-cols-4">
        {[1, 2, 3, 4].map((i) => (
          <Card key={i} className="animate-pulse">
            <CardContent className="pt-6">
              <div className="h-4 bg-muted rounded w-1/3 mb-2" />
              <div className="h-8 bg-muted rounded w-2/3" />
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  return (
    <div className="grid gap-4 md:grid-cols-4">
      <StatCard
        title="Risk Score"
        value={`${data.overall_risk_score}/100`}
        icon={<Shield className="h-4 w-4" />}
        subtitle={data.overall_risk_level}
        className={cn(
          data.overall_risk_level === "LOW" && "border-green-200 dark:border-green-800",
          data.overall_risk_level === "MEDIUM" && "border-yellow-200 dark:border-yellow-800",
          data.overall_risk_level === "HIGH" && "border-orange-200 dark:border-orange-800",
          data.overall_risk_level === "CRITICAL" && "border-red-200 dark:border-red-800"
        )}
      />
      <StatCard
        title="Total Exposure"
        value={formatCurrency(data.stats.total_exposure_usd)}
        icon={<Activity className="h-4 w-4" />}
      />
      <StatCard
        title="Avg Health Factor"
        value={data.stats.avg_health_factor.toFixed(2)}
        icon={<Zap className="h-4 w-4" />}
        className={
          data.stats.avg_health_factor < 1.5
            ? "border-yellow-200 dark:border-yellow-800"
            : ""
        }
      />
      <StatCard
        title="Alerts (24h)"
        value={data.stats.alerts_24h.toString()}
        icon={<Bell className="h-4 w-4" />}
        subtitle={
          data.stats.critical_alerts > 0
            ? `${data.stats.critical_alerts} critical`
            : "No critical alerts"
        }
        className={
          data.stats.critical_alerts > 0
            ? "border-red-200 dark:border-red-800"
            : ""
        }
      />
    </div>
  );
}

// Depeg Monitor Component
function DepegMonitor({
  indicators,
  loading,
}: {
  indicators: DepegRiskIndicator[];
  loading: boolean;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="grid gap-4 md:grid-cols-2">
      {indicators.map((indicator) => (
        <Card key={indicator.token} className="overflow-hidden">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className={cn("w-3 h-3 rounded-full", TOKEN_COLORS[indicator.token])} />
                <CardTitle className="text-lg">{indicator.token}</CardTitle>
              </div>
              <Badge className={getRiskLevelColor(indicator.risk_level)}>
                {indicator.risk_level}
              </Badge>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Current Price</span>
                <span className="text-xl font-bold">${indicator.current_price.toFixed(4)}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Deviation from Peg</span>
                <span
                  className={cn(
                    "font-medium",
                    Math.abs(indicator.deviation_percent) < 0.1
                      ? "text-green-600 dark:text-green-400"
                      : Math.abs(indicator.deviation_percent) < 0.5
                      ? "text-yellow-600 dark:text-yellow-400"
                      : "text-red-600 dark:text-red-400"
                  )}
                >
                  {formatPercent(indicator.deviation_percent)}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">24h Change</span>
                <span
                  className={cn(
                    "flex items-center gap-1",
                    indicator.deviation_24h >= 0
                      ? "text-green-600 dark:text-green-400"
                      : "text-red-600 dark:text-red-400"
                  )}
                >
                  {indicator.deviation_24h >= 0 ? (
                    <TrendingDown className="h-3 w-3 rotate-180" />
                  ) : (
                    <TrendingDown className="h-3 w-3" />
                  )}
                  {formatPercent(indicator.deviation_24h)}
                </span>
              </div>
              {/* Mini price chart visualization */}
              <div className="h-12 flex items-end gap-0.5">
                {indicator.price_history.slice(0, 24).reverse().map((point, idx) => {
                  const min = Math.min(...indicator.price_history.map((p) => p.price));
                  const max = Math.max(...indicator.price_history.map((p) => p.price));
                  const height = max === min ? 50 : ((point.price - min) / (max - min)) * 100;
                  return (
                    <div
                      key={idx}
                      className={cn(
                        "flex-1 rounded-t",
                        point.price >= 0.999
                          ? "bg-green-500/50"
                          : point.price >= 0.995
                          ? "bg-yellow-500/50"
                          : "bg-red-500/50"
                      )}
                      style={{ height: `${Math.max(height, 10)}%` }}
                    />
                  );
                })}
              </div>
              <div className="text-xs text-muted-foreground text-center">
                Last 24 hours
              </div>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

// Exposure Chart Component (Protocol Allocation)
function ExposureChart({
  exposure,
  loading,
}: {
  exposure: ProtocolExposure[];
  loading: boolean;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const total = exposure.reduce((acc, e) => acc + e.total_deposited_usd, 0);

  const PROTOCOL_COLORS: Record<YieldProtocol, string> = {
    aave: "bg-purple-500",
    compound: "bg-green-500",
    morpho: "bg-blue-500",
    yearn: "bg-yellow-500",
  };

  return (
    <div className="space-y-4">
      {/* Pie chart visualization */}
      <div className="flex items-center justify-center py-4">
        <div className="relative w-40 h-40">
          <svg viewBox="0 0 100 100" className="transform -rotate-90">
            {exposure.reduce(
              (acc, item, idx) => {
                const startAngle = acc.offset;
                const angle = (item.percentage / 100) * 360;
                const endAngle = startAngle + angle;

                const startRad = (startAngle * Math.PI) / 180;
                const endRad = (endAngle * Math.PI) / 180;

                const x1 = 50 + 40 * Math.cos(startRad);
                const y1 = 50 + 40 * Math.sin(startRad);
                const x2 = 50 + 40 * Math.cos(endRad);
                const y2 = 50 + 40 * Math.sin(endRad);

                const largeArc = angle > 180 ? 1 : 0;

                const colors: Record<YieldProtocol, string> = {
                  aave: "#8b5cf6",
                  compound: "#22c55e",
                  morpho: "#3b82f6",
                  yearn: "#eab308",
                };

                acc.paths.push(
                  <path
                    key={item.protocol}
                    d={`M 50 50 L ${x1} ${y1} A 40 40 0 ${largeArc} 1 ${x2} ${y2} Z`}
                    fill={colors[item.protocol]}
                    className="hover:opacity-80 transition-opacity cursor-pointer"
                  />
                );

                acc.offset = endAngle;
                return acc;
              },
              { paths: [] as JSX.Element[], offset: 0 }
            ).paths}
          </svg>
          <div className="absolute inset-0 flex items-center justify-center flex-col">
            <span className="text-lg font-bold">{formatCurrency(total)}</span>
            <span className="text-xs text-muted-foreground">Total</span>
          </div>
        </div>
      </div>

      {/* Legend */}
      <div className="space-y-3">
        {exposure.map((item) => (
          <div key={item.protocol} className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <div className={cn("w-3 h-3 rounded-full", PROTOCOL_COLORS[item.protocol])} />
              <span className="font-medium">{PROTOCOL_NAMES[item.protocol]}</span>
              <Badge variant="outline" className="text-xs">
                {item.positions_count} positions
              </Badge>
            </div>
            <div className="flex items-center gap-4">
              <span className="text-sm text-muted-foreground">
                HF: {item.health_factor > 0 ? item.health_factor.toFixed(2) : "N/A"}
              </span>
              <span className="font-medium w-24 text-right">
                {formatCurrency(item.total_deposited_usd)}
              </span>
              <span className="text-sm text-muted-foreground w-12 text-right">
                {item.percentage}%
              </span>
              <Badge className={getRiskLevelColor(item.risk_level)}>{item.risk_level}</Badge>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// Concentration Risk Component
function ConcentrationRiskPanel({
  byToken,
  byChain,
  loading,
}: {
  byToken: ConcentrationRisk[];
  byChain: ConcentrationRisk[];
  loading: boolean;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* By Token */}
      <div>
        <h4 className="text-sm font-medium mb-3 flex items-center gap-2">
          <PieChart className="h-4 w-4" />
          By Token
        </h4>
        <div className="space-y-2">
          {byToken.map((item) => (
            <div key={item.identifier} className="flex items-center gap-2">
              <div
                className={cn(
                  "w-2 h-2 rounded-full",
                  TOKEN_COLORS[item.identifier as StablecoinSymbol] || "bg-gray-500"
                )}
              />
              <span className="text-sm w-12">{item.identifier}</span>
              <div className="flex-1">
                <Progress value={item.percentage} className="h-2" />
              </div>
              <span className={cn("text-sm w-12 text-right", getConcentrationStatusColor(item.status))}>
                {item.percentage}%
              </span>
              <span className="text-xs text-muted-foreground w-12">/ {item.limit}%</span>
              {item.status === "OK" ? (
                <CheckCircle className="h-4 w-4 text-green-500" />
              ) : item.status === "WARNING" ? (
                <AlertTriangle className="h-4 w-4 text-yellow-500" />
              ) : (
                <XCircle className="h-4 w-4 text-red-500" />
              )}
            </div>
          ))}
        </div>
      </div>

      {/* By Chain */}
      <div>
        <h4 className="text-sm font-medium mb-3 flex items-center gap-2">
          <BarChart3 className="h-4 w-4" />
          By Chain
        </h4>
        <div className="space-y-2">
          {byChain.map((item) => (
            <div key={item.identifier} className="flex items-center gap-2">
              <div
                className={cn(
                  "w-2 h-2 rounded-full",
                  CHAIN_COLORS[item.identifier as ChainId] || "bg-gray-500"
                )}
              />
              <span className="text-sm w-20">{CHAIN_NAMES[item.identifier as ChainId] || item.identifier}</span>
              <div className="flex-1">
                <Progress value={item.percentage} className="h-2" />
              </div>
              <span className={cn("text-sm w-12 text-right", getConcentrationStatusColor(item.status))}>
                {item.percentage}%
              </span>
              <span className="text-xs text-muted-foreground w-12">/ {item.limit}%</span>
              {item.status === "OK" ? (
                <CheckCircle className="h-4 w-4 text-green-500" />
              ) : item.status === "WARNING" ? (
                <AlertTriangle className="h-4 w-4 text-yellow-500" />
              ) : (
                <XCircle className="h-4 w-4 text-red-500" />
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// Health Factor Monitor Component
function HealthFactorMonitorPanel({
  healthFactors,
  loading,
}: {
  healthFactors: HealthFactorMonitor[];
  loading: boolean;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {healthFactors.map((hf, idx) => (
        <div key={idx} className="p-3 rounded-lg border bg-card">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <span className="font-medium">{PROTOCOL_NAMES[hf.protocol]}</span>
              <Badge variant="outline">{CHAIN_NAMES[hf.chain]}</Badge>
            </div>
            <Badge className={getRiskLevelColor(hf.status)}>{hf.status}</Badge>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">Health Factor:</span>
            <span
              className={cn(
                "text-lg font-bold",
                hf.health_factor >= 2
                  ? "text-green-600 dark:text-green-400"
                  : hf.health_factor >= 1.5
                  ? "text-yellow-600 dark:text-yellow-400"
                  : "text-red-600 dark:text-red-400"
              )}
            >
              {hf.health_factor.toFixed(2)}
            </span>
            <div className="flex-1">
              <Progress
                value={Math.min((hf.health_factor / 3) * 100, 100)}
                className="h-2"
              />
            </div>
          </div>
          <div className="flex items-center justify-between mt-2 text-xs text-muted-foreground">
            <span>Liquidation: {hf.liquidation_threshold}</span>
            <span>Warning: {hf.warning_threshold}</span>
            <span>Position: {formatCurrency(hf.position_value_usd)}</span>
          </div>
        </div>
      ))}
    </div>
  );
}

// Alert History Component
function AlertHistory({
  alerts,
  loading,
  onAcknowledge,
}: {
  alerts: RiskAlert[];
  loading: boolean;
  onAcknowledge: (id: string) => void;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (alerts.length === 0) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        No alerts in the last 24 hours.
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {alerts.map((alert) => (
        <div
          key={alert.id}
          className={cn(
            "p-4 rounded-lg border",
            alert.resolved
              ? "bg-muted/30"
              : alert.severity === "CRITICAL"
              ? "bg-red-50 dark:bg-red-500/10 border-red-200 dark:border-red-800"
              : alert.severity === "WARNING"
              ? "bg-yellow-50 dark:bg-yellow-500/10 border-yellow-200 dark:border-yellow-800"
              : "bg-card"
          )}
        >
          <div className="flex items-start justify-between gap-4">
            <div className="flex items-start gap-3">
              {getAlertSeverityIcon(alert.severity)}
              <div>
                <div className="flex items-center gap-2">
                  <span className="font-medium">{alert.title}</span>
                  {alert.resolved && (
                    <Badge variant="outline" className="text-green-600 border-green-600">
                      Resolved
                    </Badge>
                  )}
                </div>
                <p className="text-sm text-muted-foreground mt-1">{alert.description}</p>
                <div className="flex items-center gap-3 mt-2 text-xs text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {getTimeAgo(alert.created_at)}
                  </span>
                  <span>Source: {alert.source}</span>
                </div>
              </div>
            </div>
            {!alert.acknowledged && !alert.resolved && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => onAcknowledge(alert.id)}
              >
                Acknowledge
              </Button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

// Main Risk Page
export default function RiskPage() {
  const [data, setData] = useState<RiskDashboardData | null>(null);
  const [loading, setLoading] = useState(true);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const { toast } = useToast();

  const fetchData = useCallback(async () => {
    try {
      // In production, replace with actual API call
      // const response = await riskApi.getDashboard();
      // setData(response);

      // Using mock data for now
      await new Promise((resolve) => setTimeout(resolve, 500));
      setData(generateMockData());
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Failed to load risk data";
      console.error("Failed to fetch risk data:", error);
      toast({
        variant: "destructive",
        title: "Error",
        description: message,
      });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  // Auto-refresh every 30 seconds
  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      fetchData();
    }, 30000);

    return () => clearInterval(interval);
  }, [autoRefresh, fetchData]);

  const handleAcknowledgeAlert = (id: string) => {
    if (!data) return;

    setData({
      ...data,
      recent_alerts: data.recent_alerts.map((alert) =>
        alert.id === id ? { ...alert, acknowledged: true } : alert
      ),
    });

    toast({
      title: "Alert acknowledged",
      description: "The alert has been marked as acknowledged.",
    });
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Risk Management</h1>
          <p className="text-muted-foreground">
            Stablecoin exposure and risk metrics monitoring
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant={autoRefresh ? "default" : "outline"}
            size="sm"
            onClick={() => setAutoRefresh(!autoRefresh)}
          >
            <Activity className={cn("h-4 w-4 mr-2", autoRefresh && "animate-pulse")} />
            {autoRefresh ? "Live" : "Paused"}
          </Button>
          <Button variant="outline" size="icon" onClick={fetchData} disabled={loading}>
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        </div>
      </div>

      {/* Risk Overview */}
      <RiskOverview data={data} loading={loading} />

      {/* Main Content */}
      <div className="grid gap-6 lg:grid-cols-3">
        {/* Left Column - Depeg Monitor & Exposure */}
        <div className="lg:col-span-2 space-y-6">
          <Tabs defaultValue="depeg" className="space-y-4">
            <TabsList>
              <TabsTrigger value="depeg">
                <TrendingDown className="h-4 w-4 mr-2" />
                Depeg Monitor
              </TabsTrigger>
              <TabsTrigger value="exposure">
                <PieChart className="h-4 w-4 mr-2" />
                Protocol Exposure
              </TabsTrigger>
              <TabsTrigger value="health">
                <Zap className="h-4 w-4 mr-2" />
                Health Factors
              </TabsTrigger>
            </TabsList>

            <TabsContent value="depeg">
              <Card>
                <CardHeader>
                  <CardTitle>Stablecoin Depeg Risk</CardTitle>
                  <CardDescription>
                    Real-time price deviation tracking for all stablecoins
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <DepegMonitor
                    indicators={data?.depeg_indicators || []}
                    loading={loading}
                  />
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="exposure">
              <Card>
                <CardHeader>
                  <CardTitle>Protocol Allocation</CardTitle>
                  <CardDescription>
                    Distribution of assets across DeFi protocols
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <ExposureChart
                    exposure={data?.protocol_exposure || []}
                    loading={loading}
                  />
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="health">
              <Card>
                <CardHeader>
                  <CardTitle>Health Factor Monitoring</CardTitle>
                  <CardDescription>
                    Liquidation risk monitoring for lending positions
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <HealthFactorMonitorPanel
                    healthFactors={data?.health_factors || []}
                    loading={loading}
                  />
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>

          {/* Alert History */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <Bell className="h-5 w-5" />
                    Recent Alerts
                  </CardTitle>
                  <CardDescription>Risk events from the last 24 hours</CardDescription>
                </div>
                {data && data.recent_alerts.filter((a) => !a.acknowledged).length > 0 && (
                  <Badge variant="destructive">
                    {data.recent_alerts.filter((a) => !a.acknowledged).length} unread
                  </Badge>
                )}
              </div>
            </CardHeader>
            <CardContent>
              <AlertHistory
                alerts={data?.recent_alerts || []}
                loading={loading}
                onAcknowledge={handleAcknowledgeAlert}
              />
            </CardContent>
          </Card>
        </div>

        {/* Right Column - Concentration Risk */}
        <div>
          <Card className="sticky top-4">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Shield className="h-5 w-5" />
                Concentration Risk
              </CardTitle>
              <CardDescription>
                Portfolio diversification limits
              </CardDescription>
            </CardHeader>
            <CardContent>
              <ConcentrationRiskPanel
                byToken={data?.concentration_by_token || []}
                byChain={data?.concentration_by_chain || []}
                loading={loading}
              />
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
