"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
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
import {
  PageHeader,
  Panel,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";
import type { StatusSeverity } from "@/components/shared";

export default function DomainsPage() {
  const [domains, setDomains] = useState<Domain[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [newDomain, setNewDomain] = useState("");
  const [isAdding, setIsAdding] = useState(false);
  const [isDialogOpen, setIsDialogOpen] = useState(false);

  const fetchDomains = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await api.domains.list();
      setDomains(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load domains.";
      setError(message);
      toast({ title: "Error", description: message, variant: "destructive" });
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
      toast({ title: "Domain Added", description: `${newDomain} has been added. Please configure DNS.` });
      setNewDomain("");
      setIsDialogOpen(false);
      fetchDomains();
    } catch {
      toast({ title: "Error", description: "Failed to add domain. It may already exist.", variant: "destructive" });
    } finally {
      setIsAdding(false);
    }
  };

  const handleCopyCname = () => {
    navigator.clipboard.writeText("cname.rampos.io").then(() => {
      toast({ title: "Copied", description: "CNAME value copied to clipboard." });
    });
  };

  const handleVerifyDns = async (id: string, domain: string) => {
    toast({ title: "Verifying DNS", description: `Checking DNS records for ${domain}...` });
    try {
      await api.domains.verifyDns(id);
      toast({ title: "Verification Complete", description: `DNS verified for ${domain}.` });
      fetchDomains();
    } catch {
      toast({ title: "Verification Failed", description: "Could not verify DNS records. Please check your settings.", variant: "destructive" });
    }
  };

  const handleProvisionSsl = async (id: string, domain: string) => {
    toast({ title: "Provisioning SSL", description: `Requesting SSL certificate for ${domain}...` });
    try {
      await api.domains.provisionSsl(id);
      toast({ title: "SSL Provisioned", description: `SSL certificate provisioned for ${domain}.` });
      fetchDomains();
    } catch {
      toast({ title: "SSL Failed", description: "Could not provision SSL certificate.", variant: "destructive" });
    }
  };

  const handleDeleteDomain = async (id: string, domain: string) => {
    if (!window.confirm(`Are you sure you want to remove ${domain}? This action cannot be undone.`)) return;
    try {
      await api.domains.delete(id);
      toast({ title: "Domain Removed", description: `${domain} has been removed.` });
      fetchDomains();
    } catch {
      toast({ title: "Error", description: "Failed to delete domain.", variant: "destructive" });
    }
  };

  const addAction = (
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
            <Label htmlFor="domain" className="text-right">Domain</Label>
            <Input
              id="domain"
              placeholder="app.example.com"
              className="col-span-3 border-white/[0.08] bg-[#09090B]"
              value={newDomain}
              onChange={(e) => setNewDomain(e.target.value)}
            />
          </div>
        </div>
        <DialogFooter>
          <Button onClick={handleAddDomain} disabled={isAdding || !newDomain}>
            {isAdding && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Add Domain
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Custom Domains"
        description="Connect your own domains to your RampOS instance with automatic SSL."
        actions={addAction}
      />

      {error ? (
        <ErrorState message={error} retry={fetchDomains} />
      ) : (
        <Panel header={{ title: "Connected Domains", description: "Domains that resolve to this tenant." }} contentClassName="p-4">
          <div className="space-y-4">
            {/* System default domain — always shown */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 rounded-lg border border-white/[0.06] bg-white/[0.02]">
              <div className="flex items-start gap-4">
                <div className="mt-0.5 bg-[#00FF87]/10 p-2 rounded-full">
                  <Globe className="h-4 w-4 text-[#00FF87]" />
                </div>
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <h3 className="font-semibold">app.rampos.io</h3>
                    <StatusBadge status="Default" severity="neutral" />
                    <StatusBadge status="Secure" severity="success" />
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">System provided domain</p>
                </div>
              </div>
              <Button variant="outline" size="sm" disabled>Primary</Button>
            </div>

            {loading ? (
              <div className="flex justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : domains.length === 0 ? (
              <EmptyState
                title="No custom domains"
                description="Add a custom domain to get started."
              />
            ) : (
              domains.map((domain) => (
                <div
                  key={domain.id}
                  className={`flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 rounded-lg border transition-colors ${
                    domain.status === "failed"
                      ? "border-red-500/20 bg-red-500/5"
                      : !domain.dns_verified
                      ? "border-[#FFB800]/20 bg-[#FFB800]/5"
                      : "border-white/[0.06] bg-white/[0.02]"
                  }`}
                >
                  <div className="flex items-start gap-4">
                    <div className={`mt-0.5 p-2 rounded-full ${domain.dns_verified ? "bg-[#00FF87]/10" : "bg-[#FFB800]/10"}`}>
                      <Globe className={`h-4 w-4 ${domain.dns_verified ? "text-[#00FF87]" : "text-[#FFB800]"}`} />
                    </div>
                    <div>
                      <div className="flex flex-wrap items-center gap-2">
                        <h3 className="font-semibold">{domain.domain}</h3>
                        {domain.dns_verified ? (
                          <StatusBadge status="Active" severity="success" />
                        ) : (
                          <StatusBadge status="Pending DNS" severity="warning" />
                        )}
                      </div>

                      {domain.dns_verified ? (
                        <div className="flex flex-wrap gap-4 mt-1.5 text-xs text-muted-foreground">
                          <span>DNS: <span className="text-[#00FF87] font-medium">Verified</span></span>
                          <span>
                            SSL:{" "}
                            <span className={domain.ssl_status === "issued" ? "text-[#00FF87] font-medium" : "text-[#FFB800] font-medium"}>
                              {domain.ssl_status === "issued" ? "Valid" : "Pending"}
                            </span>
                          </span>
                        </div>
                      ) : (
                        <div className="mt-2">
                          <p className="text-xs text-muted-foreground">Add this CNAME record to your DNS provider:</p>
                          <div className="flex items-center gap-2 mt-1.5">
                            <code className="rounded border border-white/[0.08] bg-[#09090B] px-2 py-0.5 text-xs font-mono text-muted-foreground">
                              cname.rampos.io
                            </code>
                            <Button variant="ghost" size="icon" className="h-6 w-6" onClick={handleCopyCname}>
                              <Copy className="h-3 w-3" />
                            </Button>
                          </div>
                        </div>
                      )}
                    </div>
                  </div>

                  <div className="flex items-center gap-2 shrink-0">
                    {!domain.dns_verified && (
                      <Button
                        variant="outline"
                        size="sm"
                        className="border-[#FFB800]/30 text-[#FFB800] hover:bg-[#FFB800]/10"
                        onClick={() => handleVerifyDns(domain.id, domain.domain)}
                      >
                        Verify DNS
                      </Button>
                    )}
                    {domain.dns_verified && domain.ssl_status !== "issued" && (
                      <Button variant="outline" size="sm" onClick={() => handleProvisionSsl(domain.id, domain.domain)}>
                        <ShieldCheck className="mr-1.5 h-3.5 w-3.5" />
                        Provision SSL
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 text-red-400 hover:text-red-400 hover:bg-red-400/10"
                      onClick={() => handleDeleteDomain(domain.id, domain.domain)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))
            )}
          </div>
        </Panel>
      )}
    </main>
  );
}
