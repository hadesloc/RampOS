'use client'

import { motion } from 'framer-motion'
import Image from 'next/image'
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
  Play,
  Sparkles,
} from 'lucide-react'
import Link from 'next/link'

import StatsBar from '@/components/StatsBar'
import HowItWorks from '@/components/HowItWorks'
import ApiSection from '@/components/ApiSection'
import ArchSection from '@/components/ArchSection'
import CTASection from '@/components/CTASection'
import Footer from '@/components/Footer'

/* ── Feature data ── */
const features = [
  {
    title: 'Intent Engine',
    description: 'Declarative Swap, Bridge, Send, Stake across EVM, Solana, and TON — with smart route optimization and gas-aware execution.',
    icon: Layers,
    accent: 'neon-green',
    size: 'large',
  },
  {
    title: 'Compliance Engine',
    description: 'KYC tiering, AML velocity checks, FATF Travel Rule, sanctions screening, and real-time regulatory reporting.',
    icon: ShieldCheck,
    accent: 'violet',
    size: 'medium',
  },
  {
    title: 'RFQ Auction',
    description: 'Bidirectional LP price discovery for USDT ↔ VND. Competitive quoting with tenant isolation and reliability scoring.',
    icon: ArrowRightLeft,
    accent: 'cyan',
    size: 'medium',
  },
  {
    title: 'On/Off Ramp',
    description: 'Complete fiat ↔ crypto lifecycle: pay-in, pay-out, escrow, bank confirmation, and settlement.',
    icon: Wallet,
    accent: 'neon-green',
    size: 'small',
  },
  {
    title: 'Multi-Chain',
    description: 'Ethereum, Polygon, Arbitrum, Base, BSC, Solana, TON — with cross-chain bridge routing.',
    icon: Globe2,
    accent: 'violet',
    size: 'small',
  },
  {
    title: 'Double-Entry Ledger',
    description: 'Financial-grade accounting with atomic transactions, full audit trail, and reconciliation.',
    icon: BookOpenCheck,
    accent: 'cyan',
    size: 'small',
  },
]

const accentColors: Record<string, { bg: string; text: string; glow: string; border: string; glowHover: string }> = {
  'neon-green': {
    bg: 'bg-[#00FF87]/10',
    text: 'text-[#00FF87]',
    glow: 'shadow-[0_0_20px_rgba(0,255,135,0.15)]',
    glowHover: 'group-hover:shadow-[0_0_40px_rgba(0,255,135,0.25)]',
    border: 'border-[#00FF87]/20',
  },
  violet: {
    bg: 'bg-[#7B61FF]/10',
    text: 'text-[#7B61FF]',
    glow: 'shadow-[0_0_20px_rgba(123,97,255,0.15)]',
    glowHover: 'group-hover:shadow-[0_0_40px_rgba(123,97,255,0.25)]',
    border: 'border-[#7B61FF]/20',
  },
  cyan: {
    bg: 'bg-[#00D4FF]/10',
    text: 'text-[#00D4FF]',
    glow: 'shadow-[0_0_20px_rgba(0,212,255,0.15)]',
    glowHover: 'group-hover:shadow-[0_0_40px_rgba(0,212,255,0.25)]',
    border: 'border-[#00D4FF]/20',
  },
}

/* ── Trusted by logos ── */
const trustedBy = [
  'Ethereum', 'Polygon', 'Arbitrum', 'Base', 'BSC', 'Solana', 'TON',
  'Ethereum', 'Polygon', 'Arbitrum', 'Base', 'BSC', 'Solana', 'TON',
]

/* ── Animation variants ── */
const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.08, delayChildren: 0.2 },
  },
}

const item = {
  hidden: { opacity: 0, y: 30, scale: 0.95 },
  show: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.5 } },
}

