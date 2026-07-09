import { AxiosInstance } from 'axios';
import {
  AdminRfqResponse,
  CreateRfqRequest,
  FinalizeRfqResponse,
  FinalizeRfqResponseSchema,
  ListOpenRfqQuery,
  ListOpenRfqResponse,
  ListOpenRfqResponseSchema,
  RfqBidResponse,
  RfqBidResponseSchema,
  RfqDetailResponse,
  RfqDetailResponseSchema,
  RfqResponse,
  RfqResponseSchema,
  SubmitRfqBidRequest,
} from '../types/rfq';

export class RfqService {
  constructor(private readonly httpClient: AxiosInstance) {}

  async create(data: CreateRfqRequest): Promise<RfqResponse> {
    const response = await this.httpClient.post('/portal/rfq', data);
    return RfqResponseSchema.parse(response.data);
  }

  async get(id: string): Promise<RfqDetailResponse> {
    const response = await this.httpClient.get(`/portal/rfq/${id}`);
    return RfqDetailResponseSchema.parse(response.data);
  }

  async accept(id: string): Promise<RfqDetailResponse> {
    const response = await this.httpClient.post(`/portal/rfq/${id}/accept`);
    return RfqDetailResponseSchema.parse(response.data);
  }

  async cancel(id: string): Promise<RfqResponse> {
    const response = await this.httpClient.post(`/portal/rfq/${id}/cancel`);
    return RfqResponseSchema.parse(response.data);
  }

  async listOpen(query?: ListOpenRfqQuery): Promise<ListOpenRfqResponse> {
    const response = await this.httpClient.get('/admin/rfq/open', { params: query });
    return ListOpenRfqResponseSchema.parse(response.data);
  }

  async finalize(id: string): Promise<FinalizeRfqResponse> {
    const response = await this.httpClient.post(`/admin/rfq/${id}/finalize`);
    return FinalizeRfqResponseSchema.parse(response.data);
  }

  async submitBid(rfqId: string, data: SubmitRfqBidRequest): Promise<RfqBidResponse> {
    const response = await this.httpClient.post(`/lp/rfq/${rfqId}/bid`, data);
    return RfqBidResponseSchema.parse(response.data);
  }

  readonly admin = {
    listOpen: (query?: ListOpenRfqQuery): Promise<ListOpenRfqResponse> => this.listOpen(query),
    finalize: (id: string): Promise<FinalizeRfqResponse> => this.finalize(id),
  };

  readonly lp = {
    submitBid: (rfqId: string, data: SubmitRfqBidRequest): Promise<RfqBidResponse> =>
      this.submitBid(rfqId, data),
  };
}

export type { AdminRfqResponse };
