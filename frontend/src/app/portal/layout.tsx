"use client";

import { PortalSidebar } from "@/components/layout/portal-sidebar";
import { AuthProvider } from "@/contexts/auth-context";

export default function PortalLayout({ children }: { children: React.ReactNode }) {
  return (
    <AuthProvider>
      <div className="flex min-h-screen">
        <PortalSidebar />
        <main className="flex-1 p-6">{children}</main>
      </div>
    </AuthProvider>
  );
}
