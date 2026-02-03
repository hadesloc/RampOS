import { Badge } from "@/components/ui/badge"
import { ArrowDownToLine, ArrowUpFromLine, RefreshCw, ArrowRightLeft } from "lucide-react"
import { formatDistanceToNow } from "date-fns"
import { cn } from "@/lib/utils"

interface TransactionRowProps {
  id: string;
  type: 'PAYIN_VND' | 'PAYOUT_VND' | 'TRADE_EXECUTED' | string;
  amount: string;
  currency: string;
  status: string;
  createdAt: string;
  onClick?: () => void;
}

export function TransactionRow({ id, type, amount, currency, status, createdAt, onClick }: TransactionRowProps) {
    const getIcon = () => {
        if (type.includes('PAYIN')) return <ArrowDownToLine className="h-4 w-4 text-green-500" />;
        if (type.includes('PAYOUT')) return <ArrowUpFromLine className="h-4 w-4 text-red-500" />;
        if (type.includes('TRADE')) return <ArrowRightLeft className="h-4 w-4 text-blue-500" />;
        return <RefreshCw className="h-4 w-4 text-muted-foreground" />;
    };

    const getStatusColor = (status: string) => {
        switch (status.toLowerCase()) {
            case 'completed':
            case 'success':
                return "bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20";
            case 'pending':
            case 'processing':
                return "bg-yellow-500/10 text-yellow-600 dark:text-yellow-400 border-yellow-500/20";
            case 'failed':
            case 'rejected':
            case 'cancelled':
                return "bg-red-500/10 text-red-600 dark:text-red-400 border-red-500/20";
            default:
                return "bg-muted text-muted-foreground";
        }
    };

    const formatAmount = (val: string, cur: string) => {
        const num = parseFloat(val);
        if (isNaN(num)) return val;
        const safeCurrency = cur === 'USDT' ? 'USD' : cur;
        return new Intl.NumberFormat(cur === 'VND' ? 'vi-VN' : 'en-US', {
            style: 'currency',
            currency: safeCurrency,
            maximumFractionDigits: cur === 'VND' ? 0 : 2
        }).format(num);
    };

    let dateObj: Date;
    try {
        dateObj = new Date(createdAt);
        if (isNaN(dateObj.getTime())) {
             dateObj = new Date();
        }
    } catch (e) {
        dateObj = new Date();
    }

    return (
        <div
            className={cn(
                "flex items-center justify-between p-4 rounded-lg border bg-card/50 hover:bg-accent/50 transition-colors",
                onClick ? "cursor-pointer" : "cursor-default"
            )}
            onClick={onClick}
        >
            <div className="flex items-center gap-4">
                <div className={cn("p-2.5 rounded-full bg-background border shadow-sm")}>
                    {getIcon()}
                </div>
                <div>
                    <div className="font-medium text-sm">
                        {type.replace(/_/g, ' ')}
                    </div>
                    <div className="text-xs text-muted-foreground">
                        {formatDistanceToNow(dateObj, { addSuffix: true })}
                    </div>
                </div>
            </div>

            <div className="flex flex-col items-end gap-1">
                <span className={cn(
                    "font-semibold text-sm",
                    type.includes('PAYIN') ? "text-green-600 dark:text-green-400" : "text-foreground"
                )}>
                    {type.includes('PAYIN') ? '+' : type.includes('PAYOUT') ? '-' : ''}
                    {formatAmount(amount, currency)}
                </span>
                <Badge variant="outline" className={cn("text-[10px] h-5 px-1.5 font-normal", getStatusColor(status))}>
                    {status}
                </Badge>
            </div>
        </div>
    )
}
