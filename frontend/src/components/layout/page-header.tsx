"use client";

import Link from "next/link";
import { ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";

interface PageHeaderProps {
  title: string;
  description?: string;
  breadcrumbs?: { label: string; href?: string }[];
  actions?: React.ReactNode;
  className?: string;
}

export function PageHeader({
  title,
  description,
  breadcrumbs,
  actions,
  className,
}: PageHeaderProps) {
  return (
    <div className={cn("flex flex-col gap-4 pb-2 md:flex-row md:items-center md:justify-between", className)}>
      <div className="flex flex-col gap-1.5">
        {breadcrumbs && breadcrumbs.length > 0 && (
          <nav aria-label="Breadcrumb" className="mb-2">
            <ol className="flex items-center gap-1 text-sm text-muted-foreground">
              {breadcrumbs.map((item, index) => {
                const isLast = index === breadcrumbs.length - 1;
                return (
                  <li key={item.label} className="flex items-center gap-1">
                    {index > 0 && <ChevronRight className="h-3.5 w-3.5 text-gray-600" />}
                    {item.href && !isLast ? (
                      <Link
                        href={item.href}
                        className="hover:text-foreground transition-colors"
                      >
                        {item.label}
                      </Link>
                    ) : (
                      <span className={cn(isLast && "font-medium text-foreground")}>
                        {item.label}
                      </span>
                    )}
                  </li>
                );
              })}
            </ol>
          </nav>
        )}
        <h1 className="text-2xl font-bold tracking-tight text-white sm:text-3xl">{title}</h1>
        {description && (
          <p className="text-sm text-gray-500">{description}</p>
        )}
      </div>
      {actions && <div className="flex items-center gap-2 mt-2 md:mt-0">{actions}</div>}
    </div>
  );
}
