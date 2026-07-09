"use client";

import React, { createContext, useContext, useEffect, useState, useCallback } from "react";
import { useRouter } from "@/navigation";
import {
  authApi,
  walletApi,
  walletAuthApi,
  AuthUser,
  SmartAccount,
  PortalApiError,
} from "@/lib/portal-api";

interface AuthContextType {
  user: AuthUser | null;
  wallet: SmartAccount | null;
  isLoading: boolean;
  isAuthenticated: boolean;
  error: string | null;
  // Auth methods
  loginWithPassword: (email: string, password: string) => Promise<void>;
  registerWithPassword: (
    email: string,
    password: string,
    fullName?: string,
  ) => Promise<void>;
  loginWithWallet: () => Promise<void>;
  logout: () => Promise<void>;
  // Wallet methods
  refreshWallet: () => Promise<void>;
  createWallet: () => Promise<void>;
  // Utils
  clearError: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [wallet, setWallet] = useState<SmartAccount | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const router = useRouter();

  // Fail closed until the backend confirms a real authenticated session.
  useEffect(() => {
    const initSession = async () => {
      try {
        const session = await authApi.checkSession();
        if (session.authenticated && session.user) {
          setUser(session.user);
          setIsAuthenticated(true);
          try {
            const walletData = await walletApi.getAccount();
            setWallet(walletData);
          } catch {
            setWallet(null);
          }
        } else {
          setUser(null);
          setWallet(null);
          setIsAuthenticated(false);
        }
      } catch (err) {
        // The initial session probe is a background check: any failure (401,
        // network error, or a 5xx because the backend is unreachable) simply
        // means the visitor is not authenticated yet. Fail closed quietly — the
        // login screen already communicates availability, so a probe failure
        // must not surface an alarming error banner to users.
        setUser(null);
        setWallet(null);
        setIsAuthenticated(false);
        if (process.env.NODE_ENV !== "production") {
          console.debug("Portal session probe failed (treating as logged out):", err);
        }
      } finally {
        setIsLoading(false);
      }
    };

    void initSession();
  }, []);

  const logout = useCallback(async () => {
    setError(null);
    try {
      await authApi.logout();
    } finally {
      setUser(null);
      setWallet(null);
      setIsAuthenticated(false);
      router.push("/portal/login");
    }
  }, [router]);

  const refreshWallet = useCallback(async () => {
    try {
      const walletData = await walletApi.getAccount();
      setWallet(walletData);
    } catch {
      // Wallet refresh failed silently
    }
  }, []);

  const loginWithPassword = useCallback(
    async (email: string, password: string) => {
      setIsLoading(true);
      setError(null);
      try {
        const response = await authApi.login(email, password);
        setUser(response.user);
        setIsAuthenticated(true);
        await refreshWallet();
        router.push("/portal");
      } catch (err) {
        const message =
          err instanceof PortalApiError
            ? err.message
            : err instanceof Error
              ? err.message
              : "Email sign-in failed.";
        setError(message);
        setIsAuthenticated(false);
        setUser(null);
        setWallet(null);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [refreshWallet, router],
  );

  const registerWithPassword = useCallback(
    async (email: string, password: string, fullName?: string) => {
      setIsLoading(true);
      setError(null);
      try {
        const response = await authApi.register(email, password, fullName);
        setUser(response.user);
        setIsAuthenticated(true);
        router.push("/portal");
      } catch (err) {
        const message =
          err instanceof PortalApiError
            ? err.message
            : err instanceof Error
              ? err.message
              : "Account registration failed.";
        setError(message);
        setIsAuthenticated(false);
        setUser(null);
        setWallet(null);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [router],
  );

  const loginWithWallet = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      if (!window.ethereum) {
        throw new Error(
          "No Ethereum wallet detected. Install MetaMask to sign in with your wallet."
        );
      }

      // 1. Request account access
      const accounts = (await window.ethereum.request({
        method: "eth_requestAccounts",
      })) as string[];
      const address = accounts[0];
      if (!address) {
        throw new Error("No account returned from wallet.");
      }

      // 2. Get nonce + EIP-4361 message from backend
      const { message } = await walletAuthApi.getNonce(address);

      // 3. Sign the exact message string — do NOT modify it
      const signature = (await window.ethereum.request({
        method: "personal_sign",
        params: [message, address],
      })) as string;

      // 4. Verify signature with backend → receive session cookies + user
      const res = await walletAuthApi.verify(message, signature);
      setUser(res.user);
      setIsAuthenticated(true);

      // 5. Load smart account provisioned during verify
      await refreshWallet();

      router.push("/portal");
    } catch (err) {
      // EIP-1193 user rejection
      const code = (err as { code?: number }).code;
      const message =
        code === 4001
          ? "Wallet sign-in cancelled."
          : err instanceof PortalApiError
          ? err.message
          : err instanceof Error
          ? err.message
          : "Wallet sign-in failed.";
      setError(message);
      setIsAuthenticated(false);
      setUser(null);
      setWallet(null);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [router, refreshWallet]);

  const createWallet = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const walletData = await walletApi.createAccount();
      setWallet(walletData);
    } catch (err) {
      const message =
        err instanceof PortalApiError
          ? err.message
          : "Failed to create wallet";
      setError(message);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const value: AuthContextType = {
    user,
    wallet,
    isLoading,
    isAuthenticated,
    error,
    loginWithPassword,
    registerWithPassword,
    loginWithWallet,
    logout,
    refreshWallet,
    createWallet,
    clearError,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}

// HOC for protected routes
export function withAuth<P extends object>(
  Component: React.ComponentType<P>
): React.FC<P> {
  return function ProtectedRoute(props: P) {
    const { isLoading, isAuthenticated } = useAuth();
    const router = useRouter();

    useEffect(() => {
      if (!isLoading && !isAuthenticated) {
        router.push("/portal/login");
      }
    }, [isAuthenticated, isLoading, router]);

    if (isLoading) {
      return (
        <div className="flex min-h-screen items-center justify-center">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        </div>
      );
    }

    if (!isAuthenticated) {
      return null;
    }

    return <Component {...props} />;
  };
}
