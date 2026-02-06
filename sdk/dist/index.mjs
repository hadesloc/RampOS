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
var SessionKeySchema = z4.object({
  id: z4.string().optional(),
  publicKey: z4.string(),
  permissions: z4.array(z4.string()),
  validUntil: z4.number(),
  validAfter: z4.number().optional()
});
var AddSessionKeyParamsSchema = z4.object({
  accountAddress: z4.string(),
  sessionKey: SessionKeySchema
});
var RemoveSessionKeyParamsSchema = z4.object({
  accountAddress: z4.string(),
  keyId: z4.string()
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
   * Add a session key to an account.
   * @param params Add Session Key Params
   * @returns Void (throws on error)
   */
  async addSessionKey(params) {
    void params;
    throw new Error("Session key management is not exposed via the API");
  }
  /**
   * Remove a session key from an account.
   * @param params Remove Session Key Params
   * @returns Void (throws on error)
   */
  async removeSessionKey(params) {
    void params;
    throw new Error("Session key management is not exposed via the API");
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
    this.webhooks = new WebhookVerifier();
  }
};
export {
  AAService,
  AddSessionKeyParamsSchema,
  BalanceSchema,
  BankAccountSchema,
  ConfirmPayinRequestSchema,
  ConfirmPayinResponseSchema,
  CreateAccountParamsSchema,
  CreateAccountResponseSchema,
  CreatePayInSchema,
  CreatePayOutSchema,
  CreatePayinRequestSchema,
  CreatePayinResponseSchema,
  CreatePayoutRequestSchema,
  CreatePayoutResponseSchema,
  EstimateGasRequestSchema,
  GasEstimateSchema,
  GetAccountResponseSchema,
  IntentFilterSchema,
  IntentSchema,
  IntentService,
  IntentType,
  KycStatus,
  LedgerEntrySchema,
  LedgerEntryType,
  LedgerFilterSchema,
  LedgerService,
  RampOSClient,
  RemoveSessionKeyParamsSchema,
  SendUserOperationRequestSchema,
  SendUserOperationResponseSchema,
  SessionKeySchema,
  SmartAccountSchema,
  StateHistoryEntrySchema,
  UserBalanceSchema,
  UserBalancesResponseSchema,
  UserKycStatusSchema,
  UserOpReceiptSchema,
  UserOperationSchema,
  UserService,
  VirtualAccountSchema,
  WebhookVerifier,
  withRetry
};
