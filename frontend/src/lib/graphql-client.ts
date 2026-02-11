import { Client, cacheExchange, fetchExchange, subscriptionExchange } from 'urql';
import { createClient as createWSClient } from 'graphql-ws';

function getGraphQLUrl(): string {
  const base = process.env.NEXT_PUBLIC_API_URL || '';
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

export const graphqlClient = createGraphQLClient();
