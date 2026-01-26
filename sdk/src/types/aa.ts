import { z } from 'zod';

export const SmartAccountSchema = z.object({
  address: z.string(),
  owner: z.string(),
  factoryAddress: z.string(),
  deployed: z.boolean(),
  balance: z.string().optional(),
});

export type SmartAccount = z.infer<typeof SmartAccountSchema>;

export const CreateAccountParamsSchema = z.object({
  owner: z.string(),
  salt: z.string().optional(),
});

export type CreateAccountParams = z.infer<typeof CreateAccountParamsSchema>;

export const SessionKeySchema = z.object({
  id: z.string().optional(), // ID might be assigned by backend
  publicKey: z.string(),
  permissions: z.array(z.string()),
  validUntil: z.number(), // timestamp
  validAfter: z.number().optional(), // timestamp
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

// Raw UserOperation (ERC-4337)
export const UserOperationSchema = z.object({
  sender: z.string(),
  nonce: z.string(),
  initCode: z.string(),
  callData: z.string(),
  callGasLimit: z.string(),
  verificationGasLimit: z.string(),
  preVerificationGas: z.string(),
  maxFeePerGas: z.string(),
  maxPriorityFeePerGas: z.string(),
  paymasterAndData: z.string(),
  signature: z.string(),
});

export type UserOperation = z.infer<typeof UserOperationSchema>;

// High-level parameters for sending a transaction via AA
export const UserOperationParamsSchema = z.object({
  target: z.string(),
  value: z.string().default('0'),
  data: z.string().default('0x'),
  sponsored: z.boolean().optional(),
  accountAddress: z.string().optional(), // If not inferred from client context
});

export type UserOperationParams = z.infer<typeof UserOperationParamsSchema>;

export const GasEstimateSchema = z.object({
  preVerificationGas: z.string(),
  verificationGas: z.string(),
  callGasLimit: z.string(),
  total: z.string().optional(),
});

export type GasEstimate = z.infer<typeof GasEstimateSchema>;

export const UserOpReceiptSchema = z.object({
  userOpHash: z.string(),
  txHash: z.string().optional(),
  success: z.boolean().optional(),
});

export type UserOpReceipt = z.infer<typeof UserOpReceiptSchema>;
