import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { ArrowDownToLine, ArrowUpFromLine, RefreshCw } from "lucide-react"

export default function PortalPage() {
  return (
    <div className="space-y-8">
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Welcome back</h2>
          <p className="text-muted-foreground">
            Here's an overview of your account
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm">
            <RefreshCw className="mr-2 h-4 w-4" />
            Refresh
          </Button>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              Total Balance
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">0 VND</div>
            <p className="text-xs text-muted-foreground mt-1">
              ≈ $0.00 USD
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              Available
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">0 VND</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              Locked
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">0 VND</div>
          </CardContent>
        </Card>
      </div>

      <div>
        <h3 className="text-lg font-semibold mb-4">Quick Actions</h3>
        <div className="flex gap-4">
          <Button className="gap-2" size="lg">
            <ArrowDownToLine className="h-5 w-5" />
            Deposit
          </Button>
          <Button variant="outline" className="gap-2" size="lg">
            <ArrowUpFromLine className="h-5 w-5" />
            Withdraw
          </Button>
        </div>
      </div>
    </div>
  )
}
