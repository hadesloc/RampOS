import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { useState } from "react"
import { Loader2 } from "lucide-react"

interface WithdrawCardProps {
  type?: 'VND' | 'CRYPTO';
  availableBalance: string;
  onSubmit?: (amount: string, destination: string) => void;
  loading?: boolean;
}

export function WithdrawCard({
    type = 'VND',
    availableBalance,
    onSubmit,
    loading
}: WithdrawCardProps) {
    const [activeType, setActiveType] = useState<'VND' | 'CRYPTO'>(type);
    const [amount, setAmount] = useState('');
    const [destination, setDestination] = useState('');
    const [isSubmitting, setIsSubmitting] = useState(false);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!amount || !destination) return;

        setIsSubmitting(true);
        try {
            await onSubmit?.(amount, destination);
        } finally {
            setIsSubmitting(false);
        }
    }

    const handleMax = () => {
        setAmount(availableBalance);
    }

    const formatCurrency = (val: string) => {
         const num = parseFloat(val);
         if (isNaN(num)) return val;
         return new Intl.NumberFormat(activeType === 'VND' ? 'vi-VN' : 'en-US', {
            style: 'currency',
            currency: activeType === 'VND' ? 'VND' : 'USD',
            maximumFractionDigits: activeType === 'VND' ? 0 : 2
         }).format(num);
    }

    if (loading) {
        return <Card className="w-full h-[380px] animate-pulse bg-muted" />
    }

    return (
         <Card className="w-full">
            <CardHeader>
                <CardTitle>Withdraw</CardTitle>
                <CardDescription>Withdraw funds to your account</CardDescription>
            </CardHeader>
            <CardContent>
                <Tabs value={activeType} onValueChange={(v) => setActiveType(v as 'VND' | 'CRYPTO')} className="w-full">
                    <TabsList className="grid w-full grid-cols-2 mb-6">
                        <TabsTrigger value="VND">Fiat (VND)</TabsTrigger>
                        <TabsTrigger value="CRYPTO">Crypto (USDT)</TabsTrigger>
                    </TabsList>

                    <form onSubmit={handleSubmit} className="space-y-6">
                        <div className="space-y-2">
                             <div className="flex justify-between items-center">
                                <Label htmlFor="amount">Amount</Label>
                                <span
                                    className="text-xs text-muted-foreground cursor-pointer hover:text-primary transition-colors flex items-center gap-1"
                                    onClick={handleMax}
                                    role="button"
                                    tabIndex={0}
                                    onKeyDown={(e) => e.key === 'Enter' && handleMax()}
                                >
                                    Available: <span className="font-medium text-foreground">{formatCurrency(availableBalance)}</span>
                                </span>
                            </div>
                            <div className="relative">
                                <Input
                                    id="amount"
                                    placeholder="0.00"
                                    value={amount}
                                    onChange={(e) => setAmount(e.target.value)}
                                    disabled={loading || isSubmitting}
                                    type="number"
                                    step="any"
                                    className="pr-12"
                                />
                                <Button
                                    type="button"
                                    variant="ghost"
                                    size="sm"
                                    className="absolute right-1 top-1 h-7 text-xs text-primary font-bold hover:bg-primary/10"
                                    onClick={handleMax}
                                    disabled={loading || isSubmitting}
                                    aria-label="Use maximum balance"
                                >
                                    MAX
                                </Button>
                            </div>
                        </div>

                        <div className="space-y-2">
                            <Label htmlFor="destination">
                                {activeType === 'VND' ? 'Bank Account Number' : 'Wallet Address'}
                            </Label>
                             <Input
                                id="destination"
                                placeholder={activeType === 'VND' ? 'Enter bank account number' : 'Enter wallet address'}
                                value={destination}
                                onChange={(e) => setDestination(e.target.value)}
                                disabled={loading || isSubmitting}
                            />
                        </div>

                         <Button type="submit" className="w-full" disabled={loading || isSubmitting || !amount || !destination}>
                            {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                            Withdraw {activeType === 'VND' ? 'VND' : 'USDT'}
                        </Button>
                    </form>
                </Tabs>
            </CardContent>
        </Card>
    )
}
