import { AxiosInstance } from 'axios';
import {
  CreatePayInDto,
  CreatePayOutDto,
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
  async createPayIn(data: CreatePayInDto): Promise<Intent> {
    const response = await this.httpClient.post('/intents/pay-in', data);
    return IntentSchema.parse(response.data);
  }

  /**
   * Confirm a Pay-In intent.
   * @param id Intent ID
   * @param bankRef Bank Reference Code
   * @returns Updated Intent
   */
  async confirmPayIn(id: string, bankRef: string): Promise<Intent> {
    const response = await this.httpClient.post(`/intents/${id}/confirm`, { bankRef });
    return IntentSchema.parse(response.data);
  }

  /**
   * Create a new Pay-Out intent.
   * @param data Pay-Out data
   * @returns Created Intent
   */
  async createPayOut(data: CreatePayOutDto): Promise<Intent> {
    const response = await this.httpClient.post('/intents/pay-out', data);
    return IntentSchema.parse(response.data);
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
