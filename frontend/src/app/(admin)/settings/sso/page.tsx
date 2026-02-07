"use client";

import { useState } from "react";
import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { AlertCircle, Lock } from "lucide-react";
import { toast } from "@/components/ui/use-toast";

export default function SSOPage() {
  const [oktaEnabled, setOktaEnabled] = useState(true);

  const handleConfigureProvider = (provider: string) => {
    toast({
      title: "Configuration Required",
      description: `${provider} configuration requires Identity Provider setup. Contact your IdP administrator.`,
    });
  };

  const handleAddProvider = () => {
    toast({
      title: "Add Identity Provider",
      description: "Provider setup wizard is not yet available. Please contact support.",
    });
  };

  const handleOktaToggle = (checked: boolean) => {
    setOktaEnabled(checked);
    toast({
      title: checked ? "Okta Enabled" : "Okta Disabled",
      description: checked
        ? "Okta SSO integration has been enabled."
        : "Okta SSO integration has been disabled.",
    });
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

      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <CardTitle>Okta</CardTitle>
                <CardDescription>OIDC Integration</CardDescription>
              </div>
              <Badge variant="default" className={oktaEnabled ? "bg-green-500 hover:bg-green-600" : "bg-gray-400 hover:bg-gray-500"}>
                {oktaEnabled ? "Active" : "Disabled"}
              </Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between py-2 border-b">
              <span className="text-sm font-medium">Domain</span>
              <span className="text-sm text-muted-foreground">rampos.okta.com</span>
            </div>
            <div className="flex items-center justify-between py-2 border-b">
              <span className="text-sm font-medium">Client ID</span>
              <span className="text-sm font-mono text-muted-foreground">0oa...123</span>
            </div>
            <div className="flex items-center justify-between py-2 border-b">
              <span className="text-sm font-medium">Last Sync</span>
              <span className="text-sm text-muted-foreground">2 mins ago</span>
            </div>
          </CardContent>
          <CardFooter className="justify-between">
            <Button variant="outline" size="sm" onClick={() => handleConfigureProvider("Okta")}>Configure</Button>
            <div className="flex items-center gap-2">
              <Label htmlFor="okta-enabled">Enabled</Label>
              <Switch id="okta-enabled" checked={oktaEnabled} onCheckedChange={handleOktaToggle} />
            </div>
          </CardFooter>
        </Card>

        <Card className="opacity-75 border-dashed">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <CardTitle>Azure AD</CardTitle>
                <CardDescription>Microsoft Entra ID</CardDescription>
              </div>
              <Badge variant="outline">Not Configured</Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
             <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <AlertCircle className="h-4 w-4" />
                Connect your Azure AD tenant to enable login with Microsoft accounts.
             </div>
          </CardContent>
          <CardFooter>
            <Button variant="outline" className="w-full" onClick={() => handleConfigureProvider("Azure AD")}>Setup Azure AD</Button>
          </CardFooter>
        </Card>

        <Card className="opacity-75 border-dashed">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <CardTitle>Google Workspace</CardTitle>
                <CardDescription>Google OAuth2</CardDescription>
              </div>
              <Badge variant="outline">Not Configured</Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <AlertCircle className="h-4 w-4" />
                Enable login with Google Workspace accounts for your domain.
             </div>
          </CardContent>
          <CardFooter>
            <Button variant="outline" className="w-full" onClick={() => handleConfigureProvider("Google Workspace")}>Setup Google</Button>
          </CardFooter>
        </Card>

        <Card className="bg-muted/50 border-dashed flex flex-col items-center justify-center p-6 h-full">
            <div className="text-center space-y-2">
                <h3 className="font-semibold text-lg">Add Identity Provider</h3>
                <p className="text-sm text-muted-foreground">Connect SAML or OIDC providers</p>
                <Button className="mt-4" onClick={handleAddProvider}>Add Provider</Button>
            </div>
        </Card>
      </div>

      <Card>
        <CardHeader>
            <CardTitle>Role Mapping</CardTitle>
            <CardDescription>Map identity provider groups to RampOS roles.</CardDescription>
        </CardHeader>
        <CardContent>
             <div className="rounded-md border">
                <div className="grid grid-cols-3 p-3 font-medium border-b bg-muted/50">
                    <div>IdP Group</div>
                    <div>RampOS Role</div>
                    <div className="text-right">Actions</div>
                </div>
                <div className="divide-y">
                    <div className="grid grid-cols-3 p-3 items-center">
                        <div className="flex items-center gap-2">
                            <Badge variant="outline">Okta</Badge>
                            <span className="font-mono text-sm">rampos-admins</span>
                        </div>
                        <div>
                            <Badge>Tenant Admin</Badge>
                        </div>
                        <div className="text-right">
                            <Button variant="ghost" size="icon" className="h-8 w-8">...</Button>
                        </div>
                    </div>
                     <div className="grid grid-cols-3 p-3 items-center">
                        <div className="flex items-center gap-2">
                            <Badge variant="outline">Okta</Badge>
                            <span className="font-mono text-sm">rampos-finance</span>
                        </div>
                        <div>
                            <Badge variant="secondary">Finance Manager</Badge>
                        </div>
                        <div className="text-right">
                            <Button variant="ghost" size="icon" className="h-8 w-8">...</Button>
                        </div>
                    </div>
                </div>
             </div>
        </CardContent>
      </Card>
    </div>
  );
}
