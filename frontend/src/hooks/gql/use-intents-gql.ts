import { useQuery } from 'urql';
import { GET_INTENT, GET_INTENTS } from '@/lib/graphql/documents';

export interface IntentGql {
  id: string;
  tenantId: string;
  userId: string;
  intentType: string;
  state: string;
  stateHistory?: unknown;
  amount: string;
  currency: string;
  actualAmount?: string | null;
  railsProvider?: string | null;
  referenceCode?: string | null;
  bankTxId?: string | null;
  chainId?: string | null;
  txHash?: string | null;
  fromAddress?: string | null;
  toAddress?: string | null;
  metadata?: unknown;
  idempotencyKey?: string | null;
  createdAt: string;
  updatedAt: string;
  expiresAt?: string | null;
  completedAt?: string | null;
}

export interface IntentEdge {
  cursor: string;
  node: IntentGql;
}

export interface IntentsConnection {
  edges: IntentEdge[];
  pageInfo: {
    hasNextPage: boolean;
    endCursor: string | null;
  };
  totalCount?: number;
}

export interface IntentFilter {
  intentType?: string;
  state?: string;
  userId?: string;
}

export function useIntentQuery(tenantId: string, id: string) {
  return useQuery<{ intent: IntentGql | null }>({
    query: GET_INTENT,
    variables: { tenantId, id },
    pause: !tenantId || !id,
  });
}

export interface UseIntentsOptions {
  tenantId: string;
  filter?: IntentFilter;
  first?: number;
  after?: string;
  pause?: boolean;
}

export function useIntentsQuery(options: UseIntentsOptions) {
  const { tenantId, filter, first, after, pause } = options;
  return useQuery<{ intents: IntentsConnection }>({
    query: GET_INTENTS,
    variables: { tenantId, filter, first, after },
    pause: pause || !tenantId,
  });
}
