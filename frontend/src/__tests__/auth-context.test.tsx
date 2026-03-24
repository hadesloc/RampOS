import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
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

function AuthActionProbe() {
  const {
    error,
    loginWithPasskey,
    registerWithPasskey,
    loginWithMagicLink,
    verifyMagicLink,
  } = useAuth();

  return (
    <div>
      <button onClick={() => void loginWithPasskey().catch(() => {})}>passkey-login</button>
      <button onClick={() => void registerWithPasskey("user@example.com").catch(() => {})}>passkey-register</button>
      <button onClick={() => void loginWithMagicLink("user@example.com").catch(() => {})}>magic-link-login</button>
      <button onClick={() => void verifyMagicLink("token_123").catch(() => {})}>magic-link-verify</button>
      <span data-testid="error">{error ?? "none"}</span>
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

  it("fails closed for passkey login and registration", async () => {
    mockCheckSession.mockResolvedValue({
      authenticated: false,
      user: null,
    });

    render(
      <AuthProvider>
        <AuthActionProbe />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("error").textContent).toBe("none");
    });

    await act(async () => {
      fireEvent.click(screen.getByText("passkey-login"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("error").textContent).toContain(
        "Passkey sign-in is not available",
      );
    });

    await act(async () => {
      fireEvent.click(screen.getByText("passkey-register"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("error").textContent).toContain(
        "Passkey sign-in is not available",
      );
    });
  });

  it("fails closed for magic-link login and verification", async () => {
    mockCheckSession.mockResolvedValue({
      authenticated: false,
      user: null,
    });

    render(
      <AuthProvider>
        <AuthActionProbe />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("error").textContent).toBe("none");
    });

    await act(async () => {
      fireEvent.click(screen.getByText("magic-link-login"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("error").textContent).toContain(
        "Magic link sign-in is not available",
      );
    });

    await act(async () => {
      fireEvent.click(screen.getByText("magic-link-verify"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("error").textContent).toContain(
        "Magic link sign-in is not available",
      );
    });
  });
});
