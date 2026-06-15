import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import VenueFundingPage from "@/app/[locale]/portal/venues/page";

const pushMock = vi.fn();
const listVenuesMock = vi.fn();
const connectMock = vi.fn();
const getEligibilityMock = vi.fn();
const prepareMock = vi.fn();
const submitMock = vi.fn();
const getStatusMock = vi.fn();

vi.mock("@/navigation", () => ({
  Link: ({ children, href, ...props }: any) => {
    const React = require("react");
    return React.createElement("a", { href, ...props }, children);
  },
  useRouter: () => ({
    push: pushMock,
    replace: vi.fn(),
    prefetch: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
}));

vi.mock("@/contexts/auth-context", () => ({
  useAuth: () => ({
    user: { email: "user@example.com" },
    wallet: { address: "0x123", owner: "user", factoryAddress: "factory", deployed: true },
    isLoading: false,
    isAuthenticated: true,
    error: null,
    loginWithPasskey: vi.fn(),
    registerWithPasskey: vi.fn(),
    loginWithMagicLink: vi.fn(),
    verifyMagicLink: vi.fn(),
    logout: vi.fn(),
    refreshWallet: vi.fn(),
    createWallet: vi.fn(),
    clearError: vi.fn(),
  }),
}));

vi.mock("@/lib/portal-api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/portal-api")>("@/lib/portal-api");
  return {
    ...actual,
    venueFundingApi: {
      listVenues: (...args: unknown[]) => listVenuesMock(...args),
      connect: (...args: unknown[]) => connectMock(...args),
      getEligibility: (...args: unknown[]) => getEligibilityMock(...args),
      prepare: (...args: unknown[]) => prepareMock(...args),
      submit: (...args: unknown[]) => submitMock(...args),
      getStatus: (...args: unknown[]) => getStatusMock(...args),
    },
  };
});

describe("VenueFundingPage", () => {
  beforeEach(() => {
    pushMock.mockReset();
    listVenuesMock.mockReset();
    connectMock.mockReset();
    getEligibilityMock.mockReset();
    prepareMock.mockReset();
    submitMock.mockReset();
    getStatusMock.mockReset();
  });

  it("renders the venue funding workspace with curated venues", async () => {
    listVenuesMock.mockResolvedValue({
      venues: [
        {
          venueKey: "kraken",
          displayName: "Kraken",
          status: "active",
          supportsWalletFunding: false,
        },
        {
          venueKey: "hyperliquid",
          displayName: "Hyperliquid",
          status: "active",
          supportsWalletFunding: true,
        },
      ],
    });

    render(<VenueFundingPage />);

    expect(screen.getByRole("heading", { level: 1, name: "Venue Funding" })).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "Hyperliquid" })).toBeInTheDocument();
    });

    expect(screen.getByLabelText(/funding amount/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/venue connection id/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/venue account id/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/wallet attestation id/i)).toBeInTheDocument();
    expect(
      screen.getByText(/currently funds only the hyperliquid wallet-linked pilot/i)
    ).toBeInTheDocument();
  });
});
