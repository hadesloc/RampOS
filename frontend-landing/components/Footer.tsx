import Link from 'next/link'
import { Github, Twitter, Linkedin } from 'lucide-react'

export default function Footer() {
  return (
    <footer className="w-full border-t border-white/10 bg-black pt-16 pb-8">
      <div className="container mx-auto px-4">
        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-8 mb-16">
          <div className="col-span-2 lg:col-span-2">
            <Link href="/" className="inline-block mb-4">
              <span className="text-xl font-bold tracking-tight text-white">RAMP OS</span>
            </Link>
            <p className="text-gray-400 max-w-sm mb-6">
              Global financial infrastructure for the internet economy.
              We make it easy to accept payments, manage funds, and scale your business.
            </p>
            <div className="flex gap-4">
              <SocialLink href="https://twitter.com" icon={<Twitter className="w-5 h-5" />} />
              <SocialLink href="https://github.com" icon={<Github className="w-5 h-5" />} />
              <SocialLink href="https://linkedin.com" icon={<Linkedin className="w-5 h-5" />} />
            </div>
          </div>

          <div>
            <h3 className="font-semibold text-white mb-4">Product</h3>
            <ul className="space-y-3">
              <FooterLink href="/features">Features</FooterLink>
              <FooterLink href="/pricing">Pricing</FooterLink>
              <FooterLink href="/integrations">Integrations</FooterLink>
              <FooterLink href="/changelog">Changelog</FooterLink>
            </ul>
          </div>

          <div>
            <h3 className="font-semibold text-white mb-4">Resources</h3>
            <ul className="space-y-3">
              <FooterLink href="/docs">Documentation</FooterLink>
              <FooterLink href="/api">API Reference</FooterLink>
              <FooterLink href="/guides">Guides</FooterLink>
              <FooterLink href="/status">Status</FooterLink>
            </ul>
          </div>

          <div>
            <h3 className="font-semibold text-white mb-4">Company</h3>
            <ul className="space-y-3">
              <FooterLink href="/about">About</FooterLink>
              <FooterLink href="/careers">Careers</FooterLink>
              <FooterLink href="/blog">Blog</FooterLink>
              <FooterLink href="/contact">Contact</FooterLink>
            </ul>
          </div>
        </div>

        <div className="pt-8 border-t border-white/10 flex flex-col md:flex-row justify-between items-center gap-4 text-sm text-gray-500">
          <div>
            &copy; {new Date().getFullYear()} RampOS Inc. All rights reserved.
          </div>
          <div className="flex gap-8">
            <Link href="/privacy" className="hover:text-white transition-colors">Privacy Policy</Link>
            <Link href="/terms" className="hover:text-white transition-colors">Terms of Service</Link>
            <Link href="/cookies" className="hover:text-white transition-colors">Cookie Policy</Link>
          </div>
        </div>
      </div>
    </footer>
  )
}

function FooterLink({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <li>
      <Link href={href} className="text-gray-400 hover:text-white transition-colors">
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
      className="w-10 h-10 rounded-full bg-white/5 border border-white/10 flex items-center justify-center text-gray-400 hover:bg-white/10 hover:text-white transition-colors"
    >
      {icon}
    </a>
  )
}
