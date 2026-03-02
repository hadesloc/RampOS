"use client";

import React, { createContext, useContext, useEffect, useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import {
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

const GUEST_USER: AuthUser = {
  id: "guest-user",
  email: "guest@rampos.local",
  kycStatus: "NONE",
  kycTier: 0,
  status: "ACTIVE",
  createdAt: new Date(0).toISOString(),
};

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user] = useState<AuthUser | null>(GUEST_USER);
  const [wallet, setWallet] = useState<SmartAccount | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const router = useRouter();

  const isAuthenticated = true;

  // Login is disabled: bootstrap guest state and attempt wallet fetch.
  useEffect(() => {
    const initGuest = async () => {
      try {
        const walletData = await walletApi.getAccount();
        setWallet(walletData);
      } catch {
        // Guest mode may not have a wallet yet.
      } finally {
        setIsLoading(false);
      }
    };

    initGuest();
  }, []);

  const loginWithPasskey = useCallback(async (_email?: string) => {
    setIsLoading(true);
    setError(null);
    router.push("/portal");
    setIsLoading(false);
  }, [router]);

  const registerWithPasskey = useCallback(async (_email: string) => {
    setIsLoading(true);
    setError(null);
    router.push("/portal");
    setIsLoading(false);
  }, [router]);

  const loginWithMagicLink = useCallback(async (_email: string) => {
    setIsLoading(true);
    setError(null);
    router.push("/portal");
    setIsLoading(false);
  }, [router]);

  const verifyMagicLink = useCallback(async (_token: string) => {
    setIsLoading(true);
    setError(null);
    router.push("/portal");
    setIsLoading(false);
  }, [router]);

  const logout = useCallback(async () => {
    setError(null);
    router.push("/portal");
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
    const { isLoading } = useAuth();

    if (isLoading) {
      return (
        <div className="flex min-h-screen items-center justify-center">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        </div>
      );
    }

    return <Component {...props} />;
  };
}
