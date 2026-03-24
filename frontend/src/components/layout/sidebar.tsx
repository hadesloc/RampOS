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
  KeyRound,
  Gavel,
  Droplets,
  FlaskConical,
  Banknote,
  FileText,
  FileCheck2,
  ShieldCheck,
  Landmark,
  Radio,
  AlertTriangle,
  Activity,
  Scale,
  UserCheck,
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
import { NotificationCenter } from "@/components/layout/notification-center";

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
        { title: t('dashboard'), href: "/", icon: LayoutDashboard },
      ],
    },
    {
      title: "Operations",
      items: [
        { title: "Intents", href: "/intents", icon: ArrowLeftRight },
        { title: t('users'), href: "/users", icon: Users },
        { title: "Offramp", href: "/offramp", icon: Banknote },
        { title: "Onboarding", href: "/onboarding", icon: UserCheck },
        { title: "Settlement", href: "/settlement", icon: Scale },
      ],
    },
    {
      title: "Compliance",
      items: [
        { title: t('compliance'), href: "/compliance", icon: ShieldAlert },
        { title: "Venue Review", href: "/venue", icon: ShieldAlert },
        { title: "Risk Lab", href: "/risk-lab", icon: FlaskConical },
        { title: "Risk", href: "/risk", icon: Activity },
        { title: "Fraud", href: "/fraud", icon: AlertTriangle },
        { title: "Reports", href: "/reports", icon: FileText },
        { title: "Documents", href: "/documents", icon: FileCheck2 },
      ],
    },
    {
      title: "Finance",
      items: [
        { title: "Ledger", href: "/ledger", icon: BookOpen },
        { title: "Reconciliation", href: "/reconciliation", icon: ShieldCheck },
        { title: "Treasury", href: "/treasury", icon: Landmark },
        { title: "Limits", href: "/limits", icon: Banknote },
      ],
    },
    {
      title: "Marketplace",
      items: [
        { title: "RFQ Auctions", href: "/rfq", icon: Gavel },
        { title: "Liquidity", href: "/liquidity", icon: Droplets },
        { title: "Sandbox", href: "/sandbox", icon: FlaskConical },
      ],
    },
    {
      title: "DeFi",
      items: [
        { title: "Swap", href: "/swap", icon: RefreshCw },
        { title: "Bridge", href: "/bridge", icon: Network },
        { title: "Yield", href: "/yield", icon: TrendingUp },
        { title: "Custody", href: "/custody", icon: KeyRound },
      ],
    },
    {
      title: "System",
      items: [
        { title: "Webhooks", href: "/webhooks", icon: Webhook },
        { title: "Events", href: "/events", icon: Radio },
        { title: "Incidents", href: "/incidents", icon: AlertTriangle },
        { title: "Monitoring", href: "/monitoring", icon: Activity },
        { title: "Licensing", href: "/licensing", icon: KeyRound },
        { title: t('settings'), href: "/settings", icon: Settings },
      ],
    },
  ];

  return (
    <TooltipProvider delayDuration={0}>
      {/* Mobile Toggle */}
      <div className="md:hidden fixed top-4 left-4 z-50 flex items-center gap-2">
        <Button variant="outline" size="icon" onClick={() => setIsOpen(!isOpen)} aria-label="Toggle navigation menu" className="border-white/[0.06] bg-[#111113] hover:bg-white/5">
          <Menu className="h-4 w-4" />
        </Button>
        <div className="bg-[#111113]/80 backdrop-blur-sm rounded-md border border-white/[0.06] shadow-sm p-0.5">
          <NotificationCenter />
        </div>
      </div>

      {/* Mobile Overlay */}
      {isOpen && (
        <div
          className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm md:hidden"
          onClick={() => setIsOpen(false)}
        />
      )}

      <aside
        className={cn(
          "fixed inset-y-0 left-0 z-50 flex h-full flex-col border-r border-white/[0.06] bg-[#0A0A0C] transition-all duration-300 ease-in-out md:static",
          isOpen ? "translate-x-0" : "-translate-x-full md:translate-x-0",
          isCollapsed ? "md:w-[72px]" : "md:w-[260px]",
          "w-[260px]"
        )}
      >
        {/* Logo area */}
        <div className={cn("flex items-center h-16 px-4", isCollapsed ? "md:justify-center justify-between" : "justify-between")}>
          <div className={cn("flex items-center gap-2.5 transition-opacity", isCollapsed ? "md:hidden" : "block")}>
            <span className="relative flex h-2 w-2 shrink-0">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#00FF87] opacity-75" />
              <span className="relative inline-flex rounded-full h-2 w-2 bg-[#00FF87]" />
            </span>
            <h1 className="text-lg font-bold tracking-tight text-white truncate">
              RAMP·OS
            </h1>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              className="md:hidden text-muted-foreground hover:text-white"
              onClick={() => setIsOpen(false)}
              aria-label="Close navigation menu"
            >
              <X className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className={cn("hidden md:flex h-7 w-7 text-muted-foreground hover:text-white hover:bg-white/5", isCollapsed && "h-7 w-7")}
              onClick={() => setIsCollapsed(!isCollapsed)}
              aria-label={isCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            >
              {isCollapsed ? (
                <ChevronRight className="h-3.5 w-3.5" />
              ) : (
                <ChevronLeft className="h-3.5 w-3.5" />
              )}
              <span className="sr-only">Toggle Sidebar</span>
            </Button>
          </div>
        </div>

        {/* Gradient separator */}
        <div className="h-[1px] bg-gradient-to-r from-transparent via-white/10 to-transparent mx-2" />

        <div className="flex-1 overflow-y-auto py-4">
          <nav className="space-y-5 px-2">
            {sidebarSections.map((section, index) => (
              <div key={section.title}>
                <h2 className={cn(
                  "mb-2 px-3 text-[10px] font-semibold uppercase tracking-[0.15em] text-muted-foreground/50 transition-opacity",
                  isCollapsed ? "md:hidden" : "block"
                )}>
                  {section.title}
                </h2>
                <div className="space-y-0.5">
                  {section.items.map((item) => {
                    const isActive = pathname === item.href;

                    return (
                      <div key={item.href}>
                        {/* Desktop Collapsed */}
                        <div className={isCollapsed ? "hidden md:block" : "hidden"}>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Link
                                href={item.href}
                                className={cn(
                                  "flex h-9 w-full items-center justify-center rounded-lg transition-all duration-200",
                                  isActive
                                    ? "bg-[#00FF87]/10 text-[#00FF87] glow-green-sm"
                                    : "text-muted-foreground hover:bg-white/5 hover:text-white"
                                )}
                              >
                                <item.icon className="h-4.5 w-4.5" />
                                <span className="sr-only">{item.title}</span>
                              </Link>
                            </TooltipTrigger>
                            <TooltipContent side="right" className="font-medium bg-[#111113] border-white/[0.06]">
                              {item.title}
                            </TooltipContent>
                          </Tooltip>
                        </div>

                        {/* Standard */}
                        <div className={cn("block", isCollapsed && "md:hidden")}>
                          <Link
                            href={item.href}
                            className={cn(
                              "group flex items-center gap-3 rounded-lg px-3 py-2 text-[13px] font-medium transition-all duration-200",
                              isActive
                                ? "bg-[#00FF87]/8 text-[#00FF87] border-l-2 border-[#00FF87] rounded-l-none glow-green-sm"
                                : "text-muted-foreground hover:bg-white/[0.03] hover:text-white"
                            )}
                            onClick={() => setIsOpen(false)}
                          >
                            <item.icon className={cn("h-4 w-4 transition-colors", isActive ? "text-[#00FF87]" : "text-muted-foreground group-hover:text-white")} />
                            {item.title}
                          </Link>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            ))}
          </nav>
        </div>

        {/* Footer */}
        <div className="border-t border-white/[0.06] p-3 bg-[#0A0A0C]">
          {/* Desktop Collapsed Footer */}
          <div className={isCollapsed ? "hidden md:flex flex-col items-center gap-3" : "hidden"}>
            <NotificationCenter />
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex justify-center cursor-pointer">
                  <div className="h-8 w-8 rounded-full bg-[#00FF87]/10 flex items-center justify-center text-xs font-bold text-[#00FF87] ring-1 ring-[#00FF87]/20">
                    A
                  </div>
                </div>
              </TooltipTrigger>
              <TooltipContent side="right" className="bg-[#111113] border-white/[0.06]">
                <p className="font-medium">Administrator</p>
                <p className="text-xs text-muted-foreground">admin@rampos.io</p>
              </TooltipContent>
            </Tooltip>
          </div>

          {/* Standard Footer */}
          <div className={cn("flex flex-col gap-3", isCollapsed && "md:hidden")}>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2.5">
                <div className="h-8 w-8 rounded-full bg-[#00FF87]/10 flex items-center justify-center text-xs font-bold text-[#00FF87] ring-1 ring-[#00FF87]/20">
                  A
                </div>
                <div className="flex flex-col overflow-hidden">
                  <p className="text-sm font-medium truncate text-white">Administrator</p>
                  <p className="text-[11px] text-muted-foreground truncate">admin@rampos.io</p>
                </div>
              </div>
              <NotificationCenter />
            </div>
            <LocaleSwitcher />
          </div>
        </div>
      </aside>
    </TooltipProvider>
  );
}
