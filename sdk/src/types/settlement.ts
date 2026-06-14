import { z } from 'zod';

export const SettlementOutcomeSchema = z.enum(['COMPLETED', 'FAILED']);
export type SettlementOutcome = z.infer<typeof SettlementOutcomeSchema>;

export const SettlementWorkbenchQuerySchema = z.object({
  scenario: z.string().optional(),
});

export type SettlementWorkbenchQuery = z.infer<typeof SettlementWorkbenchQuerySchema>;

export const SettlementExportQuerySchema = z.object({
  scenario: z.string().optional(),
  format: z.enum(['json', 'csv']).optional(),
});

export type SettlementExportQuery = z.infer<typeof SettlementExportQuerySchema>;

export const SettlementOutcomeRequestSchema = z.object({
  outcome: SettlementOutcomeSchema,
  errorMessage: z.string().optional(),
});

export type SettlementOutcomeRequest = z.infer<typeof SettlementOutcomeRequestSchema>;

export const SettlementOutcomeResponseSchema = z.object({
  settlementId: z.string(),
  offrampIntentId: z.string(),
  status: z.string(),
  rfqId: z.string().optional().nullable(),
  lpId: z.string().optional().nullable(),
  finalRate: z.string().optional().nullable(),
});

export type SettlementOutcomeResponse = z.infer<typeof SettlementOutcomeResponseSchema>;

export const SettlementWorkbenchResponseSchema = z.object({
  snapshot: z.unknown(),
  actionMode: z.string(),
  approvalMode: z.string(),
  proposalCount: z.number(),
  exportFormats: z.array(z.string()),
});

export type SettlementWorkbenchResponse = z.infer<typeof SettlementWorkbenchResponseSchema>;
