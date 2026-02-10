var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);

// src/client.ts
import axios from "axios";

// src/types/intent.ts
import { z } from "zod";
var IntentType = /* @__PURE__ */ ((IntentType2) => {
  IntentType2["PAYIN"] = "PAYIN";
  IntentType2["PAYOUT"] = "PAYOUT";
  IntentType2["TRADE"] = "TRADE";
  return IntentType2;
})(IntentType || {});
var StateHistoryEntrySchema = z.object({
  state: z.string(),
  timestamp: z.string(),
  reason: z.string().optional()
});
var IntentSchema = z.object({
  id: z.string(),
  userId: z.string().optional(),
  intentType: z.string(),
  state: z.string(),
  amount: z.string(),
  currency: z.string(),
  actualAmount: z.string().optional(),
  referenceCode: z.string().optional(),
  bankTxId: z.string().optional(),
  chainId: z.string().optional(),
  txHash: z.string().optional(),
  stateHistory: z.array(StateHistoryEntrySchema).optional(),
  createdAt: z.string(),
  updatedAt: z.string(),
  expiresAt: z.string().optional(),
  completedAt: z.string().optional(),
  metadata: z.record(z.any()).optional()
});
var VirtualAccountSchema = z.object({
  bank: z.string(),
  accountNumber: z.string(),
  accountName: z.string()
});
var BankAccountSchema = z.object({
  bankCode: z.string(),
  accountNumber: z.string(),
  accountName: z.string()
});
var CreatePayinRequestSchema = z.object({
  tenantId: z.string(),
  userId: z.string(),
  amountVnd: z.number(),
  railsProvider: z.string(),
  metadata: z.record(z.any()).optional()
});
var CreatePayinResponseSchema = z.object({
  intentId: z.string(),
  referenceCode: z.string(),
  virtualAccount: VirtualAccountSchema.optional(),
  expiresAt: z.string(),
  status: z.string()
});
var ConfirmPayinRequestSchema = z.object({
  tenantId: z.string(),
  referenceCode: z.string(),
  status: z.string(),
  bankTxId: z.string(),
  amountVnd: z.number(),
  settledAt: z.string(),
  rawPayloadHash: z.string()
});
var ConfirmPayinResponseSchema = z.object({
  intentId: z.string(),
  status: z.string()
});
var CreatePayoutRequestSchema = z.object({
  tenantId: z.string(),
  userId: z.string(),
  amountVnd: z.number(),
  railsProvider: z.string(),
  bankAccount: BankAccountSchema,
  metadata: z.record(z.any()).optional()
});
var CreatePayoutResponseSchema = z.object({
  intentId: z.string(),
  status: z.string()
});
var CreatePayInSchema = CreatePayinRequestSchema;
var CreatePayOutSchema = CreatePayoutRequestSchema;
var IntentFilterSchema = z.object({
  userId: z.string().optional(),
  intentType: z.string().optional(),
  state: z.string().optional(),
  limit: z.number().optional(),
  offset: z.number().optional()
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
import { z as z2 } from "zod";
var BalanceSchema = z2.object({
  accountType: z2.string(),
  currency: z2.string(),
  balance: z2.string()
});
var UserBalancesResponseSchema = z2.object({
  balances: z2.array(BalanceSchema)
});
var UserBalanceSchema = BalanceSchema;
var KycStatus = /* @__PURE__ */ ((KycStatus2) => {
  KycStatus2["NONE"] = "NONE";
  KycStatus2["PENDING"] = "PENDING";
  KycStatus2["VERIFIED"] = "VERIFIED";
  KycStatus2["REJECTED"] = "REJECTED";
  return KycStatus2;
})(KycStatus || {});
var UserKycStatusSchema = z2.object({
  userId: z2.string(),
  status: z2.nativeEnum(KycStatus),
  updatedAt: z2.string()
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
import { z as z3 } from "zod";
var LedgerEntryType = /* @__PURE__ */ ((LedgerEntryType2) => {
  LedgerEntryType2["CREDIT"] = "CREDIT";
  LedgerEntryType2["DEBIT"] = "DEBIT";
  return LedgerEntryType2;
})(LedgerEntryType || {});
var LedgerEntrySchema = z3.object({
  id: z3.string(),
  tenantId: z3.string(),
  transactionId: z3.string(),
  type: z3.nativeEnum(LedgerEntryType),
  amount: z3.string(),
  currency: z3.string(),
  balanceAfter: z3.string(),
  referenceId: z3.string().optional(),
  description: z3.string().optional(),
  createdAt: z3.string()
});
var LedgerFilterSchema = z3.object({
  transactionId: z3.string().optional(),
  referenceId: z3.string().optional(),
  startDate: z3.string().optional(),
  endDate: z3.string().optional(),
  limit: z3.number().optional(),
  offset: z3.number().optional()
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
import { z as z4 } from "zod";
var CreateAccountParamsSchema = z4.object({
  tenantId: z4.string(),
  userId: z4.string(),
  ownerAddress: z4.string()
});
var CreateAccountResponseSchema = z4.object({
  address: z4.string(),
  owner: z4.string(),
  accountType: z4.string(),
  isDeployed: z4.boolean(),
  chainId: z4.number(),
  entryPoint: z4.string()
});
var GetAccountResponseSchema = z4.object({
  address: z4.string(),
  owner: z4.string(),
  isDeployed: z4.boolean(),
  chainId: z4.number(),
  entryPoint: z4.string(),
  accountType: z4.string()
});
var SmartAccountSchema = GetAccountResponseSchema;
var UserOperationSchema = z4.object({
  sender: z4.string(),
  nonce: z4.string(),
  initCode: z4.string().optional(),
  callData: z4.string(),
  callGasLimit: z4.string(),
  verificationGasLimit: z4.string(),
  preVerificationGas: z4.string(),
  maxFeePerGas: z4.string(),
  maxPriorityFeePerGas: z4.string(),
  paymasterAndData: z4.string().optional(),
  signature: z4.string().optional()
});
var SendUserOperationRequestSchema = z4.object({
  tenantId: z4.string(),
  userOperation: UserOperationSchema,
  sponsor: z4.boolean().optional()
});
var SendUserOperationResponseSchema = z4.object({
  userOpHash: z4.string(),
  sender: z4.string(),
  nonce: z4.string(),
  status: z4.string(),
  sponsored: z4.boolean()
});
var EstimateGasRequestSchema = z4.object({
  tenantId: z4.string(),
  userOperation: UserOperationSchema
});
var GasEstimateSchema = z4.object({
  preVerificationGas: z4.string(),
  verificationGasLimit: z4.string(),
  callGasLimit: z4.string(),
  maxFeePerGas: z4.string(),
  maxPriorityFeePerGas: z4.string()
});
var UserOpReceiptSchema = z4.object({
  userOpHash: z4.string(),
  sender: z4.string(),
  nonce: z4.string(),
  success: z4.boolean(),
  actualGasCost: z4.string(),
  actualGasUsed: z4.string(),
  paymaster: z4.string().optional(),
  transactionHash: z4.string(),
  blockHash: z4.string(),
  blockNumber: z4.string()
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
import { z as z5 } from "zod";
var PasskeyCredentialSchema = z5.object({
  credentialId: z5.string(),
  userId: z5.string(),
  publicKeyX: z5.string(),
  publicKeyY: z5.string(),
  smartAccountAddress: z5.string().nullable().optional(),
  displayName: z5.string(),
  isActive: z5.boolean(),
  createdAt: z5.string(),
  lastUsedAt: z5.string().nullable().optional()
});
var RegisterPasskeyParamsSchema = z5.object({
  userId: z5.string(),
  credentialId: z5.string(),
  publicKeyX: z5.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid P256 x coordinate"),
  publicKeyY: z5.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid P256 y coordinate"),
  displayName: z5.string()
});
var RegisterPasskeyResponseSchema = z5.object({
  credentialId: z5.string(),
  smartAccountAddress: z5.string().nullable().optional(),
  createdAt: z5.string()
});
var CreatePasskeyWalletParamsSchema = z5.object({
  userId: z5.string(),
  credentialId: z5.string(),
  publicKeyX: z5.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid P256 x coordinate"),
  publicKeyY: z5.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid P256 y coordinate"),
  displayName: z5.string(),
  ownerAddress: z5.string().optional(),
  salt: z5.string().optional()
});
var CreatePasskeyWalletResponseSchema = z5.object({
  credentialId: z5.string(),
  smartAccountAddress: z5.string(),
  publicKeyX: z5.string(),
  publicKeyY: z5.string(),
  isDeployed: z5.boolean(),
  createdAt: z5.string()
});
var LinkSmartAccountParamsSchema = z5.object({
  userId: z5.string(),
  credentialId: z5.string(),
  smartAccountAddress: z5.string()
});
var PasskeySignatureSchema = z5.object({
  r: z5.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid signature r component"),
  s: z5.string().regex(/^(0x)?[0-9a-fA-F]{1,64}$/, "Invalid signature s component")
});
var WebAuthnAssertionSchema = z5.object({
  authenticatorData: z5.string(),
  clientDataJSON: z5.string(),
  signature: PasskeySignatureSchema,
  credentialId: z5.string()
});
var SignTransactionParamsSchema = z5.object({
  userId: z5.string(),
  credentialId: z5.string(),
  userOperation: z5.object({
    sender: z5.string(),
    nonce: z5.string(),
    callData: z5.string(),
    callGasLimit: z5.string().optional(),
    verificationGasLimit: z5.string().optional(),
    preVerificationGas: z5.string().optional(),
    maxFeePerGas: z5.string().optional(),
    maxPriorityFeePerGas: z5.string().optional()
  }),
  assertion: WebAuthnAssertionSchema
});
var SignTransactionResponseSchema = z5.object({
  userOpHash: z5.string(),
  sender: z5.string(),
  nonce: z5.string(),
  signature: z5.string(),
  status: z5.string()
});
var GetCounterfactualAddressParamsSchema = z5.object({
  publicKeyX: z5.string(),
  publicKeyY: z5.string(),
  salt: z5.string().optional()
});
var GetCounterfactualAddressResponseSchema = z5.object({
  address: z5.string(),
  isDeployed: z5.boolean()
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
import { JsonRpcProvider, ethers } from "ethers";

// src/types/multichain.ts
import { z as z6 } from "zod";
var ChainType = /* @__PURE__ */ ((ChainType2) => {
  ChainType2["EVM"] = "EVM";
  ChainType2["SOLANA"] = "SOLANA";
  return ChainType2;
})(ChainType || {});
var ChainConfigSchema = z6.object({
  chainId: z6.number(),
  name: z6.string(),
  type: z6.nativeEnum(ChainType),
  rpcUrl: z6.string().url().optional(),
  explorerUrl: z6.string().url().optional(),
  nativeCurrency: z6.object({
    name: z6.string(),
    symbol: z6.string(),
    decimals: z6.number()
  }),
  entryPointAddress: z6.string().optional(),
  paymasterAddress: z6.string().optional(),
  isTestnet: z6.boolean().default(false)
});
var CrossChainIntentSchema = z6.object({
  id: z6.string().optional(),
  sourceChainId: z6.number(),
  targetChainId: z6.number(),
  type: z6.enum(["BRIDGE", "SWAP", "TRANSFER", "MINT", "BURN"]),
  fromAddress: z6.string(),
  toAddress: z6.string(),
  tokenAddress: z6.string().optional(),
  amount: z6.string(),
  slippageTolerance: z6.number().min(0).max(100).optional(),
  deadline: z6.number().optional(),
  metadata: z6.record(z6.unknown()).optional()
});
var CrossChainIntentResponseSchema = z6.object({
  intentId: z6.string(),
  status: z6.enum(["PENDING", "SUBMITTED", "BRIDGING", "COMPLETED", "FAILED"]),
  sourceChainId: z6.number(),
  targetChainId: z6.number(),
  sourceTxHash: z6.string().optional(),
  targetTxHash: z6.string().optional(),
  estimatedTime: z6.number().optional(),
  bridgeFee: z6.string().optional(),
  createdAt: z6.string(),
  updatedAt: z6.string()
});
var MultiChainAccountSchema = z6.object({
  address: z6.string(),
  chainId: z6.number(),
  chainName: z6.string(),
  accountType: z6.enum(["EOA", "SMART_ACCOUNT", "EIP7702"]),
  isDeployed: z6.boolean(),
  nonce: z6.string().optional(),
  balance: z6.string().optional()
});
var MultiChainPortfolioSchema = z6.object({
  accounts: z6.array(MultiChainAccountSchema),
  totalBalanceUsd: z6.string().optional()
});
var BridgeQuoteRequestSchema = z6.object({
  sourceChainId: z6.number(),
  targetChainId: z6.number(),
  tokenAddress: z6.string(),
  amount: z6.string(),
  fromAddress: z6.string(),
  toAddress: z6.string().optional()
});
var BridgeQuoteSchema = z6.object({
  sourceChainId: z6.number(),
  targetChainId: z6.number(),
  inputAmount: z6.string(),
  outputAmount: z6.string(),
  bridgeFee: z6.string(),
  gasFee: z6.string(),
  estimatedTimeSeconds: z6.number(),
  bridgeProvider: z6.string(),
  expiresAt: z6.string()
});
var BridgeTransactionSchema = z6.object({
  id: z6.string(),
  status: z6.enum(["PENDING", "SOURCE_CONFIRMED", "BRIDGING", "TARGET_CONFIRMED", "COMPLETED", "FAILED"]),
  sourceChainId: z6.number(),
  targetChainId: z6.number(),
  sourceTxHash: z6.string().optional(),
  targetTxHash: z6.string().optional(),
  amount: z6.string(),
  fee: z6.string(),
  createdAt: z6.string(),
  completedAt: z6.string().optional()
});
var ChainTokenSchema = z6.object({
  chainId: z6.number(),
  address: z6.string(),
  symbol: z6.string(),
  name: z6.string(),
  decimals: z6.number(),
  logoUri: z6.string().optional(),
  priceUsd: z6.string().optional()
});
var Eip7702AuthorizationSchema = z6.object({
  chainId: z6.number(),
  delegateAddress: z6.string(),
  nonce: z6.number(),
  signature: z6.string().optional()
});
var Eip7702SessionSchema = z6.object({
  sessionId: z6.string(),
  delegator: z6.string(),
  delegate: z6.string(),
  chainId: z6.number(),
  validAfter: z6.number(),
  validUntil: z6.number(),
  permissions: z6.object({
    allowedTargets: z6.array(z6.string()).optional(),
    maxValuePerTx: z6.string().optional(),
    maxTotalValue: z6.string().optional(),
    allowedSelectors: z6.array(z6.string()).optional()
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
      this.providers.set(Number(chainId), new JsonRpcProvider(url));
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
        this.providers.set(chainId, new JsonRpcProvider(config.rpcUrl));
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
    return ethers.formatUnits(balance, 18);
  }
  /**
   * Get provider for a specific chain
   */
  getProvider(chainId) {
    const provider = this.providers.get(chainId);
    if (!provider) {
      const config = getChainConfig(chainId);
      if (config && config.rpcUrl) {
        const newProvider = new JsonRpcProvider(config.rpcUrl);
        this.providers.set(chainId, newProvider);
        return newProvider;
      }
      throw new Error(`Provider not initialized for chain ${chainId}`);
    }
    return provider;
  }
};

// src/utils/webhook.ts
import { createHmac, timingSafeEqual } from "crypto";
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
    const hmac = createHmac("sha256", secret);
    const digest = hmac.update(payload).digest("hex");
    const expectedSignature = `sha256=${digest}`;
    const signatureBuffer = Buffer.from(signature);
    const expectedBuffer = Buffer.from(expectedSignature);
    if (signatureBuffer.length !== expectedBuffer.length) {
      return false;
    }
    return timingSafeEqual(signatureBuffer, expectedBuffer);
  }
};

// src/utils/crypto.ts
import { createHmac as createHmac2 } from "crypto";
function signRequest(_apiKey, apiSecret, method, path, body, timestamp) {
  const message = `${method}
${path}
${timestamp}
${body}`;
  return createHmac2("sha256", apiSecret).update(message).digest("hex");
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
    this.httpClient = axios.create({
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
export {
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
};
