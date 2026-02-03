"use client";

import React, { createContext, useContext, useEffect, useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import {
  authApi,
  walletApi,
  setAuthToken,
  getAuthToken,
  AuthUser,
  AuthSession,
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

// WebAuthn helpers
function base64UrlEncode(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let str = "";
  for (let i = 0; i < bytes.length; i++) {
    str += String.fromCharCode(bytes[i]);
  }
  return btoa(str).replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
}

function base64UrlDecode(str: string): ArrayBuffer {
  const base64 = str.replace(/-/g, "+").replace(/_/g, "/");
  const padding = "=".repeat((4 - (base64.length % 4)) % 4);
  const raw = atob(base64 + padding);
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) {
    bytes[i] = raw.charCodeAt(i);
  }
  return bytes.buffer;
}

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [wallet, setWallet] = useState<SmartAccount | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const router = useRouter();

  const isAuthenticated = !!user;

  // Load user on mount
  useEffect(() => {
    const initAuth = async () => {
      const token = getAuthToken();
      if (!token) {
        setIsLoading(false);
        return;
      }

      try {
        const userData = await authApi.getMe();
        setUser(userData);

        // Also fetch wallet
        try {
          const walletData = await walletApi.getAccount();
          setWallet(walletData);
        } catch {
          // Wallet might not exist yet
        }
      } catch {
        // Token is invalid, clear it
        setAuthToken(null);
      } finally {
        setIsLoading(false);
      }
    };

    initAuth();
  }, []);

  const handleAuthSuccess = useCallback(async (session: AuthSession) => {
    setAuthToken(session.accessToken);
    setUser(session.user);

    // Store refresh token
    if (typeof window !== "undefined") {
      // TODO: Migrate to httpOnly cookies for better security
      localStorage.setItem("refresh_token", session.refreshToken);
    }

    // Fetch wallet
    try {
      const walletData = await walletApi.getAccount();
      setWallet(walletData);
    } catch {
      // Wallet might not exist yet
    }

    // Redirect to portal
    router.push("/portal");
  }, [router]);

  const loginWithPasskey = useCallback(async (email?: string) => {
    setIsLoading(true);
    setError(null);

    try {
      // Get challenge from server
      const challenge = await authApi.getAuthenticationChallenge(email);

      // Create credential request options
      const publicKeyCredentialRequestOptions: PublicKeyCredentialRequestOptions = {
        challenge: base64UrlDecode(challenge.challenge),
        rpId: challenge.rpId,
        timeout: challenge.timeout,
        userVerification: "preferred",
      };

      // Request credential from browser
      const credential = (await navigator.credentials.get({
        publicKey: publicKeyCredentialRequestOptions,
      })) as PublicKeyCredential;

      if (!credential) {
        throw new Error("No credential returned");
      }

      const response = credential.response as AuthenticatorAssertionResponse;

      // Send credential to server
      const session = await authApi.completeAuthentication({
        id: credential.id,
        rawId: base64UrlEncode(credential.rawId),
        type: "public-key",
        response: {
          clientDataJSON: base64UrlEncode(response.clientDataJSON),
          authenticatorData: base64UrlEncode(response.authenticatorData),
          signature: base64UrlEncode(response.signature),
        },
      });

      await handleAuthSuccess(session);
    } catch (err) {
      const message =
        err instanceof PortalApiError
          ? err.message
          : err instanceof Error
          ? err.message
          : "Authentication failed";
      setError(message);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [handleAuthSuccess]);

  const registerWithPasskey = useCallback(async (email: string) => {
    setIsLoading(true);
    setError(null);

    try {
      // Get challenge from server
      const challenge = await authApi.getRegistrationChallenge(email);

      // Create credential creation options
      const publicKeyCredentialCreationOptions: PublicKeyCredentialCreationOptions = {
        challenge: base64UrlDecode(challenge.challenge),
        rp: {
          id: challenge.rpId,
          name: challenge.rpName,
        },
        user: {
          id: base64UrlDecode(challenge.userId),
          name: challenge.userName,
          displayName: challenge.userDisplayName,
        },
        pubKeyCredParams: challenge.pubKeyCredParams,
        timeout: challenge.timeout,
        attestation: challenge.attestation,
        authenticatorSelection: challenge.authenticatorSelection,
        excludeCredentials: challenge.excludeCredentials?.map((cred) => ({
          id: base64UrlDecode(cred.id),
          type: cred.type,
          transports: cred.transports as AuthenticatorTransport[],
        })),
      };

      // Create credential
      const credential = (await navigator.credentials.create({
        publicKey: publicKeyCredentialCreationOptions,
      })) as PublicKeyCredential;

      if (!credential) {
        throw new Error("No credential returned");
      }

      const response = credential.response as AuthenticatorAttestationResponse;

      // Send credential to server
      const session = await authApi.completeRegistration(email, {
        id: credential.id,
        rawId: base64UrlEncode(credential.rawId),
        type: "public-key",
        response: {
          clientDataJSON: base64UrlEncode(response.clientDataJSON),
          attestationObject: base64UrlEncode(response.attestationObject),
        },
      });

      await handleAuthSuccess(session);
    } catch (err) {
      const message =
        err instanceof PortalApiError
          ? err.message
          : err instanceof Error
          ? err.message
          : "Registration failed";
      setError(message);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [handleAuthSuccess]);

  const loginWithMagicLink = useCallback(async (email: string) => {
    setIsLoading(true);
    setError(null);

    try {
      await authApi.requestMagicLink(email);
      // Don't set loading to false, show "check email" message
    } catch (err) {
      const message =
        err instanceof PortalApiError
          ? err.message
          : "Failed to send magic link";
      setError(message);
      setIsLoading(false);
      throw err;
    }
  }, []);

  const verifyMagicLink = useCallback(async (token: string) => {
    setIsLoading(true);
    setError(null);

    try {
      const session = await authApi.verifyMagicLink(token);
      await handleAuthSuccess(session);
    } catch (err) {
      const message =
        err instanceof PortalApiError
          ? err.message
          : "Invalid or expired magic link";
      setError(message);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [handleAuthSuccess]);

  const logout = useCallback(async () => {
    setIsLoading(true);
    try {
      await authApi.logout();
    } catch {
      // Ignore logout errors
    } finally {
      setAuthToken(null);
      if (typeof window !== "undefined") {
        localStorage.removeItem("refresh_token");
      }
      setUser(null);
      setWallet(null);
      setIsLoading(false);
      router.push("/portal/login");
    }
  }, [router]);

  const refreshWallet = useCallback(async () => {
    if (!user) return;

    try {
      const walletData = await walletApi.getAccount();
      setWallet(walletData);
    } catch {
      // Wallet refresh failed silently
    }
  }, [user]);

  const createWallet = useCallback(async () => {
    if (!user) {
      setError("Must be logged in to create wallet");
      return;
    }

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
  }, [user]);

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
    const { isAuthenticated, isLoading } = useAuth();
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
