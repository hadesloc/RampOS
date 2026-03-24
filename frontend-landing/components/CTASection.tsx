'use client'

import { motion } from 'framer-motion'
import Link from 'next/link'
import { ArrowRight } from 'lucide-react'

export default function CTASection() {
  return (
    <section className="w-full py-32 relative overflow-hidden section-glow">
      {/* Aurora background */}
      <div className="absolute inset-0 aurora-bg-intense" />
      <div className="absolute inset-0 bg-[#050505]/60" />

      {/* Grid overlay */}
      <div className="absolute inset-0 grid-bg opacity-50" />

      {/* Top glow line */}
      <div className="absolute top-0 left-1/2 -translate-x-1/2 w-3/4 h-[1px] bg-gradient-to-r from-transparent via-[#00FF87]/40 to-transparent" />

      <div className="container mx-auto px-4 relative z-10">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.8 }}
          className="max-w-4xl mx-auto text-center"
        >
          <h2 className="text-4xl md:text-6xl lg:text-7xl font-display font-extrabold mb-6 tracking-tight text-white leading-[1.1]">
            Your exchange.
            <br />
            <span className="text-shimmer">Our infrastructure.</span>
          </h2>
          <p className="text-xl md:text-2xl text-gray-400 mb-12 max-w-2xl mx-auto font-light leading-relaxed">
            From fiat rails to blockchain settlement, from KYC to custody — RampOS is the operating system your exchange needs.
          </p>

          <div className="flex flex-col sm:flex-row gap-4 justify-center items-center">
            <Link
              href="/vi/portal"
              className="group px-8 py-4 bg-[#00FF87] text-black rounded-full font-display font-bold text-lg transition-all hover:scale-105 active:scale-95 flex items-center gap-2.5 shadow-[0_0_40px_rgba(0,255,135,0.3)] hover:shadow-[0_0_60px_rgba(0,255,135,0.5)]"
            >
              Get API Keys <ArrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
            </Link>
            <Link
              href="/docs"
              className="px-8 py-4 glass rounded-full font-display font-semibold text-lg text-white hover:bg-white/10 transition-all hover:scale-105"
            >
              Talk to Sales
            </Link>
          </div>
        </motion.div>
      </div>
    </section>
  )
}
