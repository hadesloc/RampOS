import { ReactNode } from "react"
import Link from "next/link"
import { cn } from "@/lib/utils"

interface QuickAction {
  label: string
  icon: ReactNode
  href: string
  accentColor?: "green" | "violet" | "cyan" | "amber"
}

interface QuickActionsProps {
  actions: QuickAction[]
}

const accentMap: Record<string, { bg: string; icon: string; border: string; hover: string }> = {
  green:  { bg: "bg-[#00FF87]/10",  icon: "text-[#00FF87]",  border: "border-[#00FF87]/20",  hover: "hover:border-[#00FF87]/40 hover:bg-[#00FF87]/10" },
  violet: { bg: "bg-[#7B61FF]/10",  icon: "text-[#7B61FF]",  border: "border-[#7B61FF]/20",  hover: "hover:border-[#7B61FF]/40 hover:bg-[#7B61FF]/10" },
  cyan:   { bg: "bg-[#00D4FF]/10",  icon: "text-[#00D4FF]",  border: "border-[#00D4FF]/20",  hover: "hover:border-[#00D4FF]/40 hover:bg-[#00D4FF]/10" },
  amber:  { bg: "bg-[#FFB800]/10",  icon: "text-[#FFB800]",  border: "border-[#FFB800]/20",  hover: "hover:border-[#FFB800]/40 hover:bg-[#FFB800]/10" },
}

export function QuickActions({ actions }: QuickActionsProps) {
  if (!actions || actions.length === 0) return null

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
      {actions.map((action, index) => {
        const color = action.accentColor ?? "green"
        const accent = accentMap[color]
        return (
          <Link key={index} href={action.href} className="w-full group">
            <div
              className={cn(
                "flex flex-col items-center justify-center gap-3 p-5 rounded-xl border border-white/[0.06] bg-[#111113]/80 backdrop-blur-sm transition-all duration-200 cursor-pointer text-center h-full",
                accent.hover
              )}
            >
              <div className={cn("p-3 rounded-xl border transition-colors", accent.bg, accent.border)}>
                <span className={cn("block", accent.icon)}>{action.icon}</span>
              </div>
              <span className="text-xs font-semibold text-muted-foreground group-hover:text-foreground transition-colors">
                {action.label}
              </span>
            </div>
          </Link>
        )
      })}
    </div>
  )
}
