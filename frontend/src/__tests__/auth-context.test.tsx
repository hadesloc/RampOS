import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AuthProvider, useAuth, withAuth } from "@/contexts/auth-context";

const { mockPush, mockCheckSession, mockGetAccount, mockLogin, mockRegister } = vi.hoisted(() => ({
  mockPush: vi.fn(),
  mockCheckSession: vi.fn(),
  mockGetAccount: vi.fn(),
  mockLogin: vi.fn(),
  mockRegister: vi.fn(),
}));

vi.mock("@/navigation", () => ({
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
      login: mockLogin,
      register: mockRegister,
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
  const { error, loginWithPassword, registerWithPassword } = useAuth();

  return (
    <div>
      <button
        onClick={() =>
          void loginWithPassword("user@example.com", "password-value").catch(() => {})
        }
      >
        password-login
      </button>
      <button
        onClick={() =>
          void registerWithPassword(
            "user@example.com",
            "password-value",
            "Portal User",
          ).catch(() => {})
        }
      >
        password-register
      </button>
      <span data-testid="error">{error ?? "none"}</span>
    </div>
  );
}

describe("AuthProvider", () => {
  beforeEach(() => {
    mockPush.mockReset();
    mockCheckSession.mockReset();
    mockGetAccount.mockReset();
    mockLogin.mockReset();
    mockRegister.mockReset();
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

  it("authenticates with password login and registration", async () => {
    mockCheckSession.mockResolvedValue({
      authenticated: false,
      user: null,
    });
    const user = {
      id: "portal-user",
      email: "user@example.com",
      kycStatus: "NONE",
      kycTier: 0,
      status: "ACTIVE",
      createdAt: "2026-06-25T00:00:00Z",
    };
    mockLogin.mockResolvedValue({ user, expiresAt: 123 });
    mockRegister.mockResolvedValue({ user, expiresAt: 123 });
    mockGetAccount.mockRejectedValue(new Error("not provisioned"));

    render(
      <AuthProvider>
        <AuthActionProbe />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("error").textContent).toBe("none");
    });

    await act(async () => {
      fireEvent.click(screen.getByText("password-login"));
    });
    await waitFor(() => {
      expect(mockLogin).toHaveBeenCalledWith("user@example.com", "password-value");
      expect(mockPush).toHaveBeenCalledWith("/portal");
    });

    await act(async () => {
      fireEvent.click(screen.getByText("password-register"));
    });
    await waitFor(() => {
      expect(mockRegister).toHaveBeenCalledWith(
        "user@example.com",
        "password-value",
        "Portal User",
      );
      expect(screen.getByTestId("error").textContent).toBe("none");
    });
  });
});
