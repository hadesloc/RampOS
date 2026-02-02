import { AxiosInstance } from 'axios';
import {
  SmartAccount,
  SmartAccountSchema,
  CreateAccountParams,
  AddSessionKeyParams,
  RemoveSessionKeyParams,
  UserOperationParams,
  GasEstimate,
  GasEstimateSchema,
  UserOpReceipt,
  UserOpReceiptSchema
} from '../types/aa';

export class AAService {
  constructor(private readonly httpClient: AxiosInstance) {}

  /**
   * Create a smart account for a user.
   * @param params Create Account Params
   * @returns Smart Account Info
   */
  async createSmartAccount(params: CreateAccountParams): Promise<SmartAccount> {
    const response = await this.httpClient.post(`/aa/accounts`, params);
    return SmartAccountSchema.parse(response.data);
  }

  /**
   * Get smart account info for a user.
   * @param address Smart Account Address
   * @returns Smart Account Info
   */
  async getSmartAccount(address: string): Promise<SmartAccount> {
    const response = await this.httpClient.get(`/aa/accounts/${address}`);
    return SmartAccountSchema.parse(response.data);
  }

  /**
   * Add a session key to an account.
   * @param params Add Session Key Params
   * @returns Void (throws on error)
   */
  async addSessionKey(params: AddSessionKeyParams): Promise<void> {
    await this.httpClient.post(`/aa/accounts/${params.accountAddress}/sessions`, params.sessionKey);
  }

  /**
   * Remove a session key from an account.
   * @param params Remove Session Key Params
   * @returns Void (throws on error)
   */
  async removeSessionKey(params: RemoveSessionKeyParams): Promise<void> {
    await this.httpClient.delete(`/aa/accounts/${params.accountAddress}/sessions/${params.keyId}`);
  }

  /**
   * Send a user operation.
   * @param params User Operation Params
   * @returns User Operation Receipt
   */
  async sendUserOperation(params: UserOperationParams): Promise<UserOpReceipt> {
    const response = await this.httpClient.post(`/aa/bundler/user-op`, params);
    return UserOpReceiptSchema.parse(response.data);
  }

  /**
   * Estimate gas for a user operation.
   * @param params User Operation Params
   * @returns Gas Estimate
   */
  async estimateGas(params: UserOperationParams): Promise<GasEstimate> {
    const response = await this.httpClient.post(`/aa/bundler/estimate-gas`, params);
    return GasEstimateSchema.parse(response.data);
  }
}
