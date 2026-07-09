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
  UserRound,
} from "lucide-react";
import { useTranslations } from "next-intl";

export default function RegisterPage() {
  const t = useTranslations("Portal.auth.register");
  const { error, isAuthenticated, isLoading, registerWithPassword, clearError } =
    useAuth();
  const router = useRouter();
  const [fullName, setFullName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  useEffect(() => {
    if (isAuthenticated) {
      router.push("/portal");
    }
  }, [isAuthenticated, router]);

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    clearError();
    try {
      await registerWithPassword(email, password, fullName || undefined);
    } catch {
      // Auth context exposes the user-facing error.
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-[#09090B] px-4 py-8">
      <div className="w-full max-w-md">
        <div className="mb-8 flex flex-col items-center">
          <div className="mb-5 flex h-14 w-14 items-center justify-center rounded-lg border border-[#7B61FF]/20 bg-[#111113] shadow-[0_0_24px_rgba(123,97,255,0.15)]">
            <Shield className="h-7 w-7 text-[#7B61FF]" />
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

          <form className="space-y-4" onSubmit={handleSubmit}>
            <div className="space-y-2">
              <Label htmlFor="fullName" className="text-white/70">
                {t("name_label")}
              </Label>
              <div className="relative">
                <UserRound className="pointer-events-none absolute left-3 top-3 h-4 w-4 text-white/30" />
                <Input
                  id="fullName"
                  autoComplete="name"
                  value={fullName}
                  onChange={(event) => setFullName(event.target.value)}
                  className="h-10 border-white/10 bg-[#09090B] pl-10 text-white"
                />
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="email" className="text-white/70">
                {t("email_label")}
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
                  autoComplete="new-password"
                  minLength={12}
                  maxLength={128}
                  required
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  className="h-10 border-white/10 bg-[#09090B] pl-10 text-white"
                />
              </div>
              <p className="text-xs text-white/30">{t("password_hint")}</p>
            </div>

            <Button
              type="submit"
              className="h-11 w-full bg-[#7B61FF] font-semibold text-white hover:bg-[#7B61FF]/90"
              disabled={isLoading}
            >
              {isLoading ? <Loader2 className="h-4 w-4 animate-spin" /> : t("create_btn")}
            </Button>
          </form>

          <p className="text-center text-sm text-white/40">
            {t("already_have_account")}{" "}
            <Link
              href="/portal/login"
              className="text-[#00FF87] transition-colors hover:text-[#00FF87]/80"
            >
              {t("sign_in")}
            </Link>
          </p>
        </div>
      </div>
    </div>
  );
}