export default function Home() {
  return (
    <main className="flex min-h-screen flex-col items-center overflow-hidden bg-[#050505] text-white">
      {/* ═══ Background Effects ═══ */}
      <div className="fixed inset-0 z-0 pointer-events-none">
        {/* Aurora orbs — enhanced */}
        <div className="absolute top-[-25%] left-[-15%] w-[70%] h-[70%] rounded-full bg-[#00FF87]/[0.06] blur-[200px] animate-pulse" style={{ animationDuration: '8s' }} />
        <div className="absolute top-[5%] right-[-20%] w-[60%] h-[80%] rounded-full bg-[#7B61FF]/[0.06] blur-[200px] animate-pulse" style={{ animationDuration: '12s' }} />
        <div className="absolute bottom-[-25%] left-[5%] w-[80%] h-[70%] rounded-full bg-[#00D4FF]/[0.04] blur-[220px] animate-pulse" style={{ animationDuration: '16s' }} />
        <div className="absolute top-[40%] left-[40%] w-[30%] h-[30%] rounded-full bg-[#00FF87]/[0.03] blur-[150px] animate-pulse" style={{ animationDuration: '20s' }} />

        {/* Grid */}
        <div className="absolute inset-0 grid-bg [mask-image:radial-gradient(ellipse_60%_50%_at_50%_30%,#000_10%,transparent_100%)]" />

        {/* Perspective grid floor */}
        <div className="absolute bottom-0 left-0 right-0 h-[40vh] perspective-grid opacity-20" />
      </div>

      {/* ═══ Navigation ═══ */}
      <motion.nav
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6 }}
        className="fixed top-6 left-1/2 -translate-x-1/2 z-50 flex items-center gap-1 px-2 py-2 rounded-full glass-strong"
      >
        <Link href="/" className="flex items-center gap-2 px-4 py-1.5 font-display font-bold text-white tracking-tight">
          <span className="relative flex h-2.5 w-2.5">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#00FF87] opacity-75" />
            <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-[#00FF87]" />
          </span>
          RAMP·OS
        </Link>
        <div className="hidden md:flex items-center gap-1">
          {['Solutions', 'Developers', 'Pricing'].map(label => (
            <a key={label} href="#" className="px-3.5 py-1.5 text-sm text-gray-400 hover:text-white rounded-full hover:bg-white/5 transition-all duration-300">
              {label}
            </a>
          ))}
          <Link href="/docs" className="px-3.5 py-1.5 text-sm text-gray-400 hover:text-white rounded-full hover:bg-white/5 transition-all duration-300">
            Docs
          </Link>
        </div>
        <div className="flex items-center gap-2 ml-2">
          <a href="https://github.com/hadesloc/RampOS" target="_blank" rel="noopener noreferrer" className="p-2 text-gray-500 hover:text-white transition-colors">
            <Code2 className="w-4 h-4" />
          </a>
          <Link
            href="/vi/portal"
            className="px-4 py-1.5 text-sm font-semibold text-black bg-[#00FF87] rounded-full hover:brightness-110 transition-all shadow-[0_0_20px_rgba(0,255,135,0.3)] hover:shadow-[0_0_30px_rgba(0,255,135,0.5)]"
          >
            Launch App
          </Link>
        </div>
      </motion.nav>

      {/* ═══ Hero Section ═══ */}
      <section className="relative z-10 flex flex-col items-center justify-center min-h-screen w-full px-4 pt-32">
        <div className="flex flex-col lg:flex-row items-center gap-16 max-w-7xl mx-auto w-full">
          {/* Left: Text Content */}
          <div className="flex-1 text-center lg:text-left">
            {/* Status Badge */}
            <motion.div
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ duration: 0.5 }}
              className="inline-flex items-center gap-3 px-5 py-2.5 rounded-full glass mb-10"
            >
              <span className="relative flex h-2.5 w-2.5">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#00FF87] opacity-75" />
                <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-[#00FF87]" />
              </span>
              <span className="text-sm font-medium text-gray-300 tracking-wide">Production-Ready · Rust-Powered · Compliance-First</span>
            </motion.div>

            {/* Headline */}
            <motion.h1
              initial={{ opacity: 0, y: 40 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.8, ease: 'easeOut' }}
              className="text-5xl md:text-7xl lg:text-[5.5rem] font-display font-extrabold tracking-[-0.03em] mb-8 leading-[1.05]"
            >
              <span className="text-transparent bg-clip-text bg-gradient-to-b from-white via-gray-200 to-gray-500">Money Moves at the</span>
              <br />
              <span className="text-shimmer">Speed of Intent</span>
            </motion.h1>

            {/* Subtitle */}
            <motion.p
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.8, delay: 0.2 }}
              className="text-lg md:text-xl text-gray-400 mb-12 max-w-2xl font-body font-light leading-relaxed lg:mx-0 mx-auto"
            >
              The world&apos;s first intent-native financial OS. Express what you want —
              swap, bridge, ramp, settle — and let the engine find the perfect path.
            </motion.p>

            {/* CTA Buttons */}
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.8, delay: 0.4 }}
              className="flex flex-col sm:flex-row gap-4 justify-center lg:justify-start items-center mb-12"
            >
              <Link
                href="/vi/portal"
                className="group relative px-8 py-4 bg-[#00FF87] text-black rounded-full font-display font-bold text-lg overflow-hidden transition-all hover:scale-105 active:scale-95 flex items-center gap-2.5 animate-pulse-glow"
              >
                Start Building <ArrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
              </Link>
              <button className="group px-8 py-4 glass rounded-full font-display font-semibold text-lg text-white hover:bg-white/10 transition-all flex items-center gap-2.5">
                <Play className="w-5 h-5 text-[#00FF87]" /> Watch Demo
              </button>
            </motion.div>

            {/* Mini Stats */}
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.8, delay: 0.6 }}
              className="flex flex-wrap justify-center lg:justify-start gap-3"
            >
              {[
                { value: '7', label: 'Rust Crates' },
                { value: '$2.4B+', label: 'Processed' },
                { value: '140+', label: 'Countries' },
                { value: '99.99%', label: 'Uptime' },
              ].map((stat, i) => (
                <div key={i} className="flex items-center gap-2.5 px-4 py-2 rounded-full glass text-sm">
                  <span className="font-display font-bold text-white tabular-nums">{stat.value}</span>
                  <span className="text-gray-500">{stat.label}</span>
                </div>
              ))}
            </motion.div>
          </div>

          {/* Right: Holographic Globe Illustration */}
          <motion.div
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 1, delay: 0.3 }}
            className="flex-1 hidden lg:flex items-center justify-center relative"
          >
            <div className="relative animate-hero-float">
              {/* Glow behind globe */}
              <div className="absolute inset-0 bg-[#00FF87]/10 blur-[80px] rounded-full scale-90" />
              <Image
                src="/images/holographic-globe.png"
                alt="RampOS Global Network"
                width={600}
                height={600}
                className="relative z-10 drop-shadow-[0_0_60px_rgba(0,255,135,0.15)]"
                priority
              />
            </div>
          </motion.div>
        </div>
      </section>

      {/* ═══ Trusted By Marquee ═══ */}
      <section className="w-full py-12 relative z-10 overflow-hidden section-glow">
        <div className="text-center mb-8">
          <span className="text-xs font-medium text-gray-600 uppercase tracking-[0.2em]">Trusted Across Networks</span>
        </div>
        <div className="relative overflow-hidden">
          <div className="absolute left-0 top-0 bottom-0 w-40 bg-gradient-to-r from-[#050505] to-transparent z-10" />
          <div className="absolute right-0 top-0 bottom-0 w-40 bg-gradient-to-l from-[#050505] to-transparent z-10" />
          <div className="flex animate-marquee">
            {trustedBy.map((chain, i) => (
              <div key={i} className="flex items-center gap-3 px-8 py-3 shrink-0">
                <Sparkles className="w-4 h-4 text-gray-600" />
                <span className="text-gray-500 font-display font-semibold text-lg whitespace-nowrap">{chain}</span>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* ═══ Stats Bar ═══ */}
      <StatsBar />

      {/* ═══ Bento Features Grid ═══ */}
      <section className="py-32 relative z-10 w-full section-glow">
        <div className="container mx-auto px-4">
          <div className="text-center max-w-3xl mx-auto mb-20">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              className="inline-flex items-center gap-2 px-4 py-2 rounded-full glass text-sm font-medium text-[#00FF87] tracking-wider uppercase mb-6"
            >
              <Sparkles className="w-3.5 h-3.5" />
              The Ramp Stack
            </motion.div>
            <motion.h2
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.6 }}
              className="text-4xl md:text-6xl font-display font-bold mb-6 tracking-tight"
            >
              Everything to{' '}
              <span className="text-shimmer">Build an Exchange</span>
            </motion.h2>
            <motion.p
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.6, delay: 0.2 }}
              className="text-xl text-gray-400 font-light"
            >
              A complete financial stack — from fiat rails to blockchain settlement.
            </motion.p>
          </div>

          {/* Bento Grid */}
          <motion.div
            variants={container}
            initial="hidden"
            whileInView="show"
            viewport={{ once: true, margin: '-50px' }}
            className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 lg:gap-5 max-w-7xl mx-auto"
          >
            {features.map((feature, idx) => (
              <BentoCard key={idx} feature={feature} index={idx} />
            ))}
          </motion.div>
        </div>
      </section>

      {/* ═══ Remaining Sections ═══ */}
      <HowItWorks />
      <ApiSection />
      <ArchSection />
      <CTASection />
      <Footer />
    </main>
  )
}

