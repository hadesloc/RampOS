// CDN entry point - bundles React + all components + web components
import './web-components/checkout-element';
import './web-components/kyc-element';
import './web-components/wallet-element';

import { RampOSEventEmitter, onRampOSMessage } from './utils/events';
import { RampOSApiClient } from './utils/api';

// Expose global namespace for CDN usage
const RampOSWidget = {
  version: '1.0.0',
  EventEmitter: RampOSEventEmitter,
  ApiClient: RampOSApiClient,
  onMessage: onRampOSMessage,
  init(config: { apiKey: string; environment?: 'sandbox' | 'production' }) {
    console.log('[RampOS Widget] Initialized', { version: '1.0.0', environment: config.environment || 'sandbox' });
    return new RampOSApiClient({
      apiKey: config.apiKey,
      environment: config.environment,
    });
  },
};

(window as Record<string, unknown>).RampOSWidget = RampOSWidget;

export default RampOSWidget;
