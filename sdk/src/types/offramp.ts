import { z } from 'zod';

export const OfframpStateSchema = z.enum([
  'QUOTE_CREATED',
  'CRYPTO_PENDING',
  'CRYPTO_RECEIVED',
  'VND_TRANSFERRING',
  'COMPLETED',
  'FAILED',
  'EXPIRED',
]);

export type OfframpState = z.infer<typeof OfframpStateSchema>;

export const OfframpQuoteRequestSchema = z.object({
  cryptoAsset: z.string(),
  amount: z.string(),
  bankCode: z.string(),
  accountNumber: z.string(),
  accountName: z.string(),
});

export type OfframpQuoteRequest = z.infer<typeof OfframpQuoteRequestSchema>;

export const OfframpQuoteResponseSchema = z.object({
  quoteId: z.string(),
  cryptoAsset: z.string(),
  cryptoAmount: z.string(),
  exchangeRate: z.string(),
  grossVndAmount: z.string(),
  netVndAmount: z.string(),
  feeTotal: z.string(),
  expiresAt: z.string(),
});

export type OfframpQuoteResponse = z.infer<typeof OfframpQuoteResponseSchema>;

export const CreateOfframpRequestSchema = z.object({
  quoteId: z.string(),
  chainId: z.number().optional(),
});

export type CreateOfframpRequest = z.infer<typeof CreateOfframpRequestSchema>;

export const OfframpCryptoReceivedRequestSchema = z.object({
  txHash: z.string(),
  chainId: z.number(),
  fromAddress: z.string(),
  toAddress: z.string(),
  blockNumber: z.number().optional(),
  confirmations: z.number().optional(),
  rawPayload: z.unknown().optional(),
});

export type OfframpCryptoReceivedRequest = z.infer<typeof OfframpCryptoReceivedRequestSchema>;

export const OfframpIntentResponseSchema = z.object({
  id: z.string(),
  state: z.string(),
  cryptoAsset: z.string(),
  cryptoAmount: z.string(),
  exchangeRate: z.string(),
  netVndAmount: z.string(),
  grossVndAmount: z.string(),
  depositAddress: z.string().optional().nullable(),
  chainId: z.number().optional().nullable(),
  txHash: z.string().optional().nullable(),
  bankReference: z.string().optional().nullable(),
  linkedRfqId: z.string().optional().nullable(),
  winningLpId: z.string().optional().nullable(),
  matchedRate: z.string().optional().nullable(),
  settlementId: z.string().optional().nullable(),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type OfframpIntentResponse = z.infer<typeof OfframpIntentResponseSchema>;

export const AdminOfframpResponseSchema = OfframpIntentResponseSchema.extend({
  userId: z.string(),
});

export type AdminOfframpResponse = z.infer<typeof AdminOfframpResponseSchema>;

export const ListOfframpQuerySchema = z.object({
  limit: z.number().optional(),
  offset: z.number().optional(),
});

export type ListOfframpQuery = z.infer<typeof ListOfframpQuerySchema>;

export const ListOfframpResponseSchema = z.object({
  data: z.array(AdminOfframpResponseSchema),
  total: z.number(),
  limit: z.number(),
  offset: z.number(),
});

export type ListOfframpResponse = z.infer<typeof ListOfframpResponseSchema>;

export const RejectOfframpRequestSchema = z.object({
  reason: z.string(),
});

export type RejectOfframpRequest = z.infer<typeof RejectOfframpRequestSchema>;
