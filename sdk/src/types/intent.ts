import { z } from 'zod';

export enum IntentType {
  PAYIN = 'PAYIN',
  PAYOUT = 'PAYOUT',
  TRADE = 'TRADE',
}

export const StateHistoryEntrySchema = z.object({
  state: z.string(),
  timestamp: z.string(),
  reason: z.string().optional(),
});

export type StateHistoryEntry = z.infer<typeof StateHistoryEntrySchema>;

export const IntentSchema = z.object({
  id: z.string(),
  userId: z.string().optional(),
  intentType: z.string(),
  state: z.string(),
  amount: z.string(),
  currency: z.string(),
  actualAmount: z.string().optional(),
  referenceCode: z.string().optional(),
  bankTxId: z.string().optional(),
  chainId: z.string().optional(),
  txHash: z.string().optional(),
  stateHistory: z.array(StateHistoryEntrySchema).optional(),
  createdAt: z.string(),
  updatedAt: z.string(),
  expiresAt: z.string().optional(),
  completedAt: z.string().optional(),
  metadata: z.record(z.any()).optional(),
});

export type Intent = z.infer<typeof IntentSchema>;

export const VirtualAccountSchema = z.object({
  bank: z.string(),
  accountNumber: z.string(),
  accountName: z.string(),
});

export type VirtualAccount = z.infer<typeof VirtualAccountSchema>;

export const BankAccountSchema = z.object({
  bankCode: z.string(),
  accountNumber: z.string(),
  accountName: z.string(),
});

export type BankAccount = z.infer<typeof BankAccountSchema>;

export const CreatePayinRequestSchema = z.object({
  tenantId: z.string(),
  userId: z.string(),
  amountVnd: z.number(),
  railsProvider: z.string(),
  metadata: z.record(z.any()).optional(),
});

export type CreatePayinRequest = z.infer<typeof CreatePayinRequestSchema>;

export const CreatePayinResponseSchema = z.object({
  intentId: z.string(),
  referenceCode: z.string(),
  virtualAccount: VirtualAccountSchema.optional(),
  expiresAt: z.string(),
  status: z.string(),
});

export type CreatePayinResponse = z.infer<typeof CreatePayinResponseSchema>;

export const ConfirmPayinRequestSchema = z.object({
  tenantId: z.string(),
  referenceCode: z.string(),
  status: z.string(),
  bankTxId: z.string(),
  amountVnd: z.number(),
  settledAt: z.string(),
  rawPayloadHash: z.string(),
});

export type ConfirmPayinRequest = z.infer<typeof ConfirmPayinRequestSchema>;

export const ConfirmPayinResponseSchema = z.object({
  intentId: z.string(),
  status: z.string(),
});

export type ConfirmPayinResponse = z.infer<typeof ConfirmPayinResponseSchema>;

export const CreatePayoutRequestSchema = z.object({
  tenantId: z.string(),
  userId: z.string(),
  amountVnd: z.number(),
  railsProvider: z.string(),
  bankAccount: BankAccountSchema,
  metadata: z.record(z.any()).optional(),
});

export type CreatePayoutRequest = z.infer<typeof CreatePayoutRequestSchema>;

export const CreatePayoutResponseSchema = z.object({
  intentId: z.string(),
  status: z.string(),
});

export type CreatePayoutResponse = z.infer<typeof CreatePayoutResponseSchema>;

export const CreatePayInSchema = CreatePayinRequestSchema;
export type CreatePayInDto = CreatePayinRequest;
export const CreatePayOutSchema = CreatePayoutRequestSchema;
export type CreatePayOutDto = CreatePayoutRequest;

export const IntentFilterSchema = z.object({
  userId: z.string().optional(),
  intentType: z.string().optional(),
  state: z.string().optional(),
  limit: z.number().optional(),
  offset: z.number().optional(),
});

export type IntentFilters = z.infer<typeof IntentFilterSchema>;
