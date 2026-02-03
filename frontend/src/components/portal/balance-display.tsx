import { Card, CardContent, CardTitle } from "@/components/ui/card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Skeleton } from "@/components/ui/skeleton"
import { Lock, Unlock } from "lucide-react"

interface BalanceDisplayProps {
  balances: { currency: string; total: string; available: string; locked: string }[];
  loading?: boolean;
}

export function BalanceDisplay({ balances, loading }: BalanceDisplayProps) {
  if (loading) {
    return (
      <Card className="w-full h-[220px]">
        <div className="p-6 pb-0 flex justify-between items-center">
          <Skeleton className="h-4 w-24" />
          <Skeleton className="h-8 w-32" />
        </div>
        <CardContent className="p-6 pt-4 space-y-6">
          <Skeleton className="h-10 w-48" />
          <div className="grid grid-cols-2 gap-4">
            <Skeleton className="h-20 w-full" />
            <Skeleton className="h-20 w-full" />
          </div>
        </CardContent>
      </Card>
    );
  }

  const formatCurrency = (value: string, currency: string) => {
    const num = parseFloat(value);
    if (isNaN(num)) return value;

    const safeCurrency = currency === 'USDT' ? 'USD' : currency;
    return new Intl.NumberFormat(currency === 'VND' ? 'vi-VN' : 'en-US', {
      style: 'currency',
      currency: safeCurrency,
      maximumFractionDigits: currency === 'VND' ? 0 : 2
    }).format(num);
  };

  if (!balances || balances.length === 0) {
      return (
          <Card>
              <CardContent className="p-6 text-center text-muted-foreground">
                  No balance information available
              </CardContent>
          </Card>
      )
  }

  return (
    <Card>
      <Tabs defaultValue={balances[0]?.currency} className="w-full">
        <div className="p-6 pb-0 flex items-center justify-between">
            <CardTitle className="text-sm font-medium text-muted-foreground">Total Balance</CardTitle>
            <TabsList className="h-8">
                {balances.map((balance) => (
                    <TabsTrigger key={balance.currency} value={balance.currency} className="text-xs px-2 h-6">
                        {balance.currency}
                    </TabsTrigger>
                ))}
            </TabsList>
        </div>

        {balances.map((balance) => (
            <TabsContent key={balance.currency} value={balance.currency} className="mt-0">
                 <CardContent className="p-6 pt-4 space-y-6">
                    <div>
                        <div className="text-4xl font-bold tracking-tight">
                            {formatCurrency(balance.total, balance.currency)}
                        </div>
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                        <div className="p-4 rounded-xl bg-muted/30 border space-y-1">
                            <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                                <Unlock className="h-3 w-3" />
                                <span>Available</span>
                            </div>
                            <p className="text-lg font-semibold">{formatCurrency(balance.available, balance.currency)}</p>
                        </div>
                         <div className="p-4 rounded-xl bg-muted/30 border space-y-1">
                            <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                                <Lock className="h-3 w-3" />
                                <span>Locked</span>
                            </div>
                            <p className="text-lg font-semibold">{formatCurrency(balance.locked, balance.currency)}</p>
                        </div>
                    </div>
                 </CardContent>
            </TabsContent>
        ))}
      </Tabs>
    </Card>
  )
}
