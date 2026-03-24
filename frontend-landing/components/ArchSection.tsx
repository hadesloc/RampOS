'use client'

import { motion } from 'framer-motion'
import Image from 'next/image'

const layers = [
  {
    label: 'Frontend Layer',
    items: ['Admin Dashboard', 'User Portal', 'Embeddable Widget'],
    gradient: 'from-[#00FF87]/15 to-[#00FF87]/5',
    border: 'border-[#00FF87]/20',
    text: 'text-[#00FF87]',
    glow: 'hover:shadow-[0_0_30px_rgba(0,255,135,0.1)]',
  },
  {
    label: 'API Gateway (Axum)',
    items: ['Auth · Rate Limit · Idempotency · OTel'],
    gradient: 'from-[#00D4FF]/15 to-[#00D4FF]/5',
    border: 'border-[#00D4FF]/20',
    text: 'text-[#00D4FF]',
    glow: 'hover:shadow-[0_0_30px_rgba(0,212,255,0.1)]',
  },
  {
    label: 'Business Logic (ramp-core)',
    items: ['Intent Engine', 'Workflow Engine', '15+ Services', 'Double-Entry Ledger'],
    gradient: 'from-[#7B61FF]/15 to-[#7B61FF]/5',
    border: 'border-[#7B61FF]/20',
    text: 'text-[#7B61FF]',
    glow: 'hover:shadow-[0_0_30px_rgba(123,97,255,0.1)]',
  },
  {
    label: 'Compliance (ramp-compliance)',
    items: ['KYC/AML', 'Travel Rule', 'Risk Lab', 'Sanctions'],
    gradient: 'from-[#00FF87]/15 to-[#00FF87]/5',
    border: 'border-[#00FF87]/20',
    text: 'text-[#00FF87]',
    glow: 'hover:shadow-[0_0_30px_rgba(0,255,135,0.1)]',
  },
  {
    label: 'Infrastructure',
    items: ['PostgreSQL 16', 'Redis 7', 'NATS JetStream', 'ClickHouse'],
    gradient: 'from-[#00D4FF]/15 to-[#00D4FF]/5',
    border: 'border-[#00D4FF]/20',
    text: 'text-[#00D4FF]',
    glow: 'hover:shadow-[0_0_30px_rgba(0,212,255,0.1)]',
  },
]

const externals = [
  { label: 'Bank / PSP Rails', desc: 'VCB · MB · Any PSP', accent: '#00FF87' },
  { label: 'Blockchain Networks', desc: 'EVM · Solana · TON', accent: '#7B61FF' },
  { label: 'Compliance Providers', desc: 'Onfido · Chainalysis · SBV', accent: '#00D4FF' },
]

export default function ArchSection() {
  return (
    <section className="w-full py-32 relative overflow-hidden section-glow">
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[600px] bg-[#7B61FF]/5 blur-[200px] rounded-full pointer-events-none" />

      <div className="container mx-auto px-4 max-w-5xl relative z-10">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-20"
        >
          <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full glass text-sm font-medium text-[#7B61FF] tracking-wider uppercase mb-6">
            Architecture
          </div>
          <h2 className="text-4xl md:text-6xl font-display font-extrabold tracking-tight mb-6">
            Built with{' '}
            <span className="text-shimmer">Rust</span>{' '}
            for Production
          </h2>
          <p className="text-xl text-gray-400 font-light max-w-2xl mx-auto">
            7 specialized crates, event-driven architecture, and durable workflow execution — battle-tested for financial workloads.
          </p>
        </motion.div>

        {/* Stack Layers */}
        <div className="space-y-3 mb-12">
          {layers.map((layer, i) => (
            <motion.div
              key={i}
              initial={{ opacity: 0, x: -30 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: i * 0.08 }}
              className={`rounded-xl border ${layer.border} bg-gradient-to-r ${layer.gradient} p-5 transition-all duration-500 ${layer.glow} group cursor-default`}
            >
              <div className="flex flex-col md:flex-row md:items-center gap-3">
                <span className={`font-display font-bold text-sm tracking-wide min-w-[220px] ${layer.text}`}>
                  {layer.label}
                </span>
                <div className="flex flex-wrap gap-2">
                  {layer.items.map((it, j) => (
                    <span key={j} className="px-3 py-1 rounded-full bg-black/40 text-gray-300 text-sm font-medium border border-white/[0.06] group-hover:border-white/10 transition-colors">
                      {it}
                    </span>
                  ))}
                </div>
              </div>
            </motion.div>
          ))}
        </div>

        {/* External Connections */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.4 }}
          className="grid grid-cols-1 md:grid-cols-3 gap-4"
        >
          {externals.map((ext, i) => (
            <div
              key={i}
              className="rounded-xl glass p-5 text-center group hover:scale-105 transition-all duration-300"
              style={{ '--accent': ext.accent } as React.CSSProperties}
            >
              <div className="text-sm font-display font-bold text-white mb-1">{ext.label}</div>
              <div className="text-xs text-gray-500 font-mono">{ext.desc}</div>
            </div>
          ))}
        </motion.div>

        {/* Server Infrastructure Illustration */}
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.8, delay: 0.5 }}
          className="mt-16 flex justify-center"
        >
          <div className="relative">
            <div className="absolute inset-0 bg-[#00D4FF]/5 blur-[80px] rounded-full scale-125" />
            <Image
              src="/images/server-infrastructure.png"
              alt="RampOS Infrastructure"
              width={600}
              height={400}
              className="relative z-10 drop-shadow-[0_0_40px_rgba(0,212,255,0.1)] rounded-xl"
            />
          </div>
        </motion.div>
      </div>
    </section>
  )
}
