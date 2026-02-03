import { AxiosInstance } from 'axios';
import {
  CreatePayinRequest,
  CreatePayinResponse,
  CreatePayinResponseSchema,
  ConfirmPayinRequest,
  ConfirmPayinResponse,
  ConfirmPayinResponseSchema,
  CreatePayoutRequest,
  CreatePayoutResponse,
  CreatePayoutResponseSchema,
  Intent,
  IntentFilters,
  IntentSchema,
} from '../types/intent';

export class IntentService {
  constructor(private readonly httpClient: AxiosInstance) {}

  /**
   * Create a new Pay-In intent.
   * @param data Pay-In data
   * @returns Created Intent
   */
  async createPayIn(data: CreatePayinRequest): Promise<CreatePayinResponse> {
    const response = await this.httpClient.post('/intents/payin', data);
    return CreatePayinResponseSchema.parse(response.data);
  }

  /**
   * Confirm a Pay-In intent.
   * @param data Confirm Pay-In data
   * @returns Confirmation result
   */
  async confirmPayIn(data: ConfirmPayinRequest): Promise<ConfirmPayinResponse> {
    const response = await this.httpClient.post('/intents/payin/confirm', data);
    return ConfirmPayinResponseSchema.parse(response.data);
  }

  /**
   * Create a new Pay-Out intent.
   * @param data Pay-Out data
   * @returns Created Intent
   */
  async createPayOut(data: CreatePayoutRequest): Promise<CreatePayoutResponse> {
    const response = await this.httpClient.post('/intents/payout', data);
    return CreatePayoutResponseSchema.parse(response.data);
  }

  /**
   * Get an intent by ID.
   * @param id Intent ID
   * @returns Intent
   */
  async get(id: string): Promise<Intent> {
    const response = await this.httpClient.get(`/intents/${id}`);
    return IntentSchema.parse(response.data);
  }

  /**
   * List intents with filters.
   * @param filters Filter criteria
   * @returns List of Intents
   */
  async list(filters?: IntentFilters): Promise<Intent[]> {
    const response = await this.httpClient.get('/intents', { params: filters });
    // Assuming the API returns an array of intents directly or wrapped in data property
    // Adjust based on actual API response structure. Here assuming array.
    if (Array.isArray(response.data)) {
        return response.data.map((item: unknown) => IntentSchema.parse(item));
    }
    // If wrapped in 'data' field
    if (response.data && Array.isArray(response.data.data)) {
        return response.data.data.map((item: unknown) => IntentSchema.parse(item));
    }
     return [];
  }
}
