'use client'

import { motion } from 'framer-motion'
import {
  Shield,
  FileCheck,
  Zap,
  Code2,
  Globe,
  HeadphonesIcon
} from 'lucide-react'

const features = [
  {
    title: "Enterprise Security",
    description: "SOC2 Type II certified. Bank-grade encryption and isolated infrastructure for maximum security.",
    icon: Shield,
    color: "text-green-500"
  },
  {
    title: "Global Compliance",
    description: "Built-in KYC/AML screening with automated reporting. Regulatory compliance across 150+ jurisdictions.",
    icon: FileCheck,
    color: "text-blue-500"
  },
  {
    title: "Instant Settlement",
    description: "Lightning fast settlement times with automated reconciliation and real-time ledger updates.",
    icon: Zap,
    color: "text-yellow-500"
  },
  {
    title: "Developer First",
    description: "Modern API design with typed SDKs, webhooks, and comprehensive documentation for easy integration.",
    icon: Code2,
    color: "text-purple-500"
  },
  {
    title: "Global Coverage",
    description: "Accept payments from 150+ countries. Support for local payment methods and 40+ fiat currencies.",
    icon: Globe,
    color: "text-cyan-500"
  },
  {
    title: "24/7 Support",
    description: "Dedicated account managers and round-the-clock technical support for your critical infrastructure.",
    icon: HeadphonesIcon,
    color: "text-pink-500"
  }
]

const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: {
      staggerChildren: 0.1
    }
  }
}

const item = {
  hidden: { opacity: 0, y: 20 },
  show: { opacity: 1, y: 0 }
}

export function FeaturesSection() {
  return (
    <section className="py-24 relative z-10">
      <div className="container mx-auto px-4">
        <div className="text-center max-w-3xl mx-auto mb-16">
          <motion.h2
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
            className="text-3xl md:text-5xl font-bold mb-6 bg-clip-text text-transparent bg-gradient-to-r from-white to-gray-400"
          >
            Everything you need to scale
          </motion.h2>
          <motion.p
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6, delay: 0.2 }}
            className="text-lg text-gray-400"
          >
            A complete financial stack designed for high-growth platforms.
            Reliable, scalable, and secure by default.
          </motion.p>
        </div>

        <motion.div
          variants={container}
          initial="hidden"
          whileInView="show"
          viewport={{ once: true }}
          className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8"
        >
          {features.map((feature, idx) => (
            <FeatureCard key={idx} feature={feature} />
          ))}
        </motion.div>
      </div>
    </section>
  )
}

function FeatureCard({ feature }: { feature: typeof features[0] }) {
  const Icon = feature.icon

  return (
    <motion.div
      variants={item}
      whileHover={{ y: -5 }}
      className="group relative p-8 rounded-3xl border border-gray-800 bg-gray-900/30 hover:bg-gray-900/50 hover:border-gray-700 transition-all duration-300"
    >
      <div className="absolute inset-0 bg-gradient-to-br from-white/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity rounded-3xl" />

      <div className="relative z-10">
        <div className={`p-3 rounded-xl bg-gray-800/50 w-fit mb-6 ${feature.color}`}>
          <Icon className="w-8 h-8" />
        </div>

        <h3 className="text-xl font-bold mb-3 text-white group-hover:text-blue-400 transition-colors">
          {feature.title}
        </h3>

        <p className="text-gray-400 leading-relaxed">
          {feature.description}
        </p>
      </div>
    </motion.div>
  )
}
