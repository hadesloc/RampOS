import { z } from 'zod';

export enum IntentType {
  PAY_IN = 'PAY_IN',
  PAY_OUT = 'PAY_OUT',
  TRADE = 'TRADE',
}

export enum IntentStatus {
  CREATED = 'CREATED',
  PENDING = 'PENDING',
  COMPLETED = 'COMPLETED',
  FAILED = 'FAILED',
  CANCELLED = 'CANCELLED',
}

export const IntentSchema = z.object({
  id: z.string(),
  tenantId: z.string(),
  type: z.nativeEnum(IntentType),
  status: z.nativeEnum(IntentStatus),
  amount: z.string(),
  currency: z.string(),
  bankAccount: z.string().optional(),
  bankRef: z.string().optional(),
  metadata: z.record(z.any()).optional(),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type Intent = z.infer<typeof IntentSchema>;

export const CreatePayInSchema = z.object({
  amount: z.string(),
  currency: z.string(),
  metadata: z.record(z.any()).optional(),
});

export type CreatePayInDto = z.infer<typeof CreatePayInSchema>;

export const CreatePayOutSchema = z.object({
  amount: z.string(),
  currency: z.string(),
  bankAccount: z.string(),
  metadata: z.record(z.any()).optional(),
});

export type CreatePayOutDto = z.infer<typeof CreatePayOutSchema>;

export const IntentFilterSchema = z.object({
  type: z.nativeEnum(IntentType).optional(),
  status: z.nativeEnum(IntentStatus).optional(),
  startDate: z.string().optional(),
  endDate: z.string().optional(),
  limit: z.number().optional(),
  offset: z.number().optional(),
});

export type IntentFilters = z.infer<typeof IntentFilterSchema>;
