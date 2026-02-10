"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);

// src/index.ts
var index_exports = {};
__export(index_exports, {
  AAService: () => AAService,
  BalanceSchema: () => BalanceSchema,
  BankAccountSchema: () => BankAccountSchema,
  ConfirmPayinRequestSchema: () => ConfirmPayinRequestSchema,
  ConfirmPayinResponseSchema: () => ConfirmPayinResponseSchema,
  CreateAccountParamsSchema: () => CreateAccountParamsSchema,
  CreateAccountResponseSchema: () => CreateAccountResponseSchema,
  CreatePasskeyWalletParamsSchema: () => CreatePasskeyWalletParamsSchema,
  CreatePasskeyWalletResponseSchema: () => CreatePasskeyWalletResponseSchema,
  CreatePayInSchema: () => CreatePayInSchema,
  CreatePayOutSchema: () => CreatePayOutSchema,
  CreatePayinRequestSchema: () => CreatePayinRequestSchema,
  CreatePayinResponseSchema: () => CreatePayinResponseSchema,
  CreatePayoutRequestSchema: () => CreatePayoutRequestSchema,
  CreatePayoutResponseSchema: () => CreatePayoutResponseSchema,
  EstimateGasRequestSchema: () => EstimateGasRequestSchema,
  GasEstimateSchema: () => GasEstimateSchema,
  GetAccountResponseSchema: () => GetAccountResponseSchema,
  GetCounterfactualAddressParamsSchema: () => GetCounterfactualAddressParamsSchema,
  GetCounterfactualAddressResponseSchema: () => GetCounterfactualAddressResponseSchema,
  IntentFilterSchema: () => IntentFilterSchema,
  IntentSchema: () => IntentSchema,
  IntentService: () => IntentService,
  IntentType: () => IntentType,
  KycStatus: () => KycStatus,
  LedgerEntrySchema: () => LedgerEntrySchema,
  LedgerEntryType: () => LedgerEntryType,
  LedgerFilterSchema: () => LedgerFilterSchema,
  LedgerService: () => LedgerService,
  LinkSmartAccountParamsSchema: () => LinkSmartAccountParamsSchema,
  MultichainProvider: () => MultichainProvider,
  PasskeyCredentialSchema: () => PasskeyCredentialSchema,
  PasskeySignatureSchema: () => PasskeySignatureSchema,
  PasskeyWalletService: () => PasskeyWalletService,
  RampOSClient: () => RampOSClient,
  RegisterPasskeyParamsSchema: () => RegisterPasskeyParamsSchema,
  RegisterPasskeyResponseSchema: () => RegisterPasskeyResponseSchema,
  SendUserOperationRequestSchema: () => SendUserOperationRequestSchema,
  SendUserOperationResponseSchema: () => SendUserOperationResponseSchema,
  SignTransactionParamsSchema: () => SignTransactionParamsSchema,
  SignTransactionResponseSchema: () => SignTransactionResponseSchema,
  SmartAccountSchema: () => SmartAccountSchema,
  StateHistoryEntrySchema: () => StateHistoryEntrySchema,
  UserBalanceSchema: () => UserBalanceSchema,
  UserBalancesResponseSchema: () => UserBalancesResponseSchema,
  UserKycStatusSchema: () => UserKycStatusSchema,
  UserOpReceiptSchema: () => UserOpReceiptSchema,
  UserOperationSchema: () => UserOperationSchema,
  UserService: () => UserService,
  VirtualAccountSchema: () => VirtualAccountSchema,
  WebAuthnAssertionSchema: () => WebAuthnAssertionSchema,
  WebhookVerifier: () => WebhookVerifier,
  withRetry: () => withRetry
});
module.exports = __toCommonJS(index_exports);

// src/client.ts
var import_axios = __toESM(require("axios"));

