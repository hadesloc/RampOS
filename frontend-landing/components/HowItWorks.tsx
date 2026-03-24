'use client'

import { motion } from 'framer-motion'
import Image from 'next/image'
import { FileSignature, Route, Cog, Wallet } from 'lucide-react'

const steps = [
  {
    id: 1,
    title: 'Declare Intent',
    description: 'Users express what they want: Swap, Bridge, Send, Ramp, or Stake — across any chain. No manual routing needed.',
    icon: FileSignature,
    accent: '#00FF87',
  },
  {
    id: 2,
    title: 'Route & Price',
    description: 'IntentSolver evaluates every path, scoring by gas, speed, liquidity depth, and step count to find the optimum.',
    icon: Route,
    accent: '#7B61FF',
  },
  {
    id: 3,
    title: 'Execute & Settle',
    description: 'WorkflowEngine executes each step with durable state, built-in compensation, and automatic rollback on failure.',
    icon: Cog,
    accent: '#00D4FF',
  },
  {
    id: 4,
    title: 'Comply & Report',
    description: 'Built-in KYC/AML screening, Travel Rule compliance, sanctions checks, and regulatory reporting — all automatic.',
    icon: Wallet,
    accent: '#00FF87',
  },
]

export default function HowItWorks() {
  return (
    <section className="w-full py-32 relative overflow-hidden">
      {/* Background glow */}
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[400px] bg-[#7B61FF]/5 blur-[180px] rounded-full pointer-events-none" />

      <div className="container mx-auto px-4 relative z-10 max-w-6xl">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-20"
        >
          <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full glass text-sm font-medium text-[#7B61FF] tracking-wider uppercase mb-6">
            Intent Lifecycle
          </div>
          <h2 className="text-4xl md:text-6xl font-display font-extrabold tracking-tight mb-6">
            From Intent to{' '}
            <span className="text-shimmer">Settlement</span>
          </h2>
          <p className="text-gray-400 text-xl max-w-2xl mx-auto font-light leading-relaxed">
            Every operation follows a declarative pipeline — users express <em>what</em>, the engine decides <em>how</em>.
          </p>
        </motion.div>

        {/* Vertical Timeline + Illustration */}
        <div className="grid grid-cols-1 lg:grid-cols-5 gap-12 items-start">
        <div className="lg:col-span-3 relative">
          {/* Glowing line */}
          <div className="absolute left-8 md:left-12 top-0 bottom-0 w-[2px]">
            <div className="h-full w-full bg-gradient-to-b from-[#00FF87]/40 via-[#7B61FF]/40 to-[#00D4FF]/40 rounded-full" />
            <div className="absolute inset-0 bg-gradient-to-b from-[#00FF87] via-[#7B61FF] to-[#00D4FF] rounded-full animate-neon-pulse" style={{ filter: 'blur(4px)' }} />
          </div>

          <div className="space-y-16">
            {steps.map((step, index) => (
              <motion.div
                key={step.id}
                initial={{ opacity: 0, x: -30 }}
                whileInView={{ opacity: 1, x: 0 }}
                viewport={{ once: true, margin: '-50px' }}
                transition={{ duration: 0.6, delay: index * 0.1 }}
                className="relative flex gap-8 md:gap-12 group"
              >
                {/* Number Circle */}
                <div className="relative z-10 shrink-0">
                  <div
                    className="w-16 h-16 md:w-24 md:h-24 rounded-full border-2 flex items-center justify-center bg-[#050505] font-display text-2xl md:text-3xl font-bold transition-all duration-500 group-hover:scale-110"
                    style={{
                      borderColor: `${step.accent}40`,
                      color: step.accent,
                      boxShadow: `0 0 0 0 ${step.accent}00`,
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.boxShadow = `0 0 30px ${step.accent}30, 0 0 60px ${step.accent}10`
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.boxShadow = `0 0 0 0 ${step.accent}00`
                    }}
                  >
                    {step.id}
                  </div>
                </div>

                {/* Content */}
                <div className="pt-2 md:pt-5 flex-1">
                  <div className="flex items-center gap-3 mb-3">
                    <step.icon className="w-5 h-5" style={{ color: step.accent }} />
                    <h3 className="text-xl md:text-2xl font-display font-bold text-white group-hover:text-transparent group-hover:bg-clip-text group-hover:bg-gradient-to-r group-hover:from-white group-hover:to-gray-400 transition-all duration-300">
                      {step.title}
                    </h3>
                  </div>
                  <p className="text-gray-400 text-lg leading-relaxed font-light">
                    {step.description}
                  </p>
                </div>
              </motion.div>
            ))}
          </div>
        </div>

        {/* Data Pipeline Illustration */}
        <motion.div
          initial={{ opacity: 0, x: 30 }}
          whileInView={{ opacity: 1, x: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.8, delay: 0.3 }}
          className="hidden lg:flex lg:col-span-2 items-center justify-center sticky top-32"
        >
          <div className="relative">
            <div className="absolute inset-0 bg-[#7B61FF]/5 blur-[60px] rounded-full scale-110" />
            <Image
              src="/images/data-pipeline.png"
              alt="Intent Processing Pipeline"
              width={500}
              height={500}
              className="relative z-10 drop-shadow-[0_0_40px_rgba(123,97,255,0.1)]"
            />
          </div>
        </motion.div>
        </div>
      </div>
    </section>
  )
}
