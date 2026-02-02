var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);

// src/client.ts
import axios from "axios";

// src/types/intent.ts
import { z } from "zod";
var IntentType = /* @__PURE__ */ ((IntentType2) => {
  IntentType2["PAY_IN"] = "PAY_IN";
  IntentType2["PAY_OUT"] = "PAY_OUT";
  IntentType2["TRADE"] = "TRADE";
  return IntentType2;
})(IntentType || {});
var IntentStatus = /* @__PURE__ */ ((IntentStatus2) => {
  IntentStatus2["CREATED"] = "CREATED";
  IntentStatus2["PENDING"] = "PENDING";
  IntentStatus2["COMPLETED"] = "COMPLETED";
  IntentStatus2["FAILED"] = "FAILED";
  IntentStatus2["CANCELLED"] = "CANCELLED";
  return IntentStatus2;
})(IntentStatus || {});
var IntentSchema = z.object({
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
  updatedAt: z.string()
});
var CreatePayInSchema = z.object({
  amount: z.string(),
  currency: z.string(),
  metadata: z.record(z.any()).optional()
});
var CreatePayOutSchema = z.object({
  amount: z.string(),
  currency: z.string(),
  bankAccount: z.string(),
  metadata: z.record(z.any()).optional()
});
var IntentFilterSchema = z.object({
  type: z.nativeEnum(IntentType).optional(),
  status: z.nativeEnum(IntentStatus).optional(),
  startDate: z.string().optional(),
  endDate: z.string().optional(),
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
    const response = await this.httpClient.post("/intents/pay-in", data);
    return IntentSchema.parse(response.data);
  }
  /**
   * Confirm a Pay-In intent.
   * @param id Intent ID
   * @param bankRef Bank Reference Code
   * @returns Updated Intent
   */
  async confirmPayIn(id, bankRef) {
    const response = await this.httpClient.post(`/intents/${id}/confirm`, { bankRef });
    return IntentSchema.parse(response.data);
  }
  /**
   * Create a new Pay-Out intent.
   * @param data Pay-Out data
   * @returns Created Intent
   */
  async createPayOut(data) {
    const response = await this.httpClient.post("/intents/pay-out", data);
    return IntentSchema.parse(response.data);
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
var UserBalanceSchema = z2.object({
  currency: z2.string(),
  amount: z2.string(),
  locked: z2.string()
});
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
   * @param tenantId Tenant ID
   * @param userId User ID
   * @returns List of User Balances
   */
  async getBalances(tenantId, userId) {
    const response = await this.httpClient.get(`/tenants/${tenantId}/users/${userId}/balances`);
    if (Array.isArray(response.data)) {
      return response.data.map((item) => UserBalanceSchema.parse(item));
    }
    return [];
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
var SmartAccountSchema = z4.object({
  address: z4.string(),
  owner: z4.string(),
  factoryAddress: z4.string(),
  deployed: z4.boolean(),
  balance: z4.string().optional()
});
var CreateAccountParamsSchema = z4.object({
  owner: z4.string(),
  salt: z4.string().optional()
});
var SessionKeySchema = z4.object({
  id: z4.string().optional(),
  // ID might be assigned by backend
  publicKey: z4.string(),
  permissions: z4.array(z4.string()),
  validUntil: z4.number(),
  // timestamp
  validAfter: z4.number().optional()
  // timestamp
});
var AddSessionKeyParamsSchema = z4.object({
  accountAddress: z4.string(),
  sessionKey: SessionKeySchema
});
var RemoveSessionKeyParamsSchema = z4.object({
  accountAddress: z4.string(),
  keyId: z4.string()
});
var UserOperationSchema = z4.object({
  sender: z4.string(),
  nonce: z4.string(),
  initCode: z4.string(),
  callData: z4.string(),
  callGasLimit: z4.string(),
  verificationGasLimit: z4.string(),
  preVerificationGas: z4.string(),
  maxFeePerGas: z4.string(),
  maxPriorityFeePerGas: z4.string(),
  paymasterAndData: z4.string(),
  signature: z4.string()
});
var UserOperationParamsSchema = z4.object({
  target: z4.string(),
  value: z4.string().default("0"),
  data: z4.string().default("0x"),
  sponsored: z4.boolean().optional(),
  accountAddress: z4.string().optional()
  // If not inferred from client context
});
var GasEstimateSchema = z4.object({
  preVerificationGas: z4.string(),
  verificationGas: z4.string(),
  callGasLimit: z4.string(),
  total: z4.string().optional()
});
var UserOpReceiptSchema = z4.object({
  userOpHash: z4.string(),
  txHash: z4.string().optional(),
  success: z4.boolean().optional()
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
    return SmartAccountSchema.parse(response.data);
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
    await this.httpClient.post(`/aa/accounts/${params.accountAddress}/sessions`, params.sessionKey);
  }
  /**
   * Remove a session key from an account.
   * @param params Remove Session Key Params
   * @returns Void (throws on error)
   */
  async removeSessionKey(params) {
    await this.httpClient.delete(`/aa/accounts/${params.accountAddress}/sessions/${params.keyId}`);
  }
  /**
   * Send a user operation.
   * @param params User Operation Params
   * @returns User Operation Receipt
   */
  async sendUserOperation(params) {
    const response = await this.httpClient.post(`/aa/bundler/user-op`, params);
    return UserOpReceiptSchema.parse(response.data);
  }
  /**
   * Estimate gas for a user operation.
   * @param params User Operation Params
   * @returns Gas Estimate
   */
  async estimateGas(params) {
    const response = await this.httpClient.post(`/aa/bundler/estimate-gas`, params);
    return GasEstimateSchema.parse(response.data);
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

// src/client.ts
var RampOSClient = class {
  constructor(options) {
    __publicField(this, "httpClient");
    __publicField(this, "intents");
    __publicField(this, "users");
    __publicField(this, "ledger");
    __publicField(this, "aa");
    __publicField(this, "webhooks");
    const baseURL = options.baseURL || "https://api.rampos.io/v1";
    this.httpClient = axios.create({
      baseURL,
      timeout: options.timeout || 1e4,
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${options.apiKey}`
      }
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
  CreateAccountParamsSchema,
  CreatePayInSchema,
  CreatePayOutSchema,
  GasEstimateSchema,
  IntentFilterSchema,
  IntentSchema,
  IntentService,
  IntentStatus,
  IntentType,
  KycStatus,
  LedgerEntrySchema,
  LedgerEntryType,
  LedgerFilterSchema,
  LedgerService,
  RampOSClient,
  RemoveSessionKeyParamsSchema,
  SessionKeySchema,
  SmartAccountSchema,
  UserBalanceSchema,
  UserKycStatusSchema,
  UserOpReceiptSchema,
  UserOperationParamsSchema,
  UserOperationSchema,
  UserService,
  WebhookVerifier
};
