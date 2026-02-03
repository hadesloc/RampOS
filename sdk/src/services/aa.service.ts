import { AxiosInstance } from 'axios';
import {
  SmartAccount,
  SmartAccountSchema,
  CreateAccountParams,
  CreateAccountResponse,
  CreateAccountResponseSchema,
  SendUserOperationRequest,
  SendUserOperationResponse,
  SendUserOperationResponseSchema,
  EstimateGasRequest,
  UserOperation,
  UserOperationSchema,
  GasEstimate,
  GasEstimateSchema,
  UserOpReceipt,
  UserOpReceiptSchema,
  AddSessionKeyParams,
  RemoveSessionKeyParams,
} from '../types/aa';

export class AAService {
  constructor(private readonly httpClient: AxiosInstance) {}

  /**
   * Create a smart account for a user.
   * @param params Create Account Params
   * @returns Smart Account Info
   */
  async createSmartAccount(params: CreateAccountParams): Promise<CreateAccountResponse> {
    const response = await this.httpClient.post(`/aa/accounts`, params);
    return CreateAccountResponseSchema.parse(response.data);
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
    void params;
    throw new Error('Session key management is not exposed via the API');
  }

  /**
   * Remove a session key from an account.
   * @param params Remove Session Key Params
   * @returns Void (throws on error)
   */
  async removeSessionKey(params: RemoveSessionKeyParams): Promise<void> {
    void params;
    throw new Error('Session key management is not exposed via the API');
  }

  /**
   * Send a user operation.
   * @param params User Operation Params
   * @returns User Operation Receipt
   */
  async sendUserOperation(params: SendUserOperationRequest): Promise<SendUserOperationResponse> {
    const response = await this.httpClient.post(`/aa/user-operations`, params);
    return SendUserOperationResponseSchema.parse(response.data);
  }

  /**
   * Estimate gas for a user operation.
   * @param params User Operation Params
   * @returns Gas Estimate
   */
  async estimateGas(params: EstimateGasRequest): Promise<GasEstimate> {
    const response = await this.httpClient.post(`/aa/user-operations/estimate`, params);
    return GasEstimateSchema.parse(response.data);
  }

  /**
   * Get a user operation by hash.
   */
  async getUserOperation(hash: string): Promise<UserOperation> {
    const response = await this.httpClient.get(`/aa/user-operations/${hash}`);
    return UserOperationSchema.parse(response.data);
  }

  /**
   * Get a user operation receipt by hash.
   */
  async getUserOperationReceipt(hash: string): Promise<UserOpReceipt> {
    const response = await this.httpClient.get(`/aa/user-operations/${hash}/receipt`);
    return UserOpReceiptSchema.parse(response.data);
  }
}
