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
    <section className="w-full py-32 relative">
      <div className="absolute top-0 right-0 w-[500px] h-[500px] bg-[#7B61FF]/5 blur-[180px] rounded-full pointer-events-none" />

      <div className="container mx-auto px-4 max-w-7xl relative z-10">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-16 items-start">
          <motion.div
            initial={{ opacity: 0, x: -20 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.8 }}
          >
            <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full glass text-sm font-medium text-[#00D4FF] tracking-wider uppercase mb-6">
              Developer Experience
            </div>
            <h2 className="text-4xl md:text-5xl font-display font-bold mb-6 tracking-tight">
              One API.{' '}
              <span className="text-shimmer">Every Language.</span>
            </h2>
            <p className="text-xl text-gray-400 mb-8 leading-relaxed font-light">
              Integrate fiat on-ramp, cross-chain swap, and compliance in one unified API. Full type safety across every SDK.
            </p>

            <ul className="space-y-3 mb-8">
              {features.map((feat, index) => (
                <li key={index} className="flex items-center gap-3 text-gray-300">
                  <div className="w-6 h-6 rounded-full bg-[#00FF87]/10 flex items-center justify-center shrink-0 border border-[#00FF87]/20">
                    <Check className="w-3.5 h-3.5 text-[#00FF87]" />
                  </div>
                  <span className="font-light text-[15px]">{feat}</span>
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
            {/* Glow border */}
            <div className="absolute -inset-[1px] bg-gradient-to-br from-[#00FF87]/20 via-[#7B61FF]/20 to-[#00D4FF]/20 rounded-2xl blur-sm" />

            <div className="relative rounded-2xl bg-[#0A0A0C] border border-white/[0.06] overflow-hidden">
              {/* Tab bar */}
              <div className="flex items-center border-b border-white/[0.06] bg-white/[0.02] overflow-x-auto">
                {tabs.map((tab, i) => (
                  <button
                    key={i}
                    onClick={() => setActiveTab(i)}
                    className={`px-5 py-3 text-xs font-body font-medium tracking-wider transition-all whitespace-nowrap relative ${
                      i === activeTab
                        ? 'text-white'
                        : 'text-gray-500 hover:text-gray-300'
                    }`}
                  >
                    {tab.lang}
                    {i === activeTab && (
                      <motion.div
                        layoutId="activeTab"
                        className="absolute bottom-0 left-0 right-0 h-[2px] bg-gradient-to-r from-[#00FF87] to-[#00D4FF]"
                      />
                    )}
                  </button>
                ))}
                <div className="ml-auto pr-3">
                  <button
                    onClick={handleCopy}
                    className="p-2 rounded-lg hover:bg-white/5 transition-colors group"
                  >
                    {copied ? (
                      <Check className="w-4 h-4 text-[#00FF87]" />
                    ) : (
                      <Copy className="w-4 h-4 text-gray-500 group-hover:text-white transition-colors" />
                    )}
                  </button>
                </div>
              </div>

              {/* Code */}
              <div className="p-6 overflow-x-auto max-h-[450px] overflow-y-auto">
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
