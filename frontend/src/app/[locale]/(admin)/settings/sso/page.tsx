"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { AlertCircle, Lock, Loader2, Plus } from "lucide-react";
import { toast } from "@/components/ui/use-toast";
import { api, SsoProvider } from "@/lib/api";
import {
  PageHeader,
  Panel,
  SectionCard,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";
import type { StatusSeverity } from "@/components/shared";

export default function SSOPage() {
  const [providers, setProviders] = useState<SsoProvider[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);

  const fetchProviders = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await api.sso.listProviders();
      setProviders(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load SSO configuration.";
      setError(message);
      toast({ title: "Error", description: message, variant: "destructive" });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchProviders();
  }, []);

  const handleConfigureProvider = (provider: string) =>
    toast({ title: "Configuration Required", description: `${provider} configuration wizard coming soon.` });

  const handleToggleProvider = async (provider: string, enabled: boolean) => {
    try {
      setToggling(provider);
      await api.sso.toggle(provider, enabled);
      setProviders((prev) => prev.map((p) => (p.provider === provider ? { ...p, enabled } : p)));
      toast({
        title: enabled ? "Provider Enabled" : "Provider Disabled",
        description: `${provider} SSO has been ${enabled ? "enabled" : "disabled"}.`,
      });
    } catch {
      toast({ title: "Error", description: "Failed to update provider status.", variant: "destructive" });
    } finally {
      setToggling(null);
    }
  };

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Single Sign-On (SSO)"
        description="Manage enterprise identity providers and authentication policies."
      />

      {/* Enterprise notice */}
      <SectionCard
        header={{
          title: "Enterprise Feature",
          actions: <Lock className="h-4 w-4 text-[#00D4FF]" />,
        }}
        className="border-[#00D4FF]/20 bg-[#00D4FF]/5"
      >
        <p className="text-sm text-muted-foreground">
          SSO is enabled for your organization. You can configure multiple identity providers.
        </p>
      </SectionCard>

      {loading ? (
        <div className="flex justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      ) : error ? (
        <ErrorState message={error} retry={fetchProviders} />
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          {providers.length === 0 ? (
            <div className="md:col-span-2">
              <EmptyState
                title="No SSO providers"
                description="No identity providers are configured for this tenant."
              />
            </div>
          ) : (
            providers.map((p) => (
              <Panel
                key={p.provider}
                variant="glass"
                contentClassName="p-4"
                header={{
                  title: p.name || p.provider,
                  description: `${p.provider.toUpperCase()} Integration`,
                  actions: (
                    <StatusBadge
                      status={p.enabled ? "Active" : "Disabled"}
                      severity={p.enabled ? "success" : "neutral"}
                    />
                  ),
                }}
              >
                <div className="space-y-4">
                  {p.enabled ? (
                    <div className="space-y-2 text-sm">
                      <div className="flex items-center justify-between border-b border-white/[0.06] pb-2">
                        <span className="text-muted-foreground">Domain</span>
                        <span className="font-mono">{p.config?.domain || "N/A"}</span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-muted-foreground">Client ID</span>
                        <span className="font-mono">
                          {p.config?.client_id ? "••••••••" : "Not configured"}
                        </span>
                      </div>
                    </div>
                  ) : (
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <AlertCircle className="h-4 w-4 text-[#FFB800]" />
                      Connect your {p.name} tenant to enable login.
                    </div>
                  )}

                  <div className="flex items-center justify-between pt-2 border-t border-white/[0.06]">
                    <Button variant="outline" size="sm" onClick={() => handleConfigureProvider(p.provider)}>
                      Configure
                    </Button>
                    <div className="flex items-center gap-2">
                      <Label htmlFor={`${p.provider}-enabled`} className="text-sm text-muted-foreground">
                        Enabled
                      </Label>
                      <Switch
                        id={`${p.provider}-enabled`}
                        checked={p.enabled}
                        onCheckedChange={(checked) => handleToggleProvider(p.provider, checked)}
                        disabled={toggling === p.provider}
                      />
                    </div>
                  </div>
                </div>
              </Panel>
            ))
          )}

          {/* Add provider card */}
          <div
            className="rounded-lg border border-dashed border-white/[0.08] bg-white/[0.01] flex flex-col items-center justify-center p-6 min-h-[180px] cursor-pointer hover:border-[#7B61FF]/40 hover:bg-[#7B61FF]/5 transition-colors"
            onClick={() => toast({ title: "Coming Soon", description: "Custom provider wizard coming soon." })}
          >
            <div className="text-center space-y-2">
              <div className="mx-auto flex h-10 w-10 items-center justify-center rounded-full border border-white/[0.08] bg-white/[0.02]">
                <Plus className="h-5 w-5 text-muted-foreground" />
              </div>
              <h3 className="font-semibold text-sm">Add Identity Provider</h3>
              <p className="text-xs text-muted-foreground">Connect SAML or OIDC providers</p>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