// src/types/intent.ts
var import_zod = require("zod");
var IntentType = /* @__PURE__ */ ((IntentType2) => {
  IntentType2["PAYIN"] = "PAYIN";
  IntentType2["PAYOUT"] = "PAYOUT";
  IntentType2["TRADE"] = "TRADE";
  return IntentType2;
})(IntentType || {});
var StateHistoryEntrySchema = import_zod.z.object({
  state: import_zod.z.string(),
  timestamp: import_zod.z.string(),
  reason: import_zod.z.string().optional()
});
var IntentSchema = import_zod.z.object({
  id: import_zod.z.string(),
  userId: import_zod.z.string().optional(),
  intentType: import_zod.z.string(),
  state: import_zod.z.string(),
  amount: import_zod.z.string(),
  currency: import_zod.z.string(),
  actualAmount: import_zod.z.string().optional(),
  referenceCode: import_zod.z.string().optional(),
  bankTxId: import_zod.z.string().optional(),
  chainId: import_zod.z.string().optional(),
  txHash: import_zod.z.string().optional(),
  stateHistory: import_zod.z.array(StateHistoryEntrySchema).optional(),
  createdAt: import_zod.z.string(),
  updatedAt: import_zod.z.string(),
  expiresAt: import_zod.z.string().optional(),
  completedAt: import_zod.z.string().optional(),
  metadata: import_zod.z.record(import_zod.z.any()).optional()
});
var VirtualAccountSchema = import_zod.z.object({
  bank: import_zod.z.string(),
  accountNumber: import_zod.z.string(),
  accountName: import_zod.z.string()
});
var BankAccountSchema = import_zod.z.object({
  bankCode: import_zod.z.string(),
  accountNumber: import_zod.z.string(),
  accountName: import_zod.z.string()
});
var CreatePayinRequestSchema = import_zod.z.object({
  tenantId: import_zod.z.string(),
  userId: import_zod.z.string(),
  amountVnd: import_zod.z.number(),
  railsProvider: import_zod.z.string(),
  metadata: import_zod.z.record(import_zod.z.any()).optional()
});
var CreatePayinResponseSchema = import_zod.z.object({
  intentId: import_zod.z.string(),
  referenceCode: import_zod.z.string(),
  virtualAccount: VirtualAccountSchema.optional(),
  expiresAt: import_zod.z.string(),
  status: import_zod.z.string()
});
var ConfirmPayinRequestSchema = import_zod.z.object({
  tenantId: import_zod.z.string(),
  referenceCode: import_zod.z.string(),
  status: import_zod.z.string(),
  bankTxId: import_zod.z.string(),
  amountVnd: import_zod.z.number(),
  settledAt: import_zod.z.string(),
  rawPayloadHash: import_zod.z.string()
});
var ConfirmPayinResponseSchema = import_zod.z.object({
  intentId: import_zod.z.string(),
  status: import_zod.z.string()
});
var CreatePayoutRequestSchema = import_zod.z.object({
  tenantId: import_zod.z.string(),
  userId: import_zod.z.string(),
  amountVnd: import_zod.z.number(),
  railsProvider: import_zod.z.string(),
  bankAccount: BankAccountSchema,
  metadata: import_zod.z.record(import_zod.z.any()).optional()
});
var CreatePayoutResponseSchema = import_zod.z.object({
  intentId: import_zod.z.string(),
  status: import_zod.z.string()
});
var CreatePayInSchema = CreatePayinRequestSchema;
var CreatePayOutSchema = CreatePayoutRequestSchema;
var IntentFilterSchema = import_zod.z.object({
  userId: import_zod.z.string().optional(),
  intentType: import_zod.z.string().optional(),
  state: import_zod.z.string().optional(),
  limit: import_zod.z.number().optional(),
  offset: import_zod.z.number().optional()
});

// src/services/intent.service.ts
var IntentService = class {
  constructor(httpClient) {
    this.httpClient = httpClient;
  }
  /**
   * Create a new Pay-In intent.
   * @param data Pay-In data
   * @returns Created Intent
   */
  async createPayIn(data) {
    const response = await this.httpClient.post("/intents/payin", data);
    return CreatePayinResponseSchema.parse(response.data);
  }
  /**
   * Confirm a Pay-In intent.
   * @param data Confirm Pay-In data
   * @returns Confirmation result
   */
  async confirmPayIn(data) {
    const response = await this.httpClient.post("/intents/payin/confirm", data);
    return ConfirmPayinResponseSchema.parse(response.data);
  }
  /**
   * Create a new Pay-Out intent.
   * @param data Pay-Out data
   * @returns Created Intent
   */
  async createPayOut(data) {
    const response = await this.httpClient.post("/intents/payout", data);
    return CreatePayoutResponseSchema.parse(response.data);
  }
  /**
   * Get an intent by ID.
   * @param id Intent ID
   * @returns Intent
   */
  async get(id) {
    const response = await this.httpClient.get(`/intents/${id}`);
    return IntentSchema.parse(response.data);
  }
  /**
   * List intents with filters.
   * @param filters Filter criteria
   * @returns List of Intents
   */
  async list(filters) {
    const response = await this.httpClient.get("/intents", { params: filters });
    if (Array.isArray(response.data)) {
      return response.data.map((item) => IntentSchema.parse(item));
    }
    if (response.data && Array.isArray(response.data.data)) {
      return response.data.data.map((item) => IntentSchema.parse(item));
    }
    return [];
  }
};

// src/types/user.ts
var import_zod2 = require("zod");
var BalanceSchema = import_zod2.z.object({
  accountType: import_zod2.z.string(),
  currency: import_zod2.z.string(),
  balance: import_zod2.z.string()
});
var UserBalancesResponseSchema = import_zod2.z.object({
  balances: import_zod2.z.array(BalanceSchema)
});
var UserBalanceSchema = BalanceSchema;
var KycStatus = /* @__PURE__ */ ((KycStatus2) => {
  KycStatus2["NONE"] = "NONE";
  KycStatus2["PENDING"] = "PENDING";
  KycStatus2["VERIFIED"] = "VERIFIED";
  KycStatus2["REJECTED"] = "REJECTED";
  return KycStatus2;
})(KycStatus || {});
var UserKycStatusSchema = import_zod2.z.object({
  userId: import_zod2.z.string(),
  status: import_zod2.z.nativeEnum(KycStatus),
  updatedAt: import_zod2.z.string()
});

// src/services/user.service.ts
var UserService = class {
  constructor(httpClient) {
    this.httpClient = httpClient;
  }
  /**
   * Get user balances.
   * @param userId User ID
   * @returns List of User Balances
   */
  async getBalances(userId) {
    const response = await this.httpClient.get(`/balance/${userId}`);
    return UserBalancesResponseSchema.parse(response.data).balances;
  }
  /**
   * Get user KYC status.
   * @param tenantId Tenant ID
   * @param userId User ID
   * @returns User KYC Status
   */
  async getKycStatus(tenantId, userId) {
    const response = await this.httpClient.get(`/tenants/${tenantId}/users/${userId}/kyc`);
    return UserKycStatusSchema.parse(response.data);
  }
};

