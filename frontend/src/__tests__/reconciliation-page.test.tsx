import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import ReconciliationPage from "@/app/[locale]/(admin)/reconciliation/page";

const mockFetch = vi.fn();

const workbenchResponse = {
  actionMode: "recommendation_only",
  exportFormats: ["json", "csv"],
  incidentLinkHint: "/v1/admin/incidents/timeline",
  snapshot: {
    generatedAt: "2026-03-09T10:00:00Z",
    report: {
      id: "recon_demo_001",
      totalDiscrepancies: 2,
      criticalCount: 1,
      status: "CriticalIssues",
    },
    queue: [
      {
        discrepancyId: "disc_queue_001",
        reportId: "recon_demo_001",
        ownerLane: "settlement_operations",
        rootCause: "offchain_recording_gap",
        ageBucket: "aging",
        severity: "High",
        settlementId: null,
        onChainTx: "0xqueue",
        detectedAt: "2026-03-09T09:45:00Z",
        summary: "On-chain tx 0xqueue has no matching settlement record",
        suggestedMatches: [{ settlementId: "stl_recon_processing_001", confidence: "high" }],
      },
      {
        discrepancyId: "disc_status_001",
        reportId: "recon_demo_001",
        ownerLane: "banking_partner",
        rootCause: "status_drift",
        ageBucket: "fresh",
        severity: "Critical",
        settlementId: "stl_recon_status_001",
        onChainTx: "0xstatus",
        detectedAt: "2026-03-09T09:52:00Z",
        summary: "Settlement stl_recon_status_001 is COMPLETED but tx 0xstatus is not confirmed",
        suggestedMatches: [],
      },
    ],
  },
};

const evidenceResponse = {
  queueItem: {
    discrepancyId: "disc_status_001",
    summary: "Settlement stl_recon_status_001 is COMPLETED but tx 0xstatus is not confirmed",
    severity: "Critical",
    ownerLane: "banking_partner",
    rootCause: "status_drift",
  },
  settlementIds: ["stl_recon_status_001"],
  replayEntries: [
    {
      referenceId: "disc_status_001",
      label: "Reconciliation discrepancy",
      status: "STATUS_MISMATCH",
    },
  ],
  incidentEntries: [
    {
      sourceReferenceId: "stl_recon_status_001",
      label: "Settlement status",
      status: "COMPLETED",
    },
  ],
};

describe("ReconciliationPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads the workbench, displays the queue, and exports queue/evidence attachments", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => workbenchResponse,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => evidenceResponse,
      })
      .mockResolvedValueOnce({
        ok: true,
        text: async () => "csv-body",
      })
      .mockResolvedValueOnce({
        ok: true,
        text: async () => "{\"ok\":true}",
      });

    render(<ReconciliationPage />);

    expect(await screen.findByText(/reconciliation ops workbench/i)).toBeInTheDocument();
    expect(screen.getByText(/sla guardian/i)).toBeInTheDocument();
    expect(screen.getByText(/1 needs attention within 15 min/i)).toBeInTheDocument();
    expect(screen.getByText(/offchain recording gap/i)).toBeInTheDocument();

    fireEvent.click(screen.getAllByRole("button", { name: /view evidence/i })[0]);

    await waitFor(() => {
      expect(screen.getByText(/stl_recon_status_001/i)).toBeInTheDocument();
    });

    expect(screen.getByText(/recommended response target/i)).toBeInTheDocument();
    expect(screen.getByText(/page banking partner and incident commander/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /export queue csv/i }));
    await waitFor(() => {
      expect(screen.getByText(/queue export ready in csv format/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /export evidence/i }));
    await waitFor(() => {
      expect(screen.getByText(/evidence pack export ready in json format/i)).toBeInTheDocument();
    });

    expect(mockFetch).toHaveBeenNthCalledWith(1, "/api/proxy/v1/admin/reconciliation/workbench");
    expect(mockFetch).toHaveBeenNthCalledWith(
      2,
      "/api/proxy/v1/admin/reconciliation/evidence/disc_queue_001",
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      3,
      "/api/proxy/v1/admin/reconciliation/export?format=csv",
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      4,
      "/api/proxy/v1/admin/reconciliation/evidence/disc_queue_001/export",
    );
  });

  it("renders a recoverable load failure", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      json: async () => ({
        message: "Reconciliation workbench unavailable",
      }),
    });

    render(<ReconciliationPage />);

    expect(await screen.findByText(/reconciliation workbench unavailable/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /reload workbench/i })).toBeInTheDocument();
    expect(screen.getByText(/switch between the active ops demo and a clean control case/i)).toBeInTheDocument();
  });
});
