"use client"

import { useState } from "react"
import { zodResolver } from "@hookform/resolvers/zod"
import { useForm } from "react-hook-form"
import * as z from "zod"
import { toast } from "sonner"
import { ArrowRight, ShieldCheck } from "lucide-react"

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
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form"
import { Input } from "@/components/ui/input"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

const vndWithdrawSchema = z.object({
  bankName: z.string().min(1, "Please select a bank"),
  accountNumber: z.string().min(5, "Account number must be at least 5 digits"),
  accountName: z.string().min(2, "Account name is required"),
  amount: z.string().refine((val) => !isNaN(Number(val)) && Number(val) >= 50000, {
    message: "Minimum withdrawal is 50,000 VND",
  }),
})

const cryptoWithdrawSchema = z.object({
  network: z.string().min(1, "Please select a network"),
  address: z.string().min(10, "Invalid address"),
  amount: z.string().refine((val) => !isNaN(Number(val)) && Number(val) >= 10, {
    message: "Minimum withdrawal is 10 USDT",
  }),
  otp: z.string().length(6, "OTP must be 6 digits"),
})

export default function WithdrawPage() {
  const [isConfirming, setIsConfirming] = useState(false)

  const vndForm = useForm<z.infer<typeof vndWithdrawSchema>>({
    resolver: zodResolver(vndWithdrawSchema),
    defaultValues: {
      bankName: "",
      accountNumber: "",
      accountName: "",
      amount: "",
    },
  })

  const cryptoForm = useForm<z.infer<typeof cryptoWithdrawSchema>>({
    resolver: zodResolver(cryptoWithdrawSchema),
    defaultValues: {
      network: "",
      address: "",
      amount: "",
      otp: "",
    },
  })

  function onVndSubmit(values: z.infer<typeof vndWithdrawSchema>) {
    setIsConfirming(true)
    // Simulate API call
    setTimeout(() => {
      setIsConfirming(false)
      toast.success(`Withdrawal request for ${values.amount} VND submitted`)
      vndForm.reset()
    }, 1500)
  }

  function onCryptoSubmit(values: z.infer<typeof cryptoWithdrawSchema>) {
    setIsConfirming(true)
    // Simulate API call
    setTimeout(() => {
      setIsConfirming(false)
      toast.success(`Withdrawal request for ${values.amount} USDT submitted`)
      cryptoForm.reset()
    }, 1500)
  }

  return (
    <div className="container max-w-2xl py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold">Withdraw</h1>
        <p className="text-muted-foreground">
          Withdraw funds to your bank account or crypto wallet.
        </p>
      </div>

      <Tabs defaultValue="vnd" className="w-full">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="vnd">VND Withdrawal</TabsTrigger>
          <TabsTrigger value="crypto">Crypto Withdrawal</TabsTrigger>
        </TabsList>

        <TabsContent value="vnd">
          <Card>
            <CardHeader>
              <CardTitle>Withdraw to Bank</CardTitle>
              <CardDescription>
                Withdrawal to local bank account. Processing time: 5-15 minutes.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Form {...vndForm}>
                <form onSubmit={vndForm.handleSubmit(onVndSubmit)} className="space-y-4">
                  <FormField
                    control={vndForm.control}
                    name="bankName"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Bank</FormLabel>
                        <Select onValueChange={field.onChange} defaultValue={field.value}>
                          <FormControl>
                            <SelectTrigger>
                              <SelectValue placeholder="Select bank" />
                            </SelectTrigger>
                          </FormControl>
                          <SelectContent>
                            <SelectItem value="vcb">Vietcombank</SelectItem>
                            <SelectItem value="tcb">Techcombank</SelectItem>
                            <SelectItem value="mb">MB Bank</SelectItem>
                            <SelectItem value="acb">ACB</SelectItem>
                          </SelectContent>
                        </Select>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <FormField
                    control={vndForm.control}
                    name="accountNumber"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Account Number</FormLabel>
                        <FormControl>
                          <Input placeholder="Enter account number" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <FormField
                    control={vndForm.control}
                    name="accountName"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Account Name</FormLabel>
                        <FormControl>
                          <Input placeholder="Enter account holder name" {...field} />
                        </FormControl>
                        <FormDescription>Must match your verified KYC name</FormDescription>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <FormField
                    control={vndForm.control}
                    name="amount"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Amount (VND)</FormLabel>
                        <FormControl>
                          <Input placeholder="Min 50,000" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <Button type="submit" className="w-full" disabled={isConfirming}>
                    {isConfirming ? "Processing..." : "Withdraw VND"}
                  </Button>
                </form>
              </Form>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="crypto">
          <Card>
            <CardHeader>
              <CardTitle>Withdraw Crypto</CardTitle>
              <CardDescription>
                Withdraw USDT to external wallet. Network fees apply.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Form {...cryptoForm}>
                <form onSubmit={cryptoForm.handleSubmit(onCryptoSubmit)} className="space-y-4">
                  <FormField
                    control={cryptoForm.control}
                    name="network"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Network</FormLabel>
                        <Select onValueChange={field.onChange} defaultValue={field.value}>
                          <FormControl>
                            <SelectTrigger>
                              <SelectValue placeholder="Select network" />
                            </SelectTrigger>
                          </FormControl>
                          <SelectContent>
                            <SelectItem value="trc20">TRC20 (Tron)</SelectItem>
                            <SelectItem value="erc20">ERC20 (Ethereum)</SelectItem>
                            <SelectItem value="bep20">BEP20 (BSC)</SelectItem>
                          </SelectContent>
                        </Select>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <FormField
                    control={cryptoForm.control}
                    name="address"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Wallet Address</FormLabel>
                        <FormControl>
                          <Input placeholder="Enter wallet address" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <FormField
                    control={cryptoForm.control}
                    name="amount"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Amount (USDT)</FormLabel>
                        <FormControl>
                          <Input placeholder="Min 10 USDT" {...field} />
                        </FormControl>
                        <FormDescription>
                          Estimated Gas Fee: ~1.00 USDT
                        </FormDescription>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <FormField
                    control={cryptoForm.control}
                    name="otp"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>2FA / OTP Code</FormLabel>
                        <FormControl>
                          <Input placeholder="000000" maxLength={6} {...field} />
                        </FormControl>
                        <FormDescription>
                           Enter code from your authenticator app
                        </FormDescription>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <div className="rounded-md bg-muted p-4">
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <ShieldCheck className="h-4 w-4" />
                      <span>Security Check: Withdrawal requires 2FA confirmation.</span>
                    </div>
                  </div>

                  <Button type="submit" className="w-full" disabled={isConfirming}>
                    {isConfirming ? "Processing..." : (
                      <span className="flex items-center gap-2">
                        Confirm Withdrawal <ArrowRight className="h-4 w-4" />
                      </span>
                    )}
                  </Button>
                </form>
              </Form>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}
