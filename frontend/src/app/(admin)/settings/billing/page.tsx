"use client";

import { useState, useEffect } from "react";
import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { CreditCard, Download, ExternalLink, Zap, Loader2 } from "lucide-react";
import { toast } from "@/components/ui/use-toast";
import { api, Subscription, Invoice } from "@/lib/api";

export default function BillingPage() {
  const [subscription, setSubscription] = useState<Subscription | null>(null);
  const [invoices, setInvoices] = useState<Invoice[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchBillingData = async () => {
    try {
      setLoading(true);
      const [subData, invData] = await Promise.all([
        api.billing.getSubscription(),
        api.billing.getInvoices()
      ]);
      setSubscription(subData);
      setInvoices(invData.data);
    } catch (error) {
      console.error("Failed to fetch billing data:", error);
      // Fallback/Mock for now or error
      toast({
        title: "Error",
        description: "Failed to load billing information.",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchBillingData();
  }, []);

  const handleManageSubscription = () => {
    toast({
      title: "Manage Subscription",
      description: "Redirecting to subscription management portal...",
    });
  };

  const handleUpgradePlan = () => {
    toast({
      title: "Upgrade Plan",
      description: "Plan upgrade flow coming soon.",
    });
  };

  const handleContactSales = () => {
    toast({
      title: "Contact Sales",
      description: "Please email sales@rampos.io for enterprise inquiries.",
    });
  };

  const handleDownloadInvoice = (invoiceId: string) => {
    toast({
      title: "Download Invoice",
      description: `Downloading invoice ${invoiceId}...`,
    });
  };

  if (loading) {
      return (
          <div className="flex flex-col gap-6 p-6">
              <PageHeader title="Billing & Usage" description="Manage your subscription plan." />
              <div className="flex justify-center p-12">
                  <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              </div>
          </div>
      );
  }

  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader
        title="Billing & Usage"
        description="Manage your subscription plan, payment methods, and view invoices."
      />

      <div className="grid gap-6 md:grid-cols-3">
        {/* Current Plan */}
        <Card className="md:col-span-2">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle>Current Plan</CardTitle>
                <CardDescription>You are on the <span className="font-semibold text-primary capitalize">{subscription?.plan || 'Free'}</span> plan.</CardDescription>
              </div>
              <Badge className={subscription?.status === 'active' ? "bg-primary/10 text-primary hover:bg-primary/20" : "bg-yellow-100 text-yellow-800"}>
                {(subscription?.status || 'Active').toUpperCase()}
              </Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                    <div className="flex items-center justify-between text-sm">
                        <span className="text-muted-foreground">API Calls</span>
                        <span className="font-medium">
                            {(subscription?.usage.api_calls || 0).toLocaleString()} / {(subscription?.usage.api_limit || 0).toLocaleString()}
                        </span>
                    </div>
                    <Progress value={subscription ? (subscription.usage.api_calls / subscription.usage.api_limit) * 100 : 0} className="h-2" />
                    <p className="text-xs text-muted-foreground text-right">
                        Resets on {subscription?.usage.reset_date ? new Date(subscription.usage.reset_date).toLocaleDateString() : 'N/A'}
                    </p>
                </div>
                <div className="space-y-2">
                    <div className="flex items-center justify-between text-sm">
                        <span className="text-muted-foreground">Transaction Volume</span>
                        <span className="font-medium">
                            ${(subscription?.usage.transaction_volume || 0).toLocaleString()} / ${(subscription?.usage.volume_limit || 0).toLocaleString()}
                        </span>
                    </div>
                    <Progress value={subscription ? (subscription.usage.transaction_volume / subscription.usage.volume_limit) * 100 : 0} className="h-2" />
                </div>
            </div>

            <Separator />

            <div className="grid gap-4 md:grid-cols-3">
                <div>
                    <p className="text-sm font-medium text-muted-foreground">Next Invoice</p>
                    <p className="text-2xl font-bold">{subscription?.amount ? `$${subscription.amount}` : '$0.00'}</p>
                    <p className="text-xs text-muted-foreground">
                        Due on {subscription?.next_invoice_date ? new Date(subscription.next_invoice_date).toLocaleDateString() : 'N/A'}
                    </p>
                </div>
                 <div>
                    <p className="text-sm font-medium text-muted-foreground">Payment Method</p>
                    <div className="flex items-center gap-2 mt-1">
                        <CreditCard className="h-4 w-4" />
                        <span className="text-sm font-medium">
                            {subscription?.payment_method ? `•••• ${subscription.payment_method.last4}` : 'No card'}
                        </span>
                    </div>
                </div>
                 <div>
                    <p className="text-sm font-medium text-muted-foreground">Billing Email</p>
                    <p className="text-sm mt-1">{subscription?.billing_email || 'N/A'}</p>
                </div>
            </div>
          </CardContent>
          <CardFooter className="bg-muted/50 flex justify-between">
            <Button variant="ghost" onClick={handleManageSubscription}>Manage Subscription</Button>
            <Button onClick={handleUpgradePlan}>Upgrade Plan</Button>
          </CardFooter>
        </Card>

        {/* Feature Highlights */}
        <Card className="bg-primary text-primary-foreground">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
                <Zap className="h-5 w-5" />
                Enterprise Power
            </CardTitle>
            <CardDescription className="text-primary-foreground/80">
                Unlock full potential with Enterprise features.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            <div className="flex items-center gap-2 text-sm">
                <div className="h-1.5 w-1.5 rounded-full bg-white" />
                Dedicated Support Manager
            </div>
             <div className="flex items-center gap-2 text-sm">
                <div className="h-1.5 w-1.5 rounded-full bg-white" />
                99.99% SLA Guarantee
            </div>
             <div className="flex items-center gap-2 text-sm">
                <div className="h-1.5 w-1.5 rounded-full bg-white" />
                Unlimited Team Members
            </div>
             <div className="flex items-center gap-2 text-sm">
                <div className="h-1.5 w-1.5 rounded-full bg-white" />
                Custom Contracts & Audits
            </div>
          </CardContent>
          <CardFooter>
            <Button variant="secondary" className="w-full" onClick={handleContactSales}>Contact Sales</Button>
          </CardFooter>
        </Card>
      </div>

      {/* Invoices */}
      <Card>
        <CardHeader>
            <CardTitle>Invoices</CardTitle>
            <CardDescription>View and download past invoices.</CardDescription>
        </CardHeader>
        <CardContent>
            <div className="space-y-4">
                {invoices.length === 0 ? (
                    <div className="text-center py-4 text-muted-foreground">No invoices found.</div>
                ) : (
                    invoices.map((inv) => (
                        <div key={inv.id} className="flex items-center justify-between border-b last:border-0 pb-4 last:pb-0">
                            <div className="flex items-center gap-4">
                                <div className="h-10 w-10 rounded-full bg-muted flex items-center justify-center">
                                    <Download className="h-4 w-4 text-muted-foreground" />
                                </div>
                                <div>
                                    <p className="font-medium">{new Date(inv.date).toLocaleDateString()}</p>
                                    <p className="text-sm text-muted-foreground">Invoice #{inv.number}</p>
                                </div>
                            </div>
                            <div className="flex items-center gap-4">
                                <span className="font-medium">{inv.currency} {inv.amount}</span>
                                <Badge variant="outline" className={`bg-green-50 text-green-700 border-green-200 ${inv.status === 'paid' ? '' : 'bg-yellow-50 text-yellow-700 border-yellow-200'}`}>
                                    {inv.status}
                                </Badge>
                                <Button variant="ghost" size="icon" onClick={() => handleDownloadInvoice(inv.id)}>
                                    <ExternalLink className="h-4 w-4" />
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
