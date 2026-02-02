"use client";

import { Suspense, useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { useAuth } from "@/contexts/auth-context";
import { isWebAuthnSupported, isPlatformAuthenticatorAvailable } from "@/lib/webauthn";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import {
  Fingerprint,
  Mail,
  ArrowRight,
  Loader2,
  AlertCircle,
  CheckCircle2,
} from "lucide-react";

function LoginContent() {
  const [email, setEmail] = useState("");
  const [mode, setMode] = useState<"passkey" | "magic-link">("passkey");
  const [magicLinkSent, setMagicLinkSent] = useState(false);
  const [webAuthnAvailable, setWebAuthnAvailable] = useState(true);
  const [platformAuthAvailable, setPlatformAuthAvailable] = useState(false);

  const {
    loginWithPasskey,
    loginWithMagicLink,
    verifyMagicLink,
    isLoading,
    error,
    clearError,
    isAuthenticated,
  } = useAuth();

  const router = useRouter();
  const searchParams = useSearchParams();
  const magicLinkToken = searchParams.get("token");

  // Check WebAuthn availability
  useEffect(() => {
    const checkWebAuthn = async () => {
      const supported = isWebAuthnSupported();
      setWebAuthnAvailable(supported);

      if (supported) {
        const platformAvailable = await isPlatformAuthenticatorAvailable();
        setPlatformAuthAvailable(platformAvailable);
      }

      if (!supported) {
        setMode("magic-link");
      }
    };

    checkWebAuthn();
  }, []);

  // Redirect if already authenticated
  useEffect(() => {
    if (isAuthenticated) {
      router.push("/portal");
    }
  }, [isAuthenticated, router]);

  // Handle magic link verification
  useEffect(() => {
    if (magicLinkToken) {
      verifyMagicLink(magicLinkToken).catch(() => {
        // Error is handled by context
      });
    }
  }, [magicLinkToken, verifyMagicLink]);

  const handlePasskeyLogin = async () => {
    clearError();
    try {
      await loginWithPasskey(email || undefined);
    } catch {
      // Error is handled by context
    }
  };

  const handleMagicLinkLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    clearError();

    try {
      await loginWithMagicLink(email);
      setMagicLinkSent(true);
    } catch {
      // Error is handled by context
    }
  };

  // Show loading if verifying magic link
  if (magicLinkToken && isLoading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background px-4">
        <Card className="w-full max-w-md">
          <CardContent className="flex flex-col items-center py-10">
            <Loader2 className="h-12 w-12 animate-spin text-primary mb-4" />
            <p className="text-muted-foreground">Verifying your login...</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  // Show magic link sent confirmation
  if (magicLinkSent) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background px-4">
        <Card className="w-full max-w-md">
          <CardHeader className="space-y-1">
            <div className="flex items-center justify-center mb-4">
              <div className="rounded-full bg-green-100 p-3 dark:bg-green-900/30">
                <CheckCircle2 className="h-8 w-8 text-green-600 dark:text-green-400" />
              </div>
            </div>
            <CardTitle className="text-2xl font-bold tracking-tight text-center">
              Check your email
            </CardTitle>
            <CardDescription className="text-center">
              We sent a magic link to <strong>{email}</strong>. Click the link
              in the email to sign in.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <Alert>
              <AlertDescription>
                The link will expire in 10 minutes. Check your spam folder if
                you do not see the email.
              </AlertDescription>
            </Alert>
          </CardContent>
          <CardFooter className="flex flex-col space-y-2">
            <Button
              variant="outline"
              className="w-full"
              onClick={() => {
                setMagicLinkSent(false);
                setEmail("");
              }}
            >
              Use a different email
            </Button>
          </CardFooter>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4">
      <Card className="w-full max-w-md">
        <CardHeader className="space-y-1">
          <CardTitle className="text-2xl font-bold tracking-tight">
            Welcome back
          </CardTitle>
          <CardDescription>Sign in to your RampOS account</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {error && (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {mode === "passkey" ? (
            <div className="space-y-4">
              {webAuthnAvailable ? (
                <>
                  <div className="space-y-2">
                    <Label htmlFor="email-passkey">
                      Email (optional for passkey)
                    </Label>
                    <Input
                      id="email-passkey"
                      type="email"
                      placeholder="m@example.com"
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                    />
                    <p className="text-xs text-muted-foreground">
                      Leave empty to use any registered passkey
                    </p>
                  </div>

                  <Button
                    className="w-full h-12 text-base"
                    size="lg"
                    onClick={handlePasskeyLogin}
                    disabled={isLoading}
                  >
                    {isLoading ? (
                      <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                    ) : (
                      <Fingerprint className="mr-2 h-5 w-5" />
                    )}
                    Sign in with Passkey
                  </Button>

                  {platformAuthAvailable && (
                    <p className="text-xs text-center text-muted-foreground">
                      Use Touch ID, Face ID, or Windows Hello
                    </p>
                  )}
                </>
              ) : (
                <Alert>
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription>
                    Passkey authentication is not available in this browser.
                    Please use email login instead.
                  </AlertDescription>
                </Alert>
              )}

              <div className="relative">
                <div className="absolute inset-0 flex items-center">
                  <span className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs uppercase">
                  <span className="bg-background px-2 text-muted-foreground">
                    Or
                  </span>
                </div>
              </div>

              <Button
                variant="outline"
                className="w-full"
                onClick={() => {
                  setMode("magic-link");
                  clearError();
                }}
              >
                <Mail className="mr-2 h-4 w-4" />
                Continue with Email
              </Button>
            </div>
          ) : (
            <form onSubmit={handleMagicLinkLogin} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="email">Email</Label>
                <Input
                  id="email"
                  type="email"
                  placeholder="m@example.com"
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  disabled={isLoading}
                />
              </div>
              <Button
                className="w-full"
                type="submit"
                disabled={isLoading || !email}
              >
                {isLoading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                Send Magic Link
                {!isLoading && <ArrowRight className="ml-2 h-4 w-4" />}
              </Button>

              {webAuthnAvailable && (
                <Button
                  variant="ghost"
                  className="w-full"
                  type="button"
                  onClick={() => {
                    setMode("passkey");
                    clearError();
                  }}
                >
                  <Fingerprint className="mr-2 h-4 w-4" />
                  Back to Passkey Login
                </Button>
              )}
            </form>
          )}
        </CardContent>
        <CardFooter className="flex flex-col space-y-2">
          <div className="text-sm text-muted-foreground text-center">
            Do not have an account?{" "}
            <Link
              href="/portal/register"
              className="text-primary hover:underline"
            >
              Create an account
            </Link>
          </div>
        </CardFooter>
      </Card>
    </div>
  );
}

function LoginFallback() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4">
      <Card className="w-full max-w-md">
        <CardContent className="flex flex-col items-center py-10">
          <Loader2 className="h-12 w-12 animate-spin text-primary mb-4" />
          <p className="text-muted-foreground">Loading...</p>
        </CardContent>
      </Card>
    </div>
  );
}

export default function LoginPage() {
  return (
    <Suspense fallback={<LoginFallback />}>
      <LoginContent />
    </Suspense>
  );
}
