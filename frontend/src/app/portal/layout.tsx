import { PortalSidebar } from "@/components/layout/portal-sidebar"

export default function PortalLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-screen">
      <PortalSidebar />
      <main className="flex-1 p-6">{children}</main>
    </div>
  )
}
