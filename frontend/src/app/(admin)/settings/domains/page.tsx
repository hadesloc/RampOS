"use client";

import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Globe, Plus, RefreshCw, ShieldCheck, Trash2 } from "lucide-react";
import { toast } from "@/components/ui/use-toast";

export default function DomainsPage() {
  const handleAddDomain = () => {
    const domain = window.prompt("Enter the domain you want to add (e.g., pay.yourcompany.com):");
    if (domain) {
      toast({
        title: "Domain Added",
        description: `${domain} has been added. Please configure DNS to point to cname.rampos.io`,
      });
    }
  };

  const handleCopyCname = () => {
    navigator.clipboard.writeText("cname.rampos.io").then(() => {
      toast({
        title: "Copied",
        description: "CNAME value copied to clipboard.",
      });
    });
  };

  const handleCheckStatus = (domain: string) => {
    toast({
      title: "Checking Status",
      description: `Verifying DNS and SSL for ${domain}...`,
    });
  };

  const handleVerifyDns = (domain: string) => {
    toast({
      title: "DNS Verification",
      description: `DNS verification initiated for ${domain}. This may take a few minutes.`,
    });
  };

  const handleDeleteDomain = (domain: string) => {
    if (window.confirm(`Are you sure you want to remove ${domain}? This action cannot be undone.`)) {
      toast({
        title: "Domain Removed",
        description: `${domain} has been removed from your tenant.`,
      });
    }
  };

  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader
        title="Custom Domains"
        description="Connect your own domains to your RampOS instance with automatic SSL."
      />

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
                <CardTitle>Connected Domains</CardTitle>
                <CardDescription>Domains that resolve to this tenant.</CardDescription>
            </div>
            <Button onClick={handleAddDomain}>
                <Plus className="h-4 w-4 mr-2" />
                Add Domain
            </Button>
          </div>
        </CardHeader>
        <CardContent>
            <div className="space-y-6">
                {/* Primary Domain */}
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 p-4 border rounded-lg bg-muted/30">
                    <div className="flex items-start gap-4">
                        <div className="mt-1 bg-primary/10 p-2 rounded-full">
                            <Globe className="h-5 w-5 text-primary" />
                        </div>
                        <div>
                            <div className="flex items-center gap-2">
                                <h3 className="font-semibold text-lg">app.rampos.io</h3>
                                <Badge>Default</Badge>
                                <Badge variant="secondary" className="text-green-600 bg-green-50 border-green-200">
                                    <ShieldCheck className="h-3 w-3 mr-1" />
                                    Secure
                                </Badge>
                            </div>
                            <p className="text-sm text-muted-foreground mt-1">System provided domain</p>
                        </div>
                    </div>
                    <div className="flex items-center gap-2">
                        <Button variant="outline" size="sm" disabled>Primary</Button>
                    </div>
                </div>

                {/* Custom Domain 1 */}
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 p-4 border rounded-lg">
                    <div className="flex items-start gap-4">
                        <div className="mt-1 bg-orange-100 p-2 rounded-full">
                            <Globe className="h-5 w-5 text-orange-600" />
                        </div>
                        <div>
                            <div className="flex items-center gap-2">
                                <h3 className="font-semibold text-lg">payments.acmecorp.com</h3>
                                <Badge variant="secondary" className="text-green-600 bg-green-50 border-green-200">
                                    <ShieldCheck className="h-3 w-3 mr-1" />
                                    Active
                                </Badge>
                            </div>
                            <div className="flex gap-4 mt-2 text-sm text-muted-foreground">
                                <span className="flex items-center gap-1">
                                    DNS: <span className="text-green-600 font-medium">Verified</span>
                                </span>
                                <span className="flex items-center gap-1">
                                    SSL: <span className="text-green-600 font-medium">Valid (Let&apos;s Encrypt)</span>
                                </span>
                            </div>
                        </div>
                    </div>
                    <div className="flex items-center gap-2">
                        <Button variant="ghost" size="sm" onClick={() => handleCheckStatus("payments.acmecorp.com")}>
                            <RefreshCw className="h-4 w-4 mr-2" />
                            Check Status
                        </Button>
                         <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10" onClick={() => handleDeleteDomain("payments.acmecorp.com")}>
                            <Trash2 className="h-4 w-4" />
                        </Button>
                    </div>
                </div>

                {/* Custom Domain 2 (Pending) */}
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 p-4 border rounded-lg border-yellow-200 bg-yellow-50/50">
                    <div className="flex items-start gap-4">
                        <div className="mt-1 bg-yellow-100 p-2 rounded-full">
                            <Globe className="h-5 w-5 text-yellow-600" />
                        </div>
                        <div>
                            <div className="flex items-center gap-2">
                                <h3 className="font-semibold text-lg">checkout.acmecorp.com</h3>
                                <Badge variant="outline" className="text-yellow-700 border-yellow-300 bg-yellow-100">
                                    Pending DNS
                                </Badge>
                            </div>
                            <p className="text-sm text-muted-foreground mt-1">
                                Add the following CNAME record to your DNS provider:
                            </p>
                            <div className="flex items-center gap-2 mt-2">
                                <code className="bg-background px-2 py-1 rounded border text-xs font-mono">
                                    cname.rampos.io
                                </code>
                                <Button variant="ghost" size="sm" onClick={handleCopyCname}>Copy</Button>
                            </div>
                        </div>
                    </div>
                    <div className="flex items-center gap-2">
                         <Button variant="default" size="sm" className="bg-yellow-600 hover:bg-yellow-700" onClick={() => handleVerifyDns("checkout.acmecorp.com")}>
                            Verify DNS
                        </Button>
                         <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10" onClick={() => handleDeleteDomain("checkout.acmecorp.com")}>
                            <Trash2 className="h-4 w-4" />
                        </Button>
                    </div>
                </div>
            </div>
        </CardContent>
      </Card>
    </div>
  );
}
