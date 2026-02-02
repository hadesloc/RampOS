"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { useState } from "react"
import { cn } from "@/lib/utils"
import {
  LayoutDashboard,
  Wallet,
  ArrowDownToLine,
  ArrowUpFromLine,
  History,
  Settings,
  Menu,
  X,
  User
} from "lucide-react"
import { Button } from "@/components/ui/button"

const sidebarItems = [
  {
    title: "Dashboard",
    href: "/portal",
    icon: LayoutDashboard,
  },
  {
    title: "Assets",
    href: "/portal/assets",
    icon: Wallet,
  },
  {
    title: "Deposit",
    href: "/portal/deposit",
    icon: ArrowDownToLine,
  },
  {
    title: "Withdraw",
    href: "/portal/withdraw",
    icon: ArrowUpFromLine,
  },
  {
    title: "Transactions",
    href: "/portal/transactions",
    icon: History,
  },
  {
    title: "Settings",
    href: "/portal/settings",
    icon: Settings,
  },
]

export function PortalSidebar() {
  const pathname = usePathname()
  const [isOpen, setIsOpen] = useState(false)

  return (
    <>
      {/* Mobile Toggle */}
      <div className="md:hidden fixed top-4 left-4 z-50">
        <Button variant="outline" size="icon" onClick={() => setIsOpen(!isOpen)}>
          <Menu className="h-4 w-4" />
        </Button>
      </div>

      {/* Mobile Overlay */}
      {isOpen && (
        <div
          className="fixed inset-0 z-40 bg-background/80 backdrop-blur-sm md:hidden"
          onClick={() => setIsOpen(false)}
        />
      )}

      {/* Sidebar Container */}
      <aside
        className={cn(
          "fixed inset-y-0 left-0 z-50 w-64 border-r bg-card transition-transform duration-300 ease-in-out md:static md:translate-x-0",
          isOpen ? "translate-x-0" : "-translate-x-full"
        )}
      >
        <div className="flex h-full flex-col px-3 py-4">
          <div className="mb-8 flex items-center justify-between px-3">
            <h1 className="text-xl font-bold tracking-tight">RampOS Portal</h1>
            <Button
              variant="ghost"
              size="icon"
              className="md:hidden"
              onClick={() => setIsOpen(false)}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>

          <nav className="flex-1 space-y-1">
            {sidebarItems.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className={cn(
                  "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground",
                  pathname === item.href
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground"
                )}
                onClick={() => setIsOpen(false)}
              >
                <item.icon className="h-4 w-4" />
                {item.title}
              </Link>
            ))}
          </nav>

          <div className="border-t pt-4">
            <div className="flex items-center gap-3 px-3 py-2">
              <div className="flex h-9 w-9 items-center justify-center rounded-full bg-muted">
                <User className="h-4 w-4" />
              </div>
              <div className="flex flex-col">
                <p className="text-sm font-medium">User Account</p>
                <p className="text-xs text-muted-foreground">user@example.com</p>
              </div>
            </div>
          </div>
        </div>
      </aside>
    </>
  )
}
