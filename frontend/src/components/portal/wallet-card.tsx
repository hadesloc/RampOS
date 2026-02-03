import { Card, CardContent } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Copy, Wallet, Check } from "lucide-react"
import { cn } from "@/lib/utils"
import { useState } from "react"

interface WalletCardProps {
  address: string;
  deployed: boolean;
  onCopy?: () => void;
  loading?: boolean;
}

export function WalletCard({ address, deployed, onCopy, loading }: WalletCardProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    if (onCopy) {
      onCopy();
    } else {
      navigator.clipboard.writeText(address);
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const truncatedAddress = address ? `${address.slice(0, 6)}...${address.slice(-4)}` : '';

  if (loading) {
    return (
        <Card className="w-full h-[140px] animate-pulse bg-muted/50 border-none" />
    )
  }

  return (
    <Card variant="gradient" isHoverable className="overflow-hidden">
      <CardContent className="p-6">
        <div className="flex justify-between items-start mb-4">
            <div className="p-2.5 bg-primary/10 rounded-xl dark:bg-primary/20">
                <Wallet className="h-5 w-5 text-primary" />
            </div>
            <Badge
                variant={deployed ? "default" : "secondary"}
                className={cn(
                    "font-medium",
                    deployed
                        ? "bg-green-500/15 text-green-600 dark:text-green-400 hover:bg-green-500/25 border-green-500/20"
                        : "bg-muted text-muted-foreground hover:bg-muted/80"
                )}
            >
                {deployed ? "Deployed" : "Not Deployed"}
            </Badge>
        </div>

        <div className="space-y-1.5">
            <p className="text-xs text-muted-foreground font-medium uppercase tracking-wider">Wallet Address</p>
            <div className="flex items-center gap-2">
                <span className="text-xl font-bold tracking-tight font-mono text-foreground">{truncatedAddress}</span>
                <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-muted-foreground hover:text-foreground hover:bg-muted/50"
                    onClick={handleCopy}
                >
                    {copied ? <Check className="h-4 w-4 text-green-500" /> : <Copy className="h-4 w-4" />}
                    <span className="sr-only">Copy address</span>
                </Button>
            </div>
        </div>
      </CardContent>
    </Card>
  )
}
