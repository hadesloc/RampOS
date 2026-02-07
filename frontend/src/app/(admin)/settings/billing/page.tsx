"use client";

import { PageHeader } from "@/components/layout/page-header";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { CreditCard, Download, ExternalLink, Zap } from "lucide-react";
import { toast } from "@/components/ui/use-toast";

const invoices = [
  { id: "inv_001", date: "Mar 1, 2024", amount: "$499.00", status: "Paid" },
  { id: "inv_002", date: "Feb 1, 2024", amount: "$499.00", status: "Paid" },
  { id: "inv_003", date: "Jan 1, 2024", amount: "$499.00", status: "Paid" },
];

export default function BillingPage() {
  const handleManageSubscription = () => {
    toast({
      title: "Manage Subscription",
      description: "Redirecting to subscription management portal is not yet available.",
    });
  };

  const handleUpgradePlan = () => {
    toast({
      title: "Upgrade Plan",
      description: "Plan upgrade flow is not yet available. Contact sales for custom plans.",
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
      description: `Invoice ${invoiceId} download is not yet available.`,
    });
  };

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
                <CardDescription>You are on the <span className="font-semibold text-primary">Enterprise</span> plan.</CardDescription>
              </div>
              <Badge className="bg-primary/10 text-primary hover:bg-primary/20">Active</Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                    <div className="flex items-center justify-between text-sm">
                        <span className="text-muted-foreground">API Calls</span>
                        <span className="font-medium">8.5M / 10M</span>
                    </div>
                    <Progress value={85} className="h-2" />
                    <p className="text-xs text-muted-foreground text-right">Resets in 12 days</p>
                </div>
                <div className="space-y-2">
                    <div className="flex items-center justify-between text-sm">
                        <span className="text-muted-foreground">Transaction Volume</span>
                        <span className="font-medium">$45M / $100M</span>
                    </div>
                    <Progress value={45} className="h-2" />
                </div>
            </div>

            <Separator />

            <div className="grid gap-4 md:grid-cols-3">
                <div>
                    <p className="text-sm font-medium text-muted-foreground">Next Invoice</p>
                    <p className="text-2xl font-bold">$499.00</p>
                    <p className="text-xs text-muted-foreground">Due on {new Date(new Date().getFullYear(), new Date().getMonth() + 1, 1).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}</p>
                </div>
                 <div>
                    <p className="text-sm font-medium text-muted-foreground">Payment Method</p>
                    <div className="flex items-center gap-2 mt-1">
                        <CreditCard className="h-4 w-4" />
                        <span className="text-sm font-medium">&bull;&bull;&bull;&bull; 4242</span>
                    </div>
                </div>
                 <div>
                    <p className="text-sm font-medium text-muted-foreground">Billing Email</p>
                    <p className="text-sm mt-1">billing@rampos.io</p>
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
                {invoices.map((inv) => (
                    <div key={inv.id} className="flex items-center justify-between border-b last:border-0 pb-4 last:pb-0">
                        <div className="flex items-center gap-4">
                            <div className="h-10 w-10 rounded-full bg-muted flex items-center justify-center">
                                <Download className="h-4 w-4 text-muted-foreground" />
                            </div>
                            <div>
                                <p className="font-medium">{inv.date}</p>
                                <p className="text-sm text-muted-foreground">Invoice #{inv.id}</p>
                            </div>
                        </div>
                        <div className="flex items-center gap-4">
                            <span className="font-medium">{inv.amount}</span>
                            <Badge variant="outline" className="bg-green-50 text-green-700 border-green-200">{inv.status}</Badge>
                            <Button variant="ghost" size="icon" onClick={() => handleDownloadInvoice(inv.id)}>
                                <ExternalLink className="h-4 w-4" />
                            </Button>
                        </div>
                    </div>
                ))}
            </div>
        </CardContent>
      </Card>
    </div>
  );
}
