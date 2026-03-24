"use client";

import { Suspense, useEffect } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { useAuth } from "@/contexts/auth-context";
import Link from "next/link";
import { useRouter } from "@/navigation";
import { useSearchParams } from "next/navigation";
import {
  Loader2,
  AlertCircle,
  Shield,
} from "lucide-react";
import { useTranslations } from "next-intl";

function LoginContent() {
  const t = useTranslations('Portal.auth.login');

  const {
    error,
    isAuthenticated,
  } = useAuth();

  const router = useRouter();
  const searchParams = useSearchParams();
  const magicLinkToken = searchParams?.get("token");
  const portalAuthUnavailable =
    "Portal sign-in is not available in this environment. Passkey and magic-link completion flows are still disabled on the backend.";
  const tokenUnavailable =
    "This magic link cannot be completed because portal token verification is not enabled in this environment.";

  useEffect(() => {
    if (isAuthenticated) {
      router.push("/portal");
    }
  }, [isAuthenticated, router]);

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
          <p className="text-center text-sm text-muted-foreground">{t('subtitle')}</p>
        </CardHeader>
        <CardContent className="space-y-4">
          {magicLinkToken && (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>{tokenUnavailable}</AlertDescription>
            </Alert>
          )}

          {error && (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
          <Alert>
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{portalAuthUnavailable}</AlertDescription>
          </Alert>
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
          <p className="text-center text-xs text-muted-foreground">
            Existing portal routes require a pre-issued Bearer JWT. Self-serve sign-in is currently disabled.
          </p>
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
