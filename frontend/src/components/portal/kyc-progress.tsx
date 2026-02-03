import { Check, Clock, ShieldCheck, AlertCircle } from "lucide-react"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"

interface KYCProgressProps {
  currentStep: number;
  steps: { label: string; completed: boolean }[];
  status: 'NONE' | 'PENDING' | 'VERIFIED' | 'REJECTED';
}

export function KYCProgress({ currentStep, steps, status }: KYCProgressProps) {

    const getStatusContent = () => {
        switch (status) {
            case 'VERIFIED':
                return {
                    icon: <ShieldCheck className="h-3.5 w-3.5" />,
                    text: "Verified",
                    color: "bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20"
                };
            case 'REJECTED':
                return {
                    icon: <AlertCircle className="h-3.5 w-3.5" />,
                    text: "Rejected",
                    color: "bg-red-500/10 text-red-600 dark:text-red-400 border-red-500/20"
                };
            case 'PENDING':
                return {
                    icon: <Clock className="h-3.5 w-3.5" />,
                    text: "Pending",
                    color: "bg-yellow-500/10 text-yellow-600 dark:text-yellow-400 border-yellow-500/20"
                };
            default:
                 return {
                    icon: <AlertCircle className="h-3.5 w-3.5" />,
                    text: "Not Started",
                    color: "bg-muted text-muted-foreground"
                };
        }
    }

    const statusInfo = getStatusContent();

    return (
        <Card>
            <CardHeader className="flex flex-row items-center justify-between pb-2 space-y-0">
                <CardTitle className="text-sm font-medium">Identity Verification</CardTitle>
                <Badge variant="outline" className={cn("gap-1.5 pl-2 pr-2.5 py-0.5", statusInfo.color)}>
                    {statusInfo.icon}
                    <span>{statusInfo.text}</span>
                </Badge>
            </CardHeader>
            <CardContent className="pt-8 pb-8">
                 <div className="relative flex items-center justify-between w-full px-2">
                    {/* Line behind steps */}
                    <div className="absolute left-0 top-4 w-full h-0.5 bg-muted -z-10" />
                    <div
                        className="absolute left-0 top-4 h-0.5 bg-primary -z-10 transition-all duration-500 ease-in-out"
                        style={{ width: `${Math.min(100, (Math.max(0, currentStep - 1) / (Math.max(1, steps.length - 1))) * 100)}%` }}
                    />

                    {steps.map((step, index) => {
                        const isDone = index + 1 < currentStep || step.completed;
                        const isActive = index + 1 === currentStep;

                        return (
                            <div key={index} className="flex flex-col items-center relative">
                                <div
                                    className={cn(
                                        "w-8 h-8 rounded-full flex items-center justify-center border-2 transition-all duration-300 bg-background",
                                        isDone ? "bg-primary border-primary text-primary-foreground" :
                                        isActive ? "border-primary text-primary shadow-[0_0_0_4px_rgba(0,0,0,0.05)] dark:shadow-[0_0_0_4px_rgba(255,255,255,0.05)]" :
                                        "border-muted text-muted-foreground"
                                    )}
                                >
                                    {isDone ? <Check className="h-4 w-4" /> : <span className="text-xs font-semibold">{index + 1}</span>}
                                </div>
                                <span className={cn(
                                    "absolute top-10 w-32 text-center text-xs font-medium transition-colors",
                                    isActive ? "text-foreground" : "text-muted-foreground"
                                )}>
                                    {step.label}
                                </span>
                            </div>
                        )
                    })}
                 </div>
            </CardContent>
        </Card>
    )
}
