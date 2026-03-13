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
  Globe,
} from 'lucide-react'

/* ─── Types ─── */
type Lang = 'en' | 'vi'

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

/* ─── i18n Strings ─── */
const i18n: Record<Lang, {
  pageTitle: string
  pageDesc: string
  searchPlaceholder: string
  docsLabel: string
  footerNote: string
  footerLink: string
  quickLinks: { href: string; label: string }[]
}> = {
  en: {
    pageTitle: 'Documentation',
    pageDesc: 'Everything you need to integrate, deploy, and operate RampOS — from first API call to production at scale.',
    searchPlaceholder: 'Search docs...',
    docsLabel: 'docs',
    footerNote: 'All documentation is open source.',
    footerLink: 'View on GitHub →',
    quickLinks: [
      { href: '#getting-started', label: 'Quick Start' },
      { href: '#api-reference', label: 'API Reference' },
      { href: '#sdks', label: 'SDKs' },
      { href: '#deployment', label: 'Deployment' },
      { href: '#security', label: 'Security' },
    ],
  },
  vi: {
    pageTitle: 'Tài liệu',
    pageDesc: 'Mọi thứ bạn cần để tích hợp, triển khai và vận hành RampOS — từ lệnh API đầu tiên đến production quy mô lớn.',
    searchPlaceholder: 'Tìm kiếm tài liệu...',
    docsLabel: 'tài liệu',
    footerNote: 'Toàn bộ tài liệu là mã nguồn mở.',
    footerLink: 'Xem trên GitHub →',
    quickLinks: [
      { href: '#getting-started', label: 'Bắt đầu' },
      { href: '#api-reference', label: 'API' },
      { href: '#sdks', label: 'SDKs' },
      { href: '#deployment', label: 'Triển khai' },
      { href: '#security', label: 'Bảo mật' },
    ],
  },
}

/* ─── Sections (bilingual) ─── */
function getSections(lang: Lang): DocSection[] {
  if (lang === 'vi') return sectionsVi
  return sectionsEn
}

