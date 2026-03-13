'use client'

import { useState } from 'react'
import { motion } from 'framer-motion'
import Link from 'next/link'
import {
  BookOpen,
  Layers,
  Server,
  Database,
  FileCode2,
  Package,
  Terminal,
  Plug,
  Cloud,
  Building2,
  TestTubes,
  Shield,
  Settings,
  Scale,
  Code2,
  Scroll,
  ArrowLeft,
  Menu,
  X,
  ExternalLink,
  ChevronDown,
  ChevronRight,
  Search,
} from 'lucide-react'

const GITHUB_BASE = 'https://github.com/hadesloc/RampOS/blob/main'

interface DocLink {
  title: string
  path: string
  description?: string
}

interface DocSection {
  id: string
  title: string
  description: string
  icon: React.ElementType
  color: string
  docs: DocLink[]
}

const docSections: DocSection[] = [
  {
    id: 'getting-started',
    title: 'Getting Started',
    description: 'Quick start guide, core concepts, and first steps with RampOS.',
    icon: BookOpen,
    color: 'text-emerald-400',
    docs: [
      { title: 'Quick Start Guide', path: 'docs/getting-started/README.md', description: 'Installation, setup, and first API call' },
      { title: 'Core Concepts', path: 'docs/getting-started/concepts.md', description: 'Intents, tenants, ledger, compliance fundamentals' },
      { title: 'Whitepaper', path: 'docs/whitepaper.md', description: 'Full technical whitepaper and system design rationale' },
    ],
  },
  {
    id: 'architecture',
    title: 'Architecture',
    description: 'System design, state machines, ledger model, and compliance engine internals.',
    icon: Layers,
    color: 'text-cyan-400',
    docs: [
      { title: 'System Overview', path: 'docs/architecture/overview.md', description: 'High-level architecture, crate structure, data flow' },
      { title: 'State Machines', path: 'docs/architecture/state-machine.md', description: 'Pay-in, pay-out, trade, and RFQ state transitions' },
      { title: 'Double-Entry Ledger', path: 'docs/architecture/ledger.md', description: 'Accounting model, journal entries, reconciliation' },
      { title: 'Compliance Engine', path: 'docs/architecture/compliance.md', description: 'KYC/AML pipeline, risk scoring, case management' },
      { title: 'Architecture Summary', path: 'docs/architecture.md', description: 'Concise architecture reference card' },
    ],
  },
  {
    id: 'api-reference',
    title: 'API Reference',
    description: 'REST API endpoints, authentication, rate limiting, webhooks, and event catalog.',
    icon: Server,
    color: 'text-blue-400',
    docs: [
      { title: 'API Overview', path: 'docs/api/README.md', description: 'Base URL, authentication, headers, versioning' },
      { title: 'Full API Documentation', path: 'docs/API.md', description: 'Complete endpoint reference with request/response examples' },
      { title: 'Endpoints Reference', path: 'docs/api/endpoints.md', description: 'All admin, portal, and LP endpoints' },
      { title: 'Authentication', path: 'docs/api/authentication.md', description: 'JWT, API keys, LP keys, admin auth' },
      { title: 'Rate Limiting', path: 'docs/api/rate-limiting.md', description: 'Per-tenant limits, sliding window, override config' },
      { title: 'Webhooks', path: 'docs/api/webhooks.md', description: 'Event types, HMAC signing, retry policy, DLQ' },
    ],
  },
  {
    id: 'database',
    title: 'Database',
    description: 'PostgreSQL schema design, migration system, and row-level security policies.',
    icon: Database,
    color: 'text-amber-400',
    docs: [
      { title: 'Schema Reference', path: 'docs/database/schema.md', description: '49 tables, indexes, constraints, and relationships' },
      { title: 'Migrations Guide', path: 'docs/database/migrations.md', description: 'Migration system, ordering, up/down scripts' },
      { title: 'Row-Level Security', path: 'docs/database/rls.md', description: 'Tenant isolation policies, fail-closed design' },
    ],
  },
  {
    id: 'smart-contracts',
    title: 'Smart Contracts',
    description: 'Solidity contracts for account abstraction, paymaster, passkey, and ZK KYC.',
    icon: FileCode2,
    color: 'text-fuchsia-400',
    docs: [
      { title: 'Contracts Overview', path: 'docs/contracts/overview.md', description: '10 contracts, Foundry toolchain, deployment' },
      { title: 'Smart Account (ERC-4337)', path: 'docs/contracts/account.md', description: 'RampOSAccount, factory, batch execution, UUPS' },
      { title: 'Paymaster', path: 'docs/contracts/paymaster.md', description: 'Gas sponsorship, budget management, nonce protection' },
      { title: 'Security Considerations', path: 'docs/contracts/security.md', description: 'Audit findings, access control, upgrade safety' },
    ],
  },
  {
    id: 'sdks',
    title: 'SDKs',
    description: 'Client libraries for TypeScript, Go, and Python with full type safety.',
    icon: Package,
    color: 'text-indigo-400',
    docs: [
      { title: 'SDK Overview', path: 'docs/SDK.md', description: 'Installation, configuration, quick start for all SDKs' },
      { title: 'TypeScript SDK', path: 'sdk/README.md', description: '@rampos/sdk — React Query hooks, type-safe client' },
      { title: 'Go SDK', path: 'sdk-go/README.md', description: 'rampos-go — idiomatic Go client with context support' },
      { title: 'Python SDK', path: 'sdk-python/README.md', description: 'rampos — async/sync client with Pydantic models' },
    ],
  },
  {
    id: 'cli',
    title: 'CLI',
    description: 'Command-line interface for automation, scripting, and AI agent integration.',
    icon: Terminal,
    color: 'text-lime-400',
    docs: [
      { title: 'CLI Overview', path: 'docs/cli/README.md', description: 'Install, auth modes, output formats' },
      { title: 'Agent Usage', path: 'docs/cli/agent-usage.md', description: 'Machine-friendly flags for AI agent workflows' },
      { title: 'Coverage Ledger', path: 'docs/cli/coverage-ledger.md', description: 'Endpoint coverage tracking and validation' },
    ],
  },
  {
    id: 'integrations',
    title: 'Integrations',
    description: 'Connect with banks, KYC providers, compliance engines, and wallets.',
    icon: Plug,
    color: 'text-orange-400',
    docs: [
      { title: 'Bank Adapter', path: 'docs/integrations/bank-adapter.md', description: 'Pluggable bank/PSP integration with Rails trait' },
      { title: 'KYC Provider', path: 'docs/integrations/kyc-provider.md', description: 'Onfido, eKYC, custom provider integration' },
      { title: 'Compliance Rules', path: 'docs/integrations/compliance-rules.md', description: 'AML velocity, sanctions, fraud scoring config' },
      { title: 'Wallet Integration', path: 'docs/integrations/wallet-integration.md', description: 'Account abstraction, passkey, EOA delegation' },
    ],
  },
  {
    id: 'deployment',
    title: 'Deployment',
    description: 'Local development, Docker, Kubernetes, and CI/CD pipeline configuration.',
    icon: Cloud,
    color: 'text-sky-400',
    docs: [
      { title: 'Local Development', path: 'docs/deployment/local.md', description: 'Docker Compose, environment setup, hot reload' },
      { title: 'Kubernetes', path: 'docs/deployment/kubernetes.md', description: 'Kustomize, HPA, PDB, network policies' },
      { title: 'CI/CD Pipeline', path: 'docs/deployment/ci-cd.md', description: 'GitHub Actions, ArgoCD, drift detection' },
      { title: 'Deployment Guide', path: 'docs/DEPLOY.md', description: 'Step-by-step production deployment' },
      { title: 'Deployment Checklist', path: 'docs/DEPLOYMENT_CHECKLIST.md', description: 'Pre-launch verification checklist' },
    ],
  },
  {
    id: 'enterprise',
    title: 'Enterprise',
    description: 'Multi-tenant operations, SSO, API limits, and enterprise configuration.',
    icon: Building2,
    color: 'text-violet-400',
    docs: [
      { title: 'Enterprise Overview', path: 'docs/enterprise/README.md', description: 'Enterprise features and deployment options' },
      { title: 'SSO Setup', path: 'docs/enterprise/sso-setup.md', description: 'OIDC/SAML integration, provider configuration' },
      { title: 'API Limits', path: 'docs/enterprise/api-limits.md', description: 'Custom rate limits, tier management' },
      { title: 'Configuration', path: 'docs/enterprise/configuration.md', description: 'Config bundles, environment-aware settings' },
      { title: 'Operations Guide', path: 'docs/enterprise/operations.md', description: 'Monitoring, alerting, incident response' },
      { title: 'Enterprise Deployment', path: 'docs/enterprise/deployment.md', description: 'HA setup, scaling, disaster recovery' },
    ],
  },
  {
    id: 'testing',
    title: 'Testing',
    description: 'Unit tests, integration tests, load tests, and smart contract test suites.',
    icon: TestTubes,
    color: 'text-teal-400',
    docs: [
      { title: 'Testing Guide', path: 'docs/TESTING-GUIDE.md', description: 'Testing strategy and best practices' },
      { title: 'Unit Tests', path: 'docs/testing/unit-tests.md', description: 'Rust unit tests, mocking, coverage' },
      { title: 'Integration Tests', path: 'docs/testing/integration-tests.md', description: 'API integration tests, test fixtures' },
      { title: 'Load Tests', path: 'docs/testing/load-tests.md', description: 'Performance benchmarks, stress testing' },
      { title: 'Contract Tests', path: 'docs/testing/contract-tests.md', description: 'Foundry tests, fuzz testing, invariants' },
    ],
  },
  {
    id: 'security',
    title: 'Security',
    description: 'Threat model, security audits, hardening guides, and remediation reports.',
    icon: Shield,
    color: 'text-red-400',
    docs: [
      { title: 'Security Overview', path: 'docs/SECURITY.md', description: 'Security architecture and practices' },
      { title: 'Threat Model', path: 'docs/security/threat-model.md', description: 'Attack vectors, risk assessment, mitigations' },
      { title: 'Security Hardening', path: 'docs/security/hardening.md', description: 'Infrastructure and application hardening guide' },
      { title: 'Audit Report', path: 'docs/security/audit-report.md', description: 'Comprehensive security audit findings' },
      { title: 'Remediation Plan', path: 'docs/security/remediation-plan.md', description: 'Issue tracking and fix timeline' },
      { title: 'Roadmap & Hardening', path: 'docs/recent-roadmap-and-security-hardening-2026-03.md', description: 'March 2026 security hardening report' },
    ],
  },
  {
    id: 'operations',
    title: 'Operations',
    description: 'Runbooks, monitoring, disaster recovery, and release management.',
    icon: Settings,
    color: 'text-gray-400',
    docs: [
      { title: 'Monitoring Guide', path: 'docs/operations/monitoring.md', description: 'Prometheus, Grafana, alerting rules' },
      { title: 'Runbook', path: 'docs/operations/runbook-skeleton.md', description: 'Operational procedures for common scenarios' },
      { title: 'Disaster Recovery', path: 'docs/operations/disaster-recovery-plan.md', description: 'Backup, restore, and failover procedures' },
      { title: 'Release Checklist', path: 'docs/operations/release-checklist.md', description: 'Pre-release verification steps' },
      { title: 'Staging Validation', path: 'docs/operations/staging-validation-plan.md', description: 'QA matrix for staging environments' },
      { title: 'Bank-Grade Signoff', path: 'docs/operations/bank-grade-signoff-ledger.md', description: 'Financial institution signoff requirements' },
    ],
  },
  {
    id: 'licensing',
    title: 'Licensing',
    description: 'License management, API quota guide, and compliance best practices.',
    icon: Scale,
    color: 'text-yellow-400',
    docs: [
      { title: 'Licensing Overview', path: 'docs/licensing/README.md', description: 'License tiers, feature flags, expiry management' },
      { title: 'Requirements', path: 'docs/licensing/requirements.md', description: 'Per-tenant licensing requirements' },
      { title: 'API Guide', path: 'docs/licensing/api-guide.md', description: 'License management API endpoints' },
      { title: 'Compliance Practices', path: 'docs/licensing/compliance-best-practices.md', description: 'Regulatory compliance best practices' },
    ],
  },
  {
    id: 'examples',
    title: 'Examples & Guides',
    description: 'cURL examples, Postman collection, use case walkthroughs.',
    icon: Code2,
    color: 'text-pink-400',
    docs: [
      { title: 'Examples Overview', path: 'docs/examples/README.md', description: 'Available examples and how to use them' },
      { title: 'cURL Examples', path: 'docs/examples/curl-examples.md', description: 'Ready-to-run cURL commands for every endpoint' },
      { title: 'Postman Collection', path: 'docs/examples/postman.json', description: 'Import into Postman for interactive testing' },
      { title: 'Use Cases', path: 'docs/examples/use-cases.md', description: 'End-to-end scenario walkthroughs' },
    ],
  },
  {
    id: 'changelog',
    title: 'Changelog & Roadmap',
    description: 'Version history, release notes, and upcoming features.',
    icon: Scroll,
    color: 'text-emerald-300',
    docs: [
      { title: 'Changelog', path: 'CHANGELOG.md', description: 'Complete version history with all changes' },
      { title: 'Release Notes', path: 'RELEASE_NOTES.md', description: 'Latest release highlights' },
      { title: 'Contributing', path: 'CONTRIBUTING.md', description: 'How to contribute to RampOS' },
    ],
  },
]

