"use client";

import { useState, useEffect, useMemo } from "react";
import type { ColumnDef } from "@tanstack/react-table";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { CreditCard, Download, ExternalLink, Zap, Loader2 } from "lucide-react";
import { toast } from "@/components/ui/use-toast";
import { api, Subscription, Invoice } from "@/lib/api";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  SectionCard,
  DataTable,
  EmptyState,
  ErrorState,
  StatusBadge,
} from "@/components/shared";
import type { StatusSeverity } from "@/components/shared";
import { formatDate } from "@/lib/format";

export default function BillingPage() {
  const [subscription, setSubscription] = useState<Subscription | null>(null);
  const [invoices, setInvoices] = useState<Invoice[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchBillingData = async () => {
    try {
      setLoading(true);
      setError(null);
      const [subData, invData] = await Promise.all([
        api.billing.getSubscription(),
        api.billing.getInvoices(),
      ]);
      setSubscription(subData);
      setInvoices(invData.data);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load billing information.";
      setError(message);
      toast({ title: "Error", description: message, variant: "destructive" });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchBillingData();
  }, []);

  const handleManageSubscription = () =>
    toast({ title: "Manage Subscription", description: "Redirecting to subscription management portal..." });

  const handleUpgradePlan = () =>
    toast({ title: "Upgrade Plan", description: "Plan upgrade flow coming soon." });

  const handleContactSales = () =>
    toast({ title: "Contact Sales", description: "Please email sales@rampos.io for enterprise inquiries." });

  const handleDownloadInvoice = (invoiceId: string) =>
    toast({ title: "Download Invoice", description: `Downloading invoice ${invoiceId}...` });

  const apiUsagePct = subscription
    ? Math.min(100, (subscription.usage.api_calls / subscription.usage.api_limit) * 100)
    : 0;

  const volumeUsagePct = subscription
    ? Math.min(100, (subscription.usage.transaction_volume / subscription.usage.volume_limit) * 100)
    : 0;

  const invoiceColumns = useMemo<ColumnDef<Invoice>[]>(
    () => [
      {
        accessorKey: "date",
        header: "Date",
        cell: ({ row }) => (
          <span className="tabular-nums text-sm">{formatDate(row.original.date)}</span>
        ),
      },
      {
        accessorKey: "number",
        header: "Invoice #",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground">#{row.original.number}</span>
        ),
      },
      {
        id: "amount",
        header: () => <div className="text-right">Amount</div>,
        cell: ({ row }) => (
          <div className="text-right font-semibold tabular-nums">
            {row.original.currency} {row.original.amount}
          </div>
        ),
      },
      {
        accessorKey: "status",
        header: "Status",
        cell: ({ row }) => (
          <StatusBadge
            status={row.original.status}
            severity={row.original.status === "paid" ? "success" : "warning"}
          />
        ),
      },
      {
        id: "download",
        header: "",
        cell: ({ row }) => (
          <div className="flex justify-end">
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => handleDownloadInvoice(row.original.id)}
            >
              <ExternalLink className="h-3.5 w-3.5" />
            </Button>
          </div>
        ),
      },
    ],
    []
  );

  if (loading) {
    return (
      <main className="p-page flex flex-col gap-section">
        <PageHeader title="Billing & Usage" description="Manage your subscription plan, payment methods, and invoices." />
        <div className="flex justify-center py-16">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </main>
    );
  }

  if (error) {
    return (
      <main className="p-page flex flex-col gap-section">
        <PageHeader title="Billing & Usage" description="Manage your subscription plan, payment methods, and invoices." />
        <ErrorState message={error} retry={fetchBillingData} />
      </main>
    );
  }

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Billing & Usage"
        description="Manage your subscription plan, payment methods, and view invoices."
        actions={
          <div className="flex gap-2">
            <Button variant="outline" onClick={handleManageSubscription}>Manage Subscription</Button>
            <Button onClick={handleUpgradePlan}>Upgrade Plan</Button>
          </div>
        }
      />

      <StatGrid cols={3}>
        <StatCard
          title="Current Plan"
          value={subscription?.plan ? (subscription.plan.charAt(0).toUpperCase() + subscription.plan.slice(1)) : "Free"}
          subtitle={subscription?.status?.toUpperCase() ?? "ACTIVE"}
          accentColor="green"
        />
        <StatCard
          title="Next Invoice"
          value={subscription?.amount ? `$${subscription.amount}` : "$0.00"}
          subtitle={subscription?.next_invoice_date ? `Due ${formatDate(subscription.next_invoice_date)}` : undefined}
          accentColor="cyan"
        />
        <StatCard
          title="Payment Method"
          value={subscription?.payment_method ? `•••• ${subscription.payment_method.last4}` : "No card"}
          icon={<CreditCard className="h-4 w-4" />}
          accentColor="violet"
        />
      </StatGrid>

      {/* Usage */}
      <Panel
        header={{ title: "Usage This Period", description: `Resets on ${subscription?.usage?.reset_date ? formatDate(subscription.usage.reset_date) : "N/A"}` }}
        variant="glass"
      >
        <div className="grid gap-6 sm:grid-cols-2">
          <div className="space-y-2">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">API Calls</span>
              <span className="font-medium tabular-nums">
                {(subscription?.usage.api_calls || 0).toLocaleString()} / {(subscription?.usage.api_limit || 0).toLocaleString()}
              </span>
            </div>
            <Progress value={apiUsagePct} className="h-1.5 bg-white/[0.06]" />
          </div>
          <div className="space-y-2">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Transaction Volume</span>
              <span className="font-medium tabular-nums">
                ${(subscription?.usage.transaction_volume || 0).toLocaleString()} / ${(subscription?.usage.volume_limit || 0).toLocaleString()}
              </span>
            </div>
            <Progress value={volumeUsagePct} className="h-1.5 bg-white/[0.06]" />
          </div>
        </div>
      </Panel>

      {/* Enterprise upsell */}
      <SectionCard
        header={{
          title: "Enterprise Power",
          actions: <Zap className="h-4 w-4 text-[#7B61FF]" />,
        }}
        className="border-[#7B61FF]/20 bg-[#7B61FF]/5"
      >
        <div className="flex flex-col sm:flex-row sm:items-center gap-4">
          <ul className="flex-1 space-y-1 text-sm text-muted-foreground">
            {["Dedicated Support Manager", "99.99% SLA Guarantee", "Unlimited Team Members", "Custom Contracts & Audits"].map((f) => (
              <li key={f} className="flex items-center gap-2">
                <div className="h-1.5 w-1.5 rounded-full bg-[#7B61FF]" />
                {f}
              </li>
            ))}
          </ul>
          <Button variant="outline" className="border-[#7B61FF]/30 text-[#7B61FF] hover:bg-[#7B61FF]/10 shrink-0" onClick={handleContactSales}>
            Contact Sales
          </Button>
        </div>
      </SectionCard>

      {/* Invoices */}
      <Panel header={{ title: "Invoices", description: "View and download past invoices." }}>
        <DataTable
          columns={invoiceColumns}
          data={invoices}
          emptyState={
            <EmptyState title="No invoices" description="No invoices have been issued yet." />
          }
        />
      </Panel>
    </main>
  );
}
