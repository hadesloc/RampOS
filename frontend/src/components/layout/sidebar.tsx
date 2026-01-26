"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";
import {
  LayoutDashboard,
  ArrowLeftRight,
  Users,
  ShieldAlert,
  BookOpen,
  Webhook,
  Settings,
} from "lucide-react";

const sidebarItems = [
  {
    title: "Dashboard",
    href: "/",
    icon: LayoutDashboard,
  },
  {
    title: "Intents",
    href: "/intents",
    icon: ArrowLeftRight,
  },
  {
    title: "Users",
    href: "/users",
    icon: Users,
  },
  {
    title: "Compliance",
    href: "/compliance",
    icon: ShieldAlert,
  },
  {
    title: "Ledger",
    href: "/ledger",
    icon: BookOpen,
  },
  {
    title: "Webhooks",
    href: "/webhooks",
    icon: Webhook,
  },
  {
    title: "Settings",
    href: "/settings",
    icon: Settings,
  },
];

export default function Sidebar() {
  const pathname = usePathname();

  return (
    <div className="flex h-full w-64 flex-col border-r bg-card px-3 py-4">
      <div className="mb-8 flex items-center px-3">
        <h1 className="text-xl font-bold tracking-tight">RampOS Admin</h1>
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
          >
            <item.icon className="h-4 w-4" />
            {item.title}
          </Link>
        ))}
      </nav>
      <div className="border-t pt-4">
        <div className="px-3 py-2">
          <p className="text-xs text-muted-foreground">Logged in as Admin</p>
        </div>
      </div>
    </div>
  );
}
