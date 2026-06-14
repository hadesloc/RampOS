"use client";

import PassportAdminQueue from "@/components/compliance/PassportAdminQueue";
import { PageHeader } from "@/components/shared";

export default function PassportAdminPage() {
  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="KYC Passport Queue"
        description="Review shared-vault passport packages, consent posture, and destination tenant scope."
      />
      <PassportAdminQueue />
    </main>
  );
}
