'use client'

import { motion } from 'framer-motion'
import { Code2, UserCheck, Wallet, RefreshCw } from 'lucide-react'

const steps = [
  {
    id: 1,
    title: 'Integrate API',
    description: 'Get up and running in minutes with our developer-friendly API and SDKs. Drop-in UI components make integration seamless.',
    icon: Code2,
    color: 'text-blue-500',
    bg: 'bg-blue-500/10',
    border: 'border-blue-500/20'
  },
  {
    id: 2,
    title: 'KYC your users',
    description: 'Automated identity verification handles compliance globally. We support 150+ countries with instant document checks.',
    icon: UserCheck,
    color: 'text-green-500',
    bg: 'bg-green-500/10',
    border: 'border-green-500/20'
  },
  {
    id: 3,
    title: 'Accept payments',
    description: 'Process payments in local currencies including VND. Support for credit cards, bank transfers, and crypto.',
    icon: Wallet,
    color: 'text-purple-500',
    bg: 'bg-purple-500/10',
    border: 'border-purple-500/20'
  },
  {
    id: 4,
    title: 'Instant settlement',
    description: 'Real-time reconciliation and instant settlement to your preferred account. Complete transparency and control.',
    icon: RefreshCw,
    color: 'text-yellow-500',
    bg: 'bg-yellow-500/10',
    border: 'border-yellow-500/20'
  }
]

export default function HowItWorks() {
  return (
    <section className="py-32 relative overflow-hidden">
      {/* Background Elements */}
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[800px] bg-blue-900/10 rounded-full blur-[100px] -z-10" />

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="text-center mb-20">
          <motion.h2
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="text-3xl md:text-5xl font-bold bg-clip-text text-transparent bg-gradient-to-b from-white to-gray-500 mb-6"
          >
            How it works
          </motion.h2>
          <motion.p
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ delay: 0.2 }}
            className="text-gray-400 text-lg max-w-2xl mx-auto"
          >
            Seamlessly integrate financial infrastructure into your product in four simple steps.
          </motion.p>
        </div>

        <div className="relative">
          {/* Connecting Line (Desktop) */}
          <div className="hidden lg:block absolute top-1/2 left-0 w-full h-0.5 bg-gradient-to-r from-transparent via-gray-800 to-transparent -translate-y-1/2 -z-10" />

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8">
            {steps.map((step, index) => (
              <motion.div
                key={step.id}
                initial={{ opacity: 0, y: 30 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ delay: index * 0.2 }}
                className="relative group"
              >
                <div className={`
                  h-full p-8 rounded-2xl border bg-black/50 backdrop-blur-sm
                  transition-all duration-300 hover:y-[-5px]
                  ${step.border} group-hover:bg-gray-900/50
                `}>
                  <div className={`
                    w-12 h-12 rounded-xl flex items-center justify-center mb-6
                    ${step.bg} ${step.color}
                  `}>
                    <step.icon className="w-6 h-6" />
                  </div>

                  <div className="absolute -top-4 -right-4 w-8 h-8 rounded-full bg-gray-900 border border-gray-800 flex items-center justify-center text-sm font-mono text-gray-500">
                    {step.id}
                  </div>

                  <h3 className="text-xl font-semibold text-white mb-3">
                    {step.title}
                  </h3>
                  <p className="text-gray-400 text-sm leading-relaxed">
                    {step.description}
                  </p>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </div>
    </section>
  )
}
