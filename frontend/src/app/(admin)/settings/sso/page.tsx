"use client";

import { useState, useEffect } from "react";
import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { AlertCircle, Lock, Loader2 } from "lucide-react";
import { toast } from "@/components/ui/use-toast";
import { api, SsoProvider } from "@/lib/api";

export default function SSOPage() {
  const [providers, setProviders] = useState<SsoProvider[]>([]);
  const [loading, setLoading] = useState(true);
  const [toggling, setToggling] = useState<string | null>(null);

  const fetchProviders = async () => {
    try {
      setLoading(true);
      const data = await api.sso.listProviders();
      setProviders(data);
    } catch (error) {
      console.error("Failed to fetch SSO providers:", error);
      // Fallback to empty if API fails, user will see empty state or we could show error
      toast({
        title: "Error",
        description: "Failed to load SSO configuration.",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchProviders();
  }, []);

  const handleConfigureProvider = (provider: string) => {
    toast({
      title: "Configuration Required",
      description: `${provider} configuration wizard coming soon.`,
    });
  };

  const handleToggleProvider = async (provider: string, enabled: boolean) => {
    try {
      setToggling(provider);
      await api.sso.toggle(provider, enabled);
      setProviders(providers.map(p =>
        p.provider === provider ? { ...p, enabled } : p
      ));
      toast({
        title: enabled ? "Provider Enabled" : "Provider Disabled",
        description: `${provider} SSO has been ${enabled ? 'enabled' : 'disabled'}.`,
      });
    } catch (error) {
      toast({
        title: "Error",
        description: "Failed to update provider status.",
        variant: "destructive",
      });
    } finally {
      setToggling(null);
    }
  };

  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader
        title="Single Sign-On (SSO)"
        description="Manage enterprise identity providers and authentication policies."
      />

      <Alert>
        <Lock className="h-4 w-4" />
        <AlertTitle>Enterprise Feature</AlertTitle>
        <AlertDescription>
          SSO is enabled for your organization. You can configure multiple identity providers.
        </AlertDescription>
      </Alert>

      {loading ? (
          <div className="flex justify-center p-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
      ) : (
          <div className="grid gap-6 md:grid-cols-2">
            {providers.length > 0 ? providers.map((p) => (
                <Card key={p.provider} className={p.enabled ? "" : "opacity-75 border-dashed"}>
                  <CardHeader>
                    <div className="flex items-center justify-between">
                      <div className="space-y-1">
                        <CardTitle>{p.name || p.provider}</CardTitle>
                        <CardDescription>{p.provider.toUpperCase()} Integration</CardDescription>
                      </div>
                      <Badge variant={p.enabled ? "default" : "outline"} className={p.enabled ? "bg-green-500 hover:bg-green-600" : ""}>
                        {p.enabled ? "Active" : "Disabled"}
                      </Badge>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    {p.enabled ? (
                        <>
                            <div className="flex items-center justify-between py-2 border-b">
                              <span className="text-sm font-medium">Domain</span>
                              <span className="text-sm text-muted-foreground">{p.config?.domain || 'N/A'}</span>
                            </div>
                            <div className="flex items-center justify-between py-2 border-b">
                              <span className="text-sm font-medium">Client ID</span>
                              <span className="text-sm font-mono text-muted-foreground">{p.config?.client_id ? '********' : 'Not configured'}</span>
                            </div>
                        </>
                    ) : (
                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                            <AlertCircle className="h-4 w-4" />
                            Connect your {p.name} tenant to enable login.
                        </div>
                    )}
                  </CardContent>
                  <CardFooter className="justify-between">
                    <Button variant="outline" size="sm" onClick={() => handleConfigureProvider(p.provider)}>Configure</Button>
                    <div className="flex items-center gap-2">
                      <Label htmlFor={`${p.provider}-enabled`}>Enabled</Label>
                      <Switch
                        id={`${p.provider}-enabled`}
                        checked={p.enabled}
                        onCheckedChange={(checked) => handleToggleProvider(p.provider, checked)}
                        disabled={toggling === p.provider}
                      />
                    </div>
                  </CardFooter>
                </Card>
            )) : (
                <div className="col-span-2 text-center p-8 text-muted-foreground">
                    No SSO providers available.
                </div>
            )}

            <Card className="bg-muted/50 border-dashed flex flex-col items-center justify-center p-6 min-h-[200px]">
                <div className="text-center space-y-2">
                    <h3 className="font-semibold text-lg">Add Identity Provider</h3>
                    <p className="text-sm text-muted-foreground">Connect SAML or OIDC providers</p>
                    <Button className="mt-4" onClick={() => toast({ title: "Coming Soon", description: "Custom provider wizard coming soon." })}>Add Provider</Button>
                </div>
            </Card>
          </div>
      )}
    </div>
  );
}
