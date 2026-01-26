import axios, { AxiosInstance } from 'axios';
import { IntentService } from './services/intent.service';
import { UserService } from './services/user.service';
import { LedgerService } from './services/ledger.service';
import { AAService } from './services/aa.service';
import { WebhookVerifier } from './utils/webhook';

export interface RampOSClientOptions {
  baseURL?: string;
  apiKey: string;
  timeout?: number;
}

export class RampOSClient {
  private readonly httpClient: AxiosInstance;

  public readonly intents: IntentService;
  public readonly users: UserService;
  public readonly ledger: LedgerService;
  public readonly aa: AAService;
  public readonly webhooks: WebhookVerifier;

  constructor(options: RampOSClientOptions) {
    const baseURL = options.baseURL || 'https://api.rampos.io/v1';

    this.httpClient = axios.create({
      baseURL,
      timeout: options.timeout || 10000,
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${options.apiKey}`,
      },
    });

    this.intents = new IntentService(this.httpClient);
    this.users = new UserService(this.httpClient);
    this.ledger = new LedgerService(this.httpClient);
    this.aa = new AAService(this.httpClient);
    this.webhooks = new WebhookVerifier();
  }
}
