import type { Metadata } from 'next'
import { Space_Grotesk, Inter } from 'next/font/google'
import './globals.css'
import { cn } from '@/lib/utils'

const fontDisplay = Space_Grotesk({
  subsets: ['latin'],
  weight: ['300', '400', '500', '600', '700'],
  variable: '--font-display',
})

const fontBody = Inter({
  subsets: ['latin'],
  weight: ['300', '400', '500', '600', '700'],
  variable: '--font-body',
})

export const metadata: Metadata = {
  title: 'RampOS — The Financial Operating System for Fiat ↔ Crypto',
  description: 'Intent-native on/off ramp orchestration with multi-chain settlement, real-time compliance, and enterprise-grade infrastructure. Built with Rust.',
  keywords: ['fintech', 'crypto', 'fiat', 'ramp', 'blockchain', 'compliance', 'KYC', 'AML'],
  openGraph: {
    title: 'RampOS — Money Moves at the Speed of Intent',
    description: 'The world\'s first intent-native financial OS. Swap, bridge, ramp, settle — let the engine find the perfect path.',
    siteName: 'RampOS',
    type: 'website',
  },
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en" className="dark" suppressHydrationWarning>
      <body suppressHydrationWarning className={cn(
        "min-h-screen bg-[#050505] text-white antialiased font-body selection:bg-[#00FF87]/20",
        fontDisplay.variable,
        fontBody.variable
      )}>
        {children}
      </body>
    </html>
  )
}
