"use client"

import { useState } from "react"
import { zodResolver } from "@hookform/resolvers/zod"
import { useForm } from "react-hook-form"
import * as z from "zod"
import { Copy, Check, QrCode } from "lucide-react"
import { toast } from "sonner"

import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form"
import { Input } from "@/components/ui/input"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Label } from "@/components/ui/label"

const depositSchema = z.object({
  amount: z.string().refine((val) => !isNaN(Number(val)) && Number(val) > 0, {
    message: "Amount must be a positive number",
  }),
})

export default function DepositPage() {
  const [copiedField, setCopiedField] = useState<string | null>(null)

  const form = useForm<z.infer<typeof depositSchema>>({
    resolver: zodResolver(depositSchema),
    defaultValues: {
      amount: "",
    },
  })

  function onSubmit(values: z.infer<typeof depositSchema>) {
    toast.success(`Deposit request for ${values.amount} submitted`)
  }

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text)
    setCopiedField(field)
    toast.success("Copied to clipboard")
    setTimeout(() => setCopiedField(null), 2000)
  }

  const mockBankInfo = {
    bankName: "Vietcombank",
    accountName: "RAMPOS TRADING LTD",
    accountNumber: "1234567890",
    content: "RAMPOS DEPOSIT user123",
  }

  const mockCryptoInfo = {
    network: "TRC20",
    address: "T9yD14Nj9j7xAB4dbGeiX9h8unkkhxnXV",
  }

  return (
    <div className="container max-w-2xl py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold">Deposit</h1>
        <p className="text-muted-foreground">
          Add funds to your RampOS wallet using VND or Crypto.
        </p>
      </div>

      <Tabs defaultValue="vnd" className="w-full">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="vnd">VND Transfer</TabsTrigger>
          <TabsTrigger value="crypto">Crypto Deposit</TabsTrigger>
        </TabsList>

        <TabsContent value="vnd">
          <Card>
            <CardHeader>
              <CardTitle>Bank Transfer</CardTitle>
              <CardDescription>
                Transfer VND to the following bank account. Your balance will be updated automatically.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid gap-4 rounded-lg border p-4">
                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <p className="text-sm font-medium text-muted-foreground">Bank</p>
                    <p className="font-medium">{mockBankInfo.bankName}</p>
                  </div>
                </div>

                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <p className="text-sm font-medium text-muted-foreground">Account Name</p>
                    <p className="font-medium">{mockBankInfo.accountName}</p>
                  </div>
                </div>

                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <p className="text-sm font-medium text-muted-foreground">Account Number</p>
                    <p className="font-medium">{mockBankInfo.accountNumber}</p>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => copyToClipboard(mockBankInfo.accountNumber, "accountNumber")}
                  >
                    {copiedField === "accountNumber" ? (
                      <Check className="h-4 w-4" />
                    ) : (
                      <Copy className="h-4 w-4" />
                    )}
                  </Button>
                </div>

                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <p className="text-sm font-medium text-muted-foreground">Transfer Content</p>
                    <p className="font-mono font-medium">{mockBankInfo.content}</p>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => copyToClipboard(mockBankInfo.content, "content")}
                  >
                    {copiedField === "content" ? (
                      <Check className="h-4 w-4" />
                    ) : (
                      <Copy className="h-4 w-4" />
                    )}
                  </Button>
                </div>
              </div>

              <Form {...form}>
                <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                  <FormField
                    control={form.control}
                    name="amount"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Expected Amount (VND)</FormLabel>
                        <FormControl>
                          <Input placeholder="1,000,000" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <Button type="submit" className="w-full">
                    I have made the transfer
                  </Button>
                </form>
              </Form>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="crypto">
          <Card>
            <CardHeader>
              <CardTitle>Crypto Deposit</CardTitle>
              <CardDescription>
                Send USDT to the address below. Only USDT on {mockCryptoInfo.network} is supported.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="flex flex-col items-center justify-center space-y-4 p-4">
                <div className="flex h-48 w-48 items-center justify-center rounded-lg bg-muted">
                  <QrCode className="h-24 w-24 text-muted-foreground" />
                </div>
                <p className="text-xs text-muted-foreground">Scan QR to deposit</p>
              </div>

              <div className="space-y-2">
                <Label>Deposit Address ({mockCryptoInfo.network})</Label>
                <div className="flex space-x-2">
                  <Input value={mockCryptoInfo.address} readOnly />
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => copyToClipboard(mockCryptoInfo.address, "address")}
                  >
                    {copiedField === "address" ? (
                      <Check className="h-4 w-4" />
                    ) : (
                      <Copy className="h-4 w-4" />
                    )}
                  </Button>
                </div>
              </div>

              <div className="rounded-lg bg-yellow-500/10 p-4 text-sm text-yellow-600 dark:text-yellow-500">
                <strong>Important:</strong> Send only USDT to this deposit address. Sending any other coin or token to this address may result in the loss of your deposit.
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}
