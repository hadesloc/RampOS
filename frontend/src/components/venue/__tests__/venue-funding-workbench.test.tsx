import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { VenueFundingWorkbench } from "../VenueFundingWorkbench";

const mockFetch = vi.fn();

const subjectSnapshot = {
  source: "registry",
  subjectType: "user",
  subjectId: "user-venue-1",
  connections: [
    {
      connectionId: "connection-venue-1",
      venueKey: "hyperliquid",
      status: "pending",
      connectionMode: "read_only",
      metadata: {},
    },
  ],
  accounts: [
    {
      accountId: "account-venue-1",
      venueKey: "hyperliquid",
      status: "active",
      metadata: {},
    },
  ],
  beneficiaryProfiles: [],
  walletAttestations: [
    {
      attestationId: "550e8400-e29b-41d4-a716-446655440000",
      walletAddress: "0xabc",
      network: "ethereum",
      attestationStatus: "pending",
      proofArtifactUri: "s3://proofs/venue-trust-attestation.json",
      metadata: {},
    },
  ],
  sourceOfFundsPackages: [
    {
      packageId: "sof-package-1",
      reviewStatus: "draft",
      packageUri: "s3://packages/source-of-funds-1.json",
      metadata: {},
    },
  ],
};

const transferDetail = {
  source: "registry",
  transfer: {
    transferId: "transfer-venue-1",
    status: "draft",
    transferDirection: "wallet_to_venue",
    assetSymbol: "USDT",
    network: "ethereum",
    amount: "2500",
    metadata: {},
  },
  connection: {
    connectionId: "connection-venue-1",
    venueKey: "hyperliquid",
    status: "pending",
    metadata: {},
  },
  account: {
    accountId: "account-venue-1",
    status: "active",
    metadata: {},
  },
  beneficiaryProfile: null,
  sourceOfFundsPackages: [
    {
      packageId: "sof-package-1",
      reviewStatus: "draft",
      metadata: {},
    },
  ],
};

const lighterReadiness = {
  connectorKey: "lighter",
  status: "attention_required",
  connectionId: "conn-lighter-1",
  accountId: "acct-lighter-1",
  publicPoolMode: "enabled",
  proofAnchorMode: "proof_anchor_enabled",
  operatorLinkageStatus: "institutional_linked",
  institutionalEvidenceStatus: "pending",
  requirements: [
    {
      code: "lighter_operator_linkage",
      status: "satisfied",
      message: "Operator linkage metadata is present.",
    },
    {
      code: "lighter_institutional_evidence",
      status: "attention_required",
      message: "Institutional evidence still requires approval.",
    },
  ],
};

const cexReadiness = {
  connectorKey: "binance",
  status: "ready",
  connectionId: "conn-cex-1",
  accountId: "acct-cex-1",
  apiKeyMode: "scoped_read_write",
  subaccountMode: "segregated",
  withdrawalAllowlistStatus: "verified",
  custodyBoundaryMode: "exchange_custody",
  requirements: [
    {
      code: "cex_api_key_mode",
      status: "satisfied",
      message: "API key mode is configured.",
    },
    {
      code: "cex_withdrawal_allowlist",
      status: "satisfied",
      message: "Withdrawal allowlist is verified.",
    },
  ],
};

const trustReport = {
  source: "registry",
  subjectType: "user",
  subjectId: "user-venue-1",
  cashIn: {
    transferCount: 1,
    totalAmount: "2500",
    statusCounts: {
      submitted: 1,
    },
    assetSymbols: ["USDT"],
    networks: ["ethereum"],
  },
  cashOut: {
    transferCount: 1,
    totalAmount: "500",
    statusCounts: {
      settled: 1,
    },
    assetSymbols: ["USDT"],
    networks: ["ethereum"],
  },
  evidenceReferences: [
    {
      kind: "connection",
      referenceId: "connection-venue-1",
      source: "venue_connection",
    },
    {
      kind: "transfer",
      referenceId: "transfer-venue-1",
      source: "venue_transfer",
    },
  ],
};

const evidenceExport = {
  fileName: "venue_trust_evidence_user_user-venue-1.json",
  contentType: "application/json",
  contents: {
    subjectType: "user",
    subjectId: "user-venue-1",
    evidenceReferences: trustReport.evidenceReferences,
  },
};

