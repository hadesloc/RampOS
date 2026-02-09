"use client";

import { useState, useEffect } from "react";
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
import { Checkbox } from "@/components/ui/checkbox";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { useAuth } from "@/contexts/auth-context";
import { isWebAuthnSupported, isPlatformAuthenticatorAvailable } from "@/lib/webauthn";
import Link from "next/link";
import { useRouter } from "@/navigation";
import {
  Fingerprint,
  Loader2,
  AlertCircle,
  Shield,
  Smartphone,
  CheckCircle,
} from "lucide-react";
import { useTranslations } from "next-intl";

export default function RegisterPage() {
  const [email, setEmail] = useState("");
  const [termsAccepted, setTermsAccepted] = useState(false);
  const [webAuthnAvailable, setWebAuthnAvailable] = useState(true);
  const [platformAuthAvailable, setPlatformAuthAvailable] = useState(false);
  const t = useTranslations('Portal.auth.register');

  const {
    registerWithPasskey,
    isLoading,
    error,
    clearError,
    isAuthenticated,
  } = useAuth();

  const router = useRouter();

  // Check WebAuthn availability
  useEffect(() => {
    const checkWebAuthn = async () => {
      const supported = isWebAuthnSupported();
      setWebAuthnAvailable(supported);

      if (supported) {
        const platformAvailable = await isPlatformAuthenticatorAvailable();
        setPlatformAuthAvailable(platformAvailable);
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

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!termsAccepted || !email) return;

    clearError();
    try {
      await registerWithPasskey(email);
    } catch {
      // Error is handled by context
    }
  };

  if (!webAuthnAvailable) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background px-4">
        <Card className="w-full max-w-md">
          <CardHeader className="space-y-1">
            <CardTitle className="text-2xl font-bold tracking-tight">
              {t('browser_unsupported')}
            </CardTitle>
            <CardDescription>
              {t('browser_unsupported_desc')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                {t('browser_unsupported_alert')}
              </AlertDescription>
            </Alert>
          </CardContent>
          <CardFooter>
            <Link href="/portal/login" className="w-full">
              <Button variant="outline" className="w-full">
                {t('go_to_login')}
              </Button>
            </Link>
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
          <CardDescription className="text-center">
            {t('subtitle')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleRegister} className="space-y-4">
            {error && (
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            {/* Passkey benefits */}
            <div className="rounded-lg bg-muted p-4 space-y-3">
              <p className="text-sm font-medium">{t('why_passkeys')}</p>
              <div className="space-y-2 text-sm text-muted-foreground">
                <div className="flex items-start gap-2">
                  <Shield className="h-4 w-4 mt-0.5 text-green-500" />
                  <span>{t('benefit_secure')}</span>
                </div>
                <div className="flex items-start gap-2">
                  <Smartphone className="h-4 w-4 mt-0.5 text-blue-500" />
                  <span>
                    {t('benefit_biometric')}
                  </span>
                </div>
                <div className="flex items-start gap-2">
                  <CheckCircle className="h-4 w-4 mt-0.5 text-purple-500" />
                  <span>{t('benefit_easy')}</span>
                </div>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="email">{t('email_label')}</Label>
              <Input
                id="email"
                type="email"
                placeholder="m@example.com"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                disabled={isLoading}
                className="h-11"
              />
            </div>

            <div className="flex items-start space-x-2">
              <Checkbox
                id="terms"
                checked={termsAccepted}
                onCheckedChange={(checked) =>
                  setTermsAccepted(checked as boolean)
                }
                disabled={isLoading}
              />
              <Label
                htmlFor="terms"
                className="text-sm font-normal leading-snug peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
              >
                {t('terms_agree')}{" "}
                <Link href="/terms" className="text-primary hover:underline">
                  {t('terms_of_service')}
                </Link>{" "}
                {t('and')}{" "}
                <Link href="/privacy" className="text-primary hover:underline">
                  {t('privacy_policy')}
                </Link>
              </Label>
            </div>

            <Button
              className="w-full"
              type="submit"
              disabled={isLoading || !termsAccepted || !email}
            >
              {isLoading ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Fingerprint className="mr-2 h-4 w-4" />
              )}
              {isLoading ? t('setting_up') : t('create_btn')}
            </Button>

            {platformAuthAvailable && (
              <p className="text-xs text-center text-muted-foreground">
                {t('biometric_prompt')}
              </p>
            )}
          </form>
        </CardContent>
        <CardFooter className="flex flex-col space-y-2">
          <div className="text-sm text-muted-foreground text-center">
            {t('already_have_account')}{" "}
            <Link
              href="/portal/login"
              className="text-primary hover:underline"
            >
              {t('sign_in')}
            </Link>
          </div>
        </CardFooter>
      </Card>
    </div>
  );
}
