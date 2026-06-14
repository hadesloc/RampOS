"use client";

import { useState, useEffect } from "react";
import { tenantsApi, type Tenant } from "@/lib/api";
import { Key, Save, RotateCcw, Zap, Webhook } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import { useTranslations } from "next-intl";
import {
  PageHeader,
  Panel,
  EmptyState,
  ErrorState,
} from "@/components/shared";

export default function SettingsPage() {
  const [tenant, setTenant] = useState<Tenant | null>(null);
  const [tenantScopeError, setTenantScopeError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [webhookSecret, setWebhookSecret] = useState("whsec_*****************************");
  const [enabledEvents, setEnabledEvents] = useState<Record<string, boolean>>({
    "intent.payin.created": true,
    "intent.payin.confirmed": true,
    "intent.payout.created": true,
    "intent.payout.completed": true,
    "intent.trade.executed": true,
    "case.created": true,
    "case.resolved": true,
  });
  const { toast } = useToast();
  const t = useTranslations("Navigation");
  const tCommon = useTranslations("Common");

  const [settings, setSettings] = useState({
    webhookUrl: "",
    rateLimit: "100",
    minPayin: "10000",
    maxPayin: "500000000",
    minPayout: "50000",
    maxPayout: "200000000",
  });

  useEffect(() => {
    const fetchTenant = async () => {
      setLoading(true);
      try {
        const tenants = await tenantsApi.list();
        if (tenants.length === 1) {
          const currentTenant = tenants[0];
          setTenantScopeError(null);
          setTenant(currentTenant);
          setApiKey(currentTenant.api_key_prefix + "*****************************");
          const config = currentTenant.config || {};
          setSettings({
            webhookUrl: (config.webhook_url as string) || "",
            rateLimit: (config.rate_limit as string) || "100",
            minPayin: (config.min_payin as string) || "10000",
            maxPayin: (config.max_payin as string) || "500000000",
            minPayout: (config.min_payout as string) || "50000",
            maxPayout: (config.max_payout as string) || "200000000",
          });
        } else if (tenants.length > 1) {
          setTenant(null);
          setTenantScopeError(
            "Settings require a single server-resolved tenant. Refine tenant scoping before editing secrets or config.",
          );
        }
      } catch (err: any) {
        console.error("Failed to fetch tenant settings:", err);
        toast({
          variant: "destructive",
          title: tCommon("error"),
          description: "Failed to load settings",
        });
      } finally {
        setLoading(false);
      }
    };
    fetchTenant();
  }, [toast, tCommon]);

  const handleSave = async () => {
    if (!tenant) return;
    setSaving(true);
    try {
      await tenantsApi.updateConfig(tenant.id, {
        webhook_url: settings.webhookUrl,
        rate_limit: settings.rateLimit,
        min_payin: settings.minPayin,
        max_payin: settings.maxPayin,
        min_payout: settings.minPayout,
        max_payout: settings.maxPayout,
        enabled_events: enabledEvents,
      });
      toast({ title: tCommon("success"), description: "Settings saved successfully!" });
    } catch (err: any) {
      toast({
        variant: "destructive",
        title: tCommon("error"),
        description: err.message || "Failed to save settings",
      });
    } finally {
      setSaving(false);
    }
  };

  const handleRegenerateKey = async () => {
    if (!tenant) return;
    try {
      const result = await tenantsApi.regenerateKeys(tenant.id);
      setApiKey(result.api_key);
      toast({ title: tCommon("success"), description: "API Key regenerated. Copy it now, you won't see it again!" });
      setTimeout(() => {
        setApiKey(result.api_key.substring(0, 8) + "*****************************");
      }, 3000);
    } catch (err: any) {
      toast({ variant: "destructive", title: tCommon("error"), description: "Failed to regenerate API Key" });
    }
  };

  const handleRegenerateWebhookSecret = async () => {
    if (!tenant) return;
    try {
      const result = await tenantsApi.regenerateWebhookSecret(tenant.id);
      setWebhookSecret(result.webhook_secret);
      toast({ title: tCommon("success"), description: "Webhook secret regenerated. Copy it now!" });
      setTimeout(() => {
        setWebhookSecret("whsec_*****************************");
      }, 3000);
    } catch (err: any) {
      toast({
        variant: "destructive",
        title: tCommon("error"),
        description: err.message || "Failed to regenerate webhook secret",
      });
    }
  };

  const inputCls =
    "w-full rounded-md border border-white/[0.08] bg-[#09090B] px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-[#00FF87]/40 transition-colors";

  if (!loading && tenantScopeError) {
    return (
      <main className="p-page flex flex-col gap-section">
        <PageHeader title={t("settings")} description="Configure your RampOS tenant settings" />
        <ErrorState message={tenantScopeError} />
      </main>
    );
  }

  if (!loading && !tenant) {
    return (
      <main className="p-page flex flex-col gap-section">
        <PageHeader title={t("settings")} description="Configure your RampOS tenant settings" />
        <EmptyState title="No tenant found" description="No tenant configuration could be resolved." />
      </main>
    );
  }

  return (
    <main className="p-page flex flex-col gap-section max-w-2xl">
      <PageHeader
        title={t("settings")}
        description="Configure your RampOS tenant settings"
        actions={
          <Button onClick={handleSave} disabled={saving || loading}>
            {saving ? (
              <>
                <RotateCcw className="mr-2 h-4 w-4 animate-spin" />
                Saving...
              </>
            ) : (
              <>
                <Save className="mr-2 h-4 w-4" />
                {tCommon("save")}
              </>
            )}
          </Button>
        }
      />

      {/* API Configuration */}
      <Panel
        header={{ title: "API Configuration", description: "Manage your tenant API key and webhook secret." }}
        variant="glass"
      >
        <div className="space-y-5">
          <div className="space-y-2">
            <label className="text-sm font-medium text-foreground flex items-center gap-2">
              <Key className="h-3.5 w-3.5 text-[#00FF87]" />
              API Key
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                className={`${inputCls} flex-1 font-mono`}
                value={apiKey}
                readOnly
              />
              <Button variant="outline" onClick={handleRegenerateKey} disabled={loading}>
                Regenerate
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">Use this key to authenticate API requests. Keep it secret.</p>
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium text-foreground flex items-center gap-2">
              <Key className="h-3.5 w-3.5 text-[#7B61FF]" />
              Webhook Secret
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                className={`${inputCls} flex-1 font-mono`}
                value={webhookSecret}
                readOnly
              />
              <Button variant="outline" onClick={handleRegenerateWebhookSecret} disabled={loading}>
                Regenerate
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">Use this secret to verify webhook signatures.</p>
          </div>
        </div>
      </Panel>

      {/* Webhook Configuration */}
      <Panel
        header={{ title: "Webhook Configuration", description: "Set your endpoint URL and choose which events to receive." }}
        variant="glass"
      >
        <div className="space-y-5">
          <div className="space-y-2">
            <label className="text-sm font-medium text-foreground flex items-center gap-2">
              <Webhook className="h-3.5 w-3.5 text-[#00D4FF]" />
              Webhook URL
            </label>
            <input
              type="url"
              className={inputCls}
              value={settings.webhookUrl}
              onChange={(e) => setSettings({ ...settings, webhookUrl: e.target.value })}
              placeholder="https://your-server.com/webhooks"
            />
            <p className="text-xs text-muted-foreground">Webhook events will be sent to this URL.</p>
          </div>

          <div className="space-y-3">
            <label className="text-sm font-medium text-foreground">Enabled Events</label>
            <div className="grid gap-2 sm:grid-cols-2">
              {[
                "intent.payin.created",
                "intent.payin.confirmed",
                "intent.payout.created",
                "intent.payout.completed",
                "intent.trade.executed",
                "case.created",
                "case.resolved",
              ].map((event) => (
                <label key={event} className="flex items-center gap-2.5 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={enabledEvents[event] ?? false}
                    onChange={(e) => setEnabledEvents((prev) => ({ ...prev, [event]: e.target.checked }))}
                    className="rounded border-white/20 bg-[#09090B] accent-[#00FF87]"
                  />
                  <span className="text-sm font-mono text-muted-foreground">{event}</span>
                </label>
              ))}
            </div>
          </div>
        </div>
      </Panel>

      {/* Rate Limiting */}
      <Panel
        header={{ title: "Rate Limiting", description: "Maximum API requests allowed per minute." }}
        variant="glass"
      >
        <div className="space-y-2">
          <label className="text-sm font-medium text-foreground flex items-center gap-2">
            <Zap className="h-3.5 w-3.5 text-[#FFB800]" />
            Requests per minute
          </label>
          <input
            type="number"
            className={inputCls}
            value={settings.rateLimit}
            onChange={(e) => setSettings({ ...settings, rateLimit: e.target.value })}
            min="10"
            max="1000"
          />
          <p className="text-xs text-muted-foreground">Allowed range: 10–1000 requests per minute.</p>
        </div>
      </Panel>

      {/* Transaction Limits */}
      <Panel
        header={{ title: "Default Transaction Limits", description: "Min/max payin and payout amounts in VND." }}
        variant="glass"
      >
        <div>
          <div className="grid gap-4 sm:grid-cols-2">
            {[
              { label: "Min Payin (VND)", key: "minPayin" as const },
              { label: "Max Payin (VND)", key: "maxPayin" as const },
              { label: "Min Payout (VND)", key: "minPayout" as const },
              { label: "Max Payout (VND)", key: "maxPayout" as const },
            ].map(({ label, key }) => (
              <div key={key} className="space-y-2">
                <label className="text-sm font-medium text-foreground">{label}</label>
                <input
                  type="number"
                  className={inputCls}
                  value={settings[key]}
                  onChange={(e) => setSettings({ ...settings, [key]: e.target.value })}
                />
              </div>
            ))}
          </div>
        </div>
      </Panel>
    </main>
  );
}
