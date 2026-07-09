import { AxiosInstance } from 'axios';
import {
  AdminOfframpResponse,
  AdminOfframpResponseSchema,
  CreateOfframpRequest,
  ListOfframpQuery,
  ListOfframpResponse,
  ListOfframpResponseSchema,
  OfframpCryptoReceivedRequest,
  OfframpIntentResponse,
  OfframpIntentResponseSchema,
  OfframpQuoteRequest,
  OfframpQuoteResponse,
  OfframpQuoteResponseSchema,
  RejectOfframpRequest,
} from '../types/offramp';

export class OfframpService {
  constructor(private readonly httpClient: AxiosInstance) {}

  async quote(data: OfframpQuoteRequest): Promise<OfframpQuoteResponse> {
    const response = await this.httpClient.post('/portal/offramp/quote', data);
    return OfframpQuoteResponseSchema.parse(response.data);
  }

  async create(data: CreateOfframpRequest): Promise<OfframpIntentResponse> {
    const response = await this.httpClient.post('/portal/offramp/create', data);
    return OfframpIntentResponseSchema.parse(response.data);
  }

  async status(id: string): Promise<OfframpIntentResponse> {
    const response = await this.httpClient.get(`/portal/offramp/${id}/status`);
    return OfframpIntentResponseSchema.parse(response.data);
  }

  async confirm(id: string): Promise<OfframpIntentResponse> {
    const response = await this.httpClient.post(`/portal/offramp/${id}/confirm`);
    return OfframpIntentResponseSchema.parse(response.data);
  }

  async cryptoReceived(id: string, data: OfframpCryptoReceivedRequest): Promise<OfframpIntentResponse> {
    const response = await this.httpClient.post(`/portal/offramp/${id}/crypto-received`, data);
    return OfframpIntentResponseSchema.parse(response.data);
  }

  async listPending(query?: ListOfframpQuery): Promise<ListOfframpResponse> {
    const response = await this.httpClient.get('/admin/offramp/pending', { params: query });
    return ListOfframpResponseSchema.parse(response.data);
  }

  async approve(id: string): Promise<AdminOfframpResponse> {
    const response = await this.httpClient.post(`/admin/offramp/${id}/approve`);
    return AdminOfframpResponseSchema.parse(response.data);
  }

  async reject(id: string, data: RejectOfframpRequest): Promise<AdminOfframpResponse> {
    const response = await this.httpClient.post(`/admin/offramp/${id}/reject`, data);
    return AdminOfframpResponseSchema.parse(response.data);
  }

  readonly admin = {
    listPending: (query?: ListOfframpQuery): Promise<ListOfframpResponse> => this.listPending(query),
    approve: (id: string): Promise<AdminOfframpResponse> => this.approve(id),
    reject: (id: string, data: RejectOfframpRequest): Promise<AdminOfframpResponse> =>
      this.reject(id, data),
  };
}
