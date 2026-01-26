import { z } from 'zod';

export enum LedgerEntryType {
  CREDIT = 'CREDIT',
  DEBIT = 'DEBIT',
}

export const LedgerEntrySchema = z.object({
  id: z.string(),
  tenantId: z.string(),
  transactionId: z.string(),
  type: z.nativeEnum(LedgerEntryType),
  amount: z.string(),
  currency: z.string(),
  balanceAfter: z.string(),
  referenceId: z.string().optional(),
  description: z.string().optional(),
  createdAt: z.string(),
});

export type LedgerEntry = z.infer<typeof LedgerEntrySchema>;

export const LedgerFilterSchema = z.object({
  transactionId: z.string().optional(),
  referenceId: z.string().optional(),
  startDate: z.string().optional(),
  endDate: z.string().optional(),
  limit: z.number().optional(),
  offset: z.number().optional(),
});

export type LedgerFilters = z.infer<typeof LedgerFilterSchema>;