// src/types/ledger.ts
var import_zod3 = require("zod");
var LedgerEntryType = /* @__PURE__ */ ((LedgerEntryType2) => {
  LedgerEntryType2["CREDIT"] = "CREDIT";
  LedgerEntryType2["DEBIT"] = "DEBIT";
  return LedgerEntryType2;
})(LedgerEntryType || {});
var LedgerEntrySchema = import_zod3.z.object({
  id: import_zod3.z.string(),
  tenantId: import_zod3.z.string(),
  transactionId: import_zod3.z.string(),
  type: import_zod3.z.nativeEnum(LedgerEntryType),
  amount: import_zod3.z.string(),
  currency: import_zod3.z.string(),
  balanceAfter: import_zod3.z.string(),
  referenceId: import_zod3.z.string().optional(),
  description: import_zod3.z.string().optional(),
  createdAt: import_zod3.z.string()
});
var LedgerFilterSchema = import_zod3.z.object({
  transactionId: import_zod3.z.string().optional(),
  referenceId: import_zod3.z.string().optional(),
  startDate: import_zod3.z.string().optional(),
  endDate: import_zod3.z.string().optional(),
  limit: import_zod3.z.number().optional(),
  offset: import_zod3.z.number().optional()
});

// src/services/ledger.service.ts
var LedgerService = class {
  constructor(httpClient) {
    this.httpClient = httpClient;
  }
  /**
   * Get ledger entries with filters.
   * @param filters Filter criteria
   * @returns List of Ledger Entries
   */
  async getEntries(filters) {
    const response = await this.httpClient.get("/ledger", { params: filters });
    if (Array.isArray(response.data)) {
      return response.data.map((item) => LedgerEntrySchema.parse(item));
    }
    if (response.data && Array.isArray(response.data.data)) {
      return response.data.data.map((item) => LedgerEntrySchema.parse(item));
    }
    return [];
  }
};

// src/types/aa.ts
var import_zod4 = require("zod");
var CreateAccountParamsSchema = import_zod4.z.object({
  tenantId: import_zod4.z.string(),
  userId: import_zod4.z.string(),
  ownerAddress: import_zod4.z.string()
});
var CreateAccountResponseSchema = import_zod4.z.object({
  address: import_zod4.z.string(),
  owner: import_zod4.z.string(),
  accountType: import_zod4.z.string(),
  isDeployed: import_zod4.z.boolean(),
  chainId: import_zod4.z.number(),
  entryPoint: import_zod4.z.string()
});
var GetAccountResponseSchema = import_zod4.z.object({
  address: import_zod4.z.string(),
  owner: import_zod4.z.string(),
  isDeployed: import_zod4.z.boolean(),
  chainId: import_zod4.z.number(),
  entryPoint: import_zod4.z.string(),
  accountType: import_zod4.z.string()
});
var SmartAccountSchema = GetAccountResponseSchema;
var UserOperationSchema = import_zod4.z.object({
  sender: import_zod4.z.string(),
  nonce: import_zod4.z.string(),
  initCode: import_zod4.z.string().optional(),
  callData: import_zod4.z.string(),
  callGasLimit: import_zod4.z.string(),
  verificationGasLimit: import_zod4.z.string(),
  preVerificationGas: import_zod4.z.string(),
  maxFeePerGas: import_zod4.z.string(),
  maxPriorityFeePerGas: import_zod4.z.string(),
  paymasterAndData: import_zod4.z.string().optional(),
  signature: import_zod4.z.string().optional()
});
var SendUserOperationRequestSchema = import_zod4.z.object({
  tenantId: import_zod4.z.string(),
  userOperation: UserOperationSchema,
  sponsor: import_zod4.z.boolean().optional()
});
var SendUserOperationResponseSchema = import_zod4.z.object({
  userOpHash: import_zod4.z.string(),
  sender: import_zod4.z.string(),
  nonce: import_zod4.z.string(),
  status: import_zod4.z.string(),
  sponsored: import_zod4.z.boolean()
});
var EstimateGasRequestSchema = import_zod4.z.object({
  tenantId: import_zod4.z.string(),
  userOperation: UserOperationSchema
});
var GasEstimateSchema = import_zod4.z.object({
  preVerificationGas: import_zod4.z.string(),
  verificationGasLimit: import_zod4.z.string(),
  callGasLimit: import_zod4.z.string(),
  maxFeePerGas: import_zod4.z.string(),
  maxPriorityFeePerGas: import_zod4.z.string()
});
var UserOpReceiptSchema = import_zod4.z.object({
  userOpHash: import_zod4.z.string(),
  sender: import_zod4.z.string(),
  nonce: import_zod4.z.string(),
  success: import_zod4.z.boolean(),
  actualGasCost: import_zod4.z.string(),
  actualGasUsed: import_zod4.z.string(),
  paymaster: import_zod4.z.string().optional(),
  transactionHash: import_zod4.z.string(),
  blockHash: import_zod4.z.string(),
  blockNumber: import_zod4.z.string()
});

