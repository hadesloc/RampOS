import { z } from 'zod';

export const BalanceSchema = z.object({
  accountType: z.string(),
  currency: z.string(),
  balance: z.string(),
});

export type Balance = z.infer<typeof BalanceSchema>;

export const UserBalancesResponseSchema = z.object({
  balances: z.array(BalanceSchema),
});

export type UserBalancesResponse = z.infer<typeof UserBalancesResponseSchema>;

export const UserBalanceSchema = BalanceSchema;
export type UserBalance = Balance;

export enum KycStatus {
  NONE = 'NONE',
  PENDING = 'PENDING',
  VERIFIED = 'VERIFIED',
  REJECTED = 'REJECTED',
}

export const UserKycStatusSchema = z.object({
  userId: z.string(),
  status: z.nativeEnum(KycStatus),
  updatedAt: z.string(),
});

export type UserKycStatus = z.infer<typeof UserKycStatusSchema>;