const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.04, delayChildren: 0.1 },
  },
}

const item = {
  hidden: { opacity: 0, y: 20 },
  show: { opacity: 1, y: 0 },
}

export default function DocsPage() {
  const [sidebarOpen, setSidebarOpen] = useState(false)
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(docSections.map(s => s.id)))
  const [searchQuery, setSearchQuery] = useState('')

  const toggleSection = (id: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const filteredSections = searchQuery.trim()
    ? docSections.filter(s =>
        s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.docs.some(d =>
          d.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (d.description || '').toLowerCase().includes(searchQuery.toLowerCase())
        )
      )
    : docSections

  return (
    <main className="min-h-screen bg-black text-white">
      {/* Background */}
      <div className="fixed inset-0 z-0 opacity-30 pointer-events-none">
        <div className="absolute top-[-10%] left-[-10%] w-[40%] h-[40%] rounded-full bg-blue-900/40 blur-[120px]" />
        <div className="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] rounded-full bg-fuchsia-900/30 blur-[120px]" />
        <div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.02)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.02)_1px,transparent_1px)] bg-[size:40px_40px] [mask-image:radial-gradient(ellipse_60%_60%_at_50%_50%,#000_10%,transparent_100%)]" />
      </div>

      {/* Header */}
      <header className="sticky top-0 z-50 border-b border-white/10 bg-black/80 backdrop-blur-xl">
        <div className="max-w-7xl mx-auto flex items-center justify-between px-4 py-4">
          <div className="flex items-center gap-4">
            <button
              onClick={() => setSidebarOpen(!sidebarOpen)}
              className="lg:hidden p-2 rounded-lg bg-white/5 border border-white/10 hover:bg-white/10 transition-colors"
            >
              {sidebarOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
            </button>
            <Link href="/" className="flex items-center gap-3 hover:opacity-80 transition-opacity">
              <ArrowLeft className="w-4 h-4 text-gray-400" />
              <span className="text-xl font-bold tracking-tight">RAMP OS</span>
              <span className="text-xs font-mono px-2 py-1 rounded-full bg-cyan-500/10 text-cyan-400 border border-cyan-500/20">
                Docs
              </span>
            </Link>
          </div>
          <div className="hidden md:flex items-center gap-3">
            <a
              href="https://github.com/hadesloc/RampOS"
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm text-gray-400 hover:text-white transition-colors flex items-center gap-2"
            >
              <Code2 className="w-4 h-4" /> GitHub
            </a>
          </div>
        </div>
      </header>

      <div className="relative z-10 max-w-7xl mx-auto flex">
        {/* Sidebar */}
        <aside
          className={`
            fixed lg:sticky top-[65px] left-0 h-[calc(100vh-65px)] w-72 bg-black/95 lg:bg-transparent 
            border-r border-white/10 lg:border-none overflow-y-auto z-40 transition-transform duration-300
            ${sidebarOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'}
          `}
        >
          <nav className="p-4 space-y-1">
            <div className="px-3 py-2 mb-4">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500" />
                <input
                  type="text"
                  placeholder="Search docs..."
                  value={searchQuery}
                  onChange={e => setSearchQuery(e.target.value)}
                  className="w-full pl-10 pr-4 py-2.5 bg-white/5 border border-white/10 rounded-xl text-sm text-white placeholder:text-gray-500 focus:outline-none focus:border-cyan-500/50 focus:ring-1 focus:ring-cyan-500/30 transition-all"
                />
              </div>
            </div>
            {docSections.map(section => {
              const Icon = section.icon
              return (
                <a
                  key={section.id}
                  href={`#${section.id}`}
                  onClick={() => setSidebarOpen(false)}
                  className="flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm text-gray-400 hover:text-white hover:bg-white/5 transition-all group"
                >
                  <Icon className={`w-4 h-4 ${section.color} shrink-0`} />
                  <span className="truncate">{section.title}</span>
                  <span className="ml-auto text-xs text-gray-600 group-hover:text-gray-400 font-mono">
                    {section.docs.length}
                  </span>
                </a>
              )
            })}
          </nav>
        </aside>

        {/* Overlay for mobile */}
        {sidebarOpen && (
          <div
            className="fixed inset-0 z-30 bg-black/60 lg:hidden"
            onClick={() => setSidebarOpen(false)}
          />
        )}

        {/* Main Content */}
        <div className="flex-1 min-w-0 px-4 lg:px-12 py-12">
          {/* Hero */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="mb-16"
          >
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight mb-4">
              <span className="text-transparent bg-clip-text bg-gradient-to-r from-white via-gray-200 to-gray-400">
                Documentation
              </span>
            </h1>
            <p className="text-lg text-gray-400 max-w-2xl leading-relaxed">
              Everything you need to integrate, deploy, and operate RampOS — from first API call to production at scale.
            </p>
            <div className="flex flex-wrap gap-3 mt-6">
              <QuickLink href="#getting-started" label="Quick Start" />
              <QuickLink href="#api-reference" label="API Reference" />
              <QuickLink href="#sdks" label="SDKs" />
              <QuickLink href="#deployment" label="Deployment" />
              <QuickLink href="#security" label="Security" />
            </div>
          </motion.div>

          {/* Sections */}
          <motion.div
            variants={container}
            initial="hidden"
            animate="show"
            className="space-y-8"
          >
            {filteredSections.map(section => (
              <motion.section
                key={section.id}
                id={section.id}
                variants={item}
                className="scroll-mt-24"
              >
                <SectionCard
                  section={section}
                  expanded={expandedSections.has(section.id)}
                  onToggle={() => toggleSection(section.id)}
                />
              </motion.section>
            ))}
          </motion.div>

          {/* Footer note */}
          <div className="mt-20 pt-8 border-t border-white/10 text-center">
            <p className="text-sm text-gray-500">
              All documentation is open source.{' '}
              <a
                href="https://github.com/hadesloc/RampOS/tree/main/docs"
                target="_blank"
                rel="noopener noreferrer"
                className="text-cyan-400 hover:text-cyan-300 transition-colors"
              >
                View on GitHub →
              </a>
            </p>
          </div>
        </div>
      </div>
    </main>
  )
}

function QuickLink({ href, label }: { href: string; label: string }) {
  return (
    <a
      href={href}
      className="text-sm px-4 py-2 rounded-full bg-white/5 border border-white/10 text-gray-300 hover:bg-white/10 hover:text-white hover:border-white/20 transition-all"
    >
      {label}
    </a>
  )
}

function SectionCard({
  section,
  expanded,
  onToggle,
}: {
  section: DocSection
  expanded: boolean
  onToggle: () => void
}) {
  const Icon = section.icon

  return (
    <div className="rounded-2xl border border-white/[0.06] bg-white/[0.02] overflow-hidden hover:border-white/10 transition-all duration-300">
      {/* Header */}
      <button
        onClick={onToggle}
        className="w-full flex items-center gap-4 p-6 text-left hover:bg-white/[0.02] transition-colors"
      >
        <div className={`p-3 rounded-xl bg-white/5 ring-1 ring-white/10 shrink-0`}>
          <Icon className={`w-6 h-6 ${section.color}`} />
        </div>
        <div className="flex-1 min-w-0">
          <h2 className="text-xl font-bold text-white">{section.title}</h2>
          <p className="text-sm text-gray-400 mt-1">{section.description}</p>
        </div>
        <div className="flex items-center gap-3 shrink-0">
          <span className="text-xs font-mono text-gray-500 bg-white/5 px-2.5 py-1 rounded-full">
            {section.docs.length} docs
          </span>
          {expanded ? (
            <ChevronDown className="w-5 h-5 text-gray-500" />
          ) : (
            <ChevronRight className="w-5 h-5 text-gray-500" />
          )}
        </div>
      </button>

      {/* Doc list */}
      {expanded && (
        <div className="border-t border-white/[0.06] divide-y divide-white/[0.04]">
          {section.docs.map((doc, idx) => (
            <a
              key={idx}
              href={`${GITHUB_BASE}/${doc.path}`}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-start gap-4 px-6 py-4 hover:bg-white/[0.03] transition-colors group"
            >
              <div className="w-1.5 h-1.5 rounded-full bg-white/20 mt-2 shrink-0 group-hover:bg-cyan-400 transition-colors" />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-gray-200 group-hover:text-white transition-colors">
                    {doc.title}
                  </span>
                  <ExternalLink className="w-3.5 h-3.5 text-gray-600 opacity-0 group-hover:opacity-100 transition-opacity" />
                </div>
                {doc.description && (
                  <p className="text-sm text-gray-500 mt-0.5">{doc.description}</p>
                )}
              </div>
              <span className="text-xs font-mono text-gray-600 shrink-0 hidden sm:block">
                {doc.path.split('/').pop()}
              </span>
            </a>
          ))}
        </div>
      )}
    </div>
  )
}
