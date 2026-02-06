'use client'

import { motion } from 'framer-motion'
import {
  ArrowRight,
  Shield,
  FileCheck,
  Zap,
  Code2,
  Globe,
  HeadphonesIcon
} from 'lucide-react'
import Link from 'next/link'

import ApiSection from '@/components/ApiSection'
import CTASection from '@/components/CTASection'
import Footer from '@/components/Footer'

import HowItWorks from '@/components/HowItWorks'

const features = [
  {
    title: "Global Coverage",
    description: "Accept payments from 150+ countries. Support for local payment methods and 40+ fiat currencies.",
    icon: Globe,
    color: "text-cyan-500"
  },
  {
    title: "Enterprise Security",
    description: "SOC2 Type II certified. Bank-grade encryption and isolated infrastructure for maximum security.",
    icon: Shield,
    color: "text-green-500"
  },
  {
    title: "Instant Settlement",
    description: "Lightning fast settlement times with automated reconciliation and real-time ledger updates.",
    icon: Zap,
    color: "text-yellow-500"
  },
  {
    title: "Compliance Ready",
    description: "Built-in KYC/AML screening with automated reporting. Regulatory compliance across 150+ jurisdictions.",
    icon: FileCheck,
    color: "text-blue-500"
  },
  {
    title: "Developer First",
    description: "Modern API design with typed SDKs, webhooks, and comprehensive documentation for easy integration.",
    icon: Code2,
    color: "text-purple-500"
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

export default function Home() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-between p-24">
      <div className="z-10 w-full max-w-5xl items-center justify-between font-mono text-sm lg:flex">
        <p className="fixed left-0 top-0 flex w-full justify-center border-b border-gray-800 bg-gradient-to-b from-zinc-800/30 pb-6 pt-8 backdrop-blur-2xl lg:static lg:w-auto lg:rounded-xl lg:border lg:bg-gray-900/50 lg:p-4">
          RAMP OS&nbsp;
          <code className="font-mono font-bold">v1.0.0</code>
        </p>
        <div className="fixed bottom-0 left-0 flex h-48 w-full items-end justify-center bg-gradient-to-t from-black via-black lg:static lg:h-auto lg:w-auto lg:bg-none">
          <a
            className="pointer-events-none flex place-items-center gap-2 p-8 lg:pointer-events-auto lg:p-0"
            href="https://ramp.network"
            target="_blank"
            rel="noopener noreferrer"
          >
            By{' '}
            <span className="font-bold">Antigravity</span>
          </a>
        </div>
      </div>

      <div className="relative flex place-items-center before:absolute before:h-[300px] before:w-[480px] before:-translate-x-1/2 before:rounded-full before:bg-gradient-to-br before:from-transparent before:to-blue-700 before:opacity-10 before:blur-2xl before:content-[''] after:absolute after:-z-20 after:h-[180px] after:w-[240px] after:translate-x-1/3 after:bg-gradient-to-t after:from-cyan-900 after:via-cyan-800 after:blur-2xl after:content-[''] before:lg:h-[360px]">
        <div className="relative z-10 text-center max-w-4xl mx-auto">
            <motion.h1
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.8 }}
                className="text-6xl md:text-8xl font-bold tracking-tight mb-8 bg-clip-text text-transparent bg-gradient-to-b from-white to-gray-500"
            >
                Global Financial Infrastructure
            </motion.h1>

            <motion.p
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.8, delay: 0.2 }}
                className="text-xl md:text-2xl text-gray-400 mb-12 max-w-2xl mx-auto"
            >
                The fastest way to integrate fiat-to-crypto payments.
                Compliant, secure, and developer-first.
            </motion.p>

            <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.8, delay: 0.4 }}
                className="flex flex-col sm:flex-row gap-4 justify-center items-center"
            >
                <Link href="/dashboard" className="px-8 py-4 bg-white text-black rounded-full font-semibold hover:bg-gray-200 transition-colors flex items-center gap-2">
                    Get Started <ArrowRight className="w-4 h-4" />
                </Link>
                <Link href="/docs" className="px-8 py-4 bg-gray-900 text-white border border-gray-800 rounded-full font-semibold hover:bg-gray-800 transition-colors">
                    Read Documentation
                </Link>
            </motion.div>
        </div>
      </div>

      <section className="py-24 relative z-10 w-full">
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

      <HowItWorks />

      <ApiSection />
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
