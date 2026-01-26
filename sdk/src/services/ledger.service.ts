import { AxiosInstance } from 'axios';
import {
  LedgerEntry,
  LedgerEntrySchema,
  LedgerFilters,
} from '../types/ledger';

export class LedgerService {
  constructor(private readonly httpClient: AxiosInstance) {}

  /**
   * Get ledger entries with filters.
   * @param filters Filter criteria
   * @returns List of Ledger Entries
   */
  async getEntries(filters?: LedgerFilters): Promise<LedgerEntry[]> {
    const response = await this.httpClient.get('/ledger', { params: filters });
    if (Array.isArray(response.data)) {
        return response.data.map((item: unknown) => LedgerEntrySchema.parse(item));
    }
    if (response.data && Array.isArray(response.data.data)) {
        return response.data.data.map((item: unknown) => LedgerEntrySchema.parse(item));
    }
    return [];
  }
}
