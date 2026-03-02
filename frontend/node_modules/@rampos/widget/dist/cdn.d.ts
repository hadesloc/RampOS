import './web-components/checkout-element';
import './web-components/kyc-element';
import './web-components/wallet-element';
import { RampOSEventEmitter, onRampOSMessage } from './utils/events';
import { RampOSApiClient } from './utils/api';
declare const RampOSWidget: {
    version: string;
    EventEmitter: typeof RampOSEventEmitter;
    ApiClient: typeof RampOSApiClient;
    onMessage: typeof onRampOSMessage;
    init(config: {
        apiKey: string;
        environment?: "sandbox" | "production";
    }): RampOSApiClient;
};
export default RampOSWidget;
