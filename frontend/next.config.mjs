import createNextIntlPlugin from 'next-intl/plugin';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const withNextIntl = createNextIntlPlugin();
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'standalone',
  outputFileTracingRoot: path.join(__dirname, '..'),
  async headers() {
    // Next.js dev mode (webpack eval-source-map + React Fast Refresh) requires
    // 'unsafe-eval' for client modules to execute and hydrate, and a websocket
    // connection for HMR. Production needs neither, so keep the strict policy there.
    const isDev = process.env.NODE_ENV !== 'production';
    const scriptSrc = isDev
      ? "script-src 'self' 'unsafe-inline' 'unsafe-eval'"
      : "script-src 'self' 'unsafe-inline'";
    const connectSrc = isDev
      ? "connect-src 'self' ws: wss:"
      : "connect-src 'self'";
    const csp = [
      "default-src 'self'",
      scriptSrc,
      // Google Fonts are used by the white-label theming/branding feature.
      "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com",
      "img-src 'self' data:",
      "font-src 'self' https://fonts.gstatic.com data:",
      connectSrc,
    ].join('; ');
    return [{
      source: '/(.*)',
      headers: [
        { key: 'X-Frame-Options', value: 'DENY' },
        { key: 'X-Content-Type-Options', value: 'nosniff' },
        { key: 'X-XSS-Protection', value: '1; mode=block' },
        { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
        { key: 'Permissions-Policy', value: 'camera=(), microphone=(), geolocation=()' },
        { key: 'Strict-Transport-Security', value: 'max-age=31536000; includeSubDomains' },
        { key: 'Content-Security-Policy', value: csp },
      ],
    }];
  },
  async rewrites() {
    // Portal API routes only - admin routes MUST go through /api/proxy for auth
    return [
      {
        source: '/api/v1/auth/:path*',
        destination: 'http://localhost:8080/api/v1/auth/:path*',
      },
      {
        source: '/api/v1/portal/:path*',
        destination: 'http://localhost:8080/api/v1/portal/:path*',
      },
    ]
  },
};

export default withNextIntl(nextConfig);
