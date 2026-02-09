import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Copy } from "lucide-react"
import { useState } from "react"

interface DepositCardProps {
  type?: 'VND' | 'CRYPTO';
  onTypeChange?: (type: 'VND' | 'CRYPTO') => void;
  instructions?: React.ReactNode;
  qrCode?: string;
  loading?: boolean;
  bankDetails?: {
      bankName: string;
      accountName: string;
      accountNumber: string;
      content: string;
  };
  walletAddress?: string;
  network?: string;
}

export function DepositCard({
    type = 'VND',
    onTypeChange,
    instructions,
    qrCode,
    loading,
    bankDetails,
    walletAddress,
    network
}: DepositCardProps) {
    const [activeType, setActiveType] = useState<'VND' | 'CRYPTO'>(type);

    const handleTypeChange = (val: string) => {
        const newType = val as 'VND' | 'CRYPTO';
        setActiveType(newType);
        onTypeChange?.(newType);
    }

    const CopyButton = ({ text }: { text: string }) => {
        const [copied, setCopied] = useState(false);
        const handleCopy = () => {
             navigator.clipboard.writeText(text);
             setCopied(true);
             setTimeout(() => setCopied(false), 2000);
        }
        return (
            <Button variant="ghost" size="icon" className="h-6 w-6 shrink-0" onClick={handleCopy} aria-label="Copy to clipboard">
                {copied ? <span className="text-green-500 dark:text-green-400 text-xs font-bold">✓</span> : <Copy className="h-3 w-3" />}
                <span className="sr-only">Copy</span>
            </Button>
        )
    }

    if (loading) {
        return <Card className="w-full h-[450px] animate-pulse bg-muted" />
    }

    return (
        <Card className="w-full">
            <CardHeader>
                <CardTitle>Deposit</CardTitle>
                <CardDescription>Select a payment method to deposit funds</CardDescription>
            </CardHeader>
            <CardContent>
                <Tabs value={activeType} onValueChange={handleTypeChange} className="w-full">
                    <TabsList className="grid w-full grid-cols-2 mb-6">
                        <TabsTrigger value="VND">Fiat (VND)</TabsTrigger>
                        <TabsTrigger value="CRYPTO">Crypto (USDT)</TabsTrigger>
                    </TabsList>

                    <div className="flex flex-col md:flex-row gap-6">
                        <div className="flex-1 space-y-4">
                            {activeType === 'VND' && bankDetails ? (
                                <div className="space-y-4">
                                    <div className="grid gap-1">
                                        <span className="text-xs text-muted-foreground font-medium uppercase">Bank Name</span>
                                        <p className="font-medium text-sm">{bankDetails.bankName}</p>
                                    </div>
                                    <div className="grid gap-1">
                                        <span className="text-xs text-muted-foreground font-medium uppercase">Account Name</span>
                                        <p className="font-medium text-sm">{bankDetails.accountName}</p>
                                    </div>
                                    <div className="grid gap-1">
                                        <span className="text-xs text-muted-foreground font-medium uppercase">Account Number</span>
                                        <div className="flex items-center gap-2">
                                            <p className="font-mono text-lg font-semibold bg-muted/50 px-2 py-1 rounded">{bankDetails.accountNumber}</p>
                                            <CopyButton text={bankDetails.accountNumber} />
                                        </div>
                                    </div>
                                    <div className="grid gap-1">
                                        <span className="text-xs text-muted-foreground font-medium uppercase">Transfer Content</span>
                                        <div className="flex items-center gap-2">
                                            <p className="font-mono font-medium text-primary bg-primary/10 px-2 py-1 rounded border border-primary/20">{bankDetails.content}</p>
                                            <CopyButton text={bankDetails.content} />
                                        </div>
                                        <p className="text-[10px] text-muted-foreground mt-1">
                                            <span className="text-red-500 dark:text-red-400 font-bold">*IMPORTANT:</span> You must include this exact content in your bank transfer description.
                                        </p>
                                    </div>
                                </div>
                            ) : activeType === 'CRYPTO' && walletAddress ? (
                                <div className="space-y-4">
                                     <div className="grid gap-1">
                                        <span className="text-xs text-muted-foreground font-medium uppercase">Network</span>
                                        <p className="font-medium text-sm">{network || 'TRC20'}</p>
                                    </div>
                                    <div className="grid gap-1">
                                        <span className="text-xs text-muted-foreground font-medium uppercase">Wallet Address</span>
                                        <div className="flex items-center gap-2 break-all bg-muted/50 p-2 rounded border">
                                            <p className="font-mono text-sm w-full truncate">{walletAddress}</p>
                                        </div>
                                         <div className="flex justify-end">
                                            <Button variant="outline" size="sm" className="gap-2 h-8" onClick={() => navigator.clipboard.writeText(walletAddress)} aria-label="Copy wallet address">
                                                <Copy className="h-3.5 w-3.5" /> Copy Address
                                            </Button>
                                        </div>
                                    </div>
                                </div>
                            ) : (
                                <div className="text-center text-muted-foreground py-8">
                                    Loading details...
                                </div>
                            )}

                             {instructions && (
                                <div className="mt-6 pt-4 border-t text-sm text-muted-foreground">
                                    {instructions}
                                </div>
                            )}
                        </div>

                        {qrCode && (
                            <div className="flex flex-col items-center justify-start pt-2">
                                <div className="p-3 border rounded-xl bg-white">
                                    <img src={qrCode} alt="QR Code" className="w-32 h-32 md:w-40 md:h-40 object-contain mix-blend-multiply" />
                                </div>
                                <p className="text-xs text-muted-foreground mt-3 font-medium text-center">Scan to Pay</p>
                            </div>
                        )}
                    </div>
                </Tabs>
            </CardContent>
        </Card>
    )
}
