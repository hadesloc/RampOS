'use client'

import { motion } from 'framer-motion'
import Link from 'next/link'
import { ArrowRight } from 'lucide-react'

export default function CTASection() {
  return (
    <section className="w-full py-32 relative overflow-hidden">
      {/* Background Gradients */}
      <div className="absolute inset-0 bg-black">
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-full h-full max-w-4xl bg-gradient-to-b from-blue-600/10 via-fuchsia-600/5 to-transparent blur-[100px] rounded-full" />
      </div>

      <div className="container mx-auto px-4 relative z-10">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.8 }}
          className="max-w-4xl mx-auto text-center"
        >
          <h2 className="text-4xl md:text-6xl font-extrabold mb-6 tracking-tight text-white">
            Build your exchange.<br />
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-fuchsia-400">We handle the infrastructure.</span>
          </h2>
          <p className="text-xl md:text-2xl text-gray-400 mb-10 max-w-2xl mx-auto font-light">
            From fiat rails to blockchain settlement, from KYC to custody — RampOS is the operating system your exchange needs.
          </p>

          <div className="flex flex-col sm:flex-row gap-4 justify-center items-center">
            <Link
              href="/vi/portal"
              className="group px-8 py-4 bg-white text-black rounded-full font-bold text-lg hover:bg-gray-200 transition-all transform hover:scale-105 flex items-center gap-2 shadow-[0_0_30px_rgba(255,255,255,0.2)]"
            >
              Get Started <ArrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
            </Link>
            <Link
              href="/docs"
              className="px-8 py-4 bg-transparent border border-white/20 text-white rounded-full font-semibold hover:bg-white/10 transition-all transform hover:scale-105 text-lg"
            >
              Read Documentation
            </Link>
          </div>
        </motion.div>
      </div>
    </section>
  )
}
