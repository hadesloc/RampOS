import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import LiquidityPage from "@/app/[locale]/(admin)/liquidity/page";

const mockFetch = vi.fn();

describe("LiquidityPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads scorecard and policy catalog, applies filters, and activates a policy", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            lpId: "lp_alpha",
            direction: "OFFRAMP",
            windowKind: "ROLLING_30D",
            snapshotVersion: "snapshot-v1",
            quoteCount: 120,
            fillCount: 111,
            rejectCount: 6,
            settlementCount: 109,
            disputeCount: 1,
            fillRate: "0.9250",
            rejectRate: "0.0500",
            disputeRate: "0.0083",
            avgSlippageBps: "12.50",
            p95SettlementLatencySeconds: 142,
            reliabilityScore: "0.9412",
            updatedAt: "2026-03-09T10:00:00Z",
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          activeVersion: "liquidity-policy-default-v1",
          requestedDirection: "OFFRAMP",
          policies: [
            {
              version: "liquidity-policy-default-v1",
              direction: "OFFRAMP",
              reliabilityWindowKind: "ROLLING_30D",
              minReliabilityObservations: 3,
              fallbackBehavior: "BEST_PRICE_IF_POLICY_DATA_ABSENT",
              weights: {
                priceWeight: "0.20",
                reliabilityWeight: "0.40",
                fillRateWeight: "0.20",
                rejectRateWeight: "0.10",
                disputeRateWeight: "0.05",
                slippageWeight: "0.03",
                settlementLatencyWeight: "0.02",
              },
            },
            {
              version: "liquidity-policy-price-bias-v1",
              direction: "OFFRAMP",
              reliabilityWindowKind: "ROLLING_30D",
              minReliabilityObservations: 3,
              fallbackBehavior: "BEST_PRICE_IF_POLICY_DATA_ABSENT",
              weights: {
                priceWeight: "0.45",
                reliabilityWeight: "0.20",
                fillRateWeight: "0.15",
                rejectRateWeight: "0.10",
                disputeRateWeight: "0.05",
                slippageWeight: "0.03",
                settlementLatencyWeight: "0.02",
              },
            },
          ],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            lpId: "lp_beta",
            direction: "ONRAMP",
            windowKind: "ROLLING_7D",
            snapshotVersion: "snapshot-v2",
            quoteCount: 44,
            fillCount: 34,
            rejectCount: 5,
            settlementCount: 31,
            disputeCount: 0,
            fillRate: "0.7727",
            rejectRate: "0.1136",
            disputeRate: "0.0000",
            avgSlippageBps: "7.25",
            p95SettlementLatencySeconds: 86,
            reliabilityScore: "0.8011",
            updatedAt: "2026-03-09T12:00:00Z",
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          activeVersion: "liquidity-policy-default-v1",
          requestedDirection: "ONRAMP",
          policies: [
            {
              version: "liquidity-policy-default-v1",
              direction: "ONRAMP",
              reliabilityWindowKind: "ROLLING_30D",
              minReliabilityObservations: 3,
              fallbackBehavior: "BEST_PRICE_IF_POLICY_DATA_ABSENT",
              weights: {
                priceWeight: "0.20",
                reliabilityWeight: "0.40",
                fillRateWeight: "0.20",
                rejectRateWeight: "0.10",
                disputeRateWeight: "0.05",
                slippageWeight: "0.03",
                settlementLatencyWeight: "0.02",
              },
            },
            {
              version: "liquidity-policy-price-bias-v1",
              direction: "ONRAMP",
              reliabilityWindowKind: "ROLLING_30D",
              minReliabilityObservations: 3,
              fallbackBehavior: "BEST_PRICE_IF_POLICY_DATA_ABSENT",
              weights: {
                priceWeight: "0.45",
                reliabilityWeight: "0.20",
                fillRateWeight: "0.15",
                rejectRateWeight: "0.10",
                disputeRateWeight: "0.05",
                slippageWeight: "0.03",
                settlementLatencyWeight: "0.02",
              },
            },
          ],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          status: "ACTIVATED",
          version: "liquidity-policy-price-bias-v1",
          direction: "ONRAMP",
          fallbackBehavior: "BEST_PRICE_IF_POLICY_DATA_ABSENT",
        }),
      });

    render(<LiquidityPage />);

    expect(await screen.findByText(/lp_alpha/i)).toBeInTheDocument();
    expect(screen.getAllByText(/liquidity-policy-default-v1/i).length).toBeGreaterThan(0);

    fireEvent.change(screen.getByLabelText(/lp id/i), {
      target: { value: "lp_beta" },
    });
    fireEvent.change(screen.getByLabelText(/direction/i), {
      target: { value: "ONRAMP" },
    });
    fireEvent.change(screen.getByLabelText(/window kind/i), {
      target: { value: "ROLLING_7D" },
    });
    fireEvent.click(screen.getByRole("button", { name: /apply filters/i }));

    await waitFor(() => {
      expect(screen.getByText(/lp_beta/i)).toBeInTheDocument();
    });

    fireEvent.click(
      screen.getByRole("button", {
        name: /activate liquidity-policy-price-bias-v1/i,
      }),
    );

    await waitFor(() => {
      expect(screen.getByText(/activated liquidity-policy-price-bias-v1/i)).toBeInTheDocument();
    });

    expect(mockFetch).toHaveBeenNthCalledWith(
      1,
      "/api/proxy/v1/admin/liquidity/scorecard?limit=20",
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      2,
      "/api/proxy/v1/admin/liquidity/policies/compare?direction=OFFRAMP",
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      3,
      "/api/proxy/v1/admin/liquidity/scorecard?lpId=lp_beta&direction=ONRAMP&windowKind=ROLLING_7D&limit=20",
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      4,
      "/api/proxy/v1/admin/liquidity/policies/compare?direction=ONRAMP",
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      5,
      "/api/proxy/v1/admin/liquidity/policies/activate",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          version: "liquidity-policy-price-bias-v1",
          direction: "ONRAMP",
        }),
      }),
    );
  });

  it("renders an empty scorecard state without hiding the policy catalog", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          activeVersion: "liquidity-policy-default-v1",
          requestedDirection: "OFFRAMP",
          policies: [
            {
              version: "liquidity-policy-default-v1",
              direction: "OFFRAMP",
              reliabilityWindowKind: "ROLLING_30D",
              minReliabilityObservations: 3,
              fallbackBehavior: "BEST_PRICE_IF_POLICY_DATA_ABSENT",
              weights: {
                priceWeight: "0.20",
                reliabilityWeight: "0.40",
                fillRateWeight: "0.20",
                rejectRateWeight: "0.10",
                disputeRateWeight: "0.05",
                slippageWeight: "0.03",
                settlementLatencyWeight: "0.02",
              },
            },
          ],
        }),
      });

    render(<LiquidityPage />);

    expect(await screen.findByText(/no scorecard rows match the current filters/i)).toBeInTheDocument();
    expect(screen.getAllByText(/liquidity-policy-default-v1/i).length).toBeGreaterThan(0);
  });

  it("renders a request error state when the scorecard load fails", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: false,
        json: async () => ({
          message: "Liquidity scorecard unavailable",
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          activeVersion: "liquidity-policy-default-v1",
          requestedDirection: "OFFRAMP",
          policies: [],
        }),
      });

    render(<LiquidityPage />);

    expect(await screen.findByText(/liquidity scorecard unavailable/i)).toBeInTheDocument();
    expect(screen.getByText(/policy catalog will appear once compare data loads/i)).toBeInTheDocument();
  });
});