const sectionsEn: DocSection[] = [
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

const sectionsVi: DocSection[] = [
  {
    id: 'getting-started',
    title: 'Bắt đầu',
    description: 'Hướng dẫn khởi động nhanh, khái niệm cốt lõi và các bước đầu tiên với RampOS.',
    icon: BookOpen,
    color: 'text-emerald-400',
    docs: [
      { title: 'Hướng dẫn nhanh', path: 'docs/getting-started/README.md', description: 'Cài đặt, thiết lập và lệnh API đầu tiên' },
      { title: 'Khái niệm cốt lõi', path: 'docs/getting-started/concepts.md', description: 'Intents, tenants, sổ cái, nền tảng compliance' },
      { title: 'Whitepaper', path: 'docs/whitepaper.md', description: 'Whitepaper kỹ thuật đầy đủ và lý do thiết kế hệ thống' },
    ],
  },
  {
    id: 'architecture',
    title: 'Kiến trúc',
    description: 'Thiết kế hệ thống, máy trạng thái, mô hình sổ cái và nội bộ engine compliance.',
    icon: Layers,
    color: 'text-cyan-400',
    docs: [
      { title: 'Tổng quan hệ thống', path: 'docs/architecture/overview.md', description: 'Kiến trúc tổng thể, cấu trúc crate, luồng dữ liệu' },
      { title: 'Máy trạng thái', path: 'docs/architecture/state-machine.md', description: 'Chuyển trạng thái pay-in, pay-out, trade và RFQ' },
      { title: 'Sổ cái kép', path: 'docs/architecture/ledger.md', description: 'Mô hình kế toán, bút toán, đối soát' },
      { title: 'Engine Compliance', path: 'docs/architecture/compliance.md', description: 'Pipeline KYC/AML, chấm điểm rủi ro, quản lý vụ việc' },
      { title: 'Tóm tắt kiến trúc', path: 'docs/architecture.md', description: 'Thẻ tham chiếu kiến trúc ngắn gọn' },
    ],
  },
  {
    id: 'api-reference',
    title: 'Tham chiếu API',
    description: 'Endpoint REST API, xác thực, giới hạn tốc độ, webhooks và danh mục sự kiện.',
    icon: Server,
    color: 'text-blue-400',
    docs: [
      { title: 'Tổng quan API', path: 'docs/api/README.md', description: 'URL cơ sở, xác thực, headers, phiên bản' },
      { title: 'Tài liệu API đầy đủ', path: 'docs/API.md', description: 'Tham chiếu endpoint đầy đủ với ví dụ request/response' },
      { title: 'Tham chiếu Endpoints', path: 'docs/api/endpoints.md', description: 'Tất cả admin, portal và LP endpoints' },
      { title: 'Xác thực', path: 'docs/api/authentication.md', description: 'JWT, API keys, LP keys, admin auth' },
      { title: 'Giới hạn tốc độ', path: 'docs/api/rate-limiting.md', description: 'Giới hạn per-tenant, cửa sổ trượt, cấu hình ghi đè' },
      { title: 'Webhooks', path: 'docs/api/webhooks.md', description: 'Loại sự kiện, ký HMAC, chính sách retry, DLQ' },
    ],
  },
  {
    id: 'database',
    title: 'Cơ sở dữ liệu',
    description: 'Thiết kế schema PostgreSQL, hệ thống migration và chính sách bảo mật cấp hàng.',
    icon: Database,
    color: 'text-amber-400',
    docs: [
      { title: 'Tham chiếu Schema', path: 'docs/database/schema.md', description: '49 bảng, indexes, ràng buộc và quan hệ' },
      { title: 'Hướng dẫn Migrations', path: 'docs/database/migrations.md', description: 'Hệ thống migration, thứ tự, script up/down' },
      { title: 'Row-Level Security', path: 'docs/database/rls.md', description: 'Chính sách phân lập tenant, thiết kế fail-closed' },
    ],
  },
  {
    id: 'smart-contracts',
    title: 'Hợp đồng thông minh',
    description: 'Hợp đồng Solidity cho account abstraction, paymaster, passkey và ZK KYC.',
    icon: FileCode2,
    color: 'text-fuchsia-400',
    docs: [
      { title: 'Tổng quan Contracts', path: 'docs/contracts/overview.md', description: '10 contracts, Foundry toolchain, triển khai' },
      { title: 'Smart Account (ERC-4337)', path: 'docs/contracts/account.md', description: 'RampOSAccount, factory, batch execution, UUPS' },
      { title: 'Paymaster', path: 'docs/contracts/paymaster.md', description: 'Tài trợ gas, quản lý ngân sách, bảo vệ nonce' },
      { title: 'Bảo mật Contracts', path: 'docs/contracts/security.md', description: 'Kết quả audit, kiểm soát truy cập, an toàn nâng cấp' },
    ],
  },
  {
    id: 'sdks',
    title: 'SDKs',
    description: 'Thư viện client cho TypeScript, Go và Python với type safety đầy đủ.',
    icon: Package,
    color: 'text-indigo-400',
    docs: [
      { title: 'Tổng quan SDK', path: 'docs/SDK.md', description: 'Cài đặt, cấu hình, khởi động nhanh cho tất cả SDKs' },
      { title: 'TypeScript SDK', path: 'sdk/README.md', description: '@rampos/sdk — React Query hooks, type-safe client' },
      { title: 'Go SDK', path: 'sdk-go/README.md', description: 'rampos-go — Go client chuẩn mực với hỗ trợ context' },
      { title: 'Python SDK', path: 'sdk-python/README.md', description: 'rampos — async/sync client với Pydantic models' },
    ],
  },
  {
    id: 'cli',
    title: 'CLI',
    description: 'Giao diện dòng lệnh cho tự động hóa, scripting và tích hợp AI agent.',
    icon: Terminal,
    color: 'text-lime-400',
    docs: [
      { title: 'Tổng quan CLI', path: 'docs/cli/README.md', description: 'Cài đặt, chế độ auth, định dạng output' },
      { title: 'Sử dụng với Agent', path: 'docs/cli/agent-usage.md', description: 'Flags thân thiện máy cho quy trình AI agent' },
      { title: 'Coverage Ledger', path: 'docs/cli/coverage-ledger.md', description: 'Theo dõi và xác thực coverage endpoint' },
    ],
  },
  {
    id: 'integrations',
    title: 'Tích hợp',
    description: 'Kết nối với ngân hàng, nhà cung cấp KYC, engine compliance và ví.',
    icon: Plug,
    color: 'text-orange-400',
    docs: [
      { title: 'Bank Adapter', path: 'docs/integrations/bank-adapter.md', description: 'Tích hợp ngân hàng/PSP pluggable với Rails trait' },
      { title: 'Nhà cung cấp KYC', path: 'docs/integrations/kyc-provider.md', description: 'Onfido, eKYC, tích hợp nhà cung cấp tùy chỉnh' },
      { title: 'Quy tắc Compliance', path: 'docs/integrations/compliance-rules.md', description: 'AML velocity, trừng phạt, cấu hình chấm điểm gian lận' },
      { title: 'Tích hợp Ví', path: 'docs/integrations/wallet-integration.md', description: 'Account abstraction, passkey, delegation EOA' },
    ],
  },
  {
    id: 'deployment',
    title: 'Triển khai',
    description: 'Phát triển local, Docker, Kubernetes và cấu hình CI/CD pipeline.',
    icon: Cloud,
    color: 'text-sky-400',
    docs: [
      { title: 'Phát triển Local', path: 'docs/deployment/local.md', description: 'Docker Compose, thiết lập môi trường, hot reload' },
      { title: 'Kubernetes', path: 'docs/deployment/kubernetes.md', description: 'Kustomize, HPA, PDB, network policies' },
      { title: 'CI/CD Pipeline', path: 'docs/deployment/ci-cd.md', description: 'GitHub Actions, ArgoCD, phát hiện drift' },
      { title: 'Hướng dẫn triển khai', path: 'docs/DEPLOY.md', description: 'Triển khai production từng bước' },
      { title: 'Checklist triển khai', path: 'docs/DEPLOYMENT_CHECKLIST.md', description: 'Danh sách kiểm tra trước khi ra mắt' },
    ],
  },
  {
    id: 'enterprise',
    title: 'Doanh nghiệp',
    description: 'Vận hành multi-tenant, SSO, giới hạn API và cấu hình doanh nghiệp.',
    icon: Building2,
    color: 'text-violet-400',
    docs: [
      { title: 'Tổng quan Enterprise', path: 'docs/enterprise/README.md', description: 'Tính năng doanh nghiệp và tùy chọn triển khai' },
      { title: 'Thiết lập SSO', path: 'docs/enterprise/sso-setup.md', description: 'Tích hợp OIDC/SAML, cấu hình nhà cung cấp' },
      { title: 'Giới hạn API', path: 'docs/enterprise/api-limits.md', description: 'Rate limits tùy chỉnh, quản lý tier' },
      { title: 'Cấu hình', path: 'docs/enterprise/configuration.md', description: 'Config bundles, cài đặt nhận biết môi trường' },
      { title: 'Hướng dẫn vận hành', path: 'docs/enterprise/operations.md', description: 'Giám sát, cảnh báo, xử lý sự cố' },
      { title: 'Triển khai Enterprise', path: 'docs/enterprise/deployment.md', description: 'Thiết lập HA, mở rộng, khôi phục thảm họa' },
    ],
  },
  {
    id: 'testing',
    title: 'Kiểm thử',
    description: 'Unit tests, integration tests, load tests và bộ kiểm thử smart contract.',
    icon: TestTubes,
    color: 'text-teal-400',
    docs: [
      { title: 'Hướng dẫn kiểm thử', path: 'docs/TESTING-GUIDE.md', description: 'Chiến lược và best practices kiểm thử' },
      { title: 'Unit Tests', path: 'docs/testing/unit-tests.md', description: 'Rust unit tests, mocking, coverage' },
      { title: 'Integration Tests', path: 'docs/testing/integration-tests.md', description: 'Kiểm thử tích hợp API, test fixtures' },
      { title: 'Load Tests', path: 'docs/testing/load-tests.md', description: 'Benchmark hiệu năng, stress testing' },
      { title: 'Contract Tests', path: 'docs/testing/contract-tests.md', description: 'Foundry tests, fuzz testing, invariants' },
    ],
  },
  {
    id: 'security',
    title: 'Bảo mật',
    description: 'Threat model, audit bảo mật, hướng dẫn hardening và báo cáo khắc phục.',
    icon: Shield,
    color: 'text-red-400',
    docs: [
      { title: 'Tổng quan bảo mật', path: 'docs/SECURITY.md', description: 'Kiến trúc và thực hành bảo mật' },
      { title: 'Threat Model', path: 'docs/security/threat-model.md', description: 'Vectơ tấn công, đánh giá rủi ro, giảm thiểu' },
      { title: 'Hardening bảo mật', path: 'docs/security/hardening.md', description: 'Hướng dẫn hardening hạ tầng và ứng dụng' },
      { title: 'Báo cáo Audit', path: 'docs/security/audit-report.md', description: 'Kết quả audit bảo mật toàn diện' },
      { title: 'Kế hoạch khắc phục', path: 'docs/security/remediation-plan.md', description: 'Theo dõi vấn đề và timeline sửa chữa' },
      { title: 'Roadmap & Hardening', path: 'docs/recent-roadmap-and-security-hardening-2026-03.md', description: 'Báo cáo hardening bảo mật tháng 3/2026' },
    ],
  },
  {
    id: 'operations',
    title: 'Vận hành',
    description: 'Runbooks, giám sát, khôi phục thảm họa và quản lý phát hành.',
    icon: Settings,
    color: 'text-gray-400',
    docs: [
      { title: 'Hướng dẫn giám sát', path: 'docs/operations/monitoring.md', description: 'Prometheus, Grafana, quy tắc cảnh báo' },
      { title: 'Runbook', path: 'docs/operations/runbook-skeleton.md', description: 'Quy trình vận hành cho các tình huống phổ biến' },
      { title: 'Khôi phục thảm họa', path: 'docs/operations/disaster-recovery-plan.md', description: 'Quy trình backup, restore và failover' },
      { title: 'Checklist phát hành', path: 'docs/operations/release-checklist.md', description: 'Các bước xác minh trước phát hành' },
      { title: 'Xác thực Staging', path: 'docs/operations/staging-validation-plan.md', description: 'Ma trận QA cho môi trường staging' },
      { title: 'Signoff cấp Ngân hàng', path: 'docs/operations/bank-grade-signoff-ledger.md', description: 'Yêu cầu chấp thuận từ tổ chức tài chính' },
    ],
  },
  {
    id: 'licensing',
    title: 'Giấy phép',
    description: 'Quản lý license, hướng dẫn quota API và best practices compliance.',
    icon: Scale,
    color: 'text-yellow-400',
    docs: [
      { title: 'Tổng quan License', path: 'docs/licensing/README.md', description: 'Tier license, feature flags, quản lý hết hạn' },
      { title: 'Yêu cầu', path: 'docs/licensing/requirements.md', description: 'Yêu cầu licensing per-tenant' },
      { title: 'Hướng dẫn API', path: 'docs/licensing/api-guide.md', description: 'Endpoint quản lý license' },
      { title: 'Thực hành Compliance', path: 'docs/licensing/compliance-best-practices.md', description: 'Best practices tuân thủ quy định' },
    ],
  },
  {
    id: 'examples',
    title: 'Ví dụ & Hướng dẫn',
    description: 'Ví dụ cURL, bộ sưu tập Postman, hướng dẫn use case.',
    icon: Code2,
    color: 'text-pink-400',
    docs: [
      { title: 'Tổng quan ví dụ', path: 'docs/examples/README.md', description: 'Các ví dụ có sẵn và cách sử dụng' },
      { title: 'Ví dụ cURL', path: 'docs/examples/curl-examples.md', description: 'Lệnh cURL sẵn sàng chạy cho mọi endpoint' },
      { title: 'Bộ sưu tập Postman', path: 'docs/examples/postman.json', description: 'Import vào Postman để test tương tác' },
      { title: 'Use Cases', path: 'docs/examples/use-cases.md', description: 'Hướng dẫn kịch bản end-to-end' },
    ],
  },
  {
    id: 'changelog',
    title: 'Nhật ký thay đổi',
    description: 'Lịch sử phiên bản, ghi chú phát hành và tính năng sắp tới.',
    icon: Scroll,
    color: 'text-emerald-300',
    docs: [
      { title: 'Nhật ký thay đổi', path: 'CHANGELOG.md', description: 'Lịch sử phiên bản đầy đủ với tất cả thay đổi' },
      { title: 'Ghi chú phát hành', path: 'RELEASE_NOTES.md', description: 'Điểm nổi bật phiên bản mới nhất' },
      { title: 'Đóng góp', path: 'CONTRIBUTING.md', description: 'Cách đóng góp cho RampOS' },
    ],
  },
]

/* ─── Animations ─── */
const containerVariants = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.04, delayChildren: 0.1 },
  },
}

