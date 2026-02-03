"use client";

import { PortalSidebar } from "@/components/layout/portal-sidebar";
import { PageContainer } from "@/components/layout/page-container";
import { AuthProvider } from "@/contexts/auth-context";

export default function PortalLayout({ children }: { children: React.ReactNode }) {
  return (
    <AuthProvider>
      <div className="flex min-h-screen bg-background">
        <PortalSidebar />
        <main className="flex-1 overflow-y-auto h-screen">
          <PageContainer className="py-6 md:py-8" maxWidth="xl">
            {children}
          </PageContainer>
        </main>
      </div>
    </AuthProvider>
  );
}
