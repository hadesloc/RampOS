'use client'

import { motion, useInView } from 'framer-motion'
import { useEffect, useRef, useState } from 'react'

interface StatProps {
  value: number
  suffix: string
  label: string
  accent?: string
}

function AnimatedStat({ value, suffix, label, accent = '#00FF87' }: StatProps) {
  const [display, setDisplay] = useState(0)
  const ref = useRef<HTMLDivElement>(null)
  const isInView = useInView(ref, { once: true, margin: '-50px' })

  useEffect(() => {
    if (!isInView) return

    let startTime: number | null = null
    const duration = 2000

    function step(timestamp: number) {
      if (!startTime) startTime = timestamp
      const progress = Math.min((timestamp - startTime) / duration, 1)
      const eased = 1 - Math.pow(1 - progress, 3)
      setDisplay(Math.round(eased * value))
      if (progress < 1) {
        requestAnimationFrame(step)
      }
    }

    requestAnimationFrame(step)
  }, [isInView, value])

  return (
    <div ref={ref} className="flex flex-col items-center gap-3 px-6 py-5">
      <span
        className="text-4xl md:text-5xl font-display font-extrabold tracking-tight tabular-nums"
        style={{ color: accent }}
      >
        {display}{suffix}
      </span>
      <span className="text-sm text-gray-500 font-medium tracking-wider uppercase">
        {label}
      </span>
    </div>
  )
}

const stats = [
  { value: 7, suffix: '', label: 'Rust Crates', accent: '#00FF87' },
  { value: 5, suffix: '+', label: 'Blockchains', accent: '#7B61FF' },
  { value: 15, suffix: '+', label: 'Core Services', accent: '#00D4FF' },
  { value: 10, suffix: '', label: 'Smart Contracts', accent: '#00FF87' },
]

export default function StatsBar() {
  return (
    <section className="w-full py-20 relative z-10">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.6 }}
        className="container mx-auto px-4 max-w-5xl"
      >
        <div className="relative rounded-2xl overflow-hidden">
          {/* Gradient border effect */}
          <div className="absolute inset-0 rounded-2xl bg-gradient-to-r from-[#00FF87]/20 via-[#7B61FF]/20 to-[#00D4FF]/20 p-[1px]" />
          <div className="relative glass rounded-2xl">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-0 divide-x divide-white/[0.06]">
              {stats.map((s, i) => (
                <AnimatedStat key={i} {...s} />
              ))}
            </div>
          </div>
        </div>
      </motion.div>
    </section>
  )
}
