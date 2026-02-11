import { useMutation } from 'urql';
import { CREATE_PAY_IN, CONFIRM_PAY_IN, CREATE_PAYOUT } from '@/lib/graphql/documents';

// --- CreatePayIn ---

export interface CreatePayInInput {
  userId: string;
  amountVnd: string;
  railsProvider: string;
  idempotencyKey?: string;
  metadata?: unknown;
}

export interface CreatePayInResult {
  intentId: string;
  referenceCode: string;
  status: string;
  expiresAt: string;
  dailyLimit: string;
  dailyRemaining: string;
}

export function useCreatePayIn() {
  return useMutation<{ createPayIn: CreatePayInResult }>(CREATE_PAY_IN);
}

// --- ConfirmPayIn ---

export interface ConfirmPayInInput {
  referenceCode: string;
  bankTxId: string;
  amountVnd: string;
  rawPayloadHash: string;
}

export interface ConfirmPayInResult {
  intentId: string;
  success: boolean;
}

export function useConfirmPayIn() {
  return useMutation<{ confirmPayIn: ConfirmPayInResult }>(CONFIRM_PAY_IN);
}

// --- CreatePayout ---

export interface CreatePayoutInput {
  userId: string;
  amountVnd: string;
  railsProvider: string;
  bankCode: string;
  accountNumber: string;
  accountName: string;
  idempotencyKey?: string;
  metadata?: unknown;
}

export interface CreatePayoutResult {
  intentId: string;
  status: string;
  dailyLimit: string;
  dailyRemaining: string;
}

export function useCreatePayout() {
  return useMutation<{ createPayout: CreatePayoutResult }>(CREATE_PAYOUT);
}