// src/services/aa.service.ts
var AAService = class {
  constructor(httpClient) {
    this.httpClient = httpClient;
  }
  /**
   * Create a smart account for a user.
   * @param params Create Account Params
   * @returns Smart Account Info
   */
  async createSmartAccount(params) {
    const response = await this.httpClient.post(`/aa/accounts`, params);
    return CreateAccountResponseSchema.parse(response.data);
  }
  /**
   * Get smart account info for a user.
   * @param address Smart Account Address
   * @returns Smart Account Info
   */
  async getSmartAccount(address) {
    const response = await this.httpClient.get(`/aa/accounts/${address}`);
    return SmartAccountSchema.parse(response.data);
  }
  /**
   * Send a user operation.
   * @param params User Operation Params
   * @returns User Operation Receipt
   */
  async sendUserOperation(params) {
    const response = await this.httpClient.post(`/aa/user-operations`, params);
    return SendUserOperationResponseSchema.parse(response.data);
  }
  /**
   * Estimate gas for a user operation.
   * @param params User Operation Params
   * @returns Gas Estimate
   */
  async estimateGas(params) {
    const response = await this.httpClient.post(`/aa/user-operations/estimate`, params);
    return GasEstimateSchema.parse(response.data);
  }
  /**
   * Get a user operation by hash.
   */
  async getUserOperation(hash) {
    const response = await this.httpClient.get(`/aa/user-operations/${hash}`);
    return UserOperationSchema.parse(response.data);
  }
  /**
   * Get a user operation receipt by hash.
   */
  async getUserOperationReceipt(hash) {
    const response = await this.httpClient.get(`/aa/user-operations/${hash}/receipt`);
    return UserOpReceiptSchema.parse(response.data);
  }
};

// src/types/passkey.ts
var import_zod5 = require("zod");
var PasskeyCredentialSchema = import_zod5.z.object({
  credentialId: import_zod5.z.string(),
  userId: import_zod5.z.string(),
  publicKeyX: import_zod5.z.string(),
  publicKeyY: import_zod5.z.string(),
  smartAccountAddress: import_zod5.z.string().nullable().optional(),
  displayName: import_zod5.z.string(),
  isActive: import_zod5.z.boolean(),
  createdAt: import_zod5.z.string(),
  lastUsedAt: import_zod5.z.string().nullable().optional()
});
var RegisterPasskeyParamsSchema = import_zod5.z.object({
  userId: import_zod5.z.string(),
  credentialId: import_zod5.z.string(),
  publicKeyX: import_zod5.z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid P256 x coordinate"),
  publicKeyY: import_zod5.z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid P256 y coordinate"),
  displayName: import_zod5.z.string()
});
var RegisterPasskeyResponseSchema = import_zod5.z.object({
  credentialId: import_zod5.z.string(),
  smartAccountAddress: import_zod5.z.string().nullable().optional(),
  createdAt: import_zod5.z.string()
});
var CreatePasskeyWalletParamsSchema = import_zod5.z.object({
  userId: import_zod5.z.string(),
  credentialId: import_zod5.z.string(),
  publicKeyX: import_zod5.z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid P256 x coordinate"),
  publicKeyY: import_zod5.z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid P256 y coordinate"),
  displayName: import_zod5.z.string(),
  ownerAddress: import_zod5.z.string().optional(),
  salt: import_zod5.z.string().optional()
});
var CreatePasskeyWalletResponseSchema = import_zod5.z.object({
  credentialId: import_zod5.z.string(),
  smartAccountAddress: import_zod5.z.string(),
  publicKeyX: import_zod5.z.string(),
  publicKeyY: import_zod5.z.string(),
  isDeployed: import_zod5.z.boolean(),
  createdAt: import_zod5.z.string()
});
var LinkSmartAccountParamsSchema = import_zod5.z.object({
  userId: import_zod5.z.string(),
  credentialId: import_zod5.z.string(),
  smartAccountAddress: import_zod5.z.string()
});
var PasskeySignatureSchema = import_zod5.z.object({
  r: import_zod5.z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid signature r component"),
  s: import_zod5.z.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid signature s component")
});
var WebAuthnAssertionSchema = import_zod5.z.object({
  authenticatorData: import_zod5.z.string(),
  clientDataJSON: import_zod5.z.string(),
  signature: PasskeySignatureSchema,
  credentialId: import_zod5.z.string()
});
var SignTransactionParamsSchema = import_zod5.z.object({
  userId: import_zod5.z.string(),
  credentialId: import_zod5.z.string(),
  userOperation: import_zod5.z.object({
    sender: import_zod5.z.string(),
    nonce: import_zod5.z.string(),
    callData: import_zod5.z.string(),
    callGasLimit: import_zod5.z.string().optional(),
    verificationGasLimit: import_zod5.z.string().optional(),
    preVerificationGas: import_zod5.z.string().optional(),
    maxFeePerGas: import_zod5.z.string().optional(),
    maxPriorityFeePerGas: import_zod5.z.string().optional()
  }),
  assertion: WebAuthnAssertionSchema
});
var SignTransactionResponseSchema = import_zod5.z.object({
  userOpHash: import_zod5.z.string(),
  sender: import_zod5.z.string(),
  nonce: import_zod5.z.string(),
  signature: import_zod5.z.string(),
  status: import_zod5.z.string()
});
var GetCounterfactualAddressParamsSchema = import_zod5.z.object({
  publicKeyX: import_zod5.z.string(),
  publicKeyY: import_zod5.z.string(),
  salt: import_zod5.z.string().optional()
});
var GetCounterfactualAddressResponseSchema = import_zod5.z.object({
  address: import_zod5.z.string(),
  isDeployed: import_zod5.z.boolean()
});

