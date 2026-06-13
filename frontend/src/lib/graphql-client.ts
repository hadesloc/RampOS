import { Client, cacheExchange, fetchExchange, subscriptionExchange } from 'urql';
import { createClient as createWSClient } from 'graphql-ws';

function getPublicApiBaseUrl(): string {
  const configuredUrl = process.env.NEXT_PUBLIC_API_URL?.trim();
  if (configuredUrl) {
    return configuredUrl;
  }
  if (process.env.NODE_ENV?.trim().toLowerCase() === 'production') {
    throw new Error('Missing required production environment variable: NEXT_PUBLIC_API_URL');
  }
  return '';
}

function getGraphQLUrl(): string {
  const base = getPublicApiBaseUrl();
  return `${base}/graphql`;
}

function getWSUrl(): string {
  const httpUrl = getGraphQLUrl();
  return httpUrl.replace(/^http/, 'ws');
}

let wsClient: ReturnType<typeof createWSClient> | null = null;

function getWSClient() {
  if (typeof window === 'undefined') return null;
  if (!wsClient) {
    wsClient = createWSClient({
      url: getWSUrl(),
      lazy: true,
      retryAttempts: 3,
    });
  }
  return wsClient;
}

export function createGraphQLClient() {
  const ws = getWSClient();

  const exchanges = [
    cacheExchange,
    fetchExchange,
    ...(ws
      ? [
          subscriptionExchange({
            forwardSubscription(request) {
              const input = { ...request, query: request.query || '' };
              return {
                subscribe(sink) {
                  const unsubscribe = ws.subscribe(input, sink);
                  return { unsubscribe };
                },
              };
            },
          }),
        ]
      : []),
  ];

  return new Client({
    url: getGraphQLUrl(),
    exchanges,
    requestPolicy: 'cache-and-network',
  });
}

// Lazily-memoized singleton.  Do NOT construct at module load: createGraphQLClient()
// resolves NEXT_PUBLIC_API_URL (fail-closed in production), which would throw during
// Next.js build-time page-data collection / prerender.  Construction is deferred to
// first access via the `graphqlClient` getter below.
let _graphqlClient: ReturnType<typeof createGraphQLClient> | null = null;

export function getGraphQLClient(): ReturnType<typeof createGraphQLClient> {
  if (!_graphqlClient) {
    _graphqlClient = createGraphQLClient();
  }
  return _graphqlClient;
}

/**
 * Backwards-compatible `graphqlClient` binding.
 *
 * Implemented as a module-level getter so that the underlying urql Client is built
 * on first property access (e.g. when <Providers> renders in the browser) rather than
 * at import/module-evaluation time.  This preserves the production fail-closed behaviour
 * at runtime while keeping `next build` prerender from invoking the env check.
 */
export const graphqlClient: ReturnType<typeof createGraphQLClient> = new Proxy(
  {} as ReturnType<typeof createGraphQLClient>,
  {
    get(_target, prop, receiver) {
      const client = getGraphQLClient();
      const value = Reflect.get(client as object, prop, receiver);
      return typeof value === 'function' ? value.bind(client) : value;
    },
    has(_target, prop) {
      return Reflect.has(getGraphQLClient() as object, prop);
    },
    getOwnPropertyDescriptor(_target, prop) {
      return Reflect.getOwnPropertyDescriptor(getGraphQLClient() as object, prop);
    },
    ownKeys() {
      return Reflect.ownKeys(getGraphQLClient() as object);
    },
  },
);