const itemVariants = {
  hidden: { opacity: 0, y: 20 },
  show: { opacity: 1, y: 0 },
}

/* ─── Main Page ─── */
export default function DocsPage() {
  const [sidebarOpen, setSidebarOpen] = useState(false)
  const [lang, setLang] = useState<Lang>('en')
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(sectionsEn.map(s => s.id))
  )
  const [searchQuery, setSearchQuery] = useState('')

  const t = i18n[lang]
  const sections = getSections(lang)

  const toggleSection = (id: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const filteredSections = searchQuery.trim()
    ? sections.filter(s =>
        s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.docs.some(d =>
          d.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (d.description || '').toLowerCase().includes(searchQuery.toLowerCase())
        )
      )
    : sections

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

          <div className="flex items-center gap-4">
            {/* Language Toggle */}
            <div className="flex items-center bg-white/5 rounded-full border border-white/10 p-0.5">
              <button
                onClick={() => setLang('en')}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium transition-all duration-300 ${
                  lang === 'en'
                    ? 'bg-white/10 text-white shadow-sm'
                    : 'text-gray-500 hover:text-gray-300'
                }`}
              >
                <span className="text-base leading-none">🇬🇧</span>
                <span className="hidden sm:inline">EN</span>
              </button>
              <button
                onClick={() => setLang('vi')}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium transition-all duration-300 ${
                  lang === 'vi'
                    ? 'bg-white/10 text-white shadow-sm'
                    : 'text-gray-500 hover:text-gray-300'
                }`}
              >
                <span className="text-base leading-none">🇻🇳</span>
                <span className="hidden sm:inline">VI</span>
              </button>
            </div>

            <a
              href="https://github.com/hadesloc/RampOS"
              target="_blank"
              rel="noopener noreferrer"
              className="hidden md:flex text-sm text-gray-400 hover:text-white transition-colors items-center gap-2"
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
                  placeholder={t.searchPlaceholder}
                  value={searchQuery}
                  onChange={e => setSearchQuery(e.target.value)}
                  className="w-full pl-10 pr-4 py-2.5 bg-white/5 border border-white/10 rounded-xl text-sm text-white placeholder:text-gray-500 focus:outline-none focus:border-cyan-500/50 focus:ring-1 focus:ring-cyan-500/30 transition-all"
                />
              </div>
            </div>
            {sections.map(section => {
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
            key={lang}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4 }}
            className="mb-16"
          >
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight mb-4">
              <span className="text-transparent bg-clip-text bg-gradient-to-r from-white via-gray-200 to-gray-400">
                {t.pageTitle}
              </span>
            </h1>
            <p className="text-lg text-gray-400 max-w-2xl leading-relaxed">
              {t.pageDesc}
            </p>
            <div className="flex flex-wrap gap-3 mt-6">
              {t.quickLinks.map(ql => (
                <QuickLink key={ql.href} href={ql.href} label={ql.label} />
              ))}
            </div>
          </motion.div>

          {/* Sections */}
          <motion.div
            key={`sections-${lang}`}
            variants={containerVariants}
            initial="hidden"
            animate="show"
            className="space-y-8"
          >
            {filteredSections.map(section => (
              <motion.section
                key={section.id}
                id={section.id}
                variants={itemVariants}
                className="scroll-mt-24"
              >
                <SectionCard
                  section={section}
                  expanded={expandedSections.has(section.id)}
                  onToggle={() => toggleSection(section.id)}
                  docsLabel={t.docsLabel}
                />
              </motion.section>
            ))}
          </motion.div>

          {/* Footer note */}
          <div className="mt-20 pt-8 border-t border-white/10 text-center">
            <p className="text-sm text-gray-500">
              {t.footerNote}{' '}
              <a
                href="https://github.com/hadesloc/RampOS/tree/main/docs"
                target="_blank"
                rel="noopener noreferrer"
                className="text-cyan-400 hover:text-cyan-300 transition-colors"
              >
                {t.footerLink}
              </a>
            </p>
          </div>
        </div>
      </div>
    </main>
  )
}

/* ─── Sub-components ─── */

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
  docsLabel,
}: {
  section: DocSection
  expanded: boolean
  onToggle: () => void
  docsLabel: string
}) {
  const Icon = section.icon

  return (
    <div className="rounded-2xl border border-white/[0.06] bg-white/[0.02] overflow-hidden hover:border-white/10 transition-all duration-300">
      {/* Header */}
      <button
        onClick={onToggle}
        className="w-full flex items-center gap-4 p-6 text-left hover:bg-white/[0.02] transition-colors"
      >
        <div className="p-3 rounded-xl bg-white/5 ring-1 ring-white/10 shrink-0">
          <Icon className={`w-6 h-6 ${section.color}`} />
        </div>
        <div className="flex-1 min-w-0">
          <h2 className="text-xl font-bold text-white">{section.title}</h2>
          <p className="text-sm text-gray-400 mt-1">{section.description}</p>
        </div>
        <div className="flex items-center gap-3 shrink-0">
          <span className="text-xs font-mono text-gray-500 bg-white/5 px-2.5 py-1 rounded-full">
            {section.docs.length} {docsLabel}
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
