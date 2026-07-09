"use client";

import { PageHeader } from "@/components/shared";
import ReconciliationWorkbench from "@/components/reconciliation/ReconciliationWorkbench";

export default function ReconciliationPage() {
  return (
    <main className="p-6 md:p-8 flex flex-col gap-6">
      <PageHeader
        title="Reconciliation"
        description="Triage breaks by severity, aging, owner lane, and linked evidence."
        breadcrumb={[
          { label: "Admin", href: "/admin" },
          { label: "Reconciliation" },
        ]}
      />
      <ReconciliationWorkbench />
    </main>
  );
}
