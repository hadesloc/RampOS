import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { VenueFundingCard } from "../venue-funding-card";

describe("VenueFundingCard", () => {
  it("renders venues and wallet-first guidance", () => {
    render(
      <VenueFundingCard
        venues={[
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
        ]}
        selectedVenueKey="hyperliquid"
        walletTransferReference=""
        onSelectVenue={vi.fn()}
        onConnect={vi.fn()}
        onCheckEligibility={vi.fn()}
        onPrepare={vi.fn()}
        onSubmit={vi.fn()}
        onWalletTransferReferenceChange={vi.fn()}
      />
    );

    expect(screen.getByText("Wallet first, venue second")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Hyperliquid" })).toBeInTheDocument();
    expect(
      screen.getByText(/move governed wallet funds into a connected venue/i)
    ).toBeInTheDocument();
    expect(
      screen.getByText(/active wallet-funding pilot for hyperliquid usdt lanes/i)
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: /kraken visible for discovery only; wallet funding is not active for this venue active/i,
      })
    ).toBeDisabled();
  });

  it("submits the wallet transfer reference through the workflow action", () => {
    const onSubmit = vi.fn();
    const onPrepare = vi.fn();

    render(
      <VenueFundingCard
        venues={[
          {
            venueKey: "hyperliquid",
            displayName: "Hyperliquid",
            status: "active",
            supportsWalletFunding: true,
          },
        ]}
        selectedVenueKey="hyperliquid"
        preparedFlow={{
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
        }}
        walletTransferReference="wallet_tx_001"
        amount="125"
        venueConnectionId="conn_123"
        venueAccountId="acct_123"
        walletAttestationId="att_123"
        onSelectVenue={vi.fn()}
        onConnect={vi.fn()}
        onCheckEligibility={vi.fn()}
        onPrepare={onPrepare}
        onSubmit={onSubmit}
        onAmountChange={vi.fn()}
        onVenueConnectionIdChange={vi.fn()}
        onVenueAccountIdChange={vi.fn()}
        onWalletAttestationIdChange={vi.fn()}
        onWalletTransferReferenceChange={vi.fn()}
      />
    );

    expect(screen.getByLabelText(/funding amount/i)).toHaveValue("125");
    expect(screen.getByLabelText(/venue connection id/i)).toHaveValue("conn_123");
    expect(screen.getByLabelText(/venue account id/i)).toHaveValue("acct_123");
    expect(screen.getByLabelText(/wallet attestation id/i)).toHaveValue("att_123");

    fireEvent.click(screen.getByRole("button", { name: /prepare wallet funding/i }));
    expect(onPrepare).toHaveBeenCalled();

    fireEvent.click(
      screen.getByRole("button", { name: /mark wallet transfer submitted/i })
    );

    expect(onSubmit).toHaveBeenCalledWith("transfer_123", "wallet_tx_001");
  });
});
