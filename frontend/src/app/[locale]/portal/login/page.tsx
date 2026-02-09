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
import { useRouter, useSearchParams } from "@/navigation";
import {
  Fingerprint,
  Mail,
  ArrowRight,
  Loader2,
  AlertCircle,
  CheckCircle2,
  Shield,
} from "lucide-react";
import { useTranslations } from "next-intl";

function LoginContent() {
  const [email, setEmail] = useState("");
  const [mode, setMode] = useState<"passkey" | "magic-link">("passkey");
  const [magicLinkSent, setMagicLinkSent] = useState(false);
  const [webAuthnAvailable, setWebAuthnAvailable] = useState(true);
  const [platformAuthAvailable, setPlatformAuthAvailable] = useState(false);
  const t = useTranslations('Portal.auth.login');
  const tCommon = useTranslations('Common');

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
      verifyMagicLink(magicLinkToken)
        .then(() => {
          const cleanUrl = new URL(window.location.href);
          cleanUrl.searchParams.delete("token");
          window.history.replaceState(
            null,
            "",
            cleanUrl.pathname + cleanUrl.search,
          );
        })
        .catch(() => {
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
            <p className="text-muted-foreground">{t('verifying')}</p>
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
              <div className="rounded-full bg-green-100 p-3 dark:bg-green-500/15">
                <CheckCircle2 className="h-8 w-8 text-green-600 dark:text-green-400" />
              </div>
            </div>
            <CardTitle className="text-2xl font-bold tracking-tight text-center">
              {t('check_email_title')}
            </CardTitle>
            <CardDescription className="text-center">
              {t.rich('check_email_desc', {
                email: email,
                strong: (chunks) => <strong>{chunks}</strong>
              })}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <Alert>
              <AlertDescription>
                {t('check_email_alert')}
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
              {t('use_different_email')}
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
          <div className="flex justify-center mb-4">
             <div className="h-12 w-12 rounded-lg bg-primary flex items-center justify-center">
                <Shield className="h-8 w-8 text-primary-foreground" />
             </div>
          </div>
          <CardTitle className="text-2xl font-bold tracking-tight text-center">
            {t('title')}
          </CardTitle>
          <CardDescription className="text-center">{t('subtitle')}</CardDescription>
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
                      {t('email_label')}
                    </Label>
                    <Input
                      id="email-passkey"
                      type="email"
                      placeholder={t('email_placeholder')}
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                      className="h-11"
                    />
                    <p className="text-xs text-muted-foreground">
                      {t('passkey_hint')}
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
                    {t('sign_in_passkey')}
                  </Button>

                  {platformAuthAvailable && (
                    <p className="text-xs text-center text-muted-foreground">
                      {t('biometric_hint')}
                    </p>
                  )}
                </>
              ) : (
                <Alert>
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription>
                    {t('passkey_unavailable')}
                  </AlertDescription>
                </Alert>
              )}

              <div className="relative">
                <div className="absolute inset-0 flex items-center">
                  <span className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs uppercase">
                  <span className="bg-background px-2 text-muted-foreground">
                    {t('or')}
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
                {t('continue_email')}
              </Button>
            </div>
          ) : (
            <form onSubmit={handleMagicLinkLogin} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="email">{t('email_input_label')}</Label>
                <Input
                  id="email"
                  type="email"
                  placeholder={t('email_placeholder')}
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  disabled={isLoading}
                  className="h-11"
                />
              </div>
              <Button
                className="w-full"
                type="submit"
                disabled={isLoading || !email}
              >
                {isLoading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                {t('send_magic_link')}
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
                  {t('back_to_passkey')}
                </Button>
              )}
            </form>
          )}
        </CardContent>
        <CardFooter className="flex flex-col space-y-2">
          <div className="text-sm text-muted-foreground text-center">
            {t('no_account')}{" "}
            <Link
              href="/portal/register"
              className="text-primary hover:underline"
            >
              {t('create_account')}
            </Link>
          </div>
        </CardFooter>
      </Card>
    </div>
  );
}

function LoginFallback() {
  const tCommon = useTranslations('Common');
  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4">
      <Card className="w-full max-w-md">
        <CardContent className="flex flex-col items-center py-10">
          <Loader2 className="h-12 w-12 animate-spin text-primary mb-4" />
          <p className="text-muted-foreground">{tCommon('loading')}</p>
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
