import axios, { AxiosInstance, AxiosRequestConfig, AxiosResponse } from 'axios';
import { IntentService } from './services/intent.service';
import { UserService } from './services/user.service';
import { LedgerService } from './services/ledger.service';
import { AAService } from './services/aa.service';
import { PasskeyWalletService } from './services/passkey.service';
import { MultichainProvider } from './multichain/provider';
import { WebhookVerifier } from './utils/webhook';
import { signRequest } from './utils/crypto';
import { withRetry } from './utils/retry';
import { RampOSConfig } from './types';

/**
 * Type definitions for Axios HTTP methods with proper generics
 */
type HttpGetMethod = <T = unknown, R = AxiosResponse<T>, D = unknown>(
  url: string,
  config?: AxiosRequestConfig<D>
) => Promise<R>;

type HttpPostMethod = <T = unknown, R = AxiosResponse<T>, D = unknown>(
  url: string,
  data?: D,
  config?: AxiosRequestConfig<D>
) => Promise<R>;

type HttpDeleteMethod = <T = unknown, R = AxiosResponse<T>, D = unknown>(
  url: string,
  config?: AxiosRequestConfig<D>
) => Promise<R>;

type HttpMethod = HttpGetMethod | HttpPostMethod | HttpDeleteMethod;

export class RampOSClient {
  private readonly httpClient: AxiosInstance;

  public readonly intents: IntentService;
  public readonly users: UserService;
  public readonly ledger: LedgerService;
  public readonly aa: AAService;
  public readonly passkey: PasskeyWalletService;
  public readonly webhooks: WebhookVerifier;

  constructor(config: RampOSConfig) {
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
      const base = reqConfig.baseURL ?? baseURL;
      const url = new URL(reqConfig.url ?? '', base);
      const path = url.pathname;

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
    type HttpMethodName = typeof methods[number];

    methods.forEach((method: HttpMethodName) => {
      const original = this.httpClient[method] as HttpMethod;
      const wrappedMethod = <T = unknown, R = AxiosResponse<T>, D = unknown>(
        url: string,
        dataOrConfig?: D | AxiosRequestConfig<D>,
        config?: AxiosRequestConfig<D>
      ): Promise<R> => {
        return withRetry<R>(
          () => {
            // Handle different method signatures
            if (method === 'get' || method === 'delete' || method === 'head' || method === 'options') {
              return (original as HttpGetMethod)(url, dataOrConfig as AxiosRequestConfig<D>) as Promise<R>;
            }
            return (original as HttpPostMethod)(url, dataOrConfig, config) as Promise<R>;
          },
          retryConfig.maxRetries,
          retryConfig.baseDelay
        );
      };
      // Use type assertion for assignment since we're replacing methods with compatible signatures
      (this.httpClient[method] as HttpMethod) = wrappedMethod;
    });

    this.intents = new IntentService(this.httpClient);
    this.users = new UserService(this.httpClient);
    this.ledger = new LedgerService(this.httpClient);
    this.aa = new AAService(this.httpClient);
    this.passkey = new PasskeyWalletService(this.httpClient);
    this.webhooks = new WebhookVerifier();
  }

  // ============================================================================
  // Multi-chain Provider Helper
  // ============================================================================

  /**
   * Initialize a MultichainProvider for direct chain interaction
   * @param rpcUrls Optional map of chain ID to RPC URL overrides
   */
  createMultichainProvider(rpcUrls: Record<number, string> = {}): MultichainProvider {
      return new MultichainProvider(rpcUrls);
  }
}
