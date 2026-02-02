'use client'

import { motion } from 'framer-motion'
import { Check, Copy } from 'lucide-react'
import { useState } from 'react'
import { cn } from '@/lib/utils'

const codeSnippet = `// Initialize RampOS Client
const client = new RampClient({
  apiKey: 'pk_live_...',
  environment: 'production'
});

// Create a Payment Intent
const intent = await client.intents.create({
  amount: 1000,
  currency: 'USD',
  user: {
    id: 'user_123',
    email: 'user@example.com'
  }
});

console.log('Payment Intent Created:', intent.id);`

export default function ApiSection() {
  const [copied, setCopied] = useState(false)

  const handleCopy = () => {
    navigator.clipboard.writeText(codeSnippet)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <section className="w-full py-24 bg-zinc-950">
      <div className="container mx-auto px-4 max-w-6xl">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
          <motion.div
            initial={{ opacity: 0, x: -20 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.8 }}
          >
            <h2 className="text-3xl md:text-5xl font-bold mb-6 bg-clip-text text-transparent bg-gradient-to-r from-white to-gray-400">
              Developer-First API
            </h2>
            <p className="text-xl text-gray-400 mb-8 leading-relaxed">
              Built by developers, for developers. Our API is designed to be intuitive,
              predictable, and easy to integrate. Get started in minutes with our SDKs.
            </p>
            <ul className="space-y-4 mb-8">
              {['Type-safe SDKs for all major languages', 'Comprehensive documentation & examples', 'Webhooks for real-time updates', 'Sandbox environment for testing'].map((item, index) => (
                <li key={index} className="flex items-center gap-3 text-gray-300">
                  <div className="w-6 h-6 rounded-full bg-blue-500/10 flex items-center justify-center">
                    <Check className="w-4 h-4 text-blue-500" />
                  </div>
                  {item}
                </li>
              ))}
            </ul>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, x: 20 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.8 }}
            className="relative"
          >
            <div className="absolute inset-0 bg-gradient-to-r from-blue-500 to-purple-500 rounded-2xl blur-3xl opacity-10" />
            <div className="relative rounded-2xl border border-white/10 bg-black/50 backdrop-blur-xl overflow-hidden shadow-2xl">
              <div className="flex items-center justify-between px-4 py-3 border-b border-white/10 bg-white/5">
                <div className="flex gap-2">
                  <div className="w-3 h-3 rounded-full bg-red-500/20 border border-red-500/50" />
                  <div className="w-3 h-3 rounded-full bg-yellow-500/20 border border-yellow-500/50" />
                  <div className="w-3 h-3 rounded-full bg-green-500/20 border border-green-500/50" />
                </div>
                <div className="text-xs text-gray-500 font-mono">example.ts</div>
                <button
                  onClick={handleCopy}
                  className="p-1.5 rounded-md hover:bg-white/10 transition-colors"
                >
                  {copied ? (
                    <Check className="w-4 h-4 text-green-500" />
                  ) : (
                    <Copy className="w-4 h-4 text-gray-400" />
                  )}
                </button>
              </div>
              <div className="p-6 overflow-x-auto">
                <pre className="text-sm font-mono leading-relaxed">
                  <code className="text-gray-300">
                    <span className="text-purple-400">{'// Initialize RampOS Client'}</span>
                    {'\n'}
                    <span className="text-blue-400">const</span> client = <span className="text-blue-400">new</span> <span className="text-yellow-400">RampClient</span>({'{'}
                    {'\n'}  apiKey: <span className="text-green-400">&apos;pk_live_...&apos;</span>,
                    {'\n'}  environment: <span className="text-green-400">&apos;production&apos;</span>
                    {'\n'}{'}'});
                    {'\n'}
                    {'\n'}
                    <span className="text-purple-400">{'// Create a Payment Intent'}</span>
                    {'\n'}
                    <span className="text-blue-400">const</span> intent = <span className="text-blue-400">await</span> client.intents.<span className="text-yellow-400">create</span>({'{'}
                    {'\n'}  amount: <span className="text-orange-400">1000</span>,
                    {'\n'}  currency: <span className="text-green-400">&apos;USD&apos;</span>,
                    {'\n'}  user: {'{'}
                    {'\n'}    id: <span className="text-green-400">&apos;user_123&apos;</span>,
                    {'\n'}    email: <span className="text-green-400">&apos;user@example.com&apos;</span>
                    {'\n'}  {'}'}
                    {'\n'}{'}'});
                    {'\n'}
                    {'\n'}
                    console.<span className="text-yellow-400">log</span>(<span className="text-green-400">&apos;Payment Intent Created:&apos;</span>, intent.id);
                  </code>
                </pre>
              </div>
            </div>
          </motion.div>
        </div>
      </div>
    </section>
  )
}