/* ── Bento Feature Card ── */
function BentoCard({ feature, index }: { feature: typeof features[0]; index: number }) {
  const Icon = feature.icon
  const colors = accentColors[feature.accent]
  const isLarge = feature.size === 'large'

  return (
    <motion.div
      variants={item}
      className={`group relative rounded-2xl overflow-hidden transition-all duration-500 noise-overlay ${colors.glow} ${colors.glowHover} ${
        isLarge ? 'md:col-span-2 lg:col-span-1 lg:row-span-2' : ''
      }`}
    >
      {/* Background */}
      <div className="absolute inset-0 bg-surface-2 border border-white/[0.04] rounded-2xl group-hover:border-white/[0.1] transition-all duration-500" />

      {/* Gradient overlay on hover */}
      <div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-700 bg-gradient-to-br from-white/[0.02] to-transparent rounded-2xl" />

      {/* Content */}
      <div className={`relative z-10 flex flex-col ${isLarge ? 'p-8 min-h-[320px]' : 'p-7'} h-full`}>
        <div className={`p-3.5 rounded-xl ${colors.bg} w-fit mb-5 ring-1 ${colors.border} group-hover:scale-110 transition-transform duration-500`}>
          <Icon className={`w-7 h-7 ${colors.text}`} />
        </div>

        <h3 className="text-xl font-display font-bold mb-3 text-white group-hover:text-transparent group-hover:bg-clip-text group-hover:bg-gradient-to-r group-hover:from-white group-hover:to-gray-400 transition-all duration-300">
          {feature.title}
        </h3>

        <p className="text-gray-400 leading-relaxed font-light text-[15px] flex-1">
          {feature.description}
        </p>

        {isLarge && (
          <div className="mt-6 flex items-center gap-2 text-sm text-[#00FF87] font-medium opacity-0 group-hover:opacity-100 transition-opacity duration-500">
            Learn more <ArrowRight className="w-4 h-4" />
          </div>
        )}
      </div>
    </motion.div>
  )
}
