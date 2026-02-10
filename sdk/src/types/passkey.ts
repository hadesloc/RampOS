import { z } from 'zod';

// ============================================================================
// Passkey Credential Types
// ============================================================================

export const PasskeyCredentialSchema = z.object({
  credentialId: z.string(),
  userId: z.string(),
  publicKeyX: z.string(),
  publicKeyY: z.string(),
  smartAccountAddress: z.string().nullable().optional(),
  displayName: z.string(),
  isActive: z.boolean(),
  createdAt: z.string(),
  lastUsedAt: z.string().nullable().optional(),
});

export type PasskeyCredential = z.infer<typeof PasskeyCredentialSchema>;

// ============================================================================
// Register Passkey
// ============================================================================

export const RegisterPasskeyParamsSchema = z.object({
  userId: z.string(),
  credentialId: z.string(),
  publicKeyX: z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, 'Invalid P256 x coordinate'),
  publicKeyY: z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, 'Invalid P256 y coordinate'),
  displayName: z.string(),
});

export type RegisterPasskeyParams = z.infer<typeof RegisterPasskeyParamsSchema>;

export const RegisterPasskeyResponseSchema = z.object({
  credentialId: z.string(),
  smartAccountAddress: z.string().nullable().optional(),
  createdAt: z.string(),
});

export type RegisterPasskeyResponse = z.infer<typeof RegisterPasskeyResponseSchema>;

// ============================================================================
// Create Passkey Wallet
// ============================================================================

export const CreatePasskeyWalletParamsSchema = z.object({
  userId: z.string(),
  credentialId: z.string(),
  publicKeyX: z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, 'Invalid P256 x coordinate'),
  publicKeyY: z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, 'Invalid P256 y coordinate'),
  displayName: z.string(),
  ownerAddress: z.string().optional(),
  salt: z.string().optional(),
});

export type CreatePasskeyWalletParams = z.infer<typeof CreatePasskeyWalletParamsSchema>;

export const CreatePasskeyWalletResponseSchema = z.object({
  credentialId: z.string(),
  smartAccountAddress: z.string(),
  publicKeyX: z.string(),
  publicKeyY: z.string(),
  isDeployed: z.boolean(),
  createdAt: z.string(),
});

export type CreatePasskeyWalletResponse = z.infer<typeof CreatePasskeyWalletResponseSchema>;

// ============================================================================
// Link Smart Account
// ============================================================================

export const LinkSmartAccountParamsSchema = z.object({
  userId: z.string(),
  credentialId: z.string(),
  smartAccountAddress: z.string(),
});

export type LinkSmartAccountParams = z.infer<typeof LinkSmartAccountParamsSchema>;

// ============================================================================
// Sign Transaction
// ============================================================================

export const PasskeySignatureSchema = z.object({
  r: z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, 'Invalid signature r component'),
  s: z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, 'Invalid signature s component'),
});

export type PasskeySignature = z.infer<typeof PasskeySignatureSchema>;

export const WebAuthnAssertionSchema = z.object({
  authenticatorData: z.string(),
  clientDataJSON: z.string(),
  signature: PasskeySignatureSchema,
  credentialId: z.string(),
});

export type WebAuthnAssertion = z.infer<typeof WebAuthnAssertionSchema>;

export const SignTransactionParamsSchema = z.object({
  userId: z.string(),
  credentialId: z.string(),
  userOperation: z.object({
    sender: z.string(),
    nonce: z.string(),
    callData: z.string(),
    callGasLimit: z.string().optional(),
    verificationGasLimit: z.string().optional(),
    preVerificationGas: z.string().optional(),
    maxFeePerGas: z.string().optional(),
    maxPriorityFeePerGas: z.string().optional(),
  }),
  assertion: WebAuthnAssertionSchema,
});

export type SignTransactionParams = z.infer<typeof SignTransactionParamsSchema>;

export const SignTransactionResponseSchema = z.object({
  userOpHash: z.string(),
  sender: z.string(),
  nonce: z.string(),
  signature: z.string(),
  status: z.string(),
});

export type SignTransactionResponse = z.infer<typeof SignTransactionResponseSchema>;

// ============================================================================
// Get Counterfactual Address
// ============================================================================

export const GetCounterfactualAddressParamsSchema = z.object({
  publicKeyX: z.string(),
  publicKeyY: z.string(),
  salt: z.string().optional(),
});

export type GetCounterfactualAddressParams = z.infer<typeof GetCounterfactualAddressParamsSchema>;

export const GetCounterfactualAddressResponseSchema = z.object({
  address: z.string(),
  isDeployed: z.boolean(),
});

export type GetCounterfactualAddressResponse = z.infer<typeof GetCounterfactualAddressResponseSchema>;
