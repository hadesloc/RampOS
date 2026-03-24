"use client";

import { useState } from "react";
import { useRouter } from "@/navigation";
import { useTranslations } from "next-intl";
import { ArrowRight, Eye, EyeOff, Shield } from "lucide-react";

export default function AdminLoginPage() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const router = useRouter();
  const t = useTranslations('Auth');

  async function onSubmit(event: React.FormEvent) {
    event.preventDefault();
    setError(null);
    setSubmitting(true);

    try {
      const csrfRes = await fetch("/api/csrf");
      const csrfPayload = await csrfRes.json();
      const csrfToken =
        csrfPayload && typeof csrfPayload.token === "string"
          ? csrfPayload.token
          : "";

      if (!csrfToken) {
        setError(t('login_failed'));
        return;
      }

      const res = await fetch("/api/admin-login", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-csrf-token": csrfToken,
        },
        body: JSON.stringify({ email, password }),
      });

      if (!res.ok) {
        setError(t('invalid_key'));
        return;
      }

      router.push("/");
    } catch {
      setError(t('login_failed'));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="relative flex min-h-screen items-center justify-center overflow-hidden bg-[#050505]">
      {/* Aurora Gradient Orbs */}
      <div className="pointer-events-none absolute inset-0 overflow-hidden">
        <div className="absolute -left-[20%] -top-[20%] h-[60%] w-[60%] rounded-full bg-[#00FF87]/[0.07] blur-[180px] animate-pulse" style={{ animationDuration: '8s' }} />
        <div className="absolute -bottom-[20%] -right-[20%] h-[60%] w-[60%] rounded-full bg-[#7B61FF]/[0.07] blur-[180px] animate-pulse" style={{ animationDuration: '12s' }} />
        <div className="absolute left-[30%] top-[20%] h-[30%] w-[30%] rounded-full bg-[#00D4FF]/[0.04] blur-[150px] animate-pulse" style={{ animationDuration: '16s' }} />
      </div>

      {/* Perspective Grid */}
      <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.02)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.02)_1px,transparent_1px)] bg-[size:60px_60px] [mask-image:radial-gradient(ellipse_50%_50%_at_50%_50%,#000_20%,transparent_100%)]" />

      {/* Login Card */}
      <form
        onSubmit={onSubmit}
        className="relative z-10 w-full max-w-md space-y-6 rounded-2xl border border-white/[0.08] bg-[#111113]/80 p-10 backdrop-blur-xl shadow-[0_0_80px_rgba(0,255,135,0.04)]"
      >
        {/* Logo */}
        <div className="flex flex-col items-center gap-4 pb-2">
          <div className="flex h-14 w-14 items-center justify-center rounded-2xl bg-[#00FF87]/10 border border-[#00FF87]/20">
            <Shield className="h-7 w-7 text-[#00FF87]" />
          </div>
          <div className="flex items-center gap-2">
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[#00FF87] opacity-75" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-[#00FF87]" />
            </span>
            <span className="text-xl font-bold tracking-tight text-white" style={{ fontFamily: 'var(--font-sans)' }}>RAMP·OS</span>
          </div>
        </div>

        {/* Header */}
        <div className="text-center">
          <h1 className="text-2xl font-bold text-white tracking-tight">{t('login_title')}</h1>
          <p className="mt-2 text-sm text-gray-500">
            {t('login_description')}
          </p>
        </div>

        {/* Email */}
        <div className="space-y-2">
          <label className="text-xs font-medium uppercase tracking-[0.1em] text-gray-500">Email</label>
          <input
            className="w-full rounded-xl border border-white/[0.08] bg-white/[0.03] px-4 py-3 text-sm text-white placeholder:text-gray-600 transition-all focus:border-[#00FF87]/40 focus:outline-none focus:ring-1 focus:ring-[#00FF87]/20 focus:shadow-[0_0_20px_rgba(0,255,135,0.05)]"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="admin@rampos.io"
            required
          />
        </div>

        {/* Password */}
        <div className="space-y-2">
          <label className="text-xs font-medium uppercase tracking-[0.1em] text-gray-500">Password</label>
          <div className="relative">
            <input
              className="w-full rounded-xl border border-white/[0.08] bg-white/[0.03] px-4 py-3 pr-12 text-sm text-white placeholder:text-gray-600 transition-all focus:border-[#00FF87]/40 focus:outline-none focus:ring-1 focus:ring-[#00FF87]/20 focus:shadow-[0_0_20px_rgba(0,255,135,0.05)]"
              type={showPassword ? "text" : "password"}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="••••••••••••"
              required
            />
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-600 hover:text-gray-400 transition-colors"
            >
              {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </div>
        </div>

        {/* Error */}
        {error && (
          <div className="rounded-lg bg-red-500/10 border border-red-500/20 px-4 py-3 text-sm text-red-400">
            {error}
          </div>
        )}

        {/* Submit */}
        <button
          className="group w-full rounded-xl bg-[#00FF87] px-6 py-3.5 text-sm font-bold text-black transition-all hover:shadow-[0_0_40px_rgba(0,255,135,0.3)] hover:scale-[1.02] active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          type="submit"
          disabled={submitting}
        >
          {submitting ? (
            <>
              <div className="h-4 w-4 animate-spin rounded-full border-2 border-black/20 border-t-black" />
              {t('signing_in')}
            </>
          ) : (
            <>
              {t('sign_in')}
              <ArrowRight className="h-4 w-4 group-hover:translate-x-0.5 transition-transform" />
            </>
          )}
        </button>

        {/* Footer */}
        <p className="text-center text-xs text-gray-600">
          Secured by RampOS Auth Engine · 256-bit AES
        </p>
      </form>
    </div>
  );
}
