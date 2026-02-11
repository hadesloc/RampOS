import { useSubscription } from 'urql';
import { INTENT_STATUS_CHANGED } from '@/lib/graphql/documents';

export interface IntentStatusEvent {
  intentId: string;
  tenantId: string;
  newStatus: string;
  timestamp: string;
}

export function useIntentStatusSubscription(tenantId: string) {
  return useSubscription<{ intentStatusChanged: IntentStatusEvent }>({
    query: INTENT_STATUS_CHANGED,
    variables: { tenantId },
    pause: !tenantId,
  });
}
