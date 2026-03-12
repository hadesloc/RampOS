'use client'

import { motion } from 'framer-motion'
import { Check, Copy } from 'lucide-react'
import { useState } from 'react'

const tabs = [
  {
    lang: 'TypeScript',
    file: 'example.ts',
    code: `import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: 'your_api_key',
  baseUrl: 'https://api.rampos.io'
});

// Create a Pay-in Intent (Fiat → Crypto)
const payin = await client.payins.create({
  userId: 'usr_123',
  amountVnd: 10_000_000,
  railsProvider: 'VIETCOMBANK'
});

// Submit a cross-chain swap intent
const intent = await client.intents.create({
  action: 'Swap',
  from: { chain: 'ethereum', token: 'USDC' },
  to:   { chain: 'arbitrum',  token: 'USDT' },
  amount: 1000,
  constraints: { maxSlippageBps: 50 }
});

console.log(intent.executionPlan);`,
  },
  {
    lang: 'Go',
    file: 'main.go',
    code: `import "github.com/hadesloc/rampos-go"

client := rampos.NewClient("your_api_key")

payin, err := client.Payins.Create(ctx,
  &rampos.CreatePayinRequest{
    UserID:    "usr_123",
    AmountVND: 10_000_000,
    Provider:  "VIETCOMBANK",
  })

intent, err := client.Intents.Create(ctx,
  &rampos.IntentSpec{
    Action: rampos.Swap,
    From:   rampos.Asset{Chain: "ethereum", Token: "USDC"},
    To:     rampos.Asset{Chain: "arbitrum", Token: "USDT"},
    Amount: 1000,
  })`,
  },
  {
    lang: 'Python',
    file: 'app.py',
    code: `from rampos import RampOSClient

client = RampOSClient(api_key="your_api_key")

payin = client.payins.create(
    user_id="usr_123",
    amount_vnd=10_000_000,
    rails_provider="VIETCOMBANK"
)

intent = client.intents.create(
    action="Swap",
    from_asset={"chain": "ethereum", "token": "USDC"},
    to_asset={"chain": "arbitrum", "token": "USDT"},
    amount=1000,
    constraints={"max_slippage_bps": 50}
)`,
  },
  {
    lang: 'cURL',
    file: 'terminal',
    code: `curl -X POST https://api.rampos.io/v1/intents/payin \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -H "Content-Type: application/json" \\
  -H "Idempotency-Key: unique-key-123" \\
  -d '{
    "user_id": "usr_123",
    "amount_vnd": 10000000,
    "rails_provider": "VIETCOMBANK"
  }'`,
  },
]

const features = [
  'Typed SDKs — TypeScript, Go, Python',
  'Guaranteed webhook delivery with HMAC',
  'Idempotency keys for safe retries',
  'OpenTelemetry tracing built-in',
  'Rate limiting per tenant',
  'Sandbox environment for testing',
]

export default function ApiSection() {
  const [copied, setCopied] = useState(false)
  const [activeTab, setActiveTab] = useState(0)

  const handleCopy = () => {
    navigator.clipboard.writeText(tabs[activeTab].code)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <section className="w-full py-24 bg-black relative">
      <div className="absolute top-0 right-0 w-[600px] h-[600px] bg-blue-900/10 blur-[120px] rounded-full pointer-events-none" />
      
      <div className="container mx-auto px-4 max-w-7xl relative z-10">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-16 items-start">
          <motion.div
            initial={{ opacity: 0, x: -20 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.8 }}
          >
            <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-white/5 border border-white/10 mb-6 backdrop-blur-md text-sm font-medium text-blue-400 tracking-wider uppercase">
              Developer Experience
            </div>
            <h2 className="text-4xl md:text-5xl font-bold mb-6 tracking-tight">
              One API.<br />
              <span className="text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-cyan-400">Every Language.</span>
            </h2>
            <p className="text-xl text-gray-400 mb-8 leading-relaxed font-light">
              Integrate fiat on-ramp, cross-chain trading, and compliance in one unified API. SDKs for every major language with full type safety.
            </p>

            <ul className="space-y-3 mb-8">
              {features.map((item, index) => (
                <li key={index} className="flex items-center gap-3 text-gray-300">
                  <div className="w-6 h-6 rounded-full bg-blue-500/10 flex items-center justify-center shrink-0 border border-blue-500/20">
                    <Check className="w-3.5 h-3.5 text-blue-400" />
                  </div>
                  <span className="font-light">{item}</span>
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
            <div className="absolute -inset-1 bg-gradient-to-r from-blue-500/20 to-cyan-500/20 rounded-3xl blur opacity-50" />
            <div className="relative rounded-2xl border border-white/10 bg-black/60 backdrop-blur-2xl overflow-hidden shadow-2xl">
              {/* Tabs */}
              <div className="flex items-center border-b border-white/10 bg-white/5 overflow-x-auto">
                {tabs.map((tab, i) => (
                  <button
                    key={i}
                    onClick={() => setActiveTab(i)}
                    className={`px-4 py-3 text-xs font-mono tracking-wider transition-colors whitespace-nowrap ${
                      i === activeTab
                        ? 'text-white bg-white/5 border-b-2 border-blue-400'
                        : 'text-gray-500 hover:text-gray-300'
                    }`}
                  >
                    {tab.lang}
                  </button>
                ))}
                <div className="ml-auto pr-3">
                  <button onClick={handleCopy} className="p-1.5 rounded-md hover:bg-white/10 transition-colors">
                    {copied ? <Check className="w-4 h-4 text-green-400" /> : <Copy className="w-4 h-4 text-gray-400" />}
                  </button>
                </div>
              </div>

              {/* Code */}
              <div className="p-6 overflow-x-auto max-h-[420px] overflow-y-auto">
                <pre className="text-sm font-mono leading-relaxed">
                  <code className="text-gray-300 whitespace-pre">{tabs[activeTab].code}</code>
                </pre>
              </div>
            </div>
          </motion.div>
        </div>
      </div>
    </section>
  )
}
