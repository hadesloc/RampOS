import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import SettlementPage from "@/app/[locale]/(admin)/settlement/page";

const mockFetch = vi.fn();

const activeResponse = {
  actionMode: "approval_gated",
  approvalMode: "manual_approval",
  proposalCount: 2,
  exportFormats: ["json", "csv"],
  snapshot: {
    generatedAt: "2026-03-09T12:00:00Z",
    approvalMode: "manual_approval",
    actionMode: "approval_gated",
    proposals: [
      {
        id: "nsp_active_lp_alpha",
        counterpartyId: "lp_alpha",
        asset: "USDT",
        settlementIds: ["stl_active_001", "stl_active_002"],
        grossIn: "350",
        grossOut: "520",
        netAmount: "170",
        direction: "pay",
        status: "pending_approval",
        approvalRequired: true,
        summary: "lp_alpha owes a net payout after bilateral compression of same-window settlements.",
      },
    ],
    alerts: [
      {
        id: "net_settlement_pending_approval",
        severity: "medium",
        title: "At least one bilateral proposal is waiting for approval",
        summary: "Keep settlement execution manual until the bilateral proposal is explicitly approved.",
      },
    ],
  },
};

const cleanResponse = {
  ...activeResponse,
  proposalCount: 1,
  snapshot: {
    ...activeResponse.snapshot,
    proposals: [
      {
        id: "nsp_clean_lp_beta",
        counterpartyId: "lp_beta",
        asset: "USDT",
        settlementIds: ["stl_clean_001", "stl_clean_002"],
        grossIn: "250",
        grossOut: "245",
        netAmount: "5",
        direction: "receive",
        status: "draft",
        approvalRequired: true,
        summary: "Counterparty exposure is nearly flat; no bilateral settlement action is urgent.",
      },
    ],
    alerts: [],
  },
};

describe("SettlementPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads the settlement workbench and can switch to the clean control", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => activeResponse,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => cleanResponse,
      });

    render(<SettlementPage />);

    expect(await screen.findByText(/settlement workbench/i)).toBeInTheDocument();
    expect(screen.getByText(/sla guardian/i)).toBeInTheDocument();
    expect(screen.getByText(/1 proposal needs review inside 30 min/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /bilateral proposals/i })).toBeInTheDocument();
    expect(screen.getAllByText(/lp_alpha/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/recommend treasury approval review before any release/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /clean control/i }));

    await waitFor(() => {
      expect(screen.getByText(/lp_beta/i)).toBeInTheDocument();
    });
  });

  it("renders a recoverable load failure", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      json: async () => ({
        message: "Settlement workbench unavailable",
      }),
    });

    render(<SettlementPage />);

    expect(
      await screen.findByRole("heading", { name: /settlement workbench unavailable/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /reload workbench/i })).toBeInTheDocument();
  });
});
