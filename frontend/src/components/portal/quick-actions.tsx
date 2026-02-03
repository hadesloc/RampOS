import React from "react"
import { Card, CardContent } from "@/components/ui/card"
import { cn } from "@/lib/utils"
import Link from "next/link"

interface QuickAction {
    label: string;
    icon: React.ReactNode;
    href: string;
    variant?: 'default' | 'outline' | 'secondary' | 'ghost' | 'destructive';
}

interface QuickActionsProps {
  actions: QuickAction[];
}

export function QuickActions({ actions }: QuickActionsProps) {
  if (!actions || actions.length === 0) return null;

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
      {actions.map((action, index) => (
        <Link key={index} href={action.href} className="w-full">
            <Card className="hover:shadow-md transition-all hover:-translate-y-0.5 border-dashed hover:border-solid cursor-pointer h-full">
                <CardContent className="p-4 flex flex-col items-center justify-center gap-3 text-center h-full">
                    <div className={cn(
                        "p-3 rounded-full transition-colors",
                        action.variant === 'default' ? "bg-primary/10 text-primary" :
                        "bg-muted text-muted-foreground"
                    )}>
                        {action.icon}
                    </div>
                    <span className="font-medium text-sm">{action.label}</span>
                </CardContent>
            </Card>
        </Link>
      ))}
    </div>
  )
}
