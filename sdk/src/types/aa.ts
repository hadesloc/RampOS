import { z } from 'zod';

export const CreateAccountParamsSchema = z.object({
  tenantId: z.string(),
  userId: z.string(),
  ownerAddress: z.string(),
});

export type CreateAccountParams = z.infer<typeof CreateAccountParamsSchema>;

export const CreateAccountResponseSchema = z.object({
  address: z.string(),
  owner: z.string(),
  accountType: z.string(),
  isDeployed: z.boolean(),
  chainId: z.number(),
  entryPoint: z.string(),
});

export type CreateAccountResponse = z.infer<typeof CreateAccountResponseSchema>;

export const GetAccountResponseSchema = z.object({
  address: z.string(),
  isDeployed: z.boolean(),
  nonce: z.string(),
  chainId: z.number(),
  entryPoint: z.string(),
  accountType: z.string(),
});

export type GetAccountResponse = z.infer<typeof GetAccountResponseSchema>;

export const SmartAccountSchema = GetAccountResponseSchema;
export type SmartAccount = GetAccountResponse;

export const UserOperationSchema = z.object({
  sender: z.string(),
  nonce: z.string(),
  initCode: z.string().optional(),
  callData: z.string(),
  callGasLimit: z.string(),
  verificationGasLimit: z.string(),
  preVerificationGas: z.string(),
  maxFeePerGas: z.string(),
  maxPriorityFeePerGas: z.string(),
  paymasterAndData: z.string().optional(),
  signature: z.string().optional(),
});

export type UserOperation = z.infer<typeof UserOperationSchema>;

export const SendUserOperationRequestSchema = z.object({
  tenantId: z.string(),
  userOperation: UserOperationSchema,
  sponsor: z.boolean().optional(),
});

export type SendUserOperationRequest = z.infer<typeof SendUserOperationRequestSchema>;

export const SendUserOperationResponseSchema = z.object({
  userOpHash: z.string(),
  sender: z.string(),
  nonce: z.string(),
  status: z.string(),
  sponsored: z.boolean(),
});

export type SendUserOperationResponse = z.infer<typeof SendUserOperationResponseSchema>;

export const EstimateGasRequestSchema = z.object({
  tenantId: z.string(),
  userOperation: UserOperationSchema,
});

export type EstimateGasRequest = z.infer<typeof EstimateGasRequestSchema>;

export const GasEstimateSchema = z.object({
  preVerificationGas: z.string(),
  verificationGasLimit: z.string(),
  callGasLimit: z.string(),
  maxFeePerGas: z.string(),
  maxPriorityFeePerGas: z.string(),
});

export type GasEstimate = z.infer<typeof GasEstimateSchema>;

export const UserOpReceiptSchema = z.object({
  userOpHash: z.string(),
  sender: z.string(),
  nonce: z.string(),
  success: z.boolean(),
  actualGasCost: z.string(),
  actualGasUsed: z.string(),
  paymaster: z.string().optional(),
  transactionHash: z.string(),
  blockHash: z.string(),
  blockNumber: z.string(),
});

export type UserOpReceipt = z.infer<typeof UserOpReceiptSchema>;

export const SessionKeySchema = z.object({
  id: z.string().optional(),
  publicKey: z.string(),
  permissions: z.array(z.string()),
  validUntil: z.number(),
  validAfter: z.number().optional(),
});

export type SessionKey = z.infer<typeof SessionKeySchema>;

export const AddSessionKeyParamsSchema = z.object({
  accountAddress: z.string(),
  sessionKey: SessionKeySchema,
});

export type AddSessionKeyParams = z.infer<typeof AddSessionKeyParamsSchema>;

export const RemoveSessionKeyParamsSchema = z.object({
  accountAddress: z.string(),
  keyId: z.string(),
});

export type RemoveSessionKeyParams = z.infer<typeof RemoveSessionKeyParamsSchema>;
