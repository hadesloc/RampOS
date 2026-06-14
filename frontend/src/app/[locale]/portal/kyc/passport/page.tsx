"use client";

import PassportPortalView from "@/components/compliance/PassportPortalView";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";

export default function PassportPortalPage() {
  return (
    <PageContainer>
      <PageHeader
        title="Reusable KYC Passport"
        description="Review passport availability, consent status, and reuse scope for your verification package."
      />
      <PassportPortalView />
    </PageContainer>
  );
}