// src/services/passkey.service.ts
var PasskeyWalletService = class {
  constructor(httpClient) {
    this.httpClient = httpClient;
  }
  // ==========================================================================
  // Wallet Lifecycle
  // ==========================================================================
  /**
   * Create a passkey wallet: registers the credential and deploys a smart
   * account with the passkey set as a signer.
   *
   * @param params - Passkey public key coordinates + user info
   * @returns Deployed smart account address and credential info
   */
  async createWallet(params) {
    const response = await this.httpClient.post("/aa/passkey/wallets", params);
    return CreatePasskeyWalletResponseSchema.parse(response.data);
  }
  /**
   * Get the counterfactual (CREATE2) address for a passkey wallet before
   * deployment. Useful for pre-funding the account.
   *
   * @param params - Passkey public key coordinates + optional salt
   * @returns The deterministic address and deployment status
   */
  async getCounterfactualAddress(params) {
    const response = await this.httpClient.post("/aa/passkey/address", params);
    return GetCounterfactualAddressResponseSchema.parse(response.data);
  }
  // ==========================================================================
  // Transaction Signing
  // ==========================================================================
  /**
   * Sign and submit an ERC-4337 UserOperation using a passkey (WebAuthn P256).
   *
   * The client is responsible for:
   * 1. Calling `navigator.credentials.get()` to obtain the WebAuthn assertion
   * 2. Extracting the P256 signature (r, s) from the assertion
   * 3. Passing the assertion data to this method
   *
   * The backend will:
   * 1. Encode the signature with the passkey type byte (0x01)
   * 2. Submit the UserOperation to the bundler
   *
   * @param params - UserOperation + WebAuthn assertion with P256 signature
   * @returns Submitted UserOperation hash and status
   */
  async signTransaction(params) {
    const response = await this.httpClient.post("/aa/passkey/sign", params);
    return SignTransactionResponseSchema.parse(response.data);
  }
  // ==========================================================================
  // Credential Management
  // ==========================================================================
  /**
   * Register a new passkey credential for a user.
   *
   * This stores the P256 public key coordinates from a WebAuthn registration
   * ceremony. The credential can later be linked to a smart account.
   *
   * @param params - Credential ID + P256 public key (x, y) + display name
   * @returns Registered credential info
   */
  async registerCredential(params) {
    const response = await this.httpClient.post("/aa/passkey/credentials", params);
    return RegisterPasskeyResponseSchema.parse(response.data);
  }
  /**
   * Get all passkey credentials for a user.
   *
   * @param userId - The user ID to fetch credentials for
   * @returns Array of passkey credentials (only active ones)
   */
  async getCredentials(userId) {
    const response = await this.httpClient.get(`/aa/passkey/credentials/${userId}`);
    return PasskeyCredentialSchema.array().parse(response.data);
  }
  /**
   * Get a specific passkey credential by credential ID.
   *
   * @param userId - The user ID
   * @param credentialId - The credential ID from WebAuthn registration
   * @returns The passkey credential
   */
  async getCredential(userId, credentialId) {
    const response = await this.httpClient.get(
      `/aa/passkey/credentials/${userId}/${credentialId}`
    );
    return PasskeyCredentialSchema.parse(response.data);
  }
  /**
   * Link a passkey credential to an existing smart account address.
   *
   * @param params - User ID + credential ID + smart account address
   */
  async linkSmartAccount(params) {
    await this.httpClient.post("/aa/passkey/link", params);
  }
  /**
   * Deactivate a passkey credential.
   *
   * The credential will no longer be usable for signing but is kept
   * for audit purposes.
   *
   * @param userId - The user ID
   * @param credentialId - The credential ID to deactivate
   */
  async deactivateCredential(userId, credentialId) {
    await this.httpClient.delete(`/aa/passkey/credentials/${userId}/${credentialId}`);
  }
};

// src/multichain/provider.ts
var import_ethers = require("ethers");

