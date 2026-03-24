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
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  const sessionSecret = process.env.RAMPOS_ADMIN_JWT_SECRET;
  if (!sessionSecret) {
    return <div className="p-6">Admin auth not configured.</div>;
  }

  // In development, allow bypassing auth to preview UI
  const isDev = process.env.NODE_ENV === 'development';
  const cookieStore = await cookies();
  const token = cookieStore.get(ADMIN_SESSION_COOKIE)?.value;
  if (!isDev && !isAdminSessionTokenValid(token, sessionSecret)) {
    redirect("/admin-login");
  }

  return (
    <div className="flex h-screen overflow-hidden bg-[#050505] text-slate-300">
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
