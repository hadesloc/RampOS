"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";

const tabs = [
  { value: "general", label: "General", href: "/settings" },
  { value: "users", label: "Users & Roles", href: "/users" },
  { value: "branding", label: "Branding", href: "/settings/branding" },
  { value: "domains", label: "Domains", href: "/settings/domains" },
  { value: "sso", label: "SSO", href: "/settings/sso" },
  { value: "billing", label: "Billing", href: "/settings/billing" },
  { value: "audit", label: "Audit Logs", href: "/settings/audit" },
  { value: "api", label: "API Keys", href: "/settings/api" },
];

export default function SettingsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const pathname = usePathname();

  return (
    <div className="flex flex-col h-[calc(100vh-4rem)]">
      <div className="flex-none px-6 pt-6 pb-2">
        <div className="border-b">
          <div className="flex h-10 items-center overflow-x-auto">
             <nav className="flex items-center space-x-4">
                {tabs.map((tab) => {
                    const isActive = tab.href === "/settings"
                      ? pathname === "/settings"
                      : pathname.startsWith(tab.href);
                    return (
                        <Link
                            key={tab.value}
                            href={tab.href}
                            className={cn(
                                "text-sm font-medium transition-colors hover:text-primary pb-2 px-2 border-b-2",
                                isActive
                                    ? "border-primary text-primary"
                                    : "border-transparent text-muted-foreground hover:border-muted"
                            )}
                        >
                            {tab.label}
                        </Link>
                    );
                })}
             </nav>
          </div>
        </div>
      </div>
      <div className="flex-1 overflow-auto bg-muted/10">
        {children}
      </div>
    </div>
  );
}
