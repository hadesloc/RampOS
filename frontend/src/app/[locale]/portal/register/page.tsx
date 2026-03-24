"use client";

import { useEffect } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { useAuth } from "@/contexts/auth-context";
import Link from "next/link";
import { useRouter } from "@/navigation";
import {
  AlertCircle,
  Shield,
} from "lucide-react";
import { useTranslations } from "next-intl";

export default function RegisterPage() {
  const t = useTranslations('Portal.auth.register');

  const {
    error,
    isAuthenticated,
  } = useAuth();

  const router = useRouter();
  const portalRegistrationUnavailable =
    "Portal registration is not available in this environment. Passkey completion and session issuance are still disabled on the backend.";

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
          <CardDescription className="text-center">
            {t('subtitle')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {error && (
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <Alert>
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>{portalRegistrationUnavailable}</AlertDescription>
            </Alert>
          </div>
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
          <p className="text-center text-xs text-muted-foreground">
            Portal access currently requires a pre-issued Bearer JWT rather than self-serve registration.
          </p>
        </CardFooter>
      </Card>
    </div>
  );
}
