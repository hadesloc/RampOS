import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AuthProvider, useAuth, withAuth } from "@/contexts/auth-context";

const { mockPush, mockCheckSession, mockGetAccount } = vi.hoisted(() => ({
  mockPush: vi.fn(),
  mockCheckSession: vi.fn(),
  mockGetAccount: vi.fn(),
}));

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: mockPush,
  }),
}));

vi.mock("@/lib/portal-api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/portal-api")>("@/lib/portal-api");
  return {
    ...actual,
    authApi: {
      ...actual.authApi,
      checkSession: mockCheckSession,
      requestMagicLink: vi.fn(),
      verifyMagicLink: vi.fn(),
      logout: vi.fn(),
    },
    walletApi: {
      ...actual.walletApi,
      getAccount: mockGetAccount,
      createAccount: vi.fn(),
    },
  };
});

function AuthStateProbe() {
  const { isAuthenticated, user, isLoading } = useAuth();
  return (
    <div>
      <span data-testid="auth">{String(isAuthenticated)}</span>
      <span data-testid="user">{user?.email ?? "none"}</span>
      <span data-testid="loading">{String(isLoading)}</span>
    </div>
  );
}

describe("AuthProvider", () => {
  beforeEach(() => {
    mockPush.mockReset();
    mockCheckSession.mockReset();
    mockGetAccount.mockReset();
  });

  it("fails closed when the portal session is unauthenticated", async () => {
    mockCheckSession.mockResolvedValue({
      authenticated: false,
      user: null,
    });

    render(
      <AuthProvider>
        <AuthStateProbe />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("loading").textContent).toBe("false");
    });

    expect(screen.getByTestId("auth").textContent).toBe("false");
    expect(screen.getByTestId("user").textContent).toBe("none");
    expect(mockGetAccount).not.toHaveBeenCalled();
  });

  it("redirects protected components when no authenticated session exists", async () => {
    mockCheckSession.mockResolvedValue({
      authenticated: false,
      user: null,
    });

    const Protected = withAuth(() => <div>secret</div>);

    render(
      <AuthProvider>
        <Protected />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(mockPush).toHaveBeenCalledWith("/portal/login");
    });
  });
});
