"use client";

import { useEffect } from "react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { useAuth } from "@/contexts/auth-context";
import { useRouter, Link } from "@/navigation";
import { AlertCircle, Shield, Lock } from "lucide-react";
import { useTranslations } from "next-intl";

export default function RegisterPage() {
  const t = useTranslations("Portal.auth.register");

  const { error, isAuthenticated } = useAuth();

  const router = useRouter();
  const portalRegistrationUnavailable =
    "Portal registration is not available in this environment. Passkey completion and session issuance are still disabled on the backend.";

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
          <div className="h-14 w-14 rounded-2xl bg-[#111113] border border-[#7B61FF]/20 flex items-center justify-center shadow-[0_0_24px_rgba(123,97,255,0.15)] mb-5">
            <Shield className="h-7 w-7 text-[#7B61FF]" />
          </div>
          <h1 className="text-2xl font-bold tracking-tight text-white">
            {t("title")}
          </h1>
          <p className="mt-1.5 text-sm text-white/40">{t("subtitle")}</p>
        </div>

        {/* Card */}
        <div className="rounded-2xl border border-white/[0.06] bg-[#111113] p-6 space-y-4 shadow-xl">
          {error && (
            <Alert className="border-red-500/20 bg-red-500/10 text-red-400">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription className="text-red-400">{error}</AlertDescription>
            </Alert>
          )}

          <div className="flex items-start gap-3 rounded-lg border border-[#FFB800]/20 bg-[#FFB800]/[0.06] p-4">
            <Lock className="h-4 w-4 mt-0.5 shrink-0 text-[#FFB800]" />
            <p className="text-sm text-[#FFB800]/80">
              {portalRegistrationUnavailable}
            </p>
          </div>

          {/* Footer links */}
          <div className="pt-2 space-y-3 text-center">
            <p className="text-sm text-white/40">
              {t("already_have_account")}{" "}
              <Link
                href="/portal/login"
                className="text-[#00FF87] hover:text-[#00FF87]/80 transition-colors"
              >
                {t("sign_in")}
              </Link>
            </p>
            <p className="text-xs text-white/20">
              Portal access currently requires a pre-issued Bearer JWT rather than self-serve registration.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
