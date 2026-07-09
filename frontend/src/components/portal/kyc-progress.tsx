import { Check } from "lucide-react"
import { cn } from "@/lib/utils"
import { Panel, StatusBadge } from "@/components/shared"

interface KYCProgressProps {
  currentStep: number
  steps: { label: string; completed: boolean }[]
  status: "NONE" | "PENDING" | "VERIFIED" | "REJECTED"
}

function kycSeverity(status: string): "success" | "warning" | "danger" | "neutral" {
  if (status === "VERIFIED") return "success"
  if (status === "REJECTED") return "danger"
  if (status === "PENDING") return "warning"
  return "neutral"
}

function kycLabel(status: string): string {
  if (status === "VERIFIED") return "Verified"
  if (status === "REJECTED") return "Rejected"
  if (status === "PENDING") return "Pending Review"
  return "Not Started"
}

export function KYCProgress({ currentStep, steps, status }: KYCProgressProps) {
  const progressPct =
    steps.length > 1
      ? (Math.max(0, currentStep - 1) / (steps.length - 1)) * 100
      : currentStep >= 1
      ? 100
      : 0

  return (
    <Panel
      variant="solid"
      header={{
        title: "Identity Verification",
        actions: <StatusBadge status={kycLabel(status)} severity={kycSeverity(status)} />,
      }}
    >
      <div className="relative flex items-center justify-between w-full px-2 pt-2 pb-10">
        {/* Background track */}
        <div className="absolute left-0 top-4 w-full h-0.5 bg-white/[0.08]" />
        {/* Progress fill */}
        <div
          className="absolute left-0 top-4 h-0.5 bg-[#00FF87] transition-all duration-500 ease-in-out"
          style={{ width: `${Math.min(100, progressPct)}%` }}
        />

        {steps.map((step, index) => {
          const isDone = index + 1 < currentStep || step.completed
          const isActive = index + 1 === currentStep
          return (
            <div key={index} className="flex flex-col items-center relative">
              <div
                className={cn(
                  "w-8 h-8 rounded-full flex items-center justify-center border-2 transition-all duration-300 bg-[#09090B]",
                  isDone
                    ? "bg-[#00FF87] border-[#00FF87] text-black"
                    : isActive
                    ? "border-[#00FF87] text-[#00FF87] shadow-[0_0_12px_rgba(0,255,135,0.3)]"
                    : "border-white/[0.12] text-muted-foreground"
                )}
              >
                {isDone ? (
                  <Check className="h-4 w-4" />
                ) : (
                  <span className="text-xs font-bold">{index + 1}</span>
                )}
              </div>
              <span
                className={cn(
                  "absolute top-10 w-28 text-center text-[10px] font-semibold transition-colors",
                  isActive ? "text-[#00FF87]" : isDone ? "text-muted-foreground" : "text-muted-foreground/60"
                )}
              >
                {step.label}
              </span>
            </div>
          )
        })}
      </div>
    </Panel>
  )
}
