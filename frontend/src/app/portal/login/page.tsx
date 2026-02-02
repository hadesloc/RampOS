"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Label } from "@/components/ui/label"
import { startAuthentication } from "@/lib/webauthn"
import Link from "next/link"
import { Fingerprint, Mail, ArrowRight, Loader2 } from "lucide-react"

export default function LoginPage() {
  const [email, setEmail] = useState("")
  const [loading, setLoading] = useState(false)
  const [mode, setMode] = useState<"passkey" | "magic-link">("passkey")

  const handlePasskeyLogin = async () => {
    setLoading(true)
    try {
      await startAuthentication(email)
      // success logic here
    } catch (error) {
      console.error(error)
    } finally {
      setLoading(false)
    }
  }

  const handleMagicLinkLogin = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    try {
      console.log("Sending magic link to", email)
      // mock magic link
      await new Promise(resolve => setTimeout(resolve, 1000))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4">
      <Card className="w-full max-w-md">
        <CardHeader className="space-y-1">
          <CardTitle className="text-2xl font-bold tracking-tight">Welcome back</CardTitle>
          <CardDescription>
            Sign in to your RampOS account
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {mode === "passkey" ? (
            <div className="space-y-4">
              <Button
                className="w-full h-12 text-base"
                size="lg"
                onClick={handlePasskeyLogin}
                disabled={loading}
              >
                {loading ? (
                  <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                ) : (
                  <Fingerprint className="mr-2 h-5 w-5" />
                )}
                Sign in with Passkey
              </Button>
              <div className="relative">
                <div className="absolute inset-0 flex items-center">
                  <span className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs uppercase">
                  <span className="bg-background px-2 text-muted-foreground">Or</span>
                </div>
              </div>
              <Button
                variant="outline"
                className="w-full"
                onClick={() => setMode("magic-link")}
              >
                <Mail className="mr-2 h-4 w-4" />
                Continue with Email
              </Button>
            </div>
          ) : (
            <form onSubmit={handleMagicLinkLogin} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="email">Email</Label>
                <Input
                  id="email"
                  type="email"
                  placeholder="m@example.com"
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </div>
              <Button className="w-full" type="submit" disabled={loading}>
                {loading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                Send Magic Link
                {!loading && <ArrowRight className="ml-2 h-4 w-4" />}
              </Button>
              <Button
                variant="ghost"
                className="w-full"
                type="button"
                onClick={() => setMode("passkey")}
              >
                Back to Passkey Login
              </Button>
            </form>
          )}
        </CardContent>
        <CardFooter className="flex flex-col space-y-2">
          <div className="text-sm text-muted-foreground text-center">
            Don&apos;t have an account?{" "}
            <Link href="/portal/register" className="text-primary hover:underline">
              Create an account
            </Link>
          </div>
        </CardFooter>
      </Card>
    </div>
  )
}
