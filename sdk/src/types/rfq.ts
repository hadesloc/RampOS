import { z } from 'zod';

export const RfqDirectionSchema = z.enum(['OFFRAMP', 'ONRAMP']);
export type RfqDirection = z.infer<typeof RfqDirectionSchema>;

export const RfqStateSchema = z.enum(['OPEN', 'MATCHED', 'CANCELLED', 'EXPIRED']);
export type RfqState = z.infer<typeof RfqStateSchema>;

export const CreateRfqRequestSchema = z.object({
  direction: RfqDirectionSchema,
  cryptoAsset: z.string(),
  cryptoAmount: z.string(),
  vndAmount: z.string().optional(),
  offrampId: z.string().optional(),
  ttlMinutes: z.number().optional(),
});

export type CreateRfqRequest = z.infer<typeof CreateRfqRequestSchema>;

export const RfqResponseSchema = z.object({
  id: z.string(),
  direction: z.string(),
  cryptoAsset: z.string(),
  cryptoAmount: z.string(),
  vndAmount: z.string().optional().nullable(),
  state: z.string(),
  expiresAt: z.string(),
  winningLpId: z.string().optional().nullable(),
  finalRate: z.string().optional().nullable(),
  createdAt: z.string(),
});

export type RfqResponse = z.infer<typeof RfqResponseSchema>;

export const RfqBidSummarySchema = z.object({
  id: z.string(),
  lpId: z.string(),
  lpName: z.string().optional().nullable(),
  exchangeRate: z.string(),
  vndAmount: z.string(),
  validUntil: z.string(),
  state: z.string(),
});

export type RfqBidSummary = z.infer<typeof RfqBidSummarySchema>;

export const RfqDetailResponseSchema = z.object({
  rfq: RfqResponseSchema,
  bids: z.array(RfqBidSummarySchema),
  bestRate: z.string().optional().nullable(),
  bidCount: z.number(),
});

export type RfqDetailResponse = z.infer<typeof RfqDetailResponseSchema>;

export const ListOpenRfqQuerySchema = z.object({
  direction: RfqDirectionSchema.optional(),
  limit: z.number().optional(),
  offset: z.number().optional(),
});

export type ListOpenRfqQuery = z.infer<typeof ListOpenRfqQuerySchema>;

export const AdminRfqResponseSchema = z.object({
  id: z.string(),
  userId: z.string(),
  direction: z.string(),
  cryptoAsset: z.string(),
  cryptoAmount: z.string(),
  vndAmount: z.string().optional().nullable(),
  state: z.string(),
  bidCount: z.number(),
  bestRate: z.string().optional().nullable(),
  expiresAt: z.string(),
  createdAt: z.string(),
});

export type AdminRfqResponse = z.infer<typeof AdminRfqResponseSchema>;

export const ListOpenRfqResponseSchema = z.object({
  data: z.array(AdminRfqResponseSchema),
  total: z.number(),
  limit: z.number(),
  offset: z.number(),
});

export type ListOpenRfqResponse = z.infer<typeof ListOpenRfqResponseSchema>;

export const FinalizeRfqResponseSchema = z.object({
  rfqId: z.string(),
  state: z.string(),
  winningLpId: z.string(),
  finalRate: z.string(),
});

export type FinalizeRfqResponse = z.infer<typeof FinalizeRfqResponseSchema>;

export const SubmitRfqBidRequestSchema = z.object({
  exchangeRate: z.string(),
  vndAmount: z.string(),
  lpName: z.string().optional(),
  validMinutes: z.number().optional(),
});

export type SubmitRfqBidRequest = z.infer<typeof SubmitRfqBidRequestSchema>;

export const RfqBidResponseSchema = z.object({
  id: z.string(),
  rfqId: z.string(),
  lpId: z.string(),
  exchangeRate: z.string(),
  vndAmount: z.string(),
  validUntil: z.string(),
  state: z.string(),
});

export type RfqBidResponse = z.infer<typeof RfqBidResponseSchema>;
