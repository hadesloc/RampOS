import { AxiosInstance } from 'axios';
import {
  UserBalance,
  UserBalanceSchema,
  UserKycStatus,
  UserKycStatusSchema,
} from '../types/user';

export class UserService {
  constructor(private readonly httpClient: AxiosInstance) {}

  /**
   * Get user balances.
   * @param tenantId Tenant ID
   * @param userId User ID
   * @returns List of User Balances
   */
  async getBalances(tenantId: string, userId: string): Promise<UserBalance[]> {
    const response = await this.httpClient.get(`/tenants/${tenantId}/users/${userId}/balances`);
    if (Array.isArray(response.data)) {
        return response.data.map((item: unknown) => UserBalanceSchema.parse(item));
    }
    return [];
  }

  /**
   * Get user KYC status.
   * @param tenantId Tenant ID
   * @param userId User ID
   * @returns User KYC Status
   */
  async getKycStatus(tenantId: string, userId: string): Promise<UserKycStatus> {
    const response = await this.httpClient.get(`/tenants/${tenantId}/users/${userId}/kyc`);
    return UserKycStatusSchema.parse(response.data);
  }
}
