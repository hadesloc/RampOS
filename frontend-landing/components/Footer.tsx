import Link from 'next/link'
import { Github, Twitter, Linkedin } from 'lucide-react'

export default function Footer() {
  return (
    <footer className="w-full relative bg-[#050505] pt-20 pb-8">
      {/* Animated wave border */}
      <div className="absolute top-0 left-0 right-0 h-[2px] overflow-hidden">
        <div className="w-[200%] h-full bg-gradient-to-r from-[#00FF87]/40 via-[#7B61FF]/40 to-[#00D4FF]/40 animate-wave" />
      </div>

      <div className="container mx-auto px-4 max-w-6xl">
        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-8 mb-16">
          {/* Brand */}
          <div className="col-span-2 lg:col-span-2">
            <Link href="/" className="inline-flex items-center gap-2 mb-4">
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#00FF87] opacity-75" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-[#00FF87]" />
              </span>
              <span className="text-xl font-display font-bold tracking-tight text-white">RAMP·OS</span>
            </Link>
            <p className="text-gray-500 max-w-sm mb-6 text-sm leading-relaxed">
              Global financial infrastructure for the internet economy.
              Intent-native on/off ramp orchestration with multi-chain settlement.
            </p>
            <div className="flex gap-3">
              <SocialLink href="https://twitter.com" icon={<Twitter className="w-4 h-4" />} />
              <SocialLink href="https://github.com/hadesloc/RampOS" icon={<Github className="w-4 h-4" />} />
              <SocialLink href="https://linkedin.com" icon={<Linkedin className="w-4 h-4" />} />
            </div>
          </div>

          {/* Product */}
          <div>
            <h3 className="font-display font-semibold text-white text-sm mb-4 tracking-wide">Product</h3>
            <ul className="space-y-3">
              <FooterLink href="#features">Features</FooterLink>
              <FooterLink href="#pricing">Pricing</FooterLink>
              <FooterLink href="#integrations">Integrations</FooterLink>
              <FooterLink href="/docs">Documentation</FooterLink>
            </ul>
          </div>

          {/* Developers */}
          <div>
            <h3 className="font-display font-semibold text-white text-sm mb-4 tracking-wide">Developers</h3>
            <ul className="space-y-3">
              <FooterLink href="/docs">API Reference</FooterLink>
              <FooterLink href="/docs">SDKs</FooterLink>
              <FooterLink href="/docs">Guides</FooterLink>
              <FooterLink href="#">Status</FooterLink>
            </ul>
          </div>

          {/* Company */}
          <div>
            <h3 className="font-display font-semibold text-white text-sm mb-4 tracking-wide">Company</h3>
            <ul className="space-y-3">
              <FooterLink href="#">About</FooterLink>
              <FooterLink href="#">Blog</FooterLink>
              <FooterLink href="#">Careers</FooterLink>
              <FooterLink href="#">Contact</FooterLink>
            </ul>
          </div>
        </div>

        {/* Bottom bar */}
        <div className="pt-8 border-t border-white/[0.06] flex flex-col md:flex-row justify-between items-center gap-4 text-xs text-gray-600">
          <div className="font-mono">
            &copy; {new Date().getFullYear()} RampOS Inc. All rights reserved.
          </div>
          <div className="flex gap-6">
            <Link href="/privacy" className="hover:text-gray-400 transition-colors">Privacy Policy</Link>
            <Link href="/terms" className="hover:text-gray-400 transition-colors">Terms of Service</Link>
            <Link href="/security" className="hover:text-gray-400 transition-colors">Security</Link>
            <Link href="/status" className="hover:text-gray-400 transition-colors">Status</Link>
          </div>
        </div>
      </div>
    </footer>
  )
}

function FooterLink({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <li>
      <Link href={href} className="text-sm text-gray-500 hover:text-white transition-colors duration-300">
        {children}
      </Link>
    </li>
  )
}

function SocialLink({ href, icon }: { href: string; icon: React.ReactNode }) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="w-9 h-9 rounded-full glass flex items-center justify-center text-gray-500 hover:text-[#00FF87] hover:shadow-[0_0_15px_rgba(0,255,135,0.2)] transition-all duration-300"
    >
      {icon}
    </a>
  )
}