// src/types/multichain.ts
var import_zod6 = require("zod");
var ChainType = /* @__PURE__ */ ((ChainType2) => {
  ChainType2["EVM"] = "EVM";
  ChainType2["SOLANA"] = "SOLANA";
  return ChainType2;
})(ChainType || {});
var ChainConfigSchema = import_zod6.z.object({
  chainId: import_zod6.z.number(),
  name: import_zod6.z.string(),
  type: import_zod6.z.nativeEnum(ChainType),
  rpcUrl: import_zod6.z.string().url().optional(),
  explorerUrl: import_zod6.z.string().url().optional(),
  nativeCurrency: import_zod6.z.object({
    name: import_zod6.z.string(),
    symbol: import_zod6.z.string(),
    decimals: import_zod6.z.number()
  }),
  entryPointAddress: import_zod6.z.string().optional(),
  paymasterAddress: import_zod6.z.string().optional(),
  isTestnet: import_zod6.z.boolean().default(false)
});
var CrossChainIntentSchema = import_zod6.z.object({
  id: import_zod6.z.string().optional(),
  sourceChainId: import_zod6.z.number(),
  targetChainId: import_zod6.z.number(),
  type: import_zod6.z.enum(["BRIDGE", "SWAP", "TRANSFER", "MINT", "BURN"]),
  fromAddress: import_zod6.z.string(),
  toAddress: import_zod6.z.string(),
  tokenAddress: import_zod6.z.string().optional(),
  amount: import_zod6.z.string(),
  slippageTolerance: import_zod6.z.number().min(0).max(100).optional(),
  deadline: import_zod6.z.number().optional(),
  metadata: import_zod6.z.record(import_zod6.z.unknown()).optional()
});
var CrossChainIntentResponseSchema = import_zod6.z.object({
  intentId: import_zod6.z.string(),
  status: import_zod6.z.enum(["PENDING", "SUBMITTED", "BRIDGING", "COMPLETED", "FAILED"]),
  sourceChainId: import_zod6.z.number(),
  targetChainId: import_zod6.z.number(),
  sourceTxHash: import_zod6.z.string().optional(),
  targetTxHash: import_zod6.z.string().optional(),
  estimatedTime: import_zod6.z.number().optional(),
  bridgeFee: import_zod6.z.string().optional(),
  createdAt: import_zod6.z.string(),
  updatedAt: import_zod6.z.string()
});
var MultiChainAccountSchema = import_zod6.z.object({
  address: import_zod6.z.string(),
  chainId: import_zod6.z.number(),
  chainName: import_zod6.z.string(),
  accountType: import_zod6.z.enum(["EOA", "SMART_ACCOUNT", "EIP7702"]),
  isDeployed: import_zod6.z.boolean(),
  nonce: import_zod6.z.string().optional(),
  balance: import_zod6.z.string().optional()
});
var MultiChainPortfolioSchema = import_zod6.z.object({
  accounts: import_zod6.z.array(MultiChainAccountSchema),
  totalBalanceUsd: import_zod6.z.string().optional()
});
var BridgeQuoteRequestSchema = import_zod6.z.object({
  sourceChainId: import_zod6.z.number(),
  targetChainId: import_zod6.z.number(),
  tokenAddress: import_zod6.z.string(),
  amount: import_zod6.z.string(),
  fromAddress: import_zod6.z.string(),
  toAddress: import_zod6.z.string().optional()
});
var BridgeQuoteSchema = import_zod6.z.object({
  sourceChainId: import_zod6.z.number(),
  targetChainId: import_zod6.z.number(),
  inputAmount: import_zod6.z.string(),
  outputAmount: import_zod6.z.string(),
  bridgeFee: import_zod6.z.string(),
  gasFee: import_zod6.z.string(),
  estimatedTimeSeconds: import_zod6.z.number(),
  bridgeProvider: import_zod6.z.string(),
  expiresAt: import_zod6.z.string()
});
var BridgeTransactionSchema = import_zod6.z.object({
  id: import_zod6.z.string(),
  status: import_zod6.z.enum(["PENDING", "SOURCE_CONFIRMED", "BRIDGING", "TARGET_CONFIRMED", "COMPLETED", "FAILED"]),
  sourceChainId: import_zod6.z.number(),
  targetChainId: import_zod6.z.number(),
  sourceTxHash: import_zod6.z.string().optional(),
  targetTxHash: import_zod6.z.string().optional(),
  amount: import_zod6.z.string(),
  fee: import_zod6.z.string(),
  createdAt: import_zod6.z.string(),
  completedAt: import_zod6.z.string().optional()
});
var ChainTokenSchema = import_zod6.z.object({
  chainId: import_zod6.z.number(),
  address: import_zod6.z.string(),
  symbol: import_zod6.z.string(),
  name: import_zod6.z.string(),
  decimals: import_zod6.z.number(),
  logoUri: import_zod6.z.string().optional(),
  priceUsd: import_zod6.z.string().optional()
});
var Eip7702AuthorizationSchema = import_zod6.z.object({
  chainId: import_zod6.z.number(),
  delegateAddress: import_zod6.z.string(),
  nonce: import_zod6.z.number(),
  signature: import_zod6.z.string().optional()
});
var Eip7702SessionSchema = import_zod6.z.object({
  sessionId: import_zod6.z.string(),
  delegator: import_zod6.z.string(),
  delegate: import_zod6.z.string(),
  chainId: import_zod6.z.number(),
  validAfter: import_zod6.z.number(),
  validUntil: import_zod6.z.number(),
  permissions: import_zod6.z.object({
    allowedTargets: import_zod6.z.array(import_zod6.z.string()).optional(),
    maxValuePerTx: import_zod6.z.string().optional(),
    maxTotalValue: import_zod6.z.string().optional(),
    allowedSelectors: import_zod6.z.array(import_zod6.z.string()).optional()
  }).optional()
});
var DEFAULT_CHAINS = {
  [1 /* ETHEREUM */]: {
    chainId: 1 /* ETHEREUM */,
    name: "Ethereum Mainnet",
    type: "EVM" /* EVM */,
    explorerUrl: "https://etherscan.io",
    nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
    entryPointAddress: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
    isTestnet: false
  },
  [137 /* POLYGON */]: {
    chainId: 137 /* POLYGON */,
    name: "Polygon",
    type: "EVM" /* EVM */,
    explorerUrl: "https://polygonscan.com",
    nativeCurrency: { name: "MATIC", symbol: "MATIC", decimals: 18 },
    entryPointAddress: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
    isTestnet: false
  },
  [42161 /* ARBITRUM */]: {
    chainId: 42161 /* ARBITRUM */,
    name: "Arbitrum One",
    type: "EVM" /* EVM */,
    explorerUrl: "https://arbiscan.io",
    nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
    entryPointAddress: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
    isTestnet: false
  },
  [10 /* OPTIMISM */]: {
    chainId: 10 /* OPTIMISM */,
    name: "Optimism",
    type: "EVM" /* EVM */,
    explorerUrl: "https://optimistic.etherscan.io",
    nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
    entryPointAddress: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
    isTestnet: false
  },
  [8453 /* BASE */]: {
    chainId: 8453 /* BASE */,
    name: "Base",
    type: "EVM" /* EVM */,
    explorerUrl: "https://basescan.org",
    nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
    entryPointAddress: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
    isTestnet: false
  },
  [56 /* BNB_CHAIN */]: {
    chainId: 56 /* BNB_CHAIN */,
    name: "BNB Chain",
    type: "EVM" /* EVM */,
    explorerUrl: "https://bscscan.com",
    nativeCurrency: { name: "BNB", symbol: "BNB", decimals: 18 },
    entryPointAddress: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
    isTestnet: false
  },
  [43114 /* AVALANCHE */]: {
    chainId: 43114 /* AVALANCHE */,
    name: "Avalanche C-Chain",
    type: "EVM" /* EVM */,
    explorerUrl: "https://snowtrace.io",
    nativeCurrency: { name: "AVAX", symbol: "AVAX", decimals: 18 },
    entryPointAddress: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
    isTestnet: false
  },
  [101 /* SOLANA */]: {
    chainId: 101 /* SOLANA */,
    name: "Solana",
    type: "SOLANA" /* SOLANA */,
    explorerUrl: "https://solscan.io",
    nativeCurrency: { name: "SOL", symbol: "SOL", decimals: 9 },
    isTestnet: false
  }
};
function getChainConfig(chainId) {
  return DEFAULT_CHAINS[chainId];
}

