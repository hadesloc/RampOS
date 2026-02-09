"use client";

import { useState } from "react";
import { useRouter } from "@/navigation";
import { useTranslations } from "next-intl";

export default function AdminLoginPage() {
  const [key, setKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
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
        body: JSON.stringify({ key }),
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
    <div className="flex min-h-screen items-center justify-center p-6">
      <form
        onSubmit={onSubmit}
        className="w-full max-w-sm space-y-4 rounded border border-border bg-card p-6 shadow-sm"
      >
        <h1 className="text-xl font-semibold text-foreground">{t('login_title')}</h1>
        <p className="text-sm text-muted-foreground">
          {t('login_description')}
        </p>
        <input
          className="w-full rounded border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground"
          type="password"
          value={key}
          onChange={(e) => setKey(e.target.value)}
          placeholder={t('admin_key_placeholder')}
          required
        />
        {error && <p className="text-sm text-red-600 dark:text-red-400">{error}</p>}
        <button
          className="w-full rounded bg-primary px-3 py-2 text-sm font-semibold text-primary-foreground disabled:opacity-60"
          type="submit"
          disabled={submitting}
        >
          {submitting ? t('signing_in') : t('sign_in')}
        </button>
      </form>
    </div>
  );
}
