"use client";

import React, { createContext, useContext, useEffect, useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import {
  authApi,
  walletApi,
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
  loginWithPasskey: (email?: string) => Promise<void>;
  registerWithPasskey: (email: string) => Promise<void>;
  loginWithMagicLink: (email: string) => Promise<void>;
  verifyMagicLink: (token: string) => Promise<void>;
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
        setUser(null);
        setWallet(null);
        setIsAuthenticated(false);
        if (!(err instanceof PortalApiError && err.status === 401)) {
          setError(err instanceof Error ? err.message : "Failed to load session");
        }
      } finally {
        setIsLoading(false);
      }
    };

    void initSession();
  }, []);

  const loginWithPasskey = useCallback(async (_email?: string) => {
    setIsLoading(true);
    setError(null);
    try {
      throw new Error("Passkey login requires a verified backend WebAuthn flow.");
    } catch (err) {
      const message = err instanceof Error ? err.message : "Passkey login failed";
      setIsAuthenticated(false);
      setUser(null);
      setWallet(null);
      setError(message);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  const registerWithPasskey = useCallback(async (_email: string) => {
    setIsLoading(true);
    setError(null);
    try {
      throw new Error("Passkey registration requires a verified backend WebAuthn flow.");
    } catch (err) {
      const message = err instanceof Error ? err.message : "Passkey registration failed";
      setIsAuthenticated(false);
      setUser(null);
      setWallet(null);
      setError(message);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  const loginWithMagicLink = useCallback(async (email: string) => {
    setIsLoading(true);
    setError(null);
    try {
      await authApi.requestMagicLink(email);
    } catch (err) {
      const message =
        err instanceof PortalApiError ? err.message : "Magic link request failed";
      setError(message);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  const verifyMagicLink = useCallback(async (token: string) => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await authApi.verifyMagicLink(token);
      setUser(response.user);
      setIsAuthenticated(true);
      router.push("/portal");
    } catch (err) {
      const message =
        err instanceof PortalApiError ? err.message : "Magic link verification failed";
      setIsAuthenticated(false);
      setUser(null);
      setWallet(null);
      setError(message);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [router]);

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
    loginWithPasskey,
    registerWithPasskey,
    loginWithMagicLink,
    verifyMagicLink,
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
