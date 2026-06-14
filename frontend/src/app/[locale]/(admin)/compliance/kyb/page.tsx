"use client";

import KybGraphReview from "@/components/compliance/KybGraphReview";
import { PageHeader } from "@/components/shared";

export default function KybPage() {
  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="KYB Ownership Review"
        description="Review ownership graph requirements, licensing gaps, and queue flags without inventing compliance state."
      />
      <KybGraphReview />
    </main>
  );
}
