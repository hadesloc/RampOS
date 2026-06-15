"use client";

/**
 * CSRF proxy interceptor.
 *
 * The server-side `/api/proxy/*` route enforces a double-submit CSRF check:
 * every request must carry an `x-csrf-token` header whose value matches the
 * `rampos_csrf` cookie. The canonical data helpers (`lib/api.ts`,
 * `lib/sdk-client.ts`, `lib/api-client.ts`) all inject this header, but a
 * number of feature pages call `fetch('/api/proxy/...')` directly and would
 * otherwise be rejected with `403 CSRF check failed` — even against a healthy
 * backend with a valid admin session.
 *
 * Rather than refactor every call site, this installs a single, scoped fetch
 * wrapper that transparently attaches the token to same-origin `/api/proxy`
 * requests that don't already set it. All other requests (RSC, navigation,
 * `_next/*`, third-party) pass through untouched.
 */

const CSRF_COOKIE_NAME = "rampos_csrf";
const PROXY_PREFIX = "/api/proxy";

let installed = false;
let originalFetch: typeof fetch | null = null;
let inflightCsrf: Promise<string | null> | null = null;

function readCsrfCookie(): string | null {
  if (typeof document === "undefined") return null;
  const match = document.cookie.match(/(?:^|;\s*)rampos_csrf=([^;]*)/);
  return match ? decodeURIComponent(match[1]) : null;
}

/** Resolve the current CSRF token, minting one via `/api/csrf` if absent. */
async function ensureCsrfToken(): Promise<string | null> {
  const existing = readCsrfCookie();
  if (existing) return existing;
  if (!inflightCsrf) {
    const doFetch = originalFetch ?? fetch;
    inflightCsrf = doFetch("/api/csrf", { method: "GET", credentials: "same-origin" })
      .then(async (res) => {
        if (!res.ok) return null;
        const payload = (await res.json().catch(() => null)) as { token?: string } | null;
        return payload?.token && typeof payload.token === "string" ? payload.token : null;
      })
      .catch(() => null)
      .finally(() => {
        inflightCsrf = null;
      });
  }
  return inflightCsrf;
}

function isSameOriginProxyRequest(input: RequestInfo | URL): boolean {
  if (typeof window === "undefined") return false;
  try {
    const raw =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.href
          : input instanceof Request
            ? input.url
            : String(input);
    const url = new URL(raw, window.location.origin);
    return url.origin === window.location.origin && url.pathname.startsWith(PROXY_PREFIX);
  } catch {
    return false;
  }
}

function headersHaveToken(headers: HeadersInit | undefined): boolean {
  if (!headers) return false;
  try {
    return new Headers(headers).has("x-csrf-token");
  } catch {
    return false;
  }
}

/**
 * Idempotently wrap `window.fetch` so that same-origin `/api/proxy` requests
 * carry a valid CSRF token. Safe to call multiple times (e.g. React Strict
 * Mode); only the first call patches `fetch`.
 */
export function installCsrfProxyInterceptor(): void {
  if (installed || typeof window === "undefined") return;
  installed = true;
  originalFetch = window.fetch.bind(window);
  const base = originalFetch;

  window.fetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    if (!isSameOriginProxyRequest(input)) {
      return base(input, init);
    }

    // Respect an explicit token already set by a canonical helper.
    const requestHasToken =
      headersHaveToken(init?.headers) ||
      (input instanceof Request && input.headers.has("x-csrf-token"));
    if (requestHasToken) {
      return base(input, init);
    }

    const token = await ensureCsrfToken();
    if (!token) {
      return base(input, init);
    }

    if (input instanceof Request && !init) {
      const headers = new Headers(input.headers);
      headers.set("x-csrf-token", token);
      return base(new Request(input, { headers }));
    }

    const headers = new Headers(init?.headers);
    headers.set("x-csrf-token", token);
    return base(input, { ...init, headers });
  };
}
