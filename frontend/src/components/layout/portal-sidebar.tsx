"use client";

import { Link, usePathname } from "@/navigation";
import { useState } from "react";
import { cn } from "@/lib/utils";
import {
  LayoutDashboard,
  Wallet,
  ArrowDownToLine,
  ArrowUpFromLine,
  History,
  Settings,
  Menu,
  X,
  User,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useAuth } from "@/contexts/auth-context";
import { useTranslations } from "next-intl";
import LocaleSwitcher from "@/components/locale-switcher";

export function PortalSidebar() {
  const pathname = usePathname();
  const [isOpen, setIsOpen] = useState(false);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const { user } = useAuth();
  const t = useTranslations('Navigation');
  const tPortal = useTranslations('Portal.dashboard');

  const sidebarSections = [
    {
      title: t('overview'),
      items: [
        {
          title: t('dashboard'),
          href: "/portal",
          icon: LayoutDashboard,
        },
      ],
    },
    {
      title: t('finance'),
      items: [
        {
          title: t('assets'),
          href: "/portal/assets",
          icon: Wallet,
        },
        {
          title: t('deposit'),
          href: "/portal/deposit",
          icon: ArrowDownToLine,
        },
        {
          title: t('withdraw'),
          href: "/portal/withdraw",
          icon: ArrowUpFromLine,
        },
        {
          title: t('transactions'),
          href: "/portal/transactions",
          icon: History,
        },
      ],
    },
    {
      title: t('account'),
      items: [
        {
          title: t('settings'),
          href: "/portal/settings",
          icon: Settings,
        },
      ],
    },
  ];

  return (
    <TooltipProvider delayDuration={0}>
      {/* Mobile Toggle */}
      <div className="md:hidden fixed top-4 left-4 z-50">
        <Button variant="outline" size="icon" onClick={() => setIsOpen(!isOpen)} aria-label="Toggle navigation menu">
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
          "fixed inset-y-0 left-0 z-50 flex h-full flex-col border-r bg-card transition-all duration-300 ease-in-out md:static",
          isOpen ? "translate-x-0" : "-translate-x-full md:translate-x-0",
          isCollapsed ? "md:w-[80px]" : "md:w-64",
          "w-64"
        )}
      >
        <div className={cn("flex items-center h-16 px-4", isCollapsed ? "md:justify-center justify-between" : "justify-between")}>
          <h1 className={cn("text-xl font-bold tracking-tight text-foreground truncate transition-opacity", isCollapsed ? "md:hidden" : "block")}>
            RampOS
          </h1>
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              className="md:hidden"
              onClick={() => setIsOpen(false)}
              aria-label="Close navigation menu"
            >
              <X className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className={cn("hidden md:flex h-8 w-8 text-muted-foreground hover:text-foreground", isCollapsed && "h-8 w-8")}
              onClick={() => setIsCollapsed(!isCollapsed)}
              aria-label={isCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            >
              {isCollapsed ? (
                <ChevronRight className="h-4 w-4" />
              ) : (
                <ChevronLeft className="h-4 w-4" />
              )}
              <span className="sr-only">Toggle Sidebar</span>
            </Button>
          </div>
        </div>

        <Separator />

        <div className="flex-1 overflow-y-auto py-4">
          <nav className="space-y-6 px-2">
            {sidebarSections.map((section, index) => (
              <div key={section.title}>
                <h2 className={cn("mb-2 px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70 transition-opacity", isCollapsed ? "md:hidden" : "block")}>
                  {section.title}
                </h2>
                <div className="space-y-1">
                  {section.items.map((item) => {
                    const isActive = pathname === item.href;

                    return (
                      <div key={item.href}>
                        {/* Desktop Collapsed Item */}
                        <div className={isCollapsed ? "hidden md:block" : "hidden"}>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant="ghost"
                                size="icon"
                                className={cn(
                                  "h-10 w-10 rounded-md transition-colors hover:bg-primary/5 hover:text-foreground",
                                  isActive ? "bg-primary/10 text-primary" : "text-muted-foreground"
                                )}
                                asChild
                              >
                                <Link href={item.href}>
                                  <item.icon className="h-5 w-5" />
                                  <span className="sr-only">{item.title}</span>
                                </Link>
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent side="right" className="font-medium">
                              {item.title}
                            </TooltipContent>
                          </Tooltip>
                        </div>

                        {/* Standard Item (Mobile or Desktop Expanded) */}
                        <div className={cn("block", isCollapsed && "md:hidden")}>
                          <Link
                            href={item.href}
                            className={cn(
                              "group flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-all hover:bg-primary/5 hover:text-foreground",
                              isActive
                                ? "bg-primary/10 text-primary border-l-2 border-primary rounded-l-none shadow-sm"
                                : "text-muted-foreground hover:translate-x-1"
                            )}
                            onClick={() => setIsOpen(false)}
                          >
                            <item.icon className={cn("h-4 w-4 transition-colors", isActive ? "text-primary" : "text-muted-foreground group-hover:text-foreground")} />
                            {item.title}
                          </Link>
                        </div>
                      </div>
                    );
                  })}
                </div>
                {index < sidebarSections.length - 1 && (
                  <div className={cn("mt-4 px-2", isCollapsed ? "md:hidden" : "block")}>
                    <Separator className="bg-border/50" />
                  </div>
                )}
              </div>
            ))}
          </nav>
        </div>

        <div className="border-t p-4 bg-muted/20">
          {/* Desktop Collapsed Footer */}
          <div className={isCollapsed ? "hidden md:flex flex-col items-center gap-4" : "hidden"}>
            <LocaleSwitcher />
          </div>

          {/* Standard Footer */}
          <div className={cn("flex flex-col gap-4", isCollapsed && "md:hidden")}>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="h-9 w-9 rounded-full bg-primary/10 flex items-center justify-center text-sm font-bold text-primary ring-2 ring-background">
                  {user?.email?.[0].toUpperCase() || "U"}
                </div>
                <div className="flex flex-col overflow-hidden">
                  <p className="text-sm font-medium truncate text-foreground">My Account</p>
                  <p className="text-xs text-muted-foreground truncate">{user?.email}</p>
                </div>
              </div>
              <LocaleSwitcher />
            </div>
          </div>
        </div>
      </aside>
    </TooltipProvider>
  );
}