// src/multichain/provider.ts
var MultichainProvider = class {
  constructor(rpcUrls) {
    __publicField(this, "providers", /* @__PURE__ */ new Map());
    __publicField(this, "currentChainId", 1 /* ETHEREUM */);
    Object.entries(rpcUrls).forEach(([chainId, url]) => {
      this.providers.set(Number(chainId), new import_ethers.JsonRpcProvider(url));
    });
  }
  /**
   * Switch the current active chain
   * @param chainId Chain ID to switch to
   */
  async switchChain(chainId) {
    const config = getChainConfig(chainId);
    if (!config) {
      throw new Error(`Unsupported chain ID: ${chainId}`);
    }
    if (!this.providers.has(chainId)) {
      if (config.rpcUrl) {
        this.providers.set(chainId, new import_ethers.JsonRpcProvider(config.rpcUrl));
      } else {
        throw new Error(`No RPC URL configured for chain ${chainId}`);
      }
    }
    this.currentChainId = chainId;
  }
  /**
   * Get native balance for an address on a specific chain
   * @param address Wallet address
   * @param chainId Optional chain ID (defaults to current)
   */
  async getBalance(address, chainId) {
    const targetChainId = chainId || this.currentChainId;
    const provider = this.getProvider(targetChainId);
    const balance = await provider.getBalance(address);
    return import_ethers.ethers.formatUnits(balance, 18);
  }
  /**
   * Get provider for a specific chain
   */
  getProvider(chainId) {
    const provider = this.providers.get(chainId);
    if (!provider) {
      const config = getChainConfig(chainId);
      if (config && config.rpcUrl) {
        const newProvider = new import_ethers.JsonRpcProvider(config.rpcUrl);
        this.providers.set(chainId, newProvider);
        return newProvider;
      }
      throw new Error(`Provider not initialized for chain ${chainId}`);
    }
    return provider;
  }
};

// src/utils/webhook.ts
var import_crypto = require("crypto");
var WebhookVerifier = class {
  /**
   * Verifies the signature of a webhook payload.
   *
   * @param payload - The raw request body as a string.
   * @param signature - The signature header sent by RampOS (e.g., X-RampOS-Signature).
   * @param secret - The webhook signing secret provided by RampOS.
   * @returns True if the signature is valid, false otherwise.
   * @throws Error if any parameter is missing.
   */
  verify(payload, signature, secret) {
    if (!payload) throw new Error("Payload is required");
    if (!signature) throw new Error("Signature is required");
    if (!secret) throw new Error("Secret is required");
    const hmac = (0, import_crypto.createHmac)("sha256", secret);
    const digest = hmac.update(payload).digest("hex");
    const expectedSignature = `sha256=${digest}`;
    const signatureBuffer = Buffer.from(signature);
    const expectedBuffer = Buffer.from(expectedSignature);
    if (signatureBuffer.length !== expectedBuffer.length) {
      return false;
    }
    return (0, import_crypto.timingSafeEqual)(signatureBuffer, expectedBuffer);
  }
};

