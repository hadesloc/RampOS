import { z } from 'zod';

export const UserBalanceSchema = z.object({
  currency: z.string(),
  amount: z.string(),
  locked: z.string(),
});

export type UserBalance = z.infer<typeof UserBalanceSchema>;

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
