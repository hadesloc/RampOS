import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import TreasuryPage from "@/app/[locale]/(admin)/treasury/page";

const mockFetch = vi.fn();

const activeResponse = {
  actionMode: "recommendation_only",
  recommendationCount: 3,
  stressAlertCount: 2,
  snapshot: {
    generatedAt: "2026-03-09T12:00:00Z",
    forecastWindowHours: 24,
    actionMode: "recommendation_only",
    bufferTargetPercent: 20,
    policyHint: "Balanced posture keeps instant-access reserves ahead of parking lanes.",
    floatSlices: [
      {
        segment: "bank:vcb/vnd",
        asset: "VND",
        available: "210",
        reserved: "160",
        utilizationPct: 76,
        shortageRisk: "high",
      },
    ],
    forecasts: [
      {
        asset: "VND",
        horizonHours: 24,
        projectedAvailable: "210",
        projectedRequired: "250",
        shortageAmount: "40",
        confidence: "high",
      },
    ],
    exposures: [
      {
        counterpartyType: "liquidity_provider",
        counterpartyId: "lp_alpha",
        direction: "OFFRAMP",
        pressureScore: "65",
        concentration: "high",
        reliabilityScore: "63",
        p95SettlementLatencySeconds: 1900,
      },
    ],
    alerts: [
      {
        id: "alert_float_shortage",
        severity: "critical",
        title: "Bank float forecast is below requirement",
        summary: "Projected shortage of 40 VND in the next 24 hours.",
        recommendationIds: ["treasury_prefund_bank_vnd"],
      },
    ],
    recommendations: [
      {
        id: "treasury_prefund_bank_vnd",
        category: "prefund",
        title: "Prefund the highest-pressure bank rail",
        summary: "Hold an extra 40 VND buffer on the primary bank rail.",
        asset: "VND",
        amount: "40",
        sourceSegment: "chain:ethereum/usdt",
        destinationSegment: "bank:vcb/vnd",
        confidence: "high",
        mode: "recommendation_only",
      },
    ],
    yieldAllocations: [
      {
        protocol: "aave-v3",
        principalAmount: "800000",
        currentValue: "824000",
        accruedYield: "24000",
        sharePercent: "65",
        strategyPosture: "capital_preservation",
      },
    ],
  },
};

const stableResponse = {
  ...activeResponse,
  snapshot: {
    ...activeResponse.snapshot,
    forecasts: [
      {
        asset: "VND",
        horizonHours: 24,
        projectedAvailable: "650",
        projectedRequired: "120",
        shortageAmount: "0",
        confidence: "medium",
      },
    ],
    alerts: [
      {
        id: "alert_no_action",
        severity: "low",
        title: "Treasury posture is healthy",
        summary: "No recommendation crossed the current treasury action threshold.",
        recommendationIds: [],
      },
    ],
  },
  stressAlertCount: 1,
};

describe("TreasuryPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads the treasury workbench and can switch to the stable fixture", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => activeResponse,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => stableResponse,
      });

    render(<TreasuryPage />);

    expect(await screen.findByText(/treasury control tower/i)).toBeInTheDocument();
    expect(screen.getByText(/prefund the highest-pressure bank rail/i)).toBeInTheDocument();
    expect(screen.getByText(/recommendation only/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /stable control/i }));

    await waitFor(() => {
      expect(screen.getByText(/treasury posture is healthy/i)).toBeInTheDocument();
    });

    expect(mockFetch).toHaveBeenNthCalledWith(1, "/api/proxy/v1/admin/treasury/workbench");
    expect(mockFetch).toHaveBeenNthCalledWith(
      2,
      "/api/proxy/v1/admin/treasury/workbench?scenario=stable",
    );
  });

  it("renders a recoverable load failure", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      json: async () => ({
        message: "Treasury workbench unavailable",
      }),
    });

    render(<TreasuryPage />);

    expect((await screen.findAllByText(/treasury workbench unavailable/i)).length).toBeGreaterThan(
      0,
    );
    expect(screen.getByRole("button", { name: /reload workbench/i })).toBeInTheDocument();
  });
});
