"use client";

import { useState, useEffect } from "react";
import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Globe, Plus, ShieldCheck, Trash2, Loader2, Copy } from "lucide-react";
import { toast } from "@/components/ui/use-toast";
import { api, Domain } from "@/lib/api";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export default function DomainsPage() {
  const [domains, setDomains] = useState<Domain[]>([]);
  const [loading, setLoading] = useState(true);
  const [newDomain, setNewDomain] = useState("");
  const [isAdding, setIsAdding] = useState(false);
  const [isDialogOpen, setIsDialogOpen] = useState(false);

  const fetchDomains = async () => {
    try {
      setLoading(true);
      const data = await api.domains.list();
      setDomains(data);
    } catch (error) {
      console.error("Failed to fetch domains:", error);
      toast({
        title: "Error",
        description: "Failed to load domains.",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchDomains();
  }, []);

  const handleAddDomain = async () => {
    if (!newDomain) return;

    try {
      setIsAdding(true);
      await api.domains.create(newDomain);
      toast({
        title: "Domain Added",
        description: `${newDomain} has been added. Please configure DNS.`,
      });
      setNewDomain("");
      setIsDialogOpen(false);
      fetchDomains();
    } catch (error) {
        console.error("Failed to add domain:", error);
      toast({
        title: "Error",
        description: "Failed to add domain. It may already exist.",
        variant: "destructive",
      });
    } finally {
      setIsAdding(false);
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

  const handleVerifyDns = async (id: string, domain: string) => {
    try {
        toast({
            title: "Verifying DNS",
            description: `Checking DNS records for ${domain}...`,
        });
        await api.domains.verifyDns(id);
        toast({
            title: "Verification Complete",
            description: `DNS verified for ${domain}.`,
        });
        fetchDomains();
    } catch (error) {
        toast({
            title: "Verification Failed",
            description: "Could not verify DNS records. Please check your settings.",
            variant: "destructive",
        });
    }
  };

  const handleProvisionSsl = async (id: string, domain: string) => {
      try {
          toast({
              title: "Provisioning SSL",
              description: `Requesting SSL certificate for ${domain}...`,
          });
          await api.domains.provisionSsl(id);
          toast({
              title: "SSL Provisioned",
              description: `SSL certificate provisioned for ${domain}.`,
          });
          fetchDomains();
      } catch (error) {
          toast({
              title: "SSL Failed",
              description: "Could not provision SSL certificate.",
              variant: "destructive",
          });
      }
  };

  const handleDeleteDomain = async (id: string, domain: string) => {
    if (window.confirm(`Are you sure you want to remove ${domain}? This action cannot be undone.`)) {
      try {
        await api.domains.delete(id);
        toast({
            title: "Domain Removed",
            description: `${domain} has been removed.`,
        });
        fetchDomains();
      } catch (error) {
          toast({
              title: "Error",
              description: "Failed to delete domain.",
              variant: "destructive",
          });
      }
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
            <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
                <DialogTrigger asChild>
                    <Button>
                        <Plus className="h-4 w-4 mr-2" />
                        Add Domain
                    </Button>
                </DialogTrigger>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>Add Custom Domain</DialogTitle>
                        <DialogDescription>
                            Enter the domain name you want to connect (e.g., app.yourcompany.com).
                        </DialogDescription>
                    </DialogHeader>
                    <div className="grid gap-4 py-4">
                        <div className="grid grid-cols-4 items-center gap-4">
                            <Label htmlFor="domain" className="text-right">
                                Domain
                            </Label>
                            <Input
                                id="domain"
                                placeholder="app.example.com"
                                className="col-span-3"
                                value={newDomain}
                                onChange={(e) => setNewDomain(e.target.value)}
                            />
                        </div>
                    </div>
                    <DialogFooter>
                        <Button type="submit" onClick={handleAddDomain} disabled={isAdding || !newDomain}>
                            {isAdding && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                            Add Domain
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
          </div>
        </CardHeader>
        <CardContent>
            <div className="space-y-6">
                {/* Primary/System Domain (Always present) */}
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

                {loading ? (
                    <div className="flex justify-center p-8">
                        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                    </div>
                ) : domains.length === 0 ? (
                    <div className="text-center py-4 text-muted-foreground">
                        No custom domains connected.
                    </div>
                ) : (
                    domains.map((domain) => (
                        <div key={domain.id} className={`flex flex-col md:flex-row md:items-center justify-between gap-4 p-4 border rounded-lg ${domain.status === 'failed' ? 'border-red-200 bg-red-50/50' : !domain.dns_verified ? 'border-yellow-200 bg-yellow-50/50' : ''}`}>
                            <div className="flex items-start gap-4">
                                <div className={`mt-1 p-2 rounded-full ${domain.dns_verified ? 'bg-orange-100' : 'bg-yellow-100'}`}>
                                    <Globe className={`h-5 w-5 ${domain.dns_verified ? 'text-orange-600' : 'text-yellow-600'}`} />
                                </div>
                                <div>
                                    <div className="flex items-center gap-2">
                                        <h3 className="font-semibold text-lg">{domain.domain}</h3>
                                        {domain.dns_verified ? (
                                             <Badge variant="secondary" className="text-green-600 bg-green-50 border-green-200">
                                                <ShieldCheck className="h-3 w-3 mr-1" />
                                                Active
                                            </Badge>
                                        ) : (
                                            <Badge variant="outline" className="text-yellow-700 border-yellow-300 bg-yellow-100">
                                                Pending DNS
                                            </Badge>
                                        )}
                                    </div>

                                    {domain.dns_verified ? (
                                        <div className="flex gap-4 mt-2 text-sm text-muted-foreground">
                                            <span className="flex items-center gap-1">
                                                DNS: <span className="text-green-600 font-medium">Verified</span>
                                            </span>
                                            <span className="flex items-center gap-1">
                                                SSL: <span className={domain.ssl_status === 'issued' ? "text-green-600 font-medium" : "text-yellow-600 font-medium"}>
                                                    {domain.ssl_status === 'issued' ? 'Valid' : 'Pending'}
                                                </span>
                                            </span>
                                        </div>
                                    ) : (
                                        <div className="mt-2">
                                            <p className="text-sm text-muted-foreground">
                                                Add the following CNAME record to your DNS provider:
                                            </p>
                                            <div className="flex items-center gap-2 mt-2">
                                                <code className="bg-background px-2 py-1 rounded border text-xs font-mono">
                                                    cname.rampos.io
                                                </code>
                                                <Button variant="ghost" size="sm" onClick={handleCopyCname}>
                                                    <Copy className="h-3 w-3" />
                                                </Button>
                                            </div>
                                        </div>
                                    )}
                                </div>
                            </div>
                            <div className="flex items-center gap-2">
                                {!domain.dns_verified && (
                                     <Button variant="default" size="sm" className="bg-yellow-600 hover:bg-yellow-700" onClick={() => handleVerifyDns(domain.id, domain.domain)}>
                                        Verify DNS
                                    </Button>
                                )}
                                {domain.dns_verified && domain.ssl_status !== 'issued' && (
                                     <Button variant="outline" size="sm" onClick={() => handleProvisionSsl(domain.id, domain.domain)}>
                                        Provision SSL
                                    </Button>
                                )}
                                <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10" onClick={() => handleDeleteDomain(domain.id, domain.domain)}>
                                    <Trash2 className="h-4 w-4" />
                                </Button>
                            </div>
                        </div>
                    ))
                )}
            </div>
        </CardContent>
      </Card>
    </div>
  );
}