// src/utils/crypto.ts
var import_crypto2 = require("crypto");
function signRequest(_apiKey, apiSecret, method, path, body, timestamp) {
  const message = `${method}
${path}
${timestamp}
${body}`;
  return (0, import_crypto2.createHmac)("sha256", apiSecret).update(message).digest("hex");
}

// src/utils/retry.ts
async function withRetry(fn, maxRetries = 3, baseDelay = 1e3) {
  let lastError;
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error;
      if (i < maxRetries - 1) {
        await sleep(baseDelay * Math.pow(2, i));
      }
    }
  }
  throw lastError;
}
function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// src/client.ts
var RampOSClient = class {
  constructor(config) {
    __publicField(this, "httpClient");
    __publicField(this, "intents");
    __publicField(this, "users");
    __publicField(this, "ledger");
    __publicField(this, "aa");
    __publicField(this, "passkey");
    __publicField(this, "webhooks");
    const baseURL = config.baseURL || "https://api.rampos.io/v1";
    this.httpClient = import_axios.default.create({
      baseURL,
      timeout: config.timeout || 1e4,
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${config.apiKey}`
      }
    });
    this.httpClient.interceptors.request.use((reqConfig) => {
      const timestamp = Math.floor(Date.now() / 1e3);
      const method = (reqConfig.method || "GET").toUpperCase();
      const base = reqConfig.baseURL ?? baseURL;
      const url = new URL(reqConfig.url ?? "", base);
      const path = url.pathname;
      let body = "";
      if (reqConfig.data) {
        if (typeof reqConfig.data === "string") {
          body = reqConfig.data;
        } else {
          body = JSON.stringify(reqConfig.data);
          reqConfig.data = body;
        }
      }
      const signature = signRequest(
        config.apiKey,
        config.apiSecret,
        method,
        path,
        body,
        timestamp
      );
      if (reqConfig.headers) {
        reqConfig.headers["X-Timestamp"] = timestamp.toString();
        reqConfig.headers["X-Signature"] = signature;
        if (config.tenantId) {
          reqConfig.headers["X-Tenant-ID"] = config.tenantId;
        }
      }
      return reqConfig;
    });
    const retryConfig = config.retry || { maxRetries: 3, baseDelay: 1e3 };
    const methods = ["get", "post", "put", "delete", "patch", "head", "options"];
    methods.forEach((method) => {
      const original = this.httpClient[method];
      const wrappedMethod = (url, dataOrConfig, config2) => {
        return withRetry(
          () => {
            if (method === "get" || method === "delete" || method === "head" || method === "options") {
              return original(url, dataOrConfig);
            }
            return original(url, dataOrConfig, config2);
          },
          retryConfig.maxRetries,
          retryConfig.baseDelay
        );
      };
      this.httpClient[method] = wrappedMethod;
    });
    this.intents = new IntentService(this.httpClient);
    this.users = new UserService(this.httpClient);
    this.ledger = new LedgerService(this.httpClient);
    this.aa = new AAService(this.httpClient);
    this.passkey = new PasskeyWalletService(this.httpClient);
    this.webhooks = new WebhookVerifier();
  }
  // ============================================================================
  // Multi-chain Provider Helper
  // ============================================================================
  /**
   * Initialize a MultichainProvider for direct chain interaction
   * @param rpcUrls Optional map of chain ID to RPC URL overrides
   */
  createMultichainProvider(rpcUrls = {}) {
    return new MultichainProvider(rpcUrls);
  }
};
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
  AAService,
  BalanceSchema,
  BankAccountSchema,
  ConfirmPayinRequestSchema,
  ConfirmPayinResponseSchema,
  CreateAccountParamsSchema,
  CreateAccountResponseSchema,
  CreatePasskeyWalletParamsSchema,
  CreatePasskeyWalletResponseSchema,
  CreatePayInSchema,
  CreatePayOutSchema,
  CreatePayinRequestSchema,
  CreatePayinResponseSchema,
  CreatePayoutRequestSchema,
  CreatePayoutResponseSchema,
  EstimateGasRequestSchema,
  GasEstimateSchema,
  GetAccountResponseSchema,
  GetCounterfactualAddressParamsSchema,
  GetCounterfactualAddressResponseSchema,
  IntentFilterSchema,
  IntentSchema,
  IntentService,
  IntentType,
  KycStatus,
  LedgerEntrySchema,
  LedgerEntryType,
  LedgerFilterSchema,
  LedgerService,
  LinkSmartAccountParamsSchema,
  MultichainProvider,
  PasskeyCredentialSchema,
  PasskeySignatureSchema,
  PasskeyWalletService,
  RampOSClient,
  RegisterPasskeyParamsSchema,
  RegisterPasskeyResponseSchema,
  SendUserOperationRequestSchema,
  SendUserOperationResponseSchema,
  SignTransactionParamsSchema,
  SignTransactionResponseSchema,
  SmartAccountSchema,
  StateHistoryEntrySchema,
  UserBalanceSchema,
  UserBalancesResponseSchema,
  UserKycStatusSchema,
  UserOpReceiptSchema,
  UserOperationSchema,
  UserService,
  VirtualAccountSchema,
  WebAuthnAssertionSchema,
  WebhookVerifier,
  withRetry
});
