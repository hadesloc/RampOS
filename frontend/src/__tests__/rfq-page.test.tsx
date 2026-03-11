import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import RfqAdminPage from "@/app/[locale]/(admin)/rfq/page";

const mockFetch = vi.fn();

const listResponse = {
  data: [
    {
      id: "rfq_open_001",
      userId: "user_alpha",
      direction: "OFFRAMP",
      cryptoAsset: "USDT",
      cryptoAmount: "125",
      vndAmount: null,
      state: "OPEN",
      bidCount: 3,
      bestRate: "26100",
      expiresAt: "2099-03-09T12:05:00Z",
      createdAt: "2099-03-09T12:00:00Z",
    },
    {
      id: "rfq_open_002",
      userId: "user_beta",
      direction: "ONRAMP",
      cryptoAsset: "USDT",
      cryptoAmount: "50",
      vndAmount: "1250000",
      state: "OPEN",
      bidCount: 1,
      bestRate: null,
      expiresAt: "2099-03-09T12:03:00Z",
      createdAt: "2099-03-09T11:58:00Z",
    },
  ],
  total: 2,
  limit: 50,
  offset: 0,
};

describe("RfqAdminPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads request-level RFQ summaries from the admin API contract", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => listResponse,
    });

    render(<RfqAdminPage />);

    expect(await screen.findByText(/auction queue/i)).toBeInTheDocument();
    expect(screen.getByText(/active rfq requests returned by the admin api/i)).toBeInTheDocument();
    expect(screen.getByText(/user_alpha/i)).toBeInTheDocument();
    expect(screen.getByText(/user_beta/i)).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getAllByText(/26[,.]100/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/1[,.]250[,.]000/i)).toBeInTheDocument();
    expect(screen.getAllByText("OPEN").length).toBeGreaterThan(0);
  });

  it("finalizes an open RFQ and refreshes the list", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => listResponse,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          rfqId: "rfq_open_001",
          state: "MATCHED",
          winningLpId: "lp_1",
          finalRate: "26100",
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          ...listResponse,
          data: listResponse.data.slice(1),
          total: 1,
        }),
      });

    render(<RfqAdminPage />);

    const finalizeButtons = await screen.findAllByRole("button", { name: /finalize/i });
    fireEvent.click(finalizeButtons[0]);

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledTimes(3);
    });

    expect(mockFetch.mock.calls[1]?.[0]).toBe("/api/proxy/v1/admin/rfq/rfq_open_001/finalize");
  });

  it("renders a recoverable load failure", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      json: async () => ({
        message: "RFQ service unavailable",
      }),
    });

    render(<RfqAdminPage />);

    expect(await screen.findByText(/rfq service unavailable/i)).toBeInTheDocument();
  });
});
