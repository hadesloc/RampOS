'use client'

import { motion, useInView } from 'framer-motion'
import { useEffect, useRef, useState } from 'react'

interface StatProps {
  value: number
  suffix: string
  label: string
}

function AnimatedStat({ value, suffix, label }: StatProps) {
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
      const eased = 1 - Math.pow(1 - progress, 3) // easeOut
      setDisplay(Math.round(eased * value))
      if (progress < 1) {
        requestAnimationFrame(step)
      }
    }

    requestAnimationFrame(step)
  }, [isInView, value])

  return (
    <div ref={ref} className="flex flex-col items-center gap-2">
      <span className="text-4xl md:text-5xl font-extrabold tracking-tight text-white tabular-nums">
        {display}{suffix}
      </span>
      <span className="text-sm md:text-base text-gray-400 font-medium tracking-wide uppercase">
        {label}
      </span>
    </div>
  )
}

const stats = [
  { value: 7, suffix: '', label: 'Rust Crates' },
  { value: 5, suffix: '+', label: 'Blockchains' },
  { value: 15, suffix: '+', label: 'Core Services' },
  { value: 10, suffix: '', label: 'Smart Contracts' },
]

export default function StatsBar() {
  return (
    <section className="w-full py-16 relative z-10">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.6 }}
        className="container mx-auto px-4 max-w-5xl"
      >
        <div className="grid grid-cols-2 md:grid-cols-4 gap-8 md:gap-12 p-8 rounded-3xl border border-white/5 bg-white/[0.02] backdrop-blur-xl">
          {stats.map((s, i) => (
            <AnimatedStat key={i} {...s} />
          ))}
        </div>
      </motion.div>
    </section>
  )
}
