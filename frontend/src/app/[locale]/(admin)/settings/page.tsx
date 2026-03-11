"use client";

import { useState, useEffect } from "react";
import { tenantsApi, type Tenant } from "@/lib/api";
import { Loader2, Save } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import { useTranslations } from "next-intl";

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
  const t = useTranslations('Navigation');
  const tCommon = useTranslations('Common');

  const [settings, setSettings] = useState({
    webhookUrl: "",
    rateLimit: "100",
    minPayin: "10000",
    maxPayin: "500000000",
    minPayout: "50000",
    maxPayout: "200000000",
  });

  useEffect(() => {
    // In a real app we would know the current tenant ID from context/auth
    // For now, let's assume we list tenants and pick the first one or a specific one
    const fetchTenant = async () => {
      setLoading(true);
      try {
        const tenants = await tenantsApi.list();
        if (tenants.length === 1) {
          const currentTenant = tenants[0];
          setTenantScopeError(null);
          setTenant(currentTenant);
          setApiKey(currentTenant.api_key_prefix + "*****************************");

          // Populate settings from tenant config
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
            title: tCommon('error'),
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

      toast({
        title: tCommon('success'),
        description: "Settings saved successfully!",
      });
    } catch (err: any) {
      toast({
        variant: "destructive",
        title: tCommon('error'),
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
        setApiKey(result.api_key); // Show full key once
        toast({
            title: tCommon('success'),
            description: "API Key regenerated. Copy it now, you won't see it again!",
        });
        setTimeout(() => {
            setApiKey(result.api_key.substring(0, 8) + "*****************************");
        }, 3000);
    } catch (err: any) {
        toast({
            variant: "destructive",
            title: tCommon('error'),
            description: "Failed to regenerate API Key",
        });
    }
  };

  const handleRegenerateWebhookSecret = async () => {
    if (!tenant) return;
    try {
      const result = await tenantsApi.regenerateWebhookSecret(tenant.id);
      setWebhookSecret(result.webhook_secret);
      toast({
        title: tCommon('success'),
        description: "Webhook secret regenerated. Copy it now!",
      });
      setTimeout(() => {
        setWebhookSecret("whsec_*****************************");
      }, 3000);
    } catch (err: any) {
      toast({
        variant: "destructive",
        title: tCommon('error'),
        description: err.message || "Failed to regenerate webhook secret",
      });
    }
  };

  if (loading) {
      return (
          <div className="flex justify-center items-center h-64">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
      );
  }

  if (!tenant) {
      return (
          <div className="text-center py-8 text-muted-foreground">
              {tenantScopeError ?? "No tenant configuration found."}
          </div>
      );
  }

  return (
    <div className="space-y-6 max-w-2xl">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">{t('settings')}</h1>
        <p className="text-muted-foreground">
          Configure your RampOS tenant settings
        </p>
      </div>

      {/* API Configuration */}
      <div className="rounded-lg border bg-card p-6 space-y-4">
        <h2 className="text-lg font-semibold">API Configuration</h2>

        <div className="space-y-2">
          <label className="text-sm font-medium">API Key</label>
          <div className="flex gap-2">
            <input
              type="text"
              className="flex-1 rounded-md border bg-background px-3 py-2 text-sm font-mono bg-muted"
              value={apiKey}
              readOnly
            />
            <button
                onClick={handleRegenerateKey}
                className="px-4 py-2 text-sm border rounded-md hover:bg-muted bg-background"
            >
              Regenerate
            </button>
          </div>
          <p className="text-xs text-muted-foreground">
            Use this key to authenticate API requests. Keep it secret!
          </p>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Webhook Secret</label>
          <div className="flex gap-2">
            <input
              type="text"
              className="flex-1 rounded-md border bg-background px-3 py-2 text-sm font-mono bg-muted"
              value={webhookSecret}
              readOnly
            />
            <button
              onClick={handleRegenerateWebhookSecret}
              className="px-4 py-2 text-sm border rounded-md hover:bg-muted bg-background"
            >
              Regenerate
            </button>
          </div>
          <p className="text-xs text-muted-foreground">
            Use this secret to verify webhook signatures.
          </p>
        </div>
      </div>

      {/* Webhook Configuration */}
      <div className="rounded-lg border bg-card p-6 space-y-4">
        <h2 className="text-lg font-semibold">Webhook Configuration</h2>

        <div className="space-y-2">
          <label className="text-sm font-medium">Webhook URL</label>
          <input
            type="url"
            className="w-full rounded-md border bg-background px-3 py-2 text-sm"
            value={settings.webhookUrl}
            onChange={(e) => setSettings({ ...settings, webhookUrl: e.target.value })}
            placeholder="https://your-server.com/webhooks"
          />
          <p className="text-xs text-muted-foreground">
            We will send webhook events to this URL.
          </p>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Enabled Events</label>
          <div className="space-y-2">
            {[
              "intent.payin.created",
              "intent.payin.confirmed",
              "intent.payout.created",
              "intent.payout.completed",
              "intent.trade.executed",
              "case.created",
              "case.resolved",
            ].map((event) => (
              <label key={event} className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={enabledEvents[event] ?? false}
                  onChange={(e) => setEnabledEvents(prev => ({ ...prev, [event]: e.target.checked }))}
                  className="rounded"
                />
                <span className="text-sm font-mono">{event}</span>
              </label>
            ))}
          </div>
        </div>
      </div>

      {/* Rate Limiting */}
      <div className="rounded-lg border bg-card p-6 space-y-4">
        <h2 className="text-lg font-semibold">Rate Limiting</h2>

        <div className="space-y-2">
          <label className="text-sm font-medium">Requests per minute</label>
          <input
            type="number"
            className="w-full rounded-md border bg-background px-3 py-2 text-sm"
            value={settings.rateLimit}
            onChange={(e) => setSettings({ ...settings, rateLimit: e.target.value })}
            min="10"
            max="1000"
          />
          <p className="text-xs text-muted-foreground">
            Maximum API requests allowed per minute (10-1000).
          </p>
        </div>
      </div>

      {/* Transaction Limits */}
      <div className="rounded-lg border bg-card p-6 space-y-4">
        <h2 className="text-lg font-semibold">Default Transaction Limits</h2>

        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <label className="text-sm font-medium">Min Payin (VND)</label>
            <input
              type="number"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={settings.minPayin}
              onChange={(e) => setSettings({ ...settings, minPayin: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Max Payin (VND)</label>
            <input
              type="number"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={settings.maxPayin}
              onChange={(e) => setSettings({ ...settings, maxPayin: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Min Payout (VND)</label>
            <input
              type="number"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={settings.minPayout}
              onChange={(e) => setSettings({ ...settings, minPayout: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Max Payout (VND)</label>
            <input
              type="number"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={settings.maxPayout}
              onChange={(e) => setSettings({ ...settings, maxPayout: e.target.value })}
            />
          </div>
        </div>
      </div>

      {/* Save Button */}
      <div className="flex justify-end">
        <Button
            onClick={handleSave}
            disabled={saving}
            className="px-6 py-2"
        >
          {saving ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Saving...
              </>
          ) : (
              <>
                <Save className="mr-2 h-4 w-4" />
                {tCommon('save')}
              </>
          )}
        </Button>
      </div>
    </div>
  );
}
