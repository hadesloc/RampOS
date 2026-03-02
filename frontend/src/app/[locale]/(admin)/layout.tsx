import Sidebar from "@/components/layout/sidebar";
import { PageContainer } from "@/components/layout/page-container";
import { Metadata } from "next";
import { cookies } from "next/headers";
import { redirect } from "@/navigation";
import { ADMIN_SESSION_COOKIE, isAdminSessionTokenValid } from "@/lib/admin-auth";
import { CommandPalette } from "@/components/ui/command-palette";

export const metadata: Metadata = {
  title: "RampOS Admin",
  description: "Admin dashboard for RampOS",
};

export default async function AdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  // Auth temporarily disabled for development
  // const adminKey = process.env.RAMPOS_ADMIN_KEY;
  // if (!adminKey) {
  //   return <div className="p-6">Admin key not configured.</div>;
  // }
  // const cookieStore = await cookies();
  // const token = cookieStore.get(ADMIN_SESSION_COOKIE)?.value;
  // if (!isAdminSessionTokenValid(token, adminKey)) {
  //   redirect("/admin-login");
  // }

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <PageContainer className="py-6 md:py-8" maxWidth="2xl">
          {children}
        </PageContainer>
      </main>
      <CommandPalette />
    </div>
  );
}
