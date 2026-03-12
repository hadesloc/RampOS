'use client'

import { motion } from 'framer-motion'
import {
  ArrowRight,
  ShieldCheck,
  Zap,
  ArrowRightLeft,
  Code2,
  Globe2,
  BookOpenCheck,
  Layers,
  Wallet,
} from 'lucide-react'
import Link from 'next/link'

import StatsBar from '@/components/StatsBar'
import HowItWorks from '@/components/HowItWorks'
import ApiSection from '@/components/ApiSection'
import ArchSection from '@/components/ArchSection'
import CTASection from '@/components/CTASection'
import Footer from '@/components/Footer'

const features = [
  {
    title: 'Intent Engine',
    description: 'Declarative Swap, Bridge, Send, Stake across EVM, Solana, and TON — with smart route optimization.',
    icon: Layers,
    color: 'text-cyan-400',
  },
  {
    title: 'On/Off Ramp',
    description: 'Complete fiat ↔ crypto lifecycle: pay-in, pay-out, escrow, bank confirmation, and settlement.',
    icon: Wallet,
    color: 'text-emerald-400',
  },
  {
    title: 'Compliance Engine',
    description: 'KYC tiering, AML velocity checks, FATF Travel Rule, sanctions screening, and regulatory reporting.',
    icon: ShieldCheck,
    color: 'text-blue-400',
  },
  {
    title: 'RFQ Auction',
    description: 'Bidirectional LP price discovery for USDT ↔ VND. Competitive quoting with tenant isolation.',
    icon: ArrowRightLeft,
    color: 'text-fuchsia-400',
  },
  {
    title: 'Multi-Chain',
    description: 'Ethereum, Polygon, Arbitrum, Base, BSC, Solana, TON — with cross-chain bridge routing.',
    icon: Globe2,
    color: 'text-amber-400',
  },
  {
    title: 'Double-Entry Ledger',
    description: 'Financial-grade accounting with atomic transactions, full audit trail, and reconciliation.',
    icon: BookOpenCheck,
    color: 'text-indigo-400',
  },
]

const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.1, delayChildren: 0.3 },
  },
}

const item = {
  hidden: { opacity: 0, y: 30 },
  show: { opacity: 1, y: 0 },
}

