import { AxiosInstance } from 'axios';
import {
  SettlementExportQuery,
  SettlementOutcomeRequest,
  SettlementOutcomeResponse,
  SettlementOutcomeResponseSchema,
  SettlementWorkbenchQuery,
  SettlementWorkbenchResponse,
  SettlementWorkbenchResponseSchema,
} from '../types/settlement';

export class SettlementService {
  constructor(private readonly httpClient: AxiosInstance) {}

  async getWorkbench(query?: SettlementWorkbenchQuery): Promise<SettlementWorkbenchResponse> {
    const response = await this.httpClient.get('/admin/settlement/workbench', { params: query });
    return SettlementWorkbenchResponseSchema.parse(response.data);
  }

  async exportWorkbench(query?: SettlementExportQuery): Promise<unknown> {
    const response = await this.httpClient.get('/admin/settlement/export', { params: query });
    return response.data;
  }

  async applyOutcome(
    settlementId: string,
    data: SettlementOutcomeRequest
  ): Promise<SettlementOutcomeResponse> {
    const response = await this.httpClient.post(`/admin/settlement/${settlementId}/outcome`, data);
    return SettlementOutcomeResponseSchema.parse(response.data);
  }
}