describe("VenueFundingWorkbench", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads venue trust snapshot and transfer detail from the admin proxy", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => subjectSnapshot,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => lighterReadiness,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => cexReadiness,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => trustReport,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => transferDetail,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => evidenceExport,
      });

    render(<VenueFundingWorkbench />);

    fireEvent.change(screen.getByLabelText(/subject id/i), {
      target: { value: "user-venue-1" },
    });
    fireEvent.click(screen.getByRole("button", { name: /load subject snapshot/i }));

    expect(await screen.findByText(/hyperliquid/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /lighter readiness/i })).toBeInTheDocument();
    expect(screen.getByText(/proof_anchor_enabled/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /cex readiness/i })).toBeInTheDocument();
    expect(screen.getByText(/scoped_read_write/i)).toBeInTheDocument();
    expect(screen.getByText(/exchange_custody/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /trust report/i })).toBeInTheDocument();
    expect(screen.getByText(/cash in/i)).toBeInTheDocument();
    expect(screen.getByText(/total amount:\s*2500/i)).toBeInTheDocument();
    expect(screen.getByText(/total amount:\s*500/i)).toBeInTheDocument();
    expect(screen.getAllByText(/connection-venue-1/i).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByRole("heading", { name: /wallet attestation/i })).toBeInTheDocument();
    expect(screen.getByText(/s3:\/\/proofs\/venue-trust-attestation\.json/i)).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/transfer id/i), {
      target: { value: "transfer-venue-1" },
    });
    fireEvent.click(screen.getByRole("button", { name: /load transfer detail/i }));

    expect(await screen.findByText(/transfer-venue-1/i)).toBeInTheDocument();
    expect(screen.getByText(/wallet_to_venue/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /load evidence export/i }));
    expect(
      await screen.findByText(/venue_trust_evidence_user_user-venue-1\.json/i),
    ).toBeInTheDocument();
    expect(mockFetch).toHaveBeenNthCalledWith(
      1,
      "/api/proxy/v1/admin/venue-trust/subjects/user/user-venue-1",
      undefined,
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      2,
      "/api/proxy/v1/admin/venue-trust/connectors/lighter/readiness/user/user-venue-1",
      undefined,
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      3,
      "/api/proxy/v1/admin/venue-trust/connectors/cex/binance/readiness/user/user-venue-1",
      undefined,
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      4,
      "/api/proxy/v1/admin/venue-trust/reports/user/user-venue-1",
      undefined,
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      5,
      "/api/proxy/v1/admin/venue-trust/transfers/transfer-venue-1",
      undefined,
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      6,
      "/api/proxy/v1/admin/venue-trust/reports/user/user-venue-1/export",
      undefined,
    );
  });

  it("posts bounded review actions for connection and wallet attestation", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => subjectSnapshot,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => lighterReadiness,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => cexReadiness,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => trustReport,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          ...subjectSnapshot.connections[0],
          status: "active",
          metadata: {
            reviewReason: "ops_review",
          },
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          ...subjectSnapshot.walletAttestations[0],
          attestationStatus: "verified",
          metadata: {
            reviewReason: "ops_review",
          },
        }),
      });

    render(<VenueFundingWorkbench />);

    fireEvent.change(screen.getByLabelText(/subject id/i), {
      target: { value: "user-venue-1" },
    });
    fireEvent.click(screen.getByRole("button", { name: /load subject snapshot/i }));

    await screen.findAllByText(/connection-venue-1/i);

    fireEvent.change(screen.getByLabelText(/review reason/i), {
      target: { value: "ops_review" },
    });

    fireEvent.click(screen.getByRole("button", { name: /approve connection/i }));
    await waitFor(() => {
      expect(screen.getByText(/connection updated to active/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /verify wallet/i }));
    await waitFor(() => {
      expect(screen.getByText(/wallet attestation updated to verified/i)).toBeInTheDocument();
    });

    expect(mockFetch).toHaveBeenNthCalledWith(
      4,
      "/api/proxy/v1/admin/venue-trust/reports/user/user-venue-1",
      undefined,
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      5,
      "/api/proxy/v1/admin/venue-trust/connections/connection-venue-1/review",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          status: "active",
          reviewReason: "ops_review",
        }),
      }),
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      6,
      "/api/proxy/v1/admin/venue-trust/wallet-attestations/550e8400-e29b-41d4-a716-446655440000/review",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          status: "verified",
          reviewReason: "ops_review",
        }),
      }),
    );
  });
});
