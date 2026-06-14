"use client";

import { Suspense, useEffect } from "react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { useAuth } from "@/contexts/auth-context";
import Link from "next/link";
import { useRouter } from "@/navigation";
import { useSearchParams } from "next/navigation";
import { Loader2, AlertCircle, Shield, Lock } from "lucide-react";
import { useTranslations } from "next-intl";

function LoginContent() {
  const t = useTranslations("Portal.auth.login");

  const { error, isAuthenticated } = useAuth();

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
    <div className="flex min-h-screen items-center justify-center bg-[#09090B] px-4">
      <div className="w-full max-w-md">
        {/* Logo mark */}
        <div className="flex flex-col items-center mb-8">
          <div className="h-14 w-14 rounded-2xl bg-[#111113] border border-[#00FF87]/20 flex items-center justify-center shadow-[0_0_24px_rgba(0,255,135,0.15)] mb-5">
            <Shield className="h-7 w-7 text-[#00FF87]" />
          </div>
          <h1 className="text-2xl font-bold tracking-tight text-white">
            {t("title")}
          </h1>
          <p className="mt-1.5 text-sm text-white/40">{t("subtitle")}</p>
        </div>

        {/* Card */}
        <div className="rounded-2xl border border-white/[0.06] bg-[#111113] p-6 space-y-4 shadow-xl">
          {magicLinkToken && (
            <Alert className="border-red-500/20 bg-red-500/10 text-red-400">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription className="text-red-400">
                {tokenUnavailable}
              </AlertDescription>
            </Alert>
          )}

          {error && (
            <Alert className="border-red-500/20 bg-red-500/10 text-red-400">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription className="text-red-400">{error}</AlertDescription>
            </Alert>
          )}

          <div className="flex items-start gap-3 rounded-lg border border-[#FFB800]/20 bg-[#FFB800]/[0.06] p-4">
            <Lock className="h-4 w-4 mt-0.5 shrink-0 text-[#FFB800]" />
            <p className="text-sm text-[#FFB800]/80">{portalAuthUnavailable}</p>
          </div>

          {/* Footer links */}
          <div className="pt-2 space-y-3 text-center">
            <p className="text-sm text-white/40">
              {t("no_account")}{" "}
              <Link
                href="/portal/register"
                className="text-[#7B61FF] hover:text-[#7B61FF]/80 transition-colors"
              >
                {t("create_account")}
              </Link>
            </p>
            <p className="text-xs text-white/20">
              Existing portal routes require a pre-issued Bearer JWT. Self-serve sign-in is currently disabled.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

function LoginFallback() {
  const tCommon = useTranslations("Common");
  return (
    <div className="flex min-h-screen items-center justify-center bg-[#09090B] px-4">
      <div className="rounded-2xl border border-white/[0.06] bg-[#111113] p-10 flex flex-col items-center gap-4">
        <Loader2 className="h-8 w-8 animate-spin text-[#00FF87]" />
        <p className="text-sm text-white/40">{tCommon("loading")}</p>
      </div>
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