export default function Home() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-between overflow-hidden bg-black text-white selection:bg-cyan-500/30">
      {/* Background */}
      <div className="fixed inset-0 z-0 opacity-40 pointer-events-none">
        <div className="absolute top-[-10%] left-[-10%] w-[50%] h-[50%] rounded-full bg-blue-900/40 blur-[120px] mix-blend-screen animate-pulse" style={{ animationDuration: '4s' }} />
        <div className="absolute top-[20%] right-[-10%] w-[40%] h-[60%] rounded-full bg-fuchsia-900/30 blur-[120px] mix-blend-screen animate-pulse" style={{ animationDuration: '7s' }} />
        <div className="absolute bottom-[-20%] left-[20%] w-[60%] h-[60%] rounded-full bg-cyan-900/30 blur-[120px] mix-blend-screen animate-pulse" style={{ animationDuration: '10s' }} />
        <div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.03)_1px,transparent_1px)] bg-[size:40px_40px] [mask-image:radial-gradient(ellipse_60%_60%_at_50%_50%,#000_10%,transparent_100%)]" />
      </div>

      {/* Nav */}
      <div className="z-50 w-full max-w-6xl items-center justify-between font-mono text-sm lg:flex px-4 md:px-24 pt-8">
        <p className="fixed left-0 top-0 flex w-full justify-center border-b border-white/10 bg-black/50 pb-6 pt-8 backdrop-blur-xl lg:static lg:w-auto lg:rounded-2xl lg:border lg:bg-white/5 lg:p-4 z-50">
          RAMP OS&nbsp;
          <code className="font-mono font-bold text-cyan-400">v2.0</code>
        </p>
        <div className="fixed bottom-0 left-0 flex h-24 w-full items-end justify-center bg-gradient-to-t from-black via-black/90 lg:static lg:h-auto lg:w-auto lg:bg-none z-50">
          <a
            className="pointer-events-none flex place-items-center gap-2 p-8 lg:pointer-events-auto lg:p-0 text-gray-400 hover:text-white transition-colors"
            href="https://github.com/hadesloc/RampOS"
            target="_blank"
            rel="noopener noreferrer"
          >
            <Code2 className="w-5 h-5" /> GitHub
          </a>
        </div>
      </div>

      {/* Hero */}
      <div className="relative z-10 flex flex-col items-center justify-center min-h-[75vh] w-full mt-24 lg:mt-0 px-4">
        <div className="text-center max-w-5xl mx-auto">
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.5 }}
            className="inline-flex items-center gap-3 px-5 py-2.5 rounded-full bg-white/5 border border-white/10 mb-8 backdrop-blur-md shadow-[0_0_20px_rgba(34,211,238,0.1)]"
          >
            <span className="relative flex h-3 w-3">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
              <span className="relative inline-flex rounded-full h-3 w-3 bg-emerald-500"></span>
            </span>
            <span className="text-sm font-medium text-gray-200 tracking-wide">Production-Ready · Rust-Powered · Compliance-First</span>
          </motion.div>

          <motion.h1
            initial={{ opacity: 0, y: 30 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, ease: 'easeOut' }}
            className="text-5xl md:text-7xl lg:text-8xl font-extrabold tracking-tighter mb-8 leading-[1.1] pb-2"
          >
            <span className="text-transparent bg-clip-text bg-gradient-to-br from-white via-gray-200 to-gray-600">The Operating System</span>
            <br />
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 via-blue-500 to-fuchsia-500">for Fiat ↔ Crypto</span>
          </motion.h1>

          <motion.p
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.2 }}
            className="text-xl md:text-2xl text-gray-400 mb-12 max-w-3xl mx-auto font-light leading-relaxed"
          >
            End-to-end on/off ramp orchestration with intent-based routing,
            multi-chain settlement, and built-in compliance.
            Bring your own rails — we handle everything else.
          </motion.p>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.4 }}
            className="flex flex-col sm:flex-row gap-6 justify-center items-center"
          >
            <Link href="/vi/portal" className="group relative px-8 py-4 bg-white text-black rounded-full font-bold text-lg overflow-hidden transition-all hover:scale-105 active:scale-95 flex items-center gap-2 shadow-[0_0_40px_rgba(255,255,255,0.3)] hover:shadow-[0_0_60px_rgba(255,255,255,0.5)]">
              Get Started <ArrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
            </Link>
            <Link href="/docs" className="px-8 py-4 bg-white/5 text-white border border-white/20 rounded-full font-semibold text-lg hover:bg-white/10 backdrop-blur-md transition-all flex items-center gap-2">
              Read Docs
            </Link>
          </motion.div>
        </div>
      </div>

      {/* Stats */}
      <StatsBar />

      {/* Features */}
      <section className="py-32 relative z-10 w-full">
        <div className="container mx-auto px-4">
          <div className="text-center max-w-3xl mx-auto mb-20">
            <motion.h2
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: '-100px' }}
              transition={{ duration: 0.6 }}
              className="text-4xl md:text-6xl font-bold mb-6 tracking-tight"
            >
              Everything to <span className="text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-fuchsia-400">Build an Exchange</span>
            </motion.h2>
            <motion.p
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: '-100px' }}
              transition={{ duration: 0.6, delay: 0.2 }}
              className="text-xl text-gray-400 font-light"
            >
              A complete financial stack — from fiat rails to blockchain settlement, from KYC to custody.
            </motion.p>
          </div>

          <motion.div
            variants={container}
            initial="hidden"
            whileInView="show"
            viewport={{ once: true, margin: '-50px' }}
            className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 lg:gap-8 max-w-7xl mx-auto"
          >
            {features.map((feature, idx) => (
              <FeatureCard key={idx} feature={feature} />
            ))}
          </motion.div>
        </div>
      </section>

      <HowItWorks />
      <ApiSection />
      <ArchSection />
      <CTASection />
      <Footer />
    </main>
  )
}

function FeatureCard({ feature }: { feature: typeof features[0] }) {
  const Icon = feature.icon

  return (
    <motion.div
      variants={item}
      className="group relative p-8 rounded-3xl border border-white/5 bg-white/[0.02] backdrop-blur-xl hover:bg-white/[0.05] hover:border-white/10 transition-all duration-500 overflow-hidden"
    >
      <div className="absolute inset-0 bg-gradient-to-br from-white/10 to-transparent opacity-0 group-hover:opacity-10 transition-opacity duration-500 rounded-3xl" />

      <div className="relative z-10 flex flex-col h-full">
        <div className="p-4 rounded-2xl bg-white/5 w-fit mb-6 ring-1 ring-white/10 group-hover:ring-white/30 transition-all duration-500 group-hover:scale-110 group-hover:shadow-[0_0_20px_rgba(255,255,255,0.1)]">
          <Icon className={`w-8 h-8 ${feature.color}`} />
        </div>

        <h3 className="text-2xl font-bold mb-4 text-white group-hover:text-transparent group-hover:bg-clip-text group-hover:bg-gradient-to-r group-hover:from-white group-hover:to-gray-400 transition-all duration-300">
          {feature.title}
        </h3>

        <p className="text-gray-400 leading-relaxed font-light text-lg">
          {feature.description}
        </p>
      </div>
    </motion.div>
  )
}
