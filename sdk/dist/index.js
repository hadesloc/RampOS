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
  AddSessionKeyParamsSchema: () => AddSessionKeyParamsSchema,
  CreateAccountParamsSchema: () => CreateAccountParamsSchema,
  CreatePayInSchema: () => CreatePayInSchema,
  CreatePayOutSchema: () => CreatePayOutSchema,
  GasEstimateSchema: () => GasEstimateSchema,
  IntentFilterSchema: () => IntentFilterSchema,
  IntentSchema: () => IntentSchema,
  IntentService: () => IntentService,
  IntentStatus: () => IntentStatus,
  IntentType: () => IntentType,
  KycStatus: () => KycStatus,
  LedgerEntrySchema: () => LedgerEntrySchema,
  LedgerEntryType: () => LedgerEntryType,
  LedgerFilterSchema: () => LedgerFilterSchema,
  LedgerService: () => LedgerService,
  RampOSClient: () => RampOSClient,
  RemoveSessionKeyParamsSchema: () => RemoveSessionKeyParamsSchema,
  SessionKeySchema: () => SessionKeySchema,
  SmartAccountSchema: () => SmartAccountSchema,
  UserBalanceSchema: () => UserBalanceSchema,
  UserKycStatusSchema: () => UserKycStatusSchema,
  UserOpReceiptSchema: () => UserOpReceiptSchema,
  UserOperationParamsSchema: () => UserOperationParamsSchema,
  UserOperationSchema: () => UserOperationSchema,
  UserService: () => UserService,
  WebhookVerifier: () => WebhookVerifier
});
module.exports = __toCommonJS(index_exports);

// src/client.ts
var import_axios = __toESM(require("axios"));

// src/types/intent.ts
var import_zod = require("zod");
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
var IntentSchema = import_zod.z.object({
  id: import_zod.z.string(),
  tenantId: import_zod.z.string(),
  type: import_zod.z.nativeEnum(IntentType),
  status: import_zod.z.nativeEnum(IntentStatus),
  amount: import_zod.z.string(),
  currency: import_zod.z.string(),
  bankAccount: import_zod.z.string().optional(),
  bankRef: import_zod.z.string().optional(),
  metadata: import_zod.z.record(import_zod.z.any()).optional(),
  createdAt: import_zod.z.string(),
  updatedAt: import_zod.z.string()
});
var CreatePayInSchema = import_zod.z.object({
  amount: import_zod.z.string(),
  currency: import_zod.z.string(),
  metadata: import_zod.z.record(import_zod.z.any()).optional()
});
var CreatePayOutSchema = import_zod.z.object({
  amount: import_zod.z.string(),
  currency: import_zod.z.string(),
  bankAccount: import_zod.z.string(),
  metadata: import_zod.z.record(import_zod.z.any()).optional()
});
var IntentFilterSchema = import_zod.z.object({
  type: import_zod.z.nativeEnum(IntentType).optional(),
  status: import_zod.z.nativeEnum(IntentStatus).optional(),
  startDate: import_zod.z.string().optional(),
  endDate: import_zod.z.string().optional(),
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
var import_zod2 = require("zod");
var UserBalanceSchema = import_zod2.z.object({
  currency: import_zod2.z.string(),
  amount: import_zod2.z.string(),
  locked: import_zod2.z.string()
});
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
var SmartAccountSchema = import_zod4.z.object({
  address: import_zod4.z.string(),
  owner: import_zod4.z.string(),
  factoryAddress: import_zod4.z.string(),
  deployed: import_zod4.z.boolean(),
  balance: import_zod4.z.string().optional()
});
var CreateAccountParamsSchema = import_zod4.z.object({
  owner: import_zod4.z.string(),
  salt: import_zod4.z.string().optional()
});
var SessionKeySchema = import_zod4.z.object({
  id: import_zod4.z.string().optional(),
  // ID might be assigned by backend
  publicKey: import_zod4.z.string(),
  permissions: import_zod4.z.array(import_zod4.z.string()),
  validUntil: import_zod4.z.number(),
  // timestamp
  validAfter: import_zod4.z.number().optional()
  // timestamp
});
var AddSessionKeyParamsSchema = import_zod4.z.object({
  accountAddress: import_zod4.z.string(),
  sessionKey: SessionKeySchema
});
var RemoveSessionKeyParamsSchema = import_zod4.z.object({
  accountAddress: import_zod4.z.string(),
  keyId: import_zod4.z.string()
});
var UserOperationSchema = import_zod4.z.object({
  sender: import_zod4.z.string(),
  nonce: import_zod4.z.string(),
  initCode: import_zod4.z.string(),
  callData: import_zod4.z.string(),
  callGasLimit: import_zod4.z.string(),
  verificationGasLimit: import_zod4.z.string(),
  preVerificationGas: import_zod4.z.string(),
  maxFeePerGas: import_zod4.z.string(),
  maxPriorityFeePerGas: import_zod4.z.string(),
  paymasterAndData: import_zod4.z.string(),
  signature: import_zod4.z.string()
});
var UserOperationParamsSchema = import_zod4.z.object({
  target: import_zod4.z.string(),
  value: import_zod4.z.string().default("0"),
  data: import_zod4.z.string().default("0x"),
  sponsored: import_zod4.z.boolean().optional(),
  accountAddress: import_zod4.z.string().optional()
  // If not inferred from client context
});
var GasEstimateSchema = import_zod4.z.object({
  preVerificationGas: import_zod4.z.string(),
  verificationGas: import_zod4.z.string(),
  callGasLimit: import_zod4.z.string(),
  total: import_zod4.z.string().optional()
});
var UserOpReceiptSchema = import_zod4.z.object({
  userOpHash: import_zod4.z.string(),
  txHash: import_zod4.z.string().optional(),
  success: import_zod4.z.boolean().optional()
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
    this.httpClient = import_axios.default.create({
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
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
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
});
