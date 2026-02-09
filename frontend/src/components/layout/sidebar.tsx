"use client";

import { usePathname } from "@/navigation";
import { Link } from "@/navigation";
import { useState } from "react";
import { cn } from "@/lib/utils";
import {
  LayoutDashboard,
  ArrowLeftRight,
  Users,
  ShieldAlert,
  BookOpen,
  Webhook,
  Settings,
  ChevronLeft,
  ChevronRight,
  Menu,
  X,
  RefreshCw,
  Network,
  TrendingUp,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useTranslations } from "next-intl";
import LocaleSwitcher from "@/components/locale-switcher";

export default function Sidebar() {
  const pathname = usePathname();
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isOpen, setIsOpen] = useState(false);
  const t = useTranslations('Navigation');
  const tDashboard = useTranslations('Dashboard');

  const sidebarSections = [
    {
      title: "Overview",
      items: [
        {
          title: t('dashboard'),
          href: "/",
          icon: LayoutDashboard,
        },
      ],
    },
    {
      title: "Operations",
      items: [
        {
          title: "Intents",
          href: "/intents",
          icon: ArrowLeftRight,
        },
        {
          title: t('users'),
          href: "/users",
          icon: Users,
        },
        {
          title: t('compliance'),
          href: "/compliance",
          icon: ShieldAlert,
        },
        {
          title: "Ledger",
          href: "/ledger",
          icon: BookOpen,
        },
      ],
    },
    {
      title: "DeFi",
      items: [
        {
          title: "Swap",
          href: "/swap",
          icon: RefreshCw,
        },
        {
          title: "Bridge",
          href: "/bridge",
          icon: Network,
        },
        {
          title: "Yield",
          href: "/yield",
          icon: TrendingUp,
        },
      ],
    },
    {
      title: "System",
      items: [
        {
          title: "Webhooks",
          href: "/webhooks",
          icon: Webhook,
        },
        {
          title: t('settings'),
          href: "/settings",
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
                            {/* Desktop Collapsed */}
                             <div className={cn("hidden md:block", !isCollapsed && "hidden")}>
                                <Tooltip>
                                    <TooltipTrigger asChild>
                                        <Link
                                        href={item.href}
                                        className={cn(
                                            "flex h-10 w-full items-center justify-center rounded-md transition-colors hover:bg-primary/5 hover:text-foreground",
                                            isActive
                                            ? "bg-primary/10 text-primary"
                                            : "text-muted-foreground"
                                        )}
                                        >
                                        <item.icon className="h-5 w-5" />
                                        <span className="sr-only">{item.title}</span>
                                        </Link>
                                    </TooltipTrigger>
                                    <TooltipContent side="right" className="font-medium">
                                        {item.title}
                                    </TooltipContent>
                                </Tooltip>
                             </div>

                             {/* Standard */}
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
             <div className={cn("hidden md:flex flex-col items-center gap-4", !isCollapsed && "hidden")}>
                 <Tooltip>
                    <TooltipTrigger asChild>
                        <div className="flex justify-center cursor-pointer">
                            <div className="h-9 w-9 rounded-full bg-primary/10 flex items-center justify-center text-xs font-bold text-primary ring-2 ring-background">
                                A
                            </div>
                        </div>
                    </TooltipTrigger>
                    <TooltipContent side="right">
                        <p className="font-medium">Administrator</p>
                        <p className="text-xs text-muted-foreground">admin@rampos.io</p>
                    </TooltipContent>
                </Tooltip>
             </div>

             {/* Standard Footer */}
              <div className={cn("flex flex-col gap-3", isCollapsed && "md:hidden")}>
                <div className="flex items-center gap-3">
                  <div className="h-9 w-9 rounded-full bg-primary/10 flex items-center justify-center text-sm font-bold text-primary ring-2 ring-background">
                    A
                  </div>
                  <div className="flex flex-col overflow-hidden">
                    <p className="text-sm font-medium truncate text-foreground">Administrator</p>
                    <p className="text-xs text-muted-foreground truncate">admin@rampos.io</p>
                  </div>
                </div>
                <LocaleSwitcher />
            </div>
        </div>
      </aside>
    </TooltipProvider>
  );
}
