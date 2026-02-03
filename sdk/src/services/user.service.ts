import { AxiosInstance } from 'axios';
import {
  UserBalance,
  UserBalancesResponseSchema,
  UserKycStatus,
  UserKycStatusSchema,
} from '../types/user';

export class UserService {
  constructor(private readonly httpClient: AxiosInstance) {}

  /**
   * Get user balances.
   * @param userId User ID
   * @returns List of User Balances
   */
  async getBalances(userId: string): Promise<UserBalance[]> {
    const response = await this.httpClient.get(`/balance/${userId}`);
    return UserBalancesResponseSchema.parse(response.data).balances;
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
