import Sidebar from "@/components/layout/sidebar";
import { Metadata } from "next";

export const metadata: Metadata = {
  title: "RampOS Admin",
  description: "Admin dashboard for RampOS",
};

export default function AdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <main className="flex-1 overflow-y-auto p-8">
        {children}
      </main>
    </div>
  );
}
