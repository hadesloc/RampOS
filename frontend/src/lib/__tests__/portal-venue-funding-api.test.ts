import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { venueFundingApi } from "@/lib/portal-api";

const mockFetch = vi.fn();

describe("venueFundingApi", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    global.fetch = mockFetch as unknown as typeof fetch;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("lists curated venue funding venues", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({
        venues: [
          {
            venueKey: "hyperliquid",
            displayName: "Hyperliquid",
            status: "active",
            supportsWalletFunding: true,
          },
          {
            venueKey: "kraken",
            displayName: "Kraken",
            status: "active",
            supportsWalletFunding: false,
          },
        ],
      }),
    });

    const result = await venueFundingApi.listVenues();

    expect(result.venues).toHaveLength(2);
    expect(result.venues[0].venueKey).toBe("hyperliquid");
    expect(result.venues[0].supportsWalletFunding).toBe(true);
    expect(result.venues[1].supportsWalletFunding).toBe(false);
    expect(mockFetch).toHaveBeenCalledWith(
      "http://localhost:3000/v1/portal/venue-funding/venues",
      expect.objectContaining({
        headers: {
          "Content-Type": "application/json",
        },
        credentials: "include",
      })
    );
  });

  it("submits wallet transfer references for prepared venue funding flows", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({
        id: "flow_123",
        subjectType: "user",
        subjectId: "user_123",
        status: "wallet_transfer_submitted",
        eligibilityDecision: "approved",
        source: "registry",
        nextAction: "await_venue_credit",
        walletTransferReference: "wallet_tx_001",
        sourceOfFunds: {
          action: "collect",
          packageId: null,
          source: "registry",
        },
      }),
    });

    const result = await venueFundingApi.submit("flow_123", "wallet_tx_001");

    expect(result.walletTransferReference).toBe("wallet_tx_001");
    expect(mockFetch).toHaveBeenCalledWith(
      "http://localhost:3000/v1/portal/venue-funding/flow_123/submit",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ walletTransferReference: "wallet_tx_001" }),
      })
    );
  });

  it("prepares durable venue funding transfers with required identifiers", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({
        id: "transfer_123",
        subjectType: "user",
        subjectId: "user_123",
        status: "draft",
        eligibilityDecision: "approved",
        source: "registry",
        checklist: [],
        sourceOfFunds: {
          action: "collect",
          packageId: null,
          source: "registry",
        },
      }),
    });

    await venueFundingApi.prepare({
      venueKey: "hyperliquid",
      jurisdiction: "VN",
      asset: "USDT",
      network: "ethereum",
      venueConnectionId: "conn_123",
      venueAccountId: "acct_123",
      walletAttestationId: "att_123",
      amount: "125",
    });

    expect(mockFetch).toHaveBeenCalledWith(
      "http://localhost:3000/v1/portal/venue-funding/prepare",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          venueKey: "hyperliquid",
          jurisdiction: "VN",
          asset: "USDT",
          network: "ethereum",
          venueConnectionId: "conn_123",
          venueAccountId: "acct_123",
          walletAttestationId: "att_123",
          amount: "125",
        }),
      })
    );
  });
});
