'use client'

import { motion } from 'framer-motion'
import { Code2, ShieldCheck, Wallet, Zap } from 'lucide-react'

const steps = [
  {
    id: 1,
    title: 'Integrate API',
    description: 'Connect to our REST API in minutes with simple configuration.',
    icon: Code2,
  },
  {
    id: 2,
    title: 'KYC your users',
    description: 'Automated identity verification for compliance and safety.',
    icon: ShieldCheck,
  },
  {
    id: 3,
    title: 'Accept payments',
    description: 'Support for VND and Crypto payments seamlessly.',
    icon: Wallet,
  },
  {
    id: 4,
    title: 'Instant settlement',
    description: 'Real-time reconciliation and fund availability.',
    icon: Zap,
  },
]

export default function HowItWorks() {
  return (
    <section className="w-full py-24 bg-black relative overflow-hidden">
      {/* Background decoration */}
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[400px] bg-blue-900/10 blur-[100px] rounded-full" />

      <div className="container mx-auto px-4 relative z-10">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-16"
        >
          <h2 className="text-3xl md:text-5xl font-bold bg-clip-text text-transparent bg-gradient-to-b from-white to-gray-500 mb-6">
            How It Works
          </h2>
          <p className="text-gray-400 text-lg md:text-xl max-w-2xl mx-auto">
            Get started with RampOS in four simple steps. From integration to settlement in minutes.
          </p>
        </motion.div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8 relative">
          {/* Connection Line */}
          <div className="hidden lg:block absolute top-12 left-0 w-full h-0.5 bg-gradient-to-r from-gray-800 via-gray-700 to-gray-800 -z-10" />

          {steps.map((step, index) => (
            <motion.div
              key={step.id}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: index * 0.2 }}
              className="relative flex flex-col items-center text-center group"
            >
              {/* Step Number Badge */}
              <div className="w-24 h-24 rounded-2xl bg-gray-900 border border-gray-800 flex items-center justify-center mb-6 group-hover:border-blue-500/50 group-hover:shadow-[0_0_30px_-5px_rgba(59,130,246,0.3)] transition-all duration-300 relative">
                <div className="absolute inset-0 bg-blue-500/10 opacity-0 group-hover:opacity-100 transition-opacity duration-300 rounded-2xl" />
                <step.icon className="w-10 h-10 text-gray-400 group-hover:text-blue-400 transition-colors duration-300" />

                {/* Step Counter */}
                <div className="absolute -top-3 -right-3 w-8 h-8 rounded-full bg-gray-800 border border-gray-700 flex items-center justify-center text-sm font-mono text-gray-400 group-hover:bg-blue-900/50 group-hover:border-blue-500/50 group-hover:text-blue-300 transition-all duration-300">
                  {step.id}
                </div>
              </div>

              <h3 className="text-xl font-semibold text-white mb-3 group-hover:text-blue-400 transition-colors">
                {step.title}
              </h3>
              <p className="text-gray-400 text-sm leading-relaxed">
                {step.description}
              </p>

              {/* Mobile connector line */}
              {index !== steps.length - 1 && (
                <div className="lg:hidden absolute bottom-[-32px] left-1/2 w-0.5 h-8 bg-gray-800" />
              )}
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}
