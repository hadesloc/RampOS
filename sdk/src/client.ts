import axios, { AxiosInstance, InternalAxiosRequestConfig } from 'axios';
import { IntentService } from './services/intent.service';
import { UserService } from './services/user.service';
import { LedgerService } from './services/ledger.service';
import { AAService } from './services/aa.service';
import { WebhookVerifier } from './utils/webhook';
import { signRequest } from './utils/crypto';
import { withRetry } from './utils/retry';
import { RampOSConfig } from './types';

export class RampOSClient {
  private readonly httpClient: AxiosInstance;
  private readonly config: RampOSConfig;

  public readonly intents: IntentService;
  public readonly users: UserService;
  public readonly ledger: LedgerService;
  public readonly aa: AAService;
  public readonly webhooks: WebhookVerifier;

  constructor(config: RampOSConfig) {
    this.config = config;
    const baseURL = config.baseURL || 'https://api.rampos.io/v1';

    this.httpClient = axios.create({
      baseURL,
      timeout: config.timeout || 10000,
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${config.apiKey}`,
      },
    });

    // Add HMAC signature interceptor
    this.httpClient.interceptors.request.use((reqConfig) => {
      const timestamp = Math.floor(Date.now() / 1000);
      const method = (reqConfig.method || 'GET').toUpperCase();
      const path = reqConfig.url || '';

      let body = '';
      if (reqConfig.data) {
        if (typeof reqConfig.data === 'string') {
          body = reqConfig.data;
        } else {
          body = JSON.stringify(reqConfig.data);
          reqConfig.data = body;
        }
      }

      const signature = signRequest(
        config.apiKey,
        config.apiSecret,
        method,
        path,
        body,
        timestamp
      );

      if (reqConfig.headers) {
        reqConfig.headers['X-Timestamp'] = timestamp.toString();
        reqConfig.headers['X-Signature'] = signature;
        if (config.tenantId) {
          reqConfig.headers['X-Tenant-ID'] = config.tenantId;
        }
      }

      return reqConfig;
    });

    // Wrap HTTP client methods with retry policy
    const retryConfig = config.retry || { maxRetries: 3, baseDelay: 1000 };
    const methods = ['get', 'post', 'put', 'delete', 'patch', 'head', 'options'] as const;

    methods.forEach((method) => {
      const original = (this.httpClient as any)[method];
      (this.httpClient as any)[method] = async (...args: any[]) => {
        return withRetry(
          () => original.apply(this.httpClient, args),
          retryConfig.maxRetries,
          retryConfig.baseDelay
        );
      };
    });

    this.intents = new IntentService(this.httpClient);
    this.users = new UserService(this.httpClient);
    this.ledger = new LedgerService(this.httpClient);
    this.aa = new AAService(this.httpClient);
    this.webhooks = new WebhookVerifier();
  }
}
