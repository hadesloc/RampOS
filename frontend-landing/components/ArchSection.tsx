'use client'

import { motion } from 'framer-motion'

const layers = [
  {
    label: 'Frontend Layer',
    items: ['Admin Dashboard', 'User Portal', 'Embeddable Widget'],
    color: 'from-cyan-500/20 to-cyan-500/5',
    border: 'border-cyan-500/30',
    text: 'text-cyan-400',
  },
  {
    label: 'API Gateway (Axum)',
    items: ['Auth · Rate Limit · Idempotency · OTel'],
    color: 'from-blue-500/20 to-blue-500/5',
    border: 'border-blue-500/30',
    text: 'text-blue-400',
  },
  {
    label: 'Business Logic (ramp-core)',
    items: ['Intent Engine', 'Workflow Engine', '15+ Services', 'Double-Entry Ledger'],
    color: 'from-fuchsia-500/20 to-fuchsia-500/5',
    border: 'border-fuchsia-500/30',
    text: 'text-fuchsia-400',
  },
  {
    label: 'Compliance (ramp-compliance)',
    items: ['KYC/AML', 'Travel Rule', 'Risk Lab', 'Sanctions'],
    color: 'from-emerald-500/20 to-emerald-500/5',
    border: 'border-emerald-500/30',
    text: 'text-emerald-400',
  },
  {
    label: 'Infrastructure',
    items: ['PostgreSQL 16', 'Redis 7', 'NATS JetStream', 'ClickHouse'],
    color: 'from-amber-500/20 to-amber-500/5',
    border: 'border-amber-500/30',
    text: 'text-amber-400',
  },
]

const externals = [
  { label: 'Bank / PSP Rails', desc: 'VCB · MB · Any PSP' },
  { label: 'Blockchain Networks', desc: 'EVM · Solana · TON' },
  { label: 'Compliance Providers', desc: 'Onfido · Chainalysis · SBV' },
]

export default function ArchSection() {
  return (
    <section className="w-full py-32 bg-black relative overflow-hidden">
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[600px] bg-fuchsia-900/10 blur-[150px] rounded-full pointer-events-none" />

      <div className="container mx-auto px-4 max-w-5xl relative z-10">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-20"
        >
          <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-white/5 border border-white/10 mb-6 backdrop-blur-md text-sm font-medium text-fuchsia-400 tracking-wider uppercase">
            Architecture
          </div>
          <h2 className="text-4xl md:text-6xl font-extrabold tracking-tight mb-6">
            Built with <span className="text-transparent bg-clip-text bg-gradient-to-r from-fuchsia-400 to-cyan-400">Rust</span> for Production
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
              transition={{ duration: 0.5, delay: i * 0.1 }}
              className={`rounded-2xl border ${layer.border} bg-gradient-to-r ${layer.color} p-5 backdrop-blur-sm`}
            >
              <div className="flex flex-col md:flex-row md:items-center gap-3">
                <span className={`font-bold text-base tracking-wide min-w-[220px] ${layer.text}`}>
                  {layer.label}
                </span>
                <div className="flex flex-wrap gap-2">
                  {layer.items.map((item, j) => (
                    <span key={j} className="px-3 py-1 rounded-full bg-black/30 text-gray-300 text-sm font-medium border border-white/5">
                      {item}
                    </span>
                  ))}
                </div>
              </div>
            </motion.div>
          ))}
        </div>

        {/* External connections */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.5 }}
          className="grid grid-cols-1 md:grid-cols-3 gap-4"
        >
          {externals.map((ext, i) => (
            <div key={i} className="rounded-xl border border-white/10 bg-white/[0.02] p-5 text-center">
              <div className="text-sm font-bold text-white mb-1">{ext.label}</div>
              <div className="text-xs text-gray-500 font-mono">{ext.desc}</div>
            </div>
          ))}
        </motion.div>
      </div>
    </section>
  )
}
