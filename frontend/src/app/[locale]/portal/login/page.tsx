"use client";

import { FormEvent, useEffect, useState } from "react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/contexts/auth-context";
import { useRouter, Link } from "@/navigation";
import {
  AlertCircle,
  Loader2,
  LockKeyhole,
  Mail,
  Shield,
  Wallet,
} from "lucide-react";
import { useTranslations } from "next-intl";

export default function LoginPage() {
  const t = useTranslations("Portal.auth.login");
  const {
    error,
    isAuthenticated,
    isLoading,
    loginWithPassword,
    loginWithWallet,
    clearError,
  } = useAuth();
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  useEffect(() => {
    if (isAuthenticated) {
      router.push("/portal");
    }
  }, [isAuthenticated, router]);

  const handlePasswordLogin = async (event: FormEvent) => {
    event.preventDefault();
    clearError();
    try {
      await loginWithPassword(email, password);
    } catch {
      // Auth context exposes the user-facing error.
    }
  };

  const handleConnectWallet = async () => {
    clearError();
    try {
      await loginWithWallet();
    } catch {
      // Auth context exposes the user-facing error.
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-[#09090B] px-4 py-8">
      <div className="w-full max-w-md">
        <div className="mb-8 flex flex-col items-center">
          <div className="mb-5 flex h-14 w-14 items-center justify-center rounded-lg border border-[#00FF87]/20 bg-[#111113] shadow-[0_0_24px_rgba(0,255,135,0.15)]">
            <Shield className="h-7 w-7 text-[#00FF87]" />
          </div>
          <h1 className="text-2xl font-bold tracking-normal text-white">
            {t("title")}
          </h1>
          <p className="mt-1.5 text-sm text-white/40">{t("subtitle")}</p>
        </div>

        <div className="space-y-5 rounded-lg border border-white/[0.06] bg-[#111113] p-6 shadow-xl">
          {error && (
            <Alert className="border-red-500/20 bg-red-500/10 text-red-400">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription className="text-red-400">{error}</AlertDescription>
            </Alert>
          )}

          <form className="space-y-4" onSubmit={handlePasswordLogin}>
            <div className="space-y-2">
              <Label htmlFor="email" className="text-white/70">
                {t("email_input_label")}
              </Label>
              <div className="relative">
                <Mail className="pointer-events-none absolute left-3 top-3 h-4 w-4 text-white/30" />
                <Input
                  id="email"
                  type="email"
                  autoComplete="email"
                  required
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  className="h-10 border-white/10 bg-[#09090B] pl-10 text-white"
                  placeholder="m@example.com"
                />
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="password" className="text-white/70">
                {t("password_label")}
              </Label>
              <div className="relative">
                <LockKeyhole className="pointer-events-none absolute left-3 top-3 h-4 w-4 text-white/30" />
                <Input
                  id="password"
                  type="password"
                  autoComplete="current-password"
                  required
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  className="h-10 border-white/10 bg-[#09090B] pl-10 text-white"
                />
              </div>
            </div>

            <Button
              type="submit"
              className="h-11 w-full bg-[#00FF87] font-semibold text-[#09090B] hover:bg-[#00FF87]/90"
              disabled={isLoading}
            >
              {isLoading ? <Loader2 className="h-4 w-4 animate-spin" /> : t("sign_in")}
            </Button>
          </form>

          <div className="flex items-center gap-3">
            <div className="h-px flex-1 bg-white/[0.06]" />
            <span className="text-xs text-white/25">{t("or")}</span>
            <div className="h-px flex-1 bg-white/[0.06]" />
          </div>

          <Button
            type="button"
            variant="outline"
            className="h-11 w-full gap-2 border-white/10 bg-transparent text-white hover:bg-white/[0.04]"
            onClick={handleConnectWallet}
            disabled={isLoading}
          >
            <Wallet className="h-4 w-4" />
            {t("connect_wallet")}
          </Button>

          <p className="text-center text-sm text-white/40">
            {t("no_account")}{" "}
            <Link
              href="/portal/register"
              className="text-[#7B61FF] transition-colors hover:text-[#7B61FF]/80"
            >
              {t("create_account")}
            </Link>
          </p>
        </div>
      </div>
    </div>
  );
}
