'use client'

import { motion } from 'framer-motion'
import { FileSignature, Route, Cog, Wallet } from 'lucide-react'

const steps = [
  {
    id: 1,
    title: 'Express Intent',
    description: 'Users declare what they want: Swap, Bridge, Send, or Stake — across any chain.',
    icon: FileSignature,
  },
  {
    id: 2,
    title: 'Smart Routing',
    description: 'IntentSolver evaluates all routes, scoring by gas, speed, and step count to find the optimum.',
    icon: Route,
  },
  {
    id: 3,
    title: 'Durable Execution',
    description: 'WorkflowEngine executes each step with built-in compensation and automatic rollback on failure.',
    icon: Cog,
  },
  {
    id: 4,
    title: 'Settlement',
    description: 'Double-entry ledger records every movement. Escrow releases. Webhooks fire. Done.',
    icon: Wallet,
  },
]

export default function HowItWorks() {
  return (
    <section className="w-full py-32 bg-black relative overflow-hidden">
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[400px] bg-cyan-900/10 blur-[150px] rounded-full pointer-events-none" />

      <div className="container mx-auto px-4 relative z-10 max-w-7xl">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-24"
        >
          <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-white/5 border border-white/10 mb-6 backdrop-blur-md text-sm font-medium text-cyan-400 tracking-wider uppercase">
            Intent Lifecycle
          </div>
          <h2 className="text-4xl md:text-6xl font-extrabold bg-clip-text text-transparent bg-gradient-to-b from-white to-gray-500 mb-6 tracking-tight">
            From Intent to Settlement
          </h2>
          <p className="text-gray-400 text-xl max-w-2xl mx-auto font-light leading-relaxed">
            Every operation in RampOS follows a declarative intent pipeline — users express <em>what</em>, the engine decides <em>how</em>.
          </p>
        </motion.div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-12 relative mt-16">
          {/* Connection Line */}
          <div className="hidden lg:block absolute top-[48px] left-[10%] w-[80%] h-0.5 bg-gradient-to-r from-transparent via-white/10 to-transparent -z-10" />

          {steps.map((step, index) => (
            <motion.div
              key={step.id}
              initial={{ opacity: 0, y: 30 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.6, delay: index * 0.15 }}
              className="relative flex flex-col items-center text-center group"
            >
              <div className="w-24 h-24 rounded-3xl bg-black border border-white/10 flex items-center justify-center mb-8 group-hover:border-cyan-500/50 group-hover:shadow-[0_0_40px_-10px_rgba(34,211,238,0.4)] transition-all duration-500 relative overflow-hidden">
                <div className="absolute inset-0 bg-gradient-to-b from-cyan-500/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
                <step.icon className="w-10 h-10 text-gray-500 group-hover:text-cyan-400 transition-colors duration-500 relative z-10" />
                <div className="absolute -top-3 -right-3 w-8 h-8 rounded-full bg-black border border-white/20 flex items-center justify-center text-sm font-mono text-gray-400 group-hover:bg-cyan-900/50 group-hover:border-cyan-500/50 group-hover:text-cyan-300 transition-all duration-300 z-20 shadow-xl">
                  {step.id}
                </div>
              </div>

              <h3 className="text-2xl font-bold text-white mb-4 group-hover:text-cyan-400 transition-colors duration-300">
                {step.title}
              </h3>
              <p className="text-gray-400 text-lg leading-relaxed font-light px-4">
                {step.description}
              </p>

              {index !== steps.length - 1 && (
                <div className="lg:hidden absolute bottom-[-40px] left-1/2 w-0.5 h-10 bg-gradient-to-b from-white/10 to-transparent" />
              )}
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}
